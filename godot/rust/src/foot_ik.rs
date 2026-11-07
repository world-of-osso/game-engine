//! Client glue for foot IK: reads the animated leg joints, queries the visible surface
//! under each foot and writes the solved thigh, knee and foot poses back to the skeleton.
//!
//! The unit node, the collision capsule and the server position never change. The whole
//! visible skeleton takes one vertical offset so the lower planted foot reaches its surface,
//! and only the difference between the two feet bends a leg.

use game_engine_core::foot_ik::{
    LegChain, LegJoints, LegRig, LegSolution, MAX_KNEE_EXTRA_BEND, is_stance, knee_flexion,
    reuses_ground, root_offset, solve_two_bone,
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
/// smoothed visual root offset.
#[derive(Default)]
pub(crate) struct FootIkState {
    legs: [LegState; 2],
    root_offset: f32,
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
    /// Animated foot height before the solve, at the server position.
    pub animated: [f32; 2],
    /// Collision floor under each foot.
    pub floors: [f32; 2],
    /// Vertical offset applied to the visible skeleton this frame.
    pub root_offset: f32,
    /// Foot pivot height after the solve, in world yards.
    pub solved: [f32; 2],
    /// Knee bend after the solve, in radians past straight.
    pub knee_bend: [f32; 2],
    /// Knee bend of the animated pose before the solve, in radians past straight.
    pub knee_rest: [f32; 2],
}

/// The ground under one foot as the last full query found it: where the foot was, the
/// height the foot settles toward and the collision floor it was measured from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FootGround {
    x: f32,
    z: f32,
    target: f32,
    floor: f32,
}

#[derive(Default)]
struct LegState {
    weight: f32,
    ground: Option<f32>,
    /// The last full ground query, reused while the foot stands on it unchanged.
    sample: Option<FootGround>,
    /// Whether `sample` is this pose's result: a foot that found no ground still counts as
    /// sampled, so it is not asked again while its epoch holds.
    sampled: bool,
    /// The ground epoch `sample` was read from.
    epoch: u64,
}

