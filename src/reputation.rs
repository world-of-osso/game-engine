use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;
use bevy::ui::{AlignItems, BackgroundColor, JustifyContent, Node, PositionType, Val};
use shared::protocol::ReputationStateUpdate;

use crate::status::{ReputationEntry, ReputationsStatusSnapshot};

const TOAST_DURATION_SECS: f32 = 4.0;

#[derive(Resource, Debug, Default)]
pub struct ReputationToastState {
    pub queue: VecDeque<String>,
    active: Option<String>,
    timer: Option<Timer>,
}

#[derive(Component)]
struct ReputationToastRoot;

#[derive(Component)]
struct ReputationToastText;

pub struct ReputationPlugin;

impl Plugin for ReputationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReputationToastState>();
        app.add_systems(Startup, spawn_reputation_toast_overlay);
        app.add_systems(Update, update_reputation_toast_overlay);
    }
}

pub fn map_reputation_state_update(
    status: &mut ReputationsStatusSnapshot,
    toast: &mut ReputationToastState,
    update: ReputationStateUpdate,
) {
    if let Some(message) = update.message.clone()
        && is_reputation_event_message(&message)
    {
        toast.queue.push_back(message);
    }
    apply_status_update(status, update);
}

pub fn reset_runtime(toast: &mut ReputationToastState) {
    *toast = ReputationToastState::default();
}

fn is_reputation_event_message(message: &str) -> bool {
    message.starts_with("gained ") || message.starts_with("lost ")
}

fn spawn_reputation_toast_overlay(mut commands: Commands) {
    commands
        .spawn((
            ReputationToastRoot,
            Visibility::Hidden,
            BackgroundColor(Color::srgba(0.04, 0.08, 0.04, 0.92)),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(28.0),
                top: Val::Px(104.0),
                width: Val::Px(320.0),
                padding: UiRect::axes(Val::Px(18.0), Val::Px(14.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                ReputationToastText,
                Text::new("Reputation Updated"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.72, 1.0, 0.72)),
            ));
        });
}

fn update_reputation_toast_overlay(
    time: Res<Time>,
    mut state: ResMut<ReputationToastState>,
    mut visibility_q: Query<&mut Visibility, With<ReputationToastRoot>>,
    mut text_q: Query<&mut Text, With<ReputationToastText>>,
) {
    advance_active_toast(&mut state, time.delta());
    sync_toast_visibility(&state, &mut visibility_q);
    sync_toast_text(&state, &mut text_q);
}

fn advance_active_toast(state: &mut ReputationToastState, delta: Duration) {
    if state.active.is_none() {
        activate_next_toast(state);
    }
    let Some(timer) = state.timer.as_mut() else {
        return;
    };
    timer.tick(delta);
    if !timer.is_finished() {
        return;
    }
    state.active = None;
    state.timer = None;
    activate_next_toast(state);
}

fn activate_next_toast(state: &mut ReputationToastState) {
    let Some(next) = state.queue.pop_front() else {
        return;
    };
    state.active = Some(next);
    state.timer = Some(Timer::from_seconds(TOAST_DURATION_SECS, TimerMode::Once));
}

fn sync_toast_visibility(
    state: &ReputationToastState,
    visibility_q: &mut Query<&mut Visibility, With<ReputationToastRoot>>,
) {
    let visibility = if state.active.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut current in visibility_q.iter_mut() {
        *current = visibility;
    }
}

fn sync_toast_text(
    state: &ReputationToastState,
    text_q: &mut Query<&mut Text, With<ReputationToastText>>,
) {
    let text = state
        .active
        .as_deref()
        .map(format_toast_text)
        .unwrap_or_default();
    for mut current in text_q.iter_mut() {
        **current = text.clone();
    }
}

fn format_toast_text(message: &str) -> String {
    format!("Reputation Updated\n{message}")
}

fn apply_status_update(status: &mut ReputationsStatusSnapshot, update: ReputationStateUpdate) {
    if let Some(rep_snapshot) = update.snapshot {
        status.entries = rep_snapshot
            .entries
            .into_iter()
            .map(|entry| ReputationEntry {
                faction_id: entry.faction_id,
                faction_name: entry.faction_name,
                standing: entry.standing,
                value: entry.value,
            })
            .collect();
    }
    status.last_server_message = update.message;
    status.last_error = update.error;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reputation_state_update_queues_gain_toast() {
        let mut status = ReputationsStatusSnapshot::default();
        let mut toast = ReputationToastState::default();

        map_reputation_state_update(
            &mut status,
            &mut toast,
            ReputationStateUpdate {
                snapshot: None,
                message: Some("gained 10 reputation with Stormwind".into()),
                error: None,
            },
        );

        assert_eq!(toast.queue.len(), 1);
        assert_eq!(
            status.last_server_message.as_deref(),
            Some("gained 10 reputation with Stormwind")
        );
    }

    #[test]
    fn reputation_state_update_ignores_non_event_messages() {
        let mut status = ReputationsStatusSnapshot::default();
        let mut toast = ReputationToastState::default();

        map_reputation_state_update(
            &mut status,
            &mut toast,
            ReputationStateUpdate {
                snapshot: None,
                message: Some("reputation loaded".into()),
                error: None,
            },
        );

        assert!(toast.queue.is_empty());
        assert_eq!(
            status.last_server_message.as_deref(),
            Some("reputation loaded")
        );
    }

    #[test]
    fn format_toast_text_prefixes_message() {
        assert_eq!(
            format_toast_text("gained 5 reputation with Darnassus"),
            "Reputation Updated\ngained 5 reputation with Darnassus"
        );
    }
}
