//! Client glue for foot IK: reads the animated leg joints, queries the visible surface
//! under each foot and writes the solved thigh, knee and foot poses back to the skeleton.
//!
//! The character root, the collision capsule and the server position never change: only
//! bone poses are edited, after the animation player has written its frame.

use game_engine_core::foot_ik::{
    LegChain, LegJoints, LegRig, MAX_PELVIS_DROP, is_stance, pelvis_drop, solve_two_bone,
};
use glam::{Quat, Vec3};
use godot::{classes::Skeleton3D, prelude::*};
use shared::ground::STEP_UP_HEIGHT;

/// A foot above the lower foot by more than this is swinging, not planted.
pub const STANCE_LIFT: f32 = 0.12;
/// Ground target smoothing time constant, in seconds: stairs change height abruptly.
const GROUND_SMOOTHING: f32 = 0.07;
/// Blend in/out time constant, in seconds.
const BLEND_SMOOTHING: f32 = 0.12;
/// The most a foot is raised or lowered inside one animation frame before smoothing.
const MAX_TARGET_STEP: f32 = 0.35;

/// Per-unit foot IK state: blend weight and smoothed ground height for each leg, plus the
/// smoothed pelvis drop.
#[derive(Default)]
pub(crate) struct FootIkState {
    legs: [LegState; 2],
    pelvis_drop: f32,
    /// The values of the last solve, for fixture and diagnostic state.
    pub(crate) last: FootSnapshot,
}

/// The last frame's solve values, per foot, in world yards.
#[derive(Default, Clone, Copy)]
pub(crate) struct FootSnapshot {
    /// Blend weight applied to each foot.
    pub weights: [f32; 2],
    /// Smoothed target foot height (visible surface plus the bind-pose foot offset).
    pub targets: [f32; 2],
    /// Animated foot height before the solve.
    pub animated: [f32; 2],
    /// Collision floor under each foot.
    pub floors: [f32; 2],
    /// Smoothed pelvis drop.
    pub pelvis_drop: f32,
}

#[derive(Default)]
struct LegState {
    weight: f32,
    ground: Option<f32>,
}

