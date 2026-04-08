use bevy::prelude::*;
use bevy::ui::{AlignItems, BackgroundColor, JustifyContent, Node, PositionType, Val};

use crate::game_state::GameState;
use crate::scenes::game_menu::close_game_menu;
use game_engine::input_bindings::{InputAction, InputBindings};
use game_engine::status::CharacterStatsSnapshot;

const LOGOUT_DELAY_SECS: f32 = 20.0;
const LOGOUT_CANCEL_ACTIONS: [InputAction; 8] = [
    InputAction::MoveForward,
    InputAction::MoveBackward,
    InputAction::StrafeLeft,
    InputAction::StrafeRight,
    InputAction::Jump,
    InputAction::AutoRun,
    InputAction::TurnLeft,
    InputAction::TurnRight,
];

#[derive(Resource, Debug, Default)]
pub struct LogoutState {
    pending: Option<Timer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogoutRequestOutcome {
    Immediate,
    StartedCountdown,
    AlreadyPending,
    BlockedInCombat,
}

#[derive(Component)]
struct LogoutOverlayRoot;

#[derive(Component)]
struct LogoutOverlayText;

pub struct LogoutPlugin;

impl Plugin for LogoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogoutState>();
        app.add_systems(Startup, spawn_logout_overlay);
        app.add_systems(OnExit(GameState::InWorld), clear_logout_state);
        app.add_systems(
            Update,
            (
                tick_logout_countdown,
                cancel_logout_on_input,
                sync_logout_overlay,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

pub struct RequestLogoutCommand;

impl Command for RequestLogoutCommand {
    fn apply(self, world: &mut World) {
        let snapshot = world.resource::<CharacterStatsSnapshot>().clone();
        let outcome = {
            let mut logout = world.resource_mut::<LogoutState>();
            request_logout(&mut logout, &snapshot)
        };
        handle_logout_request_outcome(world, outcome);
    }
}

fn request_logout(
    logout: &mut LogoutState,
    snapshot: &CharacterStatsSnapshot,
) -> LogoutRequestOutcome {
    if snapshot.in_combat {
        return LogoutRequestOutcome::BlockedInCombat;
    }
    if snapshot.in_rest_area {
        logout.pending = None;
        return LogoutRequestOutcome::Immediate;
    }
    if logout.pending.is_some() {
        return LogoutRequestOutcome::AlreadyPending;
    }
    logout.pending = Some(Timer::from_seconds(LOGOUT_DELAY_SECS, TimerMode::Once));
    LogoutRequestOutcome::StartedCountdown
}

fn handle_logout_request_outcome(world: &mut World, outcome: LogoutRequestOutcome) {
    match outcome {
        LogoutRequestOutcome::Immediate => {
            close_game_menu(&mut world.commands());
            world
                .resource_mut::<NextState<GameState>>()
                .set(GameState::Login);
        }
        LogoutRequestOutcome::StartedCountdown | LogoutRequestOutcome::AlreadyPending => {
            close_game_menu(&mut world.commands());
        }
        LogoutRequestOutcome::BlockedInCombat => {
            warn!("Cannot logout while in combat");
        }
    }
}

fn spawn_logout_overlay(mut commands: Commands) {
    commands
        .spawn((
            LogoutOverlayRoot,
            Visibility::Hidden,
            BackgroundColor(Color::srgba(0.03, 0.02, 0.01, 0.9)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom: Val::Px(120.0),
                width: Val::Px(340.0),
                margin: UiRect::left(Val::Px(-170.0)),
                padding: UiRect::axes(Val::Px(18.0), Val::Px(14.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                LogoutOverlayText,
                Text::new("Logging out"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.82, 0.52)),
            ));
        });
}

fn tick_logout_countdown(
    time: Res<Time>,
    mut logout: ResMut<LogoutState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(timer) = logout.pending.as_mut() else {
        return;
    };
    timer.tick(time.delta());
    if !timer.is_finished() {
        return;
    }
    logout.pending = None;
    next_state.set(GameState::Login);
}

fn cancel_logout_on_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<InputBindings>,
    mut logout: ResMut<LogoutState>,
) {
    if logout.pending.is_none() || !logout_cancelled_by_input(&bindings, &keys, &mouse_buttons) {
        return;
    }
    logout.pending = None;
    info!("Logout cancelled by movement input");
}

fn logout_cancelled_by_input(
    bindings: &InputBindings,
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
) -> bool {
    LOGOUT_CANCEL_ACTIONS
        .into_iter()
        .any(|action| bindings.is_pressed(action, keys, mouse_buttons))
}

fn sync_logout_overlay(
    logout: Res<LogoutState>,
    mut visibility_q: Query<&mut Visibility, With<LogoutOverlayRoot>>,
    mut text_q: Query<&mut Text, With<LogoutOverlayText>>,
) {
    let visibility = if logout.pending.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut current in &mut visibility_q {
        *current = visibility;
    }
    let text = format_logout_overlay_text(&logout);
    for mut current in &mut text_q {
        **current = text.clone();
    }
}

fn format_logout_overlay_text(logout: &LogoutState) -> String {
    let Some(timer) = logout.pending.as_ref() else {
        return String::new();
    };
    let seconds = timer.remaining_secs().ceil().max(1.0) as u32;
    format!("Logging out in {seconds}s\nMove to cancel")
}

fn clear_logout_state(mut logout: ResMut<LogoutState>) {
    logout.pending = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::KeyCode;
    use std::time::Duration;

    #[test]
    fn in_rest_area_logs_out_immediately() {
        let mut logout = LogoutState::default();
        let snapshot = CharacterStatsSnapshot {
            in_rest_area: true,
            ..Default::default()
        };

        let outcome = request_logout(&mut logout, &snapshot);

        assert_eq!(outcome, LogoutRequestOutcome::Immediate);
        assert!(logout.pending.is_none());
    }

    #[test]
    fn in_combat_blocks_logout_request() {
        let mut logout = LogoutState::default();
        let snapshot = CharacterStatsSnapshot {
            in_combat: true,
            ..Default::default()
        };

        let outcome = request_logout(&mut logout, &snapshot);

        assert_eq!(outcome, LogoutRequestOutcome::BlockedInCombat);
        assert!(logout.pending.is_none());
    }

    #[test]
    fn open_world_starts_twenty_second_countdown() {
        let mut logout = LogoutState::default();
        let snapshot = CharacterStatsSnapshot::default();

        let outcome = request_logout(&mut logout, &snapshot);

        assert_eq!(outcome, LogoutRequestOutcome::StartedCountdown);
        let timer = logout.pending.as_ref().expect("countdown started");
        assert_eq!(timer.duration(), Duration::from_secs(20));
    }

    #[test]
    fn movement_bindings_cancel_pending_logout() {
        let bindings = InputBindings::default();
        let mut keys = ButtonInput::default();
        let mouse_buttons = ButtonInput::default();
        keys.press(KeyCode::KeyW);

        assert!(logout_cancelled_by_input(&bindings, &keys, &mouse_buttons));
    }

    #[test]
    fn overlay_text_uses_remaining_seconds() {
        let mut logout = LogoutState {
            pending: Some(Timer::from_seconds(LOGOUT_DELAY_SECS, TimerMode::Once)),
        };
        logout
            .pending
            .as_mut()
            .expect("timer")
            .tick(Duration::from_secs_f32(2.3));

        assert_eq!(
            format_logout_overlay_text(&logout),
            "Logging out in 18s\nMove to cancel"
        );
    }
}
