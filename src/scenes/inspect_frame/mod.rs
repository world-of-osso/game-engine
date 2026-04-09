use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::status::InspectStatusSnapshot;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::inspect_frame_component::{
    InspectEquipmentRow, InspectFrameState, InspectTalentRow, inspect_frame_screen,
};
use shared::components::{EquipmentAppearance, EquipmentVisualSlot};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;

struct InspectFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for InspectFrameRes {}
unsafe impl Sync for InspectFrameRes {}

#[derive(Resource)]
struct InspectFrameWrap(InspectFrameRes);

#[derive(Resource, Clone, PartialEq, Eq)]
struct InspectFrameModel(InspectFrameState);

pub struct InspectFramePlugin;

impl Plugin for InspectFramePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InWorld), build_inspect_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_inspect_frame_ui);
        app.add_systems(
            Update,
            sync_inspect_frame_state.run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_inspect_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    snapshot: Option<Res<InspectStatusSnapshot>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(snapshot.as_deref());
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(inspect_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(InspectFrameWrap(InspectFrameRes { screen, shared }));
    commands.insert_resource(InspectFrameModel(state));
}

fn teardown_inspect_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<InspectFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<InspectFrameWrap>();
    commands.remove_resource::<InspectFrameModel>();
}

fn sync_inspect_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<InspectFrameWrap>>,
    mut last_model: Option<ResMut<InspectFrameModel>>,
    snapshot: Option<Res<InspectStatusSnapshot>>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(snapshot.as_deref());
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(snapshot: Option<&InspectStatusSnapshot>) -> InspectFrameState {
    let Some(snapshot) = snapshot else {
        return InspectFrameState::default();
    };
    let Some(target_name) = snapshot.target_name.clone() else {
        return InspectFrameState::default();
    };

    InspectFrameState {
        visible: true,
        target_name,
        status_text: snapshot
            .last_server_message
            .clone()
            .or_else(|| snapshot.last_error.clone())
            .unwrap_or_default(),
        spec_summary: active_spec_summary(snapshot),
        points_remaining: snapshot.points_remaining,
        equipment_rows: build_equipment_rows(&snapshot.equipment_appearance),
        talent_rows: build_talent_rows(snapshot),
    }
}

fn active_spec_summary(snapshot: &InspectStatusSnapshot) -> String {
    let specs = snapshot
        .spec_tabs
        .iter()
        .filter(|tab| tab.active)
        .map(|tab| tab.name.as_str())
        .collect::<Vec<_>>();
    if specs.is_empty() {
        "None".into()
    } else {
        specs.join(", ")
    }
}

fn build_equipment_rows(appearance: &EquipmentAppearance) -> Vec<InspectEquipmentRow> {
    ordered_slots()
        .into_iter()
        .map(|slot| InspectEquipmentRow {
            slot_name: slot_label(slot).into(),
            value: appearance
                .entries
                .iter()
                .find(|entry| entry.slot == slot)
                .map(format_equipment_entry)
                .unwrap_or_else(|| "-".into()),
        })
        .collect()
}

fn build_talent_rows(snapshot: &InspectStatusSnapshot) -> Vec<InspectTalentRow> {
    snapshot
        .talents
        .iter()
        .filter(|talent| talent.active || talent.points_spent > 0)
        .map(|talent| InspectTalentRow {
            name: talent.name.clone(),
            points_text: format!("{}/{}", talent.points_spent, talent.max_points),
        })
        .collect()
}

fn format_equipment_entry(entry: &shared::components::EquippedAppearanceEntry) -> String {
    if entry.hidden {
        return "hidden".into();
    }
    match (entry.item_id, entry.display_info_id) {
        (Some(item_id), Some(display_id)) => format!("item {item_id} / display {display_id}"),
        (Some(item_id), None) => format!("item {item_id}"),
        (None, Some(display_id)) => format!("display {display_id}"),
        (None, None) => "-".into(),
    }
}

fn ordered_slots() -> [EquipmentVisualSlot; 13] {
    [
        EquipmentVisualSlot::Head,
        EquipmentVisualSlot::Shoulder,
        EquipmentVisualSlot::Back,
        EquipmentVisualSlot::Chest,
        EquipmentVisualSlot::Shirt,
        EquipmentVisualSlot::Tabard,
        EquipmentVisualSlot::Wrist,
        EquipmentVisualSlot::Hands,
        EquipmentVisualSlot::Waist,
        EquipmentVisualSlot::Legs,
        EquipmentVisualSlot::Feet,
        EquipmentVisualSlot::MainHand,
        EquipmentVisualSlot::OffHand,
    ]
}

fn slot_label(slot: EquipmentVisualSlot) -> &'static str {
    match slot {
        EquipmentVisualSlot::Head => "Head",
        EquipmentVisualSlot::Shoulder => "Shoulders",
        EquipmentVisualSlot::Back => "Back",
        EquipmentVisualSlot::Chest => "Chest",
        EquipmentVisualSlot::Shirt => "Shirt",
        EquipmentVisualSlot::Tabard => "Tabard",
        EquipmentVisualSlot::Wrist => "Wrist",
        EquipmentVisualSlot::Hands => "Hands",
        EquipmentVisualSlot::Waist => "Waist",
        EquipmentVisualSlot::Legs => "Legs",
        EquipmentVisualSlot::Feet => "Feet",
        EquipmentVisualSlot::MainHand => "Main Hand",
        EquipmentVisualSlot::OffHand => "Off Hand",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::status::{InspectStatusSnapshot, TalentNodeEntry, TalentSpecTabEntry};

    #[test]
    fn build_state_maps_equipment_and_talents() {
        let state = build_state(Some(&InspectStatusSnapshot {
            target_name: Some("Alice".into()),
            equipment_appearance: EquipmentAppearance {
                entries: vec![shared::components::EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Head,
                    item_id: Some(100),
                    display_info_id: Some(200),
                    inventory_type: 1,
                    hidden: false,
                }],
            },
            spec_tabs: vec![TalentSpecTabEntry {
                name: "Protection".into(),
                active: true,
            }],
            talents: vec![TalentNodeEntry {
                talent_id: 1,
                name: "Divine Strength".into(),
                points_spent: 1,
                max_points: 1,
                active: true,
            }],
            points_remaining: 50,
            last_server_message: Some("inspect ready".into()),
            last_error: None,
        }));

        assert!(state.visible);
        assert_eq!(state.target_name, "Alice");
        assert_eq!(state.spec_summary, "Protection");
        assert_eq!(state.equipment_rows[0].value, "item 100 / display 200");
        assert_eq!(state.talent_rows[0].name, "Divine Strength");
    }
}
