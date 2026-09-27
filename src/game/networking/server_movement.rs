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
use game_engine::unit_motion_data::{MotionPose, MotionTarget, follow_server_motion};

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
    for (entity, control, position, rotation, mut transform, facing, adopted) in &mut players {
        let result = follow_server_motion(
            MotionPose {
                position: transform.translation,
                rotation: transform.rotation,
            },
            MotionTarget {
                position: net_position_to_bevy(position),
                yaw: rotation.map(|rotation| rotation.y),
            },
            adopted.map(|adopted| adopted.0),
            control.epoch,
            control.controlled,
            time.delta_secs(),
        );
        if adopted.is_none_or(|adopted| adopted.0 != result.adopted_epoch) {
            commands
                .entity(entity)
                .insert(AdoptedMovementEpoch(result.adopted_epoch));
        }
        if adopted.is_some_and(|adopted| adopted.0 != control.epoch) || control.controlled {
            transform.translation = result.pose.position;
        }
        if let Some(yaw) = result.facing_yaw {
            transform.rotation = result.pose.rotation;
            if let Some(mut facing) = facing {
                facing.yaw = yaw;
            }
        }
    }
}

#[cfg(test)]
#[path = "server_movement_tests.rs"]
mod tests;
