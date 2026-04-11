use std::collections::BTreeSet;

use bevy::prelude::*;
use game_engine::bag_data::InventoryState;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::bag_frame_component::{
    ACTION_BAG_TOGGLE_PREFIX, BagContainerState, BagFrameState, BagSlotState, bag_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use crate::sound::{UiSoundKind, UiSoundQueue, queue_ui_sound};
use crate::ui_input::walk_up_for_onclick;

#[derive(Resource, Default, Debug, PartialEq, Eq)]
pub struct BagFrameOpenState {
    open_bags: BTreeSet<usize>,
}

impl BagFrameOpenState {
    pub fn is_open(&self, bag_index: usize) -> bool {
        self.open_bags.contains(&bag_index)
    }

    pub fn any_open(&self) -> bool {
        !self.open_bags.is_empty()
    }

    pub fn close_all(&mut self) {
        self.open_bags.clear();
    }

    pub fn toggle(&mut self, bag_index: usize) -> bool {
        if self.open_bags.remove(&bag_index) {
            false
        } else {
            self.open_bags.insert(bag_index);
            true
        }
    }
}

struct BagFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for BagFrameRes {}
unsafe impl Sync for BagFrameRes {}

#[derive(Resource)]
struct BagFrameWrap(BagFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct BagFrameModel(BagFrameState);

pub struct BagFramePlugin;

impl Plugin for BagFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BagFrameOpenState>();
        app.init_resource::<InventoryState>();
        app.add_systems(OnEnter(GameState::InWorld), build_bag_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_bag_frame_ui);
        app.add_systems(
            Update,
            (toggle_bag_frame, sync_bag_frame_state).run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_bag_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    inventory: Res<InventoryState>,
    open: Res<BagFrameOpenState>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&inventory, &open);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(bag_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(BagFrameWrap(BagFrameRes { screen, shared }));
    commands.insert_resource(BagFrameModel(state));
}

fn teardown_bag_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<BagFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<BagFrameWrap>();
    commands.remove_resource::<BagFrameModel>();
}

fn sync_bag_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<BagFrameWrap>>,
    mut last_model: Option<ResMut<BagFrameModel>>,
    inventory: Res<InventoryState>,
    open: Res<BagFrameOpenState>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&inventory, &open);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn toggle_bag_frame(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    inventory: Res<InventoryState>,
    mut open: ResMut<BagFrameOpenState>,
    mut sounds: Option<ResMut<UiSoundQueue>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
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
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    let _ = apply_bag_toggle_action(&action, &inventory, &mut open, sounds.as_deref_mut());
}

fn build_state(inventory: &InventoryState, open: &BagFrameOpenState) -> BagFrameState {
    BagFrameState {
        bags: inventory
            .bags
            .iter()
            .map(|bag| BagContainerState {
                bag_index: bag.index,
                title: bag.name.clone(),
                slots: inventory
                    .slots
                    .get(bag.index)
                    .into_iter()
                    .flatten()
                    .map(|slot| BagSlotState {
                        icon_fdid: slot.icon_fdid,
                        count: slot.count,
                        quality_border: slot.quality.border_color().into(),
                    })
                    .collect(),
                visible: open.is_open(bag.index),
            })
            .collect(),
    }
}

fn apply_bag_toggle_action(
    action: &str,
    inventory: &InventoryState,
    open: &mut BagFrameOpenState,
    sounds: Option<&mut UiSoundQueue>,
) -> bool {
    let Some(bag_index) = parse_bag_toggle_action(action) else {
        return false;
    };
    if !inventory.bags.iter().any(|bag| bag.index == bag_index) {
        return false;
    }

    let opened = open.toggle(bag_index);
    if let Some(sounds) = sounds {
        let sound = if opened {
            UiSoundKind::BagOpen
        } else {
            UiSoundKind::BagClose
        };
        queue_ui_sound(sounds, sound);
    }
    true
}

fn parse_bag_toggle_action(action: &str) -> Option<usize> {
    action.strip_prefix(ACTION_BAG_TOGGLE_PREFIX)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::bag_data::{BagInfo, InventorySlot, ItemQuality};

    #[test]
    fn parse_bag_toggle_action_extracts_index() {
        assert_eq!(parse_bag_toggle_action("bag_toggle:0"), Some(0));
        assert_eq!(parse_bag_toggle_action("bag_toggle:4"), Some(4));
        assert_eq!(parse_bag_toggle_action("bag_toggle:nope"), None);
        assert_eq!(parse_bag_toggle_action("guild_toggle"), None);
    }

    #[test]
    fn apply_bag_toggle_action_toggles_open_state_and_queues_sounds() {
        let inventory = InventoryState::default();
        let mut open = BagFrameOpenState::default();
        let mut sounds = UiSoundQueue::default();

        assert!(apply_bag_toggle_action(
            "bag_toggle:0",
            &inventory,
            &mut open,
            Some(&mut sounds)
        ));
        assert!(open.is_open(0));
        assert!(apply_bag_toggle_action(
            "bag_toggle:0",
            &inventory,
            &mut open,
            Some(&mut sounds)
        ));
        assert!(!open.is_open(0));
        assert_eq!(
            sounds.queued_kinds(),
            vec![UiSoundKind::BagOpen, UiSoundKind::BagClose]
        );
    }

    #[test]
    fn build_state_maps_inventory_slots_and_visibility() {
        let inventory = InventoryState {
            bags: vec![BagInfo {
                index: 0,
                name: "Backpack".into(),
                size: 2,
                icon_fdid: 123,
            }],
            slots: vec![vec![
                InventorySlot {
                    icon_fdid: 11,
                    count: 3,
                    quality: ItemQuality::Rare,
                    name: "Potion".into(),
                },
                InventorySlot::default(),
            ]],
        };
        let mut open = BagFrameOpenState::default();
        open.toggle(0);

        let state = build_state(&inventory, &open);

        assert_eq!(state.bags.len(), 1);
        assert_eq!(state.bags[0].title, "Backpack");
        assert!(state.bags[0].visible);
        assert_eq!(state.bags[0].slots[0].icon_fdid, 11);
        assert_eq!(state.bags[0].slots[0].count, 3);
        assert_eq!(state.bags[0].slots[0].quality_border, "0.0,0.44,0.87,1.0");
    }
}
