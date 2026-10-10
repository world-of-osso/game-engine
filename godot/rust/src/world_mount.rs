//! A mounted unit's mount (replicated `Mounted::mount_display_id`, TrinityCore `Unit::Mount`
//! → `UnitData::MountDisplayID`): the mount's creature model loads under the unit's node
//! and the unit's own visual rides its MountMain point (M2 attachment 0, "Shield / MountMain
//! / ItemVisual0" in the wowdev M2 attachment table). The rider holds Mount (91); the mount
//! plays the rider's locomotion clip, and while flying its flight clips (AnimationData
//! names 548 MountFlightIdle, 550 MountFlightBackwards, 552 MountFlightLeft,
//! 554 MountFlightRight, 556 MountFlightRun).

use game_engine_core::movement_animation_data::{
    ANIM_RUN, ANIM_SHUFFLE_LEFT, ANIM_SHUFFLE_RIGHT, ANIM_WALK, ANIM_WALK_BACKWARDS,
};
use game_engine_core::vehicle_seat::{VehicleSeat, read_vehicle_seat, seat_local_transform};
use godot::{
    builtin::Transform3D,
    classes::{Node, Node3D},
    prelude::*,
};
use shared::components::{Mounted, SheathState, VehiclePassenger};

use std::collections::HashMap;

use super::{Locomotion, UnitNode};
use crate::animation::WowAnimationPlayer;
use crate::world_models::{UnitAppearance, WorldModels};

/// AnimationData 91: the rider's seated clip.
pub(super) const ANIM_MOUNT: u16 = 91;
const ANIM_MOUNT_FLIGHT_IDLE: u16 = 548;
const ANIM_MOUNT_FLIGHT_BACKWARDS: u16 = 550;
const ANIM_MOUNT_FLIGHT_LEFT: u16 = 552;
const ANIM_MOUNT_FLIGHT_RIGHT: u16 = 554;
const ANIM_MOUNT_FLIGHT_RUN: u16 = 556;
/// The mount model's saddle: attachment 0 (MountMain).
const SADDLE: &str = "NpcModel/Skeleton3D/AttachmentBone0/Attachment0";

pub(super) struct UnitPassenger {
    pub state: VehiclePassenger,
    pub definition: VehicleSeat,
}

pub(super) fn sync_passenger(
    unit: &mut UnitNode,
    wanted: Option<&VehiclePassenger>,
    data_root: &std::path::Path,
) -> Result<(), String> {
    if unit.passenger.as_ref().map(|seat| seat.state) == wanted.copied() {
        return Ok(());
    }
    unit.passenger = None;
    if let Some(visual) = unit.visual.as_mut() {
        visual.set_transform(Transform3D::IDENTITY);
    }
    if let Some(state) = wanted {
        unit.passenger = Some(UnitPassenger {
            state: *state,
            definition: read_vehicle_seat(data_root, state.seat_id)?,
        });
    }
    Ok(())
}

pub(super) fn seat_passenger(
    unit: &mut UnitNode,
    mounts: &HashMap<u64, Gd<Node3D>>,
) -> Result<(), String> {
    let Some(passenger) = &unit.passenger else {
        return Ok(());
    };
    let Some(mount) = mounts.get(&passenger.state.driver) else {
        return Ok(());
    };
    let Some(visual) = unit.visual.as_mut() else {
        return Ok(());
    };
    let id = passenger.definition.attachment;
    let attachment = mount
        .try_get_node_as::<Node3D>(&format!(
            "NpcModel/Skeleton3D/AttachmentBone{id}/Attachment{id}"
        ))
        .ok_or_else(|| {
            format!(
                "VehicleSeat {}: M2 attachment {id} absent",
                passenger.state.seat_id
            )
        })?;
    let local = seat_local_transform(passenger.definition.offset, passenger.definition.rotation);
    let matrix = glam::Mat4::from(local).to_cols_array();
    let basis = Basis::from_cols(
        Vector3::new(matrix[0], matrix[1], matrix[2]),
        Vector3::new(matrix[4], matrix[5], matrix[6]),
        Vector3::new(matrix[8], matrix[9], matrix[10]),
    );
    let offset = Vector3::new(matrix[12], matrix[13], matrix[14]);
    visual
        .set_global_transform(attachment.get_global_transform() * Transform3D::new(basis, offset));
    Ok(())
}

/// The mount model of a mounted unit.
pub(super) struct UnitMount {
    pub display_id: u32,
    /// The model request still loading.
    pub request: Option<u64>,
    /// The loaded mount model, a child of the unit's node.
    pub node: Option<Gd<Node3D>>,
}

