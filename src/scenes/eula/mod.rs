use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::eula_component::{EulaAction, EulaScreenState, eula_screen};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::{GameState, PostEulaState};
use crate::ui_input::walk_up_for_onclick;

struct EulaScreenRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for EulaScreenRes {}
unsafe impl Sync for EulaScreenRes {}

#[derive(Resource)]
struct EulaScreenWrap(EulaScreenRes);

#[derive(Resource, Clone, PartialEq, Eq, Default)]
struct EulaStatus(String);

#[derive(Resource, Clone, PartialEq, Eq)]
struct EulaScreenModel(EulaScreenState);

#[derive(Resource, Clone, Copy)]
struct EulaAcceptanceSaver(fn(bool) -> Result<(), String>);

impl Default for EulaAcceptanceSaver {
    fn default() -> Self {
        Self(crate::client_options::save_eula_accepted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EulaResolution {
    Stay,
    Exit,
    Transition(GameState),
}

pub struct EulaScreenPlugin;

impl Plugin for EulaScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EulaAcceptanceSaver>();
        app.init_resource::<EulaStatus>();
        app.add_systems(OnEnter(GameState::Eula), build_eula_ui);
        app.add_systems(OnExit(GameState::Eula), teardown_eula_ui);
        app.add_systems(
            Update,
            (eula_sync_root_size, sync_eula_state, handle_eula_input)
                .run_if(in_state(GameState::Eula)),
        );
    }
}

fn build_eula_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut status: ResMut<EulaStatus>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    status.0.clear();
    let state = build_state(&status);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(eula_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(EulaScreenWrap(EulaScreenRes { screen, shared }));
    commands.insert_resource(EulaScreenModel(state));
}

fn teardown_eula_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<EulaScreenWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<EulaScreenWrap>();
    commands.remove_resource::<EulaScreenModel>();
}

fn eula_sync_root_size(mut ui: ResMut<UiState>, windows: Query<&Window, With<PrimaryWindow>>) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
}

fn sync_eula_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<EulaScreenWrap>>,
    mut last_model: Option<ResMut<EulaScreenModel>>,
    status: Res<EulaStatus>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&status);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(status: &EulaStatus) -> EulaScreenState {
    EulaScreenState {
        status_text: status.0.clone(),
    }
}

fn handle_eula_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    ui: Res<UiState>,
    saver: Res<EulaAcceptanceSaver>,
    post_eula: Option<Res<PostEulaState>>,
    mut status: ResMut<EulaStatus>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) =
        walk_up_for_onclick(&ui.registry, frame_id).and_then(|action| EulaAction::parse(&action))
    else {
        return;
    };

    let resolution = resolve_eula_action(
        action,
        saver.0,
        post_eula.as_deref().map(|state| state.0),
        &mut status.0,
    );
    apply_eula_resolution(resolution, &mut commands, &mut next_state, &mut exit);
}

fn resolve_eula_action(
    action: EulaAction,
    saver: fn(bool) -> Result<(), String>,
    post_eula: Option<GameState>,
    status: &mut String,
) -> EulaResolution {
    match action {
        EulaAction::Decline => EulaResolution::Exit,
        EulaAction::Accept => match saver(true) {
            Ok(()) => {
                status.clear();
                EulaResolution::Transition(post_eula.unwrap_or(GameState::Login))
            }
            Err(err) => {
                *status = format!("Failed to save acceptance: {err}");
                EulaResolution::Stay
            }
        },
    }
}

fn apply_eula_resolution(
    resolution: EulaResolution,
    commands: &mut Commands,
    next_state: &mut NextState<GameState>,
    exit: &mut MessageWriter<AppExit>,
) {
    match resolution {
        EulaResolution::Stay => {}
        EulaResolution::Exit => {
            exit.write(AppExit::Success);
        }
        EulaResolution::Transition(state) => {
            commands.remove_resource::<PostEulaState>();
            next_state.set(state);
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/eula_screen_tests.rs"]
mod tests;
