//! Mounts the Retail status tracking bar (the experience bar) in world and keeps it in step
//! with [`ExperienceState`], HUD edit mode and the mouse.

use bevy::prelude::*;
use game_engine::experience_data::ExperienceState;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::status_tracking_bar_component::{
    EXP_BAR_NAME, StatusTrackingBarState, status_tracking_bar_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::edit_mode::EditMode;
use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;

struct StatusTrackingBarRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for StatusTrackingBarRes {}
unsafe impl Sync for StatusTrackingBarRes {}

#[derive(Resource)]
struct StatusTrackingBarWrap(StatusTrackingBarRes);

#[derive(Resource, Clone, PartialEq)]
struct StatusTrackingBarModel(StatusTrackingBarState);

pub struct StatusTrackingBarPlugin;

impl Plugin for StatusTrackingBarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExperienceState>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_status_tracking_bar_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_status_tracking_bar_ui);
        app.add_systems(
            Update,
            sync_status_tracking_bar
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_status_tracking_bar_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    experience: Res<ExperienceState>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = StatusTrackingBarState::new(&experience, false, false);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(status_tracking_bar_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(StatusTrackingBarWrap(StatusTrackingBarRes {
        screen,
        shared,
    }));
    commands.insert_resource(StatusTrackingBarModel(state));
}

fn teardown_status_tracking_bar_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<StatusTrackingBarWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<StatusTrackingBarWrap>();
    commands.remove_resource::<StatusTrackingBarModel>();
}

fn sync_status_tracking_bar(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<StatusTrackingBarWrap>>,
    mut last_model: Option<ResMut<StatusTrackingBarModel>>,
    experience: Res<ExperienceState>,
    edit_mode: Option<Res<EditMode>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let editing = edit_mode.is_some_and(|edit| edit.is_active());
    let hovered = windows
        .single()
        .ok()
        .and_then(|window| ui_cursor_position(&ui.registry, window))
        .is_some_and(|cursor| {
            in_exp_bar(
                &ui.registry,
                find_frame_at(&ui.registry, cursor.x, cursor.y),
            )
        });
    let state = StatusTrackingBarState::new(&experience, editing, hovered);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// `frame_id` (the topmost mouse frame) is the experience bar or inside it.
pub(crate) fn in_exp_bar(registry: &FrameRegistry, mut frame_id: Option<u64>) -> bool {
    while let Some(frame) = frame_id.and_then(|id| registry.get(id)) {
        if frame.name.as_deref() == Some(EXP_BAR_NAME.0) {
            return true;
        }
        frame_id = frame.parent_id;
    }
    false
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
