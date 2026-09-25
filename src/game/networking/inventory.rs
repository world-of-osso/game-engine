//! Owner bag contents: the server's `InventorySnapshot` (entering the world) and
//! `InventoryDelta` (every later change) drive [`InventoryState`].

use bevy::prelude::*;
use game_engine::bag_data::InventoryState;
use game_engine::network_runtime::messages::MessageReceivers;
use shared::protocol::{InventoryDelta, InventorySnapshot};

use crate::game_state::GameState;

pub struct InventoryNetworkPlugin;

impl Plugin for InventoryNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<InventoryState>();
        let handler =
            register_message_handler::<InventorySnapshot, _>(app, receive_inventory, in_world);
        add_message_route::<InventoryDelta>(app, handler);
        app.add_systems(OnExit(GameState::InWorld), reset_inventory);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

/// Snapshots before deltas: one batch can carry the snapshot and its first delta.
fn receive_inventory(
    mut snapshots: MessageReceivers<InventorySnapshot>,
    mut deltas: MessageReceivers<InventoryDelta>,
    mut inventory: ResMut<InventoryState>,
) {
    for inbox in snapshots.iter_mut() {
        for snapshot in inbox.receive() {
            inventory.apply_snapshot(&snapshot);
        }
    }
    for inbox in deltas.iter_mut() {
        for delta in inbox.receive() {
            inventory.apply_delta(&delta);
        }
    }
}

fn reset_inventory(mut inventory: ResMut<InventoryState>) {
    *inventory = InventoryState::default();
}
