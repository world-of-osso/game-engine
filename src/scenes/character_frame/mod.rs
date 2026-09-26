use bevy::prelude::*;
use game_engine::bag_data::InventoryState;
use game_engine::cursor_item::CursorItem;
use game_engine::input_bindings::InputAction;
use game_engine::status::CharacterStatsSnapshot;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::character_frame_component::{
    BOTTOM_SLOT_LABELS, BOTTOM_SLOTS, CharacterFrameState, EquipmentSlotState, LEFT_SLOT_LABELS,
    LEFT_SLOTS, RIGHT_SLOT_LABELS, RIGHT_SLOTS, character_frame_screen, equipment_slot_action,
};
use shared::protocol::{EquipmentSlot, ItemLocation};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::window_manager::{WindowId, WindowManager};

struct CharacterFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for CharacterFrameRes {}
unsafe impl Sync for CharacterFrameRes {}

#[derive(Resource)]
struct CharacterFrameWrap(CharacterFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct CharacterFrameModel(CharacterFrameState);

pub struct CharacterFramePlugin;

impl Plugin for CharacterFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryState>()
            .init_resource::<CursorItem>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_character_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_character_frame_ui);
        app.add_systems(
            Update,
            (toggle_character_frame, sync_character_frame_state)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_character_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    equipment: Equipment,
    window_manager: Res<WindowManager>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(
        character_stats.as_deref(),
        &equipment,
        window_manager.is_open(WindowId::Character),
    );
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(character_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(CharacterFrameWrap(CharacterFrameRes { screen, shared }));
    commands.insert_resource(CharacterFrameModel(state));
}

fn teardown_character_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<CharacterFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<CharacterFrameWrap>();
    commands.remove_resource::<CharacterFrameModel>();
}

fn toggle_character_frame(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    mut window_manager: ResMut<WindowManager>,
) {
    if keybinds.just_pressed(InputAction::ToggleCharacter) {
        window_manager.toggle(WindowId::Character);
    }
}

fn sync_character_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<CharacterFrameWrap>>,
    mut last_model: Option<ResMut<CharacterFrameModel>>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    equipment: Equipment,
    window_manager: Res<WindowManager>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(
        character_stats.as_deref(),
        &equipment,
        window_manager.is_open(WindowId::Character),
    );
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// The equipped items and the cursor item (its source slot shows locked).
#[derive(bevy::ecs::system::SystemParam)]
struct Equipment<'w> {
    inventory: Res<'w, InventoryState>,
    cursor: Res<'w, CursorItem>,
}

fn build_state(
    character_stats: Option<&CharacterStatsSnapshot>,
    equipment: &Equipment,
    open: bool,
) -> CharacterFrameState {
    let (character_name, level, class_name) = extract_identity(character_stats);
    let health = format_resource_bar(character_stats, |s| (s.health_current, s.health_max));
    let mana = format_resource_bar(character_stats, |s| (s.mana_current, s.mana_max));
    let speed = character_stats
        .and_then(|s| s.movement_speed)
        .map(|s| format!("{:.0}%", s * 100.0))
        .unwrap_or_default();

    CharacterFrameState {
        visible: open,
        character_name,
        level,
        class_name,
        health,
        mana,
        speed,
        left_slots: build_column_slots(&LEFT_SLOT_LABELS, &LEFT_SLOTS, equipment),
        right_slots: build_column_slots(&RIGHT_SLOT_LABELS, &RIGHT_SLOTS, equipment),
        bottom_slots: build_column_slots(&BOTTOM_SLOT_LABELS, &BOTTOM_SLOTS, equipment),
    }
}

fn extract_identity(stats: Option<&CharacterStatsSnapshot>) -> (String, u16, String) {
    let name = stats.and_then(|s| s.name.clone()).unwrap_or_default();
    let level = stats.and_then(|s| s.level).unwrap_or(0);
    let class = stats
        .and_then(|s| s.class)
        .map(|class| game_engine::character_models::class_name(class).to_string())
        .unwrap_or_default();
    (name, level, class)
}

/// Paperdoll slots: the equipped item's name and icon from the item catalog, each
/// slot clickable for the cursor item (`PaperDollItemSlotButton_OnClick`).
fn build_column_slots(
    labels: &[&str],
    slots: &[EquipmentSlot],
    equipment: &Equipment,
) -> Vec<EquipmentSlotState> {
    labels
        .iter()
        .zip(slots)
        .map(|(label, &slot)| {
            let item = equipment.inventory.equipped(slot);
            EquipmentSlotState {
                slot_name: label.to_string(),
                item_name: item.map(|item| item.name.clone()).unwrap_or_default(),
                icon_fdid: item.map_or(0, |item| item.icon_fdid),
                action: equipment_slot_action(slot),
                locked: equipment.cursor.source() == Some(ItemLocation::Equipment(slot)),
            }
        })
        .collect()
}

fn format_resource_bar(
    stats: Option<&CharacterStatsSnapshot>,
    extract: impl Fn(&CharacterStatsSnapshot) -> (Option<f32>, Option<f32>),
) -> String {
    stats
        .and_then(|s| {
            let (current, max) = extract(s);
            match (current, max) {
                (Some(c), Some(m)) => Some(format!("{c:.0} / {m:.0}")),
                (Some(c), None) => Some(format!("{c:.0}")),
                _ => None,
            }
        })
        .unwrap_or_default()
}
