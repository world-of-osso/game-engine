//! Distance and visibility based sampling rate for replicated NPC animation.
//!
//! The `M2AnimPlayer` clock always advances; this only decides on which frames Bevy samples the
//! clips and writes bone transforms. Skipped frames leave joints at their last pose.

#[cfg(test)]
#[path = "lod_tests.rs"]
mod tests;

use bevy::prelude::*;

use super::M2AnimPlayer;
use crate::networking_npc::NpcVisualRoot;
use crate::rendering::camera::WowCamera;

/// NPCs closer than this to the camera sample every frame.
pub(crate) const FULL_RATE_MAX_YARDS: f32 = 30.0;
/// NPCs farther than this from the camera stop sampling even when on screen.
pub(crate) const FROZEN_MIN_YARDS: f32 = 60.0;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnimationLod {
    /// Sample every frame.
    Full,
    /// Sample every other frame, staggered by entity so half the NPCs sample each frame.
    Half,
    /// Do not sample; joints keep their last pose.
    Frozen,
}

impl AnimationLod {
    pub(crate) fn samples_frame(self, frame: u32, owner: Entity) -> bool {
        match self {
            Self::Full => true,
            Self::Half => frame.wrapping_add(owner.index().index()) % 2 == 0,
            Self::Frozen => false,
        }
    }
}

/// `visible` is last frame's frustum result for any of the model's meshes.
pub(crate) fn animation_lod(camera_distance: f32, visible: bool) -> AnimationLod {
    if !visible || camera_distance > FROZEN_MIN_YARDS {
        AnimationLod::Frozen
    } else if camera_distance > FULL_RATE_MAX_YARDS {
        AnimationLod::Half
    } else {
        AnimationLod::Full
    }
}

type NpcModelQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ChildOf,
        &'static GlobalTransform,
        Option<&'static mut AnimationLod>,
    ),
    With<M2AnimPlayer>,
>;

/// Runs before `sync_m2_animation_players`; reads the previous frame's `ViewVisibility`.
pub(crate) fn assign_npc_animation_lod(
    mut commands: Commands,
    camera: Query<&GlobalTransform, With<WowCamera>>,
    mut models: NpcModelQuery,
    npc_roots: Query<(), With<NpcVisualRoot>>,
    mesh_visibility: Query<&ViewVisibility>,
    hierarchy: Query<&Children>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    let camera_position = camera.translation();
    for (model, child_of, transform, lod) in &mut models {
        if !npc_roots.contains(child_of.parent()) {
            continue;
        }
        let visible = hierarchy
            .iter_descendants(model)
            .any(|child| mesh_visibility.get(child).is_ok_and(|v| v.get()));
        let next = animation_lod(transform.translation().distance(camera_position), visible);
        match lod {
            Some(mut lod) => {
                lod.set_if_neq(next);
            }
            None => {
                commands.entity(model).insert(next);
            }
        }
    }
}
