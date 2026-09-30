//! LootFrame scene (docs/specs/loot-frame.md): builds the Retail frame from
//! [`LootState`], placed under the cursor that opened it, and turns card clicks into
//! [`LootRequest`]s. Clicking a card loots it (`LootSlot`, LootFrame.lua:38-57); the
//! close button releases the loot (`CloseLoot`); the frame closes when the server
//! says `LootClosed`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::loot_state::{LootRequest, LootState};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::loot_frame_component::{LootFrameState, loot_frame_screen};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;

struct LootFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for LootFrameRes {}
unsafe impl Sync for LootFrameRes {}

#[derive(Resource)]
struct LootFrameWrap(LootFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct LootFrameModel(LootFrameState);

/// Where the open frame sits (top-left, UI space), fixed when it opens.
#[derive(Resource, Default, Clone, Copy, PartialEq)]
struct LootFrameAnchor {
    corpse: Option<u64>,
    at: Vec2,
}

pub struct LootFramePlugin;

impl Plugin for LootFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LootState>()
            .init_resource::<LootFrameAnchor>()
            .add_message::<LootRequest>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_loot_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_loot_frame_ui);
        app.add_systems(
            Update,
            (
                handle_loot_frame_input,
                place_loot_frame,
                sync_loot_frame_state,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

pub(crate) fn anchor_under_cursor(cursor: Vec2, screen: Vec2, rows: usize) -> Vec2 {
    Vec2::from_array(game_engine::loot_frame_data::anchor_under_cursor(
        cursor.to_array(),
        screen.to_array(),
        rows,
    ))
}

fn build_state(loot: &LootState, anchor: &LootFrameAnchor) -> LootFrameState {
    game_engine::loot_frame_data::build_state(loot, anchor.at.to_array())
}

fn build_loot_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    loot: Res<LootState>,
    anchor: Res<LootFrameAnchor>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&loot, &anchor);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(loot_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(LootFrameWrap(LootFrameRes { screen, shared }));
    commands.insert_resource(LootFrameModel(state));
}

fn teardown_loot_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<LootFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<LootFrameWrap>();
    commands.remove_resource::<LootFrameModel>();
}

/// A newly opened loot window takes its place under the cursor.
fn place_loot_frame(
    loot: Res<LootState>,
    ui: Res<UiState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut anchor: ResMut<LootFrameAnchor>,
) {
    if anchor.corpse == loot.corpse {
        return;
    }
    let screen = Vec2::new(ui.registry.screen_width, ui.registry.screen_height);
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| ui_cursor_position(&ui.registry, window))
        .unwrap_or(screen / 2.0);
    *anchor = LootFrameAnchor {
        corpse: loot.corpse,
        at: anchor_under_cursor(cursor, screen, loot.slots.len()),
    };
}

fn sync_loot_frame_state(
    mut ui: ResMut<UiState>,
    wrap: Option<ResMut<LootFrameWrap>>,
    last_model: Option<ResMut<LootFrameModel>>,
    loot: Res<LootState>,
    anchor: Res<LootFrameAnchor>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap, last_model) else {
        return;
    };
    let state = build_state(&loot, &anchor);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

#[derive(SystemParam)]
struct Pointer<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
}

impl Pointer<'_, '_> {
    /// The `onclick` action under a left or right click this frame.
    fn click(self, ui: &UiState) -> Option<String> {
        let mouse = self.mouse.as_ref()?;
        let clicked = [MouseButton::Left, MouseButton::Right]
            .into_iter()
            .any(|button| mouse.just_pressed(button));
        if !clicked || self.modal_open.is_some() {
            return None;
        }
        let window = self.windows.single().ok()?;
        let cursor = ui_cursor_position(&ui.registry, window)?;
        let frame_id = find_frame_at(&ui.registry, cursor.x, cursor.y)?;
        walk_up_for_onclick(&ui.registry, frame_id)
    }
}

/// The request a frame click sends.
pub(crate) fn request_for_action(action: &str) -> Option<LootRequest> {
    match game_engine::loot_frame_data::request_for_action(action)? {
        game_engine::loot_frame_data::LootFrameAction::Take { slot } => {
            Some(LootRequest::Take { slot })
        }
        game_engine::loot_frame_data::LootFrameAction::Release => Some(LootRequest::Release),
    }
}

fn handle_loot_frame_input(
    pointer: Pointer,
    ui: Res<UiState>,
    loot: Res<LootState>,
    mut requests: MessageWriter<LootRequest>,
) {
    if !loot.is_open() {
        return;
    }
    if let Some(request) = pointer.click(&ui).as_deref().and_then(request_for_action) {
        requests.write(request);
    }
}