impl FootIkState {
    /// Applies this frame's foot IK to `skeleton`.
    ///
    /// `enabled` is false while airborne, swimming or flying; the solve then fades out and
    /// the authored pose returns without a snap. `floor_at(x, z)` gives the collision floor
    /// and `surface_at(x, z, ceiling, floor)` the highest visible render surface.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply(
        &mut self,
        skeleton: &mut Gd<Skeleton3D>,
        rig: LegRig,
        model_from_world_scale: f32,
        enabled: bool,
        delta: f32,
        floor_at: &dyn Fn(f32, f32) -> Option<f32>,
        surface_at: &mut dyn FnMut(f32, f32, f32, f32) -> Option<f32>,
    ) {
        // All joint math stays in skeleton (model) space. The skeleton's own placement can
        // be rotated and scaled, so a world-space solve would have to undo that orientation
        // twice; this keeps the bone rotations directly writable.
        let model = skeleton.get_global_transform();
        let world_from_model = model;
        let model_from_world = model.affine_inverse();
        let mut joints = [
            leg_joints(skeleton, rig.legs[0]),
            leg_joints(skeleton, rig.legs[1]),
        ];
        let world_end = |joints: &LegJoints| {
            let end = joints.end;
            let origin = world_from_model * Vector3::new(end.x, end.y, end.z);
            Vec3::new(origin.x, origin.y, origin.z)
        };
        let lowest = world_end(&joints[0]).y.min(world_end(&joints[1]).y);
        // Targets follow the visible surface; a foot the animation lifted stays lifted.
        let mut targets = [Vec3::ZERO; 2];
        let mut weights = [0.0f32; 2];
        for (index, leg) in rig.legs.iter().enumerate() {
            let foot = world_end(&joints[index]);
            let ankle = leg.end_height * model_from_world_scale;
            let desired = surface_target(foot.x, foot.z, ankle, floor_at, surface_at);
            let state = &mut self.legs[index];
            let smoothed = match (state.ground, desired) {
                (Some(previous), Some(next)) => {
                    let next =
                        previous + (next - previous).clamp(-MAX_TARGET_STEP, MAX_TARGET_STEP);
                    previous + (next - previous) * smoothing(delta, GROUND_SMOOTHING)
                }
                (_, Some(next)) => next,
                (previous, None) => previous.unwrap_or(foot.y),
            };
            state.ground = Some(smoothed);
            let planted = is_stance(foot.y, lowest, STANCE_LIFT) || smoothed > foot.y + 0.01;
            let want = if enabled && planted { 1.0 } else { 0.0 };
            state.weight += (want - state.weight) * smoothing(delta, BLEND_SMOOTHING);
            weights[index] = state.weight;
            let model_point = model_from_world * Vector3::new(foot.x, smoothed, foot.z);
            targets[index] = Vec3::new(model_point.x, model_point.y, model_point.z);
        }
        let mut snapshot = FootSnapshot {
            weights,
            targets: [world_end(&joints[0]).y, world_end(&joints[1]).y],
            animated: [world_end(&joints[0]).y, world_end(&joints[1]).y],
            floors: [f32::NAN; 2],
            pelvis_drop: 0.0,
        };
        for index in 0..2 {
            let foot = world_end(&joints[index]);
            snapshot.floors[index] = floor_at(foot.x, foot.z).unwrap_or(f32::NAN);
            let target = targets[index];
            let world = world_from_model * Vector3::new(target.x, target.y, target.z);
            snapshot.targets[index] = world.y;
        }
        let first = solve_two_bone(joints[0], targets[0], joints[0].lower, weights[0]);
        let second = solve_two_bone(joints[1], targets[1], joints[1].lower, weights[1]);
        let wanted_drop = pelvis_drop([&first, &second]);
        self.pelvis_drop += (wanted_drop - self.pelvis_drop) * smoothing(delta, BLEND_SMOOTHING);
        let drop = self.pelvis_drop.clamp(0.0, MAX_PELVIS_DROP);
        snapshot.pelvis_drop = drop;
        if drop > 1e-4 {
            lower_pelvis(skeleton, rig.pelvis, drop, model_from_world_scale);
            joints = [
                leg_joints(skeleton, rig.legs[0]),
                leg_joints(skeleton, rig.legs[1]),
            ];
        }
        self.last = snapshot;
        if weights[0] <= 0.001 && weights[1] <= 0.001 {
            return;
        }
        for (index, leg) in rig.legs.iter().enumerate() {
            if weights[index] <= 0.001 {
                continue;
            }
            let solution = solve_two_bone(
                joints[index],
                targets[index],
                joints[index].lower,
                weights[index],
            );
            apply_leg(
                skeleton,
                *leg,
                solution.upper_rotation,
                solution.lower_rotation,
                weights[index],
            );
        }
    }
}

fn smoothing(delta: f32, time_constant: f32) -> f32 {
    if time_constant <= 0.0 {
        return 1.0;
    }
    1.0 - (-delta.max(0.0) / time_constant).exp()
}

/// The world height a foot pivot should reach: the visible surface plus the bind-pose foot
/// height, or `None` when the client has no render geometry there yet.
fn surface_target(
    x: f32,
    z: f32,
    ankle: f32,
    floor_at: &dyn Fn(f32, f32) -> Option<f32>,
    surface_at: &mut dyn FnMut(f32, f32, f32, f32) -> Option<f32>,
) -> Option<f32> {
    let floor = floor_at(x, z)?;
    let ceiling = floor + STEP_UP_HEIGHT;
    let surface = surface_at(x, z, ceiling, floor)?;
    let target = surface + ankle;
    // A surface more than a step under the animated foot is a fall, not a step.
    (target >= floor - 0.05 && target <= ceiling + ankle).then_some(target)
}

