//! Server repositioning of the local player (docs/specs/flight-master.md). The
//! client moves its own player from input and otherwise ignores its replicated
//! `Position`; `MovementControl` says when not to: a new `epoch` (graveyard
//! release, resurrection, admin move, taxi landing) snaps the player to the
//! server position, and while `controlled` (taxi flight) the player follows the
//! server position and facing like a remote unit.

use bevy::prelude::*;
use shared::components::{MovementControl, Position as NetPosition, Rotation as NetRotation};

use crate::camera::CharacterFacing;
use crate::networking::{LocalPlayer, net_position_to_bevy};

/// Follow rate of a server-driven player, the remote-unit interpolation speed.
const FOLLOW_SPEED: f32 = 10.0;

/// The last `MovementControl::epoch` the local player adopted.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AdoptedMovementEpoch(pub u32);

type ServerMovedPlayer<'a> = (
    Entity,
    &'a MovementControl,
    &'a NetPosition,
    Option<&'a NetRotation>,
    &'a mut Transform,
    Option<&'a mut CharacterFacing>,
    Option<&'a AdoptedMovementEpoch>,
);

pub(crate) fn follow_server_movement(
    time: Res<Time>,
    mut players: Query<ServerMovedPlayer, With<LocalPlayer>>,
    mut commands: Commands,
) {
    let t = (FOLLOW_SPEED * time.delta_secs()).min(1.0);
    for (entity, control, position, rotation, mut transform, facing, adopted) in &mut players {
        let target = net_position_to_bevy(position);
        if adopted.is_some_and(|adopted| adopted.0 != control.epoch) {
            transform.translation = target;
        }
        if adopted.is_none_or(|adopted| adopted.0 != control.epoch) {
            commands
                .entity(entity)
                .insert(AdoptedMovementEpoch(control.epoch));
        }
        if !control.controlled {
            continue;
        }
        transform.translation = transform.translation.lerp(target, t);
        if let Some(rotation) = rotation {
            transform.rotation = Quat::from_rotation_y(rotation.y);
            if let Some(mut facing) = facing {
                facing.yaw = rotation.y;
            }
        }
    }
}

#[cfg(test)]
#[path = "server_movement_tests.rs"]
mod tests;