impl FootIkState {
    /// Applies this frame's foot IK to `skeleton`.
    ///
    /// `enabled` is false while airborne, swimming or flying; the solve then fades out and
    /// the authored pose returns without a snap. `floor_at(x, z)` gives the collision floor
    /// and `surface_at(x, z, ceiling, floor)` the highest visible render surface.
    ///
    /// `query_ground` false reuses the ground each foot last found instead of asking again:
    /// a distant unit redraws its solve from the same surface while the queries it would
    /// spend are not worth a pixel. The solve, the blend and the pelvis offset run
    /// unchanged, so the pose never pops. A foot also reuses its own sample while it has
    /// not moved and `ground_epoch` still names the terrain and scenery it was read from.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply(
        &mut self,
        skeleton: &mut Gd<Skeleton3D>,
        rig: LegRig,
        model_from_world_scale: f32,
        enabled: bool,
        delta: f32,
        query_ground: bool,
        ground_epoch: u64,
        floor_at: &dyn Fn(f32, f32) -> Option<f32>,
        surface_at: &mut dyn FnMut(f32, f32, f32, f32) -> Option<f32>,
    ) {
        // All joint math stays in skeleton (model) space. The skeleton's own placement can
        // be rotated and scaled, so a world-space solve would have to undo that orientation
        // twice; this keeps the bone rotations directly writable. The caller restored the
        // sampled pose first when the animation was not rewriting it, so the joints below
        // are the authored pose, with the server position as the world anchor.
        let model = skeleton.get_global_transform();
        let world_from_model = model;
        let model_from_world = model.affine_inverse();
        let authored = [
            leg_joints(skeleton, rig.legs[0]),
            leg_joints(skeleton, rig.legs[1]),
        ];
        let mut joints = authored;
        let world_end = |joints: &LegJoints| {
            let end = joints.end;
            let origin = world_from_model * Vector3::new(end.x, end.y, end.z);
            Vec3::new(origin.x, origin.y, origin.z)
        };
        let lowest = world_end(&joints[0]).y.min(world_end(&joints[1]).y);
        // Targets follow the visible surface; a foot the animation lifted stays lifted.
        let mut targets = [Vec3::ZERO; 2];
        let mut deltas: [Option<f32>; 2] = [None; 2];
        let mut weights = [0.0f32; 2];
        let mut floors = [f32::NAN; 2];
        for (index, leg) in rig.legs.iter().enumerate() {
            let foot = world_end(&joints[index]);
            let ankle = leg.end_height * model_from_world_scale;
            let state = &mut self.legs[index];
            // A foot that has not moved over the same world keeps the ground it found,
            // whether or not the caller would have asked this frame. One that did move
            // asks only when its distance stride is due, which is what a distant foot
            // saves; an unsampled foot or a changed world always asks.
            let moved = state
                .sample
                .is_some_and(|sample| !reuses_ground((sample.x, sample.z), (foot.x, foot.z)));
            let reuse = state.sampled && state.epoch == ground_epoch && !(moved && query_ground);
            if !reuse {
                state.sample = surface_target(foot.x, foot.z, ankle, floor_at, surface_at).map(
                    |(target, floor)| FootGround {
                        x: foot.x,
                        z: foot.z,
                        target,
                        floor,
                    },
                );
                state.sampled = true;
                state.epoch = ground_epoch;
            }
            let (desired, floor) = match state.sample {
                Some(sample) => (Some(sample.target), sample.floor),
                None => (None, f32::NAN),
            };
            floors[index] = floor;
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
            // Only a planted foot asks the body to rise or sink. A swing foot is deliberately
            // above or below its surface and must not drag the whole body with it.
            if enabled && planted {
                deltas[index] = Some(smoothed - foot.y);
            }
            let model_point = model_from_world * Vector3::new(foot.x, smoothed, foot.z);
            targets[index] = Vec3::new(model_point.x, model_point.y, model_point.z);
        }
        // One visual offset for both legs: the lower foot's delta, clamped to one step. With
        // both feet on one tread the legs keep the animated pose; on two treads only their
        // difference bends a leg.
        let wanted_offset = root_offset(deltas, STEP_UP_HEIGHT);
        self.root_offset += (wanted_offset - self.root_offset) * smoothing(delta, BLEND_SMOOTHING);
        let offset = self.root_offset.clamp(-STEP_UP_HEIGHT, STEP_UP_HEIGHT);
        // The pelvis carries the whole body: raising or lowering it is the visual root
        // offset, and the leg joints move with it. The target keeps its place on the
        // surface, so hip and foot rise together and the leg pose is unchanged; only the
        // difference between the two feet remains to solve.
        offset_pelvis(skeleton, rig.pelvis, offset, model_from_world_scale);
        joints = [
            leg_joints(skeleton, rig.legs[0]),
            leg_joints(skeleton, rig.legs[1]),
        ];
        let mut snapshot = FootSnapshot {
            weights,
            targets: [0.0; 2],
            animated: [world_end(&authored[0]).y, world_end(&authored[1]).y],
            floors,
            root_offset: offset,
            solved: [world_end(&joints[0]).y, world_end(&joints[1]).y],
            knee_bend: [
                knee_flexion(authored[0], LegSolution::identity(authored[0])),
                knee_flexion(authored[1], LegSolution::identity(authored[1])),
            ],
            knee_rest: [
                knee_flexion(authored[0], LegSolution::identity(authored[0])),
                knee_flexion(authored[1], LegSolution::identity(authored[1])),
            ],
        };
        for index in 0..2 {
            let target = world_from_model
                * Vector3::new(targets[index].x, targets[index].y, targets[index].z);
            snapshot.targets[index] = target.y;
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
                MAX_KNEE_EXTRA_BEND,
            );
            apply_leg(
                skeleton,
                *leg,
                solution.upper_rotation,
                solution.lower_rotation,
                weights[index],
            );
            let foot =
                world_from_model * Vector3::new(solution.end.x, solution.end.y, solution.end.z);
            self.last.solved[index] = foot.y;
            self.last.knee_bend[index] = knee_flexion(joints[index], solution);
        }
    }
}

fn smoothing(delta: f32, time_constant: f32) -> f32 {
    if time_constant <= 0.0 {
        return 1.0;
    }
    1.0 - (-delta.max(0.0) / time_constant).exp()
}

/// The world height a foot pivot should reach, with the collision floor it was measured
/// from: the visible surface plus the bind-pose foot height, or `None` when the client has
/// no render geometry there yet.
fn surface_target(
    x: f32,
    z: f32,
    ankle: f32,
    floor_at: &dyn Fn(f32, f32) -> Option<f32>,
    surface_at: &mut dyn FnMut(f32, f32, f32, f32) -> Option<f32>,
) -> Option<(f32, f32)> {
    let floor = floor_at(x, z)?;
    let ceiling = floor + STEP_UP_HEIGHT;
    let surface = surface_at(x, z, ceiling, floor)?;
    let target = surface + ankle;
    // A surface more than a step under the animated foot is a fall, not a step.
    (target >= floor - 0.05 && target <= ceiling + ankle).then_some((target, floor))
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

/// Raises or lowers the pelvis, and with it every bone the body hangs from, by `offset`
/// yards along world up. This is the visual root offset: the unit node, its collision and
/// the server position stay where they are.
fn offset_pelvis(skeleton: &mut Gd<Skeleton3D>, pelvis: usize, offset: f32, scale: f32) {
    let parent = skeleton.get_bone_parent(pelvis as i32);
    if parent < 0 {
        return;
    }
    let parent_global = skeleton.get_bone_global_pose(parent);
    let local =
        parent_global.affine_inverse().basis * Vector3::new(0.0, offset / scale.max(1e-4), 0.0);
    let mut position = skeleton.get_bone_pose_position(pelvis as i32);
    position += local;
    skeleton.set_bone_pose_position(pelvis as i32, position);
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