fn leg_joints(skeleton: &Gd<Skeleton3D>, leg: LegChain) -> LegJoints {
    let pose = |index: usize| {
        let origin = skeleton.get_bone_global_pose(index as i32).origin;
        Vec3::new(origin.x, origin.y, origin.z)
    };
    LegJoints {
        upper: pose(leg.upper),
        lower: pose(leg.lower),
        end: pose(leg.end),
    }
}

/// Moves the pelvis down by `drop` world units, in the pelvis's parent bone space.
fn lower_pelvis(skeleton: &mut Gd<Skeleton3D>, pelvis: usize, drop: f32, scale: f32) {
    let parent = skeleton.get_bone_parent(pelvis as i32);
    if parent < 0 {
        return;
    }
    let parent_global = skeleton.get_bone_global_pose(parent);
    let offset =
        parent_global.affine_inverse().basis * Vector3::new(0.0, -drop / scale.max(1e-4), 0.0);
    let mut local = skeleton.get_bone_pose_position(pelvis as i32);
    local += offset;
    skeleton.set_bone_pose_position(pelvis as i32, local);
}

/// Writes the solved world rotations as local bone poses, blended by `weight`.
fn apply_leg(
    skeleton: &mut Gd<Skeleton3D>,
    leg: LegChain,
    upper_rotation: Quat,
    lower_rotation: Quat,
    weight: f32,
) {
    let global_rotation = |index: usize| {
        let basis = skeleton.get_bone_global_pose(index as i32).basis;
        let quaternion = basis.get_quaternion();
        Quat::from_xyzw(quaternion.x, quaternion.y, quaternion.z, quaternion.w)
    };
    let parent = skeleton.get_bone_parent(leg.upper as i32);
    if parent < 0 {
        return;
    }
    let parent_rotation = global_rotation(parent as usize);
    let upper = global_rotation(leg.upper);
    let lower = global_rotation(leg.lower);
    let wanted_upper = upper_rotation * upper;
    let wanted_lower = lower_rotation * lower;
    set_bone_rotation(
        skeleton,
        leg.upper,
        parent_rotation.inverse() * wanted_upper,
        weight,
    );
    set_bone_rotation(
        skeleton,
        leg.lower,
        wanted_upper.inverse() * wanted_lower,
        weight,
    );
}

fn set_bone_rotation(skeleton: &mut Gd<Skeleton3D>, bone: usize, wanted: Quat, weight: f32) {
    let current = skeleton.get_bone_pose_rotation(bone as i32);
    let current = Quat::from_xyzw(current.x, current.y, current.z, current.w);
    let blended = current.slerp(wanted.normalize(), weight);
    let blended = blended.normalize();
    skeleton.set_bone_pose_rotation(
        bone as i32,
        Quaternion::new(blended.x, blended.y, blended.z, blended.w),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(fdid: u32) -> game_engine_core::m2::Model {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
        crate::assets::read_model_file(&root.join(format!("{fdid}.m2"))).expect("model")
    }

    /// Real playable bodies expose both feet through attachment 47/48 and a thigh/knee/foot
    /// chain that descends to the sole.
    #[test]
    fn real_character_models_expose_two_leg_chains() {
        // Human female, human male HD (attachments in the .skel), vrykul male: three
        // different skeletons.
        for fdid in [119_563u32, 1_011_653, 122_815] {
            let rig = crate::animation::detect_leg_rig(&model(fdid))
                .unwrap_or_else(|| panic!("{fdid} has no leg rig"));
            assert_ne!(rig.legs[0].end, rig.legs[1].end, "{fdid}");
            assert_ne!(rig.legs[0].upper, rig.legs[1].upper, "{fdid}");
            for leg in rig.legs {
                assert!(leg.end_height >= 0.0, "{fdid} end height");
            }
        }
    }

    /// A model without the foot attachments keeps its authored pose.
    #[test]
    fn a_model_without_foot_attachments_has_no_rig() {
        let mut model = model(119_563);
        model.attachments.retain(|attachment| attachment.id != 47);
        assert!(crate::animation::detect_leg_rig(&model).is_none());
    }
}
