//! Map transfers (docs/specs/instances.md; Retail `SMSG_NEW_WORLD` /
//! `CMSG_WORLD_PORT_RESPONSE`). `NewWorld` drops the old map's terrain, puts the local
//! player at the destination, shows the loading screen and loads the new map; once the
//! map is loaded the client answers `WorldPortAck`, the server adds the player to the
//! destination and sends its objects with a new `WorldArrival` count, and the loading
//! screen ends. `TransferAborted` shows its Retail error.

use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::ui::ui_errors::UiErrors;
use shared::components::WorldArrival;
use shared::protocol::{NewWorld, TransferAborted, TransferChannel, WorldPortAck};

use crate::camera::CharacterFacing;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use crate::terrain::AdtManager;
use crate::terrain_heightmap::TerrainHeightmap;

/// The last `NewWorld`: loading its map, then loaded and owed a `WorldPortAck`, then
/// waiting for the local player's `WorldArrival` to reach `arrival`, the count that
/// comes with the destination's objects.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub(crate) enum PendingWorldPort {
    #[default]
    None,
    Loading {
        map_id: u32,
        arrival: u32,
    },
    Loaded {
        map_id: u32,
        arrival: u32,
    },
    Arriving {
        arrival: u32,
    },
}

impl PendingWorldPort {
    /// Whether the loading screen may end once the map itself is ready: a loaded map
    /// first owes its `WorldPortAck`, then waits for its arrival.
    pub(crate) fn arrived(&mut self, arrivals: Option<&WorldArrival>) -> bool {
        match *self {
            Self::None => true,
            Self::Loading { map_id, arrival } => {
                *self = Self::Loaded { map_id, arrival };
                false
            }
            Self::Loaded { .. } => false,
            Self::Arriving { arrival } => {
                let arrived = arrivals.is_some_and(|arrivals| arrivals.0 >= arrival);
                if arrived {
                    *self = Self::None;
                }
                arrived
            }
        }
    }
}

pub struct TransferNetworkPlugin;

impl Plugin for TransferNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{register_message_handler, register_outgoing_handler};

        app.init_resource::<PendingWorldPort>()
            .init_resource::<UiErrors>();
        register_message_handler::<NewWorld, _>(app, receive_new_world, |_| true);
        register_message_handler::<TransferAborted, _>(app, receive_transfer_aborted, |_| true);
        register_outgoing_handler(app, send_world_port_ack, |world| {
            matches!(
                *world.resource::<PendingWorldPort>(),
                PendingWorldPort::Loaded { .. }
            )
        });
    }
}

type LocalPlayerPlacement<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        Option<&'static mut CharacterFacing>,
        Option<&'static WorldArrival>,
    ),
    With<LocalPlayer>,
>;

fn receive_new_world(
    mut receivers: MessageReceivers<NewWorld>,
    mut commands: Commands,
    mut adt_manager: ResMut<AdtManager>,
    mut heightmap: ResMut<TerrainHeightmap>,
    mut players: LocalPlayerPlacement,
    mut pending: ResMut<PendingWorldPort>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for receiver in receivers.iter_mut() {
        for new_world in receiver.receive() {
            let position = Vec3::from(new_world.position);
            info!(
                "NewWorld: map {} ({}) at {position}",
                new_world.map_id, new_world.map_directory
            );
            crate::terrain::reset_streamed_terrain(&mut commands, &mut adt_manager, &mut heightmap);
            let tile = crate::terrain::bevy_to_tile_coords(position.x, position.z);
            crate::terrain::replace_streamed_map(
                &mut commands,
                &mut adt_manager,
                &mut heightmap,
                new_world.map_directory,
                tile,
            );
            // CharacterFacing walks along (sin yaw, cos yaw): WoW orientation + 90°; the
            // model turns by the orientation itself.
            let yaw = new_world.facing + std::f32::consts::FRAC_PI_2;
            let mut arrivals = 0;
            for (mut transform, facing, arrival) in &mut players {
                transform.translation = position;
                transform.rotation = Quat::from_rotation_y(new_world.facing);
                if let Some(mut facing) = facing {
                    facing.yaw = yaw;
                }
                arrivals = arrival.map_or(0, |arrival| arrival.0);
            }
            *pending = PendingWorldPort::Loading {
                map_id: new_world.map_id,
                arrival: arrivals + 1,
            };
            next_state.set(GameState::Loading);
        }
    }
}

fn send_world_port_ack(
    mut pending: ResMut<PendingWorldPort>,
    mut senders: MessageSenders<WorldPortAck>,
) {
    let PendingWorldPort::Loaded { map_id, arrival } = *pending else {
        return;
    };
    *pending = PendingWorldPort::Arriving { arrival };
    info!("Map {map_id} loaded; WorldPortAck, awaiting arrival {arrival}");
    for mut sender in senders.iter_mut() {
        sender.send::<TransferChannel>(WorldPortAck);
    }
}

fn receive_transfer_aborted(
    mut receivers: MessageReceivers<TransferAborted>,
    catalog: Option<Res<game_engine::instance_state::InstanceCatalog>>,
    mut errors: ResMut<UiErrors>,
) {
    for receiver in receivers.iter_mut() {
        for aborted in receiver.receive() {
            info!(
                "Transfer to map {} aborted: {:?}",
                aborted.map_id, aborted.reason
            );
            let map_name = catalog
                .as_deref()
                .map_or("", |catalog| catalog.map_name(aborted.map_id));
            errors.add(&aborted.reason.text(map_name));
        }
    }
}

#[cfg(test)]
#[path = "transfer_tests.rs"]
mod tests;
