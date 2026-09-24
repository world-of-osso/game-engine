use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::{PopupResult, PopupStack};
use game_engine::ui::screens::static_popup_component::{
    StaticPopupState, parse_popup_action, static_popup_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;

struct StaticPopupRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for StaticPopupRes {}
unsafe impl Sync for StaticPopupRes {}

#[derive(Resource)]
struct StaticPopupWrap(StaticPopupRes);

#[derive(Resource, PartialEq)]
struct StaticPopupModel(StaticPopupState);

pub struct StaticPopupPlugin;

impl Plugin for StaticPopupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PopupStack>();
        app.add_message::<PopupResult>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_static_popup_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_static_popup_ui);
        app.add_systems(
            Update,
            (
                handle_popup_clicks,
                handle_popup_enter,
                tick_popup_timeouts,
                emit_popup_results,
                sync_static_popup_ui,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn popup_state(stack: &PopupStack) -> StaticPopupState {
    StaticPopupState {
        popups: stack.visible(),
    }
}

fn build_static_popup_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    stack: Res<PopupStack>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = popup_state(&stack);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(static_popup_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(StaticPopupWrap(StaticPopupRes { screen, shared }));
    commands.insert_resource(StaticPopupModel(state));
}

fn teardown_static_popup_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<StaticPopupWrap>>,
    mut stack: ResMut<PopupStack>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    stack.clear();
    commands.remove_resource::<StaticPopupWrap>();
    commands.remove_resource::<StaticPopupModel>();
}

fn sync_static_popup_ui(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<StaticPopupWrap>>,
    mut last_model: Option<ResMut<StaticPopupModel>>,
    stack: Res<PopupStack>,
) {
    let (Some(wrap), Some(last_model)) = (wrap.as_mut(), last_model.as_mut()) else {
        return;
    };
    let state = popup_state(&stack);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn handle_popup_clicks(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    ui: Res<UiState>,
    mut stack: ResMut<PopupStack>,
) {
    if !stack.is_open() {
        return;
    }
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
    let Some((id, outcome)) =
        walk_up_for_onclick(&ui.registry, frame_id).and_then(|a| parse_popup_action(&a))
    else {
        return;
    };
    stack.resolve(id, outcome);
}

/// Enter accepts the focused (top) popup unless an editbox owns the keyboard.
/// Accepting consumes the press so Enter does not also open chat.
fn handle_popup_enter(
    keys: Option<ResMut<ButtonInput<KeyCode>>>,
    ui: Res<UiState>,
    mut stack: ResMut<PopupStack>,
) {
    let Some(mut keys) = keys else { return };
    let enter = [KeyCode::Enter, KeyCode::NumpadEnter];
    if ui.focused_frame.is_some() || stack.top().is_none() || !keys.any_just_pressed(enter) {
        return;
    }
    stack.accept_top();
    for key in enter {
        keys.clear_just_pressed(key);
    }
}

fn tick_popup_timeouts(time: Res<Time>, mut stack: ResMut<PopupStack>) {
    stack.tick(time.delta());
}

fn emit_popup_results(mut stack: ResMut<PopupStack>, mut results: MessageWriter<PopupResult>) {
    results.write_batch(stack.drain_results());
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
