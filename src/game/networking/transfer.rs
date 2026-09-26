//! Map transfers (docs/specs/instances.md; Retail `SMSG_NEW_WORLD` /
//! `CMSG_WORLD_PORT_RESPONSE`). `NewWorld` drops the old map's terrain, puts the local
//! player at the destination, shows the loading screen and loads the new map; once
//! loading completes the client answers `WorldPortAck` and the server adds the player
//! to the destination. `TransferAborted` shows its Retail error.

use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::ui::ui_errors::UiErrors;
use shared::protocol::{NewWorld, TransferAborted, TransferChannel, WorldPortAck};

use crate::camera::CharacterFacing;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use crate::terrain::AdtManager;
use crate::terrain_heightmap::TerrainHeightmap;

/// The map of the last `NewWorld`: loading it, then loaded (entered the world) and owed a
/// `WorldPortAck`.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub(crate) enum PendingWorldPort {
    #[default]
    None,
    Loading(u32),
    Loaded(u32),
}

pub struct TransferNetworkPlugin;

impl Plugin for TransferNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{register_message_handler, register_outgoing_handler};

        app.init_resource::<PendingWorldPort>()
            .init_resource::<UiErrors>()
            .add_systems(OnEnter(GameState::InWorld), finish_loading_new_world);
        register_message_handler::<NewWorld, _>(app, receive_new_world, |_| true);
        register_message_handler::<TransferAborted, _>(app, receive_transfer_aborted, |_| true);
        register_outgoing_handler(app, send_world_port_ack, |world| {
            matches!(
                *world.resource::<PendingWorldPort>(),
                PendingWorldPort::Loaded(_)
            )
        });
    }
}

type LocalPlayerPlacement<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, Option<&'static mut CharacterFacing>),
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
            for (mut transform, facing) in &mut players {
                transform.translation = position;
                transform.rotation = Quat::from_rotation_y(new_world.facing);
                if let Some(mut facing) = facing {
                    facing.yaw = yaw;
                }
            }
            *pending = PendingWorldPort::Loading(new_world.map_id);
            next_state.set(GameState::Loading);
        }
    }
}

/// The loading screen ended: the new map is in the world.
fn finish_loading_new_world(mut pending: ResMut<PendingWorldPort>) {
    if let PendingWorldPort::Loading(map_id) = *pending {
        *pending = PendingWorldPort::Loaded(map_id);
    }
}

fn send_world_port_ack(
    mut pending: ResMut<PendingWorldPort>,
    mut senders: MessageSenders<WorldPortAck>,
) {
    let PendingWorldPort::Loaded(map_id) = std::mem::take(&mut *pending) else {
        return;
    };
    info!("Map {map_id} loaded; WorldPortAck");
    for mut sender in senders.iter_mut() {
        sender.send::<TransferChannel>(WorldPortAck);
    }
}

fn receive_transfer_aborted(
    mut receivers: MessageReceivers<TransferAborted>,
    mut errors: ResMut<UiErrors>,
) {
    for receiver in receivers.iter_mut() {
        for aborted in receiver.receive() {
            info!(
                "Transfer to map {} aborted: {:?}",
                aborted.map_id, aborted.reason
            );
            errors.add(aborted.reason.text());
        }
    }
}

#[cfg(test)]
#[path = "transfer_tests.rs"]
mod tests;
