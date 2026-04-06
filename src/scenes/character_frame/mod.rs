use bevy::prelude::*;
use game_engine::status::{CharacterStatsSnapshot, EquippedGearStatusSnapshot};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::character_frame_component::{
    BOTTOM_SLOT_LABELS, CharacterFrameState, EquipmentSlotState, LEFT_SLOT_LABELS,
    RIGHT_SLOT_LABELS, character_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;

/// Tracks whether the Character panel is open.
#[derive(Resource, Default)]
pub struct CharacterFrameOpen(pub bool);

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
        app.init_resource::<CharacterFrameOpen>();
        app.add_systems(OnEnter(GameState::InWorld), build_character_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_character_frame_ui);
        app.add_systems(
            Update,
            (toggle_character_frame, sync_character_frame_state)
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_character_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    gear: Option<Res<EquippedGearStatusSnapshot>>,
    open: Res<CharacterFrameOpen>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(character_stats.as_deref(), gear.as_deref(), &open);
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
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut open: ResMut<CharacterFrameOpen>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        open.0 = !open.0;
    }
}

fn sync_character_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<CharacterFrameWrap>>,
    mut last_model: Option<ResMut<CharacterFrameModel>>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    gear: Option<Res<EquippedGearStatusSnapshot>>,
    open: Res<CharacterFrameOpen>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(character_stats.as_deref(), gear.as_deref(), &open);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(
    character_stats: Option<&CharacterStatsSnapshot>,
    gear: Option<&EquippedGearStatusSnapshot>,
    open: &CharacterFrameOpen,
) -> CharacterFrameState {
    let character_name = character_stats
        .and_then(|s| s.name.clone())
        .unwrap_or_default();
    let level = character_stats.and_then(|s| s.level).unwrap_or(0);
    let class_name = character_stats
        .and_then(|s| s.class)
        .map(class_name_from_id)
        .unwrap_or_default();
    let health = format_resource_bar(character_stats, |s| (s.health_current, s.health_max));
    let mana = format_resource_bar(character_stats, |s| (s.mana_current, s.mana_max));
    let speed = character_stats
        .and_then(|s| s.movement_speed)
        .map(|s| format!("{:.0}%", s * 100.0))
        .unwrap_or_default();

    CharacterFrameState {
        visible: open.0,
        character_name,
        level,
        class_name,
        health,
        mana,
        speed,
        left_slots: build_column_slots(&LEFT_SLOT_LABELS, gear),
        right_slots: build_column_slots(&RIGHT_SLOT_LABELS, gear),
        bottom_slots: build_column_slots(&BOTTOM_SLOT_LABELS, gear),
    }
}

fn build_column_slots(
    labels: &[&str],
    gear: Option<&EquippedGearStatusSnapshot>,
) -> Vec<EquipmentSlotState> {
    labels
        .iter()
        .map(|label| {
            let item_name = gear
                .and_then(|g| {
                    g.entries
                        .iter()
                        .find(|e| e.slot.eq_ignore_ascii_case(label))
                        .map(|e| e.path.clone())
                })
                .unwrap_or_default();
            EquipmentSlotState {
                slot_name: label.to_string(),
                item_name,
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

fn class_name_from_id(class_id: u8) -> String {
    match class_id {
        1 => "Warrior",
        2 => "Paladin",
        3 => "Hunter",
        4 => "Rogue",
        5 => "Priest",
        6 => "Death Knight",
        7 => "Shaman",
        8 => "Mage",
        9 => "Warlock",
        10 => "Monk",
        11 => "Druid",
        12 => "Demon Hunter",
        13 => "Evoker",
        _ => "Unknown",
    }
    .to_string()
}
