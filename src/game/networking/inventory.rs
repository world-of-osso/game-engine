//! Owner bags and equipment: the server's `InventorySnapshot` / `EquipmentSnapshot`
//! (entering the world) and `InventoryDelta` (every later change) drive
//! [`InventoryState`]; [`InventoryRequest`]s go out on `InventoryChannel`, and an
//! `InventoryError` shows its Retail error text.

use bevy::prelude::*;
use game_engine::bag_data::{InventoryRequest, InventoryState};
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::ui::ui_errors::UiErrors;
use lightyear::prelude::Message as NetworkMessage;
use shared::protocol::{
    DestroyItem, EquipItem, EquipmentSnapshot, InventoryChannel, InventoryDelta, InventoryError,
    InventorySnapshot, SplitItem, SwapItem, UseItem,
};

use crate::game_state::GameState;

pub struct InventoryNetworkPlugin;

impl Plugin for InventoryNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<InventoryState>()
            .init_resource::<UiErrors>()
            .add_message::<InventoryRequest>();
        let handler =
            register_message_handler::<InventorySnapshot, _>(app, receive_inventory, in_world);
        add_message_route::<EquipmentSnapshot>(app, handler);
        add_message_route::<InventoryDelta>(app, handler);
        add_message_route::<InventoryError>(app, handler);
        app.add_systems(
            Update,
            send_inventory_requests.run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(OnExit(GameState::InWorld), reset_inventory);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

#[derive(bevy::ecs::system::SystemParam)]
struct InventoryReceivers<'w, 's> {
    snapshots: MessageReceivers<'w, 's, InventorySnapshot>,
    equipment: MessageReceivers<'w, 's, EquipmentSnapshot>,
    deltas: MessageReceivers<'w, 's, InventoryDelta>,
    errors: MessageReceivers<'w, 's, InventoryError>,
}

/// Snapshots before deltas: one batch can carry the snapshot and its first delta.
fn receive_inventory(
    mut receivers: InventoryReceivers,
    mut inventory: ResMut<InventoryState>,
    mut errors: ResMut<UiErrors>,
) {
    for inbox in receivers.snapshots.iter_mut() {
        for snapshot in inbox.receive() {
            inventory.apply_snapshot(&snapshot);
        }
    }
    for inbox in receivers.equipment.iter_mut() {
        for snapshot in inbox.receive() {
            inventory.apply_equipment_snapshot(&snapshot);
        }
    }
    for inbox in receivers.deltas.iter_mut() {
        for delta in inbox.receive() {
            inventory.apply_delta(&delta);
        }
    }
    for inbox in receivers.errors.iter_mut() {
        for error in inbox.receive() {
            errors.add(error.reason.message());
        }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct InventorySenders<'w, 's> {
    swap: MessageSenders<'w, 's, SwapItem>,
    equip: MessageSenders<'w, 's, EquipItem>,
    split: MessageSenders<'w, 's, SplitItem>,
    destroy: MessageSenders<'w, 's, DestroyItem>,
    use_item: MessageSenders<'w, 's, UseItem>,
}

fn send_inventory_requests(
    mut requests: MessageReader<InventoryRequest>,
    mut senders: InventorySenders,
) {
    for request in requests.read() {
        match request.clone() {
            InventoryRequest::Swap(swap) => send(&mut senders.swap, swap),
            InventoryRequest::Equip(equip) => send(&mut senders.equip, equip),
            InventoryRequest::Split(split) => send(&mut senders.split, split),
            InventoryRequest::Destroy(destroy) => send(&mut senders.destroy, destroy),
            InventoryRequest::Use(use_item) => send(&mut senders.use_item, use_item),
        }
    }
}

fn send<M: NetworkMessage + Clone>(senders: &mut MessageSenders<M>, message: M) {
    for mut sender in senders.iter_mut() {
        sender.send::<InventoryChannel>(message.clone());
    }
}

fn reset_inventory(mut inventory: ResMut<InventoryState>) {
    *inventory = InventoryState::default();
}

#[cfg(test)]
#[path = "inventory_tests.rs"]
mod tests;
