use crate::asset::m2_anim::M2Bone;
use bevy::prelude::*;

pub fn apply_spherical_billboard_stage(stage: Mat4, billboard_world_rotation: Quat) -> Mat4 {
    let x_scale = stage.x_axis.truncate().length();
    let y_scale = stage.y_axis.truncate().length();
    let z_scale = stage.z_axis.truncate().length();
    let mut billboard = Mat4::from_quat(billboard_world_rotation);
    billboard.x_axis *= x_scale;
    billboard.y_axis *= y_scale;
    billboard.z_axis *= z_scale;
    billboard.w_axis = stage.w_axis;
    billboard
}

pub fn propagate_spherical_billboards(bones: &[M2Bone]) -> Vec<bool> {
    bones.iter().map(|bone| bone.flags & 0x8 != 0).collect()
}

fn regular_post_stage(pre_stage: Mat4, rotation: Quat, scale: Vec3, pivot: Vec3) -> Mat4 {
    pre_stage * Mat4::from_quat(rotation) * Mat4::from_scale(scale) * Mat4::from_translation(-pivot)
}

fn billboard_post_stage(
    pre_stage: Mat4,
    billboard_world_rotation: Option<Quat>,
    scale: Vec3,
    pivot: Vec3,
) -> Mat4 {
    let billboard_pre = billboard_world_rotation
        .map(|rotation| apply_spherical_billboard_stage(pre_stage, rotation))
        .unwrap_or(pre_stage);
    billboard_pre * Mat4::from_scale(scale) * Mat4::from_translation(-pivot)
}

/// Compute the pre-billboard and post-billboard world-space matrices for a bone.
///
/// Returns `(parent_idx, pre_stage, post_stage)`.
/// - `pre_stage` is the world matrix up to (and including) the pivot + translation, before rotation/scale.
/// - `post_stage` is the full world matrix after rotation/scale (or billboard rotation for billboard bones).
pub fn compute_bone_stages(
    bones: &[M2Bone],
    spherical_billboards: &[bool],
    bone_idx: usize,
    trans: Vec3,
    rot: Quat,
    scl: Vec3,
    billboard_world_rotation: Option<Quat>,
    pre_billboard_stage: &[Mat4],
    post_billboard_stage: &[Mat4],
) -> Option<(Option<usize>, Mat4, Mat4)> {
    let bone = bones.get(bone_idx)?;
    let pivot = Vec3::new(bone.pivot[0], bone.pivot[2], -bone.pivot[1]);
    let parent_idx = usize::try_from(bone.parent_bone_id).ok();
    let parent_stage = parent_idx
        .and_then(|idx| {
            let parent_billboard = spherical_billboards.get(idx).copied().unwrap_or(false);
            if parent_billboard {
                pre_billboard_stage.get(idx).copied()
            } else {
                post_billboard_stage.get(idx).copied()
            }
        })
        .unwrap_or(Mat4::IDENTITY);
    let pre_stage = parent_stage * Mat4::from_translation(pivot + trans);
    let post_stage = if spherical_billboards.get(bone_idx).copied().unwrap_or(false) {
        billboard_post_stage(pre_stage, billboard_world_rotation, scl, pivot)
    } else {
        regular_post_stage(pre_stage, rot, scl, pivot)
    };
    Some((parent_idx, pre_stage, post_stage))
}