/// The mount's clip for the rider's locomotion clip: the same gait on the ground, the
/// flight clip of its direction while flying.
pub(super) fn mount_clip(locomotion: u16, flying: bool) -> u16 {
    if !flying {
        return locomotion;
    }
    match locomotion {
        ANIM_RUN | ANIM_WALK => ANIM_MOUNT_FLIGHT_RUN,
        ANIM_WALK_BACKWARDS => ANIM_MOUNT_FLIGHT_BACKWARDS,
        ANIM_SHUFFLE_LEFT => ANIM_MOUNT_FLIGHT_LEFT,
        ANIM_SHUFFLE_RIGHT => ANIM_MOUNT_FLIGHT_RIGHT,
        _ => ANIM_MOUNT_FLIGHT_IDLE,
    }
}

/// Animate a mounted unit for `locomotion`: the rider holds Mount, the mount moves.
/// `None` for a unit that is not riding a loaded mount.
pub(super) fn animate_mounted(
    unit: &UnitNode,
    locomotion: Locomotion,
    flying: bool,
    fallbacks: &HashMap<u16, u16>,
) -> Option<Result<(), String>> {
    if let Some(passenger) = &unit.passenger {
        let mut rider = unit
            .visual
            .as_ref()?
            .try_get_node_as::<WowAnimationPlayer>("M2Animation")?;
        return Some(
            rider
                .bind_mut()
                .update_locomotion(passenger.definition.animation, false, false)
                .map_err(|error| format!("Passenger {}: {error}", unit.name)),
        );
    }
    let mount = unit.mount.as_ref()?.node.as_ref()?;
    let mut rider = unit
        .visual
        .as_ref()?
        .try_get_node_as::<WowAnimationPlayer>("M2Animation")?;
    let Some(mut animation) = mount.try_get_node_as::<WowAnimationPlayer>("NpcModel/M2Animation")
    else {
        return Some(Err(format!("Mount of {} has no bone animation", unit.name)));
    };
    let clip = mount_clip(locomotion.animation_id, flying);
    let clip = animation
        .bind()
        .resolve_clip(clip, fallbacks)
        .unwrap_or(game_engine_core::movement_animation_data::ANIM_STAND);
    let result = rider
        .bind_mut()
        .update_locomotion(ANIM_MOUNT, false, false)
        .and_then(|()| {
            animation.bind_mut().update_locomotion(
                clip,
                locomotion.jumping && !flying,
                clip == ANIM_RUN,
            )
        });
    Some(result.map_err(|error| format!("Mounted {} clip {clip}: {error}", unit.name)))
}

/// Follow the unit's replicated `Mounted`: a new or changed mount requests its model, a
/// dismount seats the rider back on the ground and frees the mount.
pub(super) fn sync_mount(unit: &mut UnitNode, mounted: Option<&Mounted>, models: &mut WorldModels) {
    let wanted = mounted.map(|mounted| mounted.mount_display_id);
    if unit.mount.as_ref().map(|mount| mount.display_id) == wanted {
        return;
    }
    if let Some(previous) = unit.mount.take() {
        seat_rider(unit);
        if let Some(node) = previous.node {
            node.free();
        }
    }
    unit.mount = wanted.map(|display_id| UnitMount {
        display_id,
        request: Some(models.request(
            &UnitAppearance::Creature {
                display_id,
                items: Default::default(),
            },
            SheathState::Unarmed,
        )),
        node: None,
    });
}

/// Put the unit's visual on its mount's saddle, or on the unit's node when it has no loaded
/// mount. Called every frame, so a re-dressed rider's new visual mounts too.
pub(super) fn seat_rider(unit: &mut UnitNode) {
    if unit.passenger.is_some() {
        return;
    }
    let Some(mut visual) = unit.visual.clone() else {
        return;
    };
    let seat = unit
        .mount
        .as_ref()
        .and_then(|mount| mount.node.as_ref())
        .and_then(|node| node.try_get_node_as::<Node3D>(SADDLE))
        .unwrap_or_else(|| unit.node.clone());
    let seated = visual
        .get_parent()
        .is_some_and(|parent| parent.instance_id() == seat.instance_id());
    if seated {
        return;
    }
    let seat: Gd<Node> = seat.upcast();
    visual
        .reparent_ex(&seat)
        .keep_global_transform(false)
        .done();
    visual.set_transform(Transform3D::IDENTITY);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine_core::movement_animation_data::ANIM_STAND;

    #[test]
    fn a_flying_mount_plays_the_flight_clip_of_the_riders_direction() {
        for (locomotion, clip) in [
            (ANIM_STAND, ANIM_MOUNT_FLIGHT_IDLE),
            (ANIM_RUN, ANIM_MOUNT_FLIGHT_RUN),
            (ANIM_WALK, ANIM_MOUNT_FLIGHT_RUN),
            (ANIM_WALK_BACKWARDS, ANIM_MOUNT_FLIGHT_BACKWARDS),
            (ANIM_SHUFFLE_LEFT, ANIM_MOUNT_FLIGHT_LEFT),
            (ANIM_SHUFFLE_RIGHT, ANIM_MOUNT_FLIGHT_RIGHT),
        ] {
            assert_eq!(mount_clip(locomotion, true), clip, "{locomotion}");
            assert_eq!(mount_clip(locomotion, false), locomotion);
        }
    }
}
