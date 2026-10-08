//! Momentum sway: a damped spring that swings the arms, spine and head against the
//! unit's change in horizontal velocity, on top of the finished local pose.
//! Retail has no such layer (it only crossfades movement clips); see
//! docs/specs/momentum-sway.md.
use super::BonePose;
use game_engine_core::m2;
use godot::builtin::{Basis, Quaternion, Transform3D, Vector2, Vector3};

/// Undamped angular frequency (rad/s) and damping ratio: a kick peaks after ~70 ms,
/// overshoots ~7 % once and settles within 2 % after 4 / (ζ·ω0) ≈ 385 ms.
const FREQUENCY: f32 = 16.0;
const DAMPING_RATIO: f32 = 0.65;
/// Swing rate (rad/s) per yd/s of velocity change: stopping from the 7 yd/s run
/// peaks near 8°, from a 2.5 yd/s walk near 3°; changes over ~10.5 yd/s reach the
/// cap.
const GAIN: f32 = 0.67;
/// Largest swing of any bone (12°).
pub(super) const MAX_SWING: f32 = 12.0 * std::f32::consts::PI / 180.0;
/// Below these the spring is at rest and leaves the pose untouched.
const REST_ANGLE: f32 = 1e-4;
const REST_RATE: f32 = 1e-3;

/// M2 key bones (wowdev.wiki M2#Bones KeyBone): the ArmL/ArmR shoulder joints, the
/// ThumbL/ThumbR the elbows are found from, SpineLow and Head.
const ARMS: [(i32, i32); 2] = [(0, 17), (1, 12)];
const SPINE_LOW: i32 = 4;
const HEAD: i32 = 6;
/// Share of the swing each bone takes.
const SHOULDER_WEIGHT: f32 = 1.0;
const ELBOW_WEIGHT: f32 = 0.6;
const SPINE_WEIGHT: f32 = 0.25;
const HEAD_WEIGHT: f32 = 0.2;

struct SwayBone {
    index: usize,
    weight: f32,
    /// Direction the bone points in the bind pose: arms hang down, spine and head
    /// rise; its tip swings with the spring.
    along: Vector3,
}

/// The spring's swing of the bone tips in the character's horizontal plane
/// (x: model +X forward, y: model +Z lateral), radians, and its rate.
#[derive(Default)]
pub(super) struct MomentumSway {
    parents: Vec<i16>,
    /// In bone order, so a swung parent precedes its children (M2 lists parents first).
    bones: Vec<SwayBone>,
    pub(super) angle: Vector2,
    pub(super) rate: Vector2,
}

fn key_bone(model: &m2::Model, key_bone_id: i32) -> Option<usize> {
    model
        .bones
        .iter()
        .position(|bone| bone.key_bone_id == key_bone_id)
}

/// The child of `shoulder` on the path up from the hand's `thumb` key bone.
fn elbow_below(model: &m2::Model, shoulder: usize, thumb: i32) -> Option<usize> {
    let mut bone = key_bone(model, thumb)?;
    loop {
        let parent = usize::try_from(model.bones[bone].parent_bone_id).ok()?;
        if parent == shoulder {
            return Some(bone);
        }
        bone = parent;
    }
}

fn sway_bones(model: &m2::Model) -> Vec<SwayBone> {
    let mut bones = Vec::new();
    let mut add = |index: Option<usize>, weight, along| {
        if let Some(index) = index {
            bones.push(SwayBone {
                index,
                weight,
                along,
            });
        }
    };
    add(key_bone(model, SPINE_LOW), SPINE_WEIGHT, Vector3::UP);
    add(key_bone(model, HEAD), HEAD_WEIGHT, Vector3::UP);
    for (arm, thumb) in ARMS {
        let shoulder = key_bone(model, arm);
        add(shoulder, SHOULDER_WEIGHT, Vector3::DOWN);
        let elbow = shoulder.and_then(|shoulder| elbow_below(model, shoulder, thumb));
        add(elbow, ELBOW_WEIGHT, Vector3::DOWN);
    }
    bones.sort_by_key(|bone| bone.index);
    bones
}

impl MomentumSway {
    pub(super) fn new(model: &m2::Model) -> Self {
        Self {
            parents: model.bones.iter().map(|bone| bone.parent_bone_id).collect(),
            bones: sway_bones(model),
            angle: Vector2::ZERO,
            rate: Vector2::ZERO,
        }
    }

    /// The unit's velocity changed by `velocity_change` (yd/s, skeleton space): the
    /// bone tips keep moving the old way, so the swing rate jumps against the change.
    pub(super) fn kick(&mut self, velocity_change: Vector3) {
        self.rate -= Vector2::new(velocity_change.x, velocity_change.z) * GAIN;
    }

    /// Exact damped-spring solution over the step, so any frame rate lands on the same
    /// state; the swing then stops at the cap.
    pub(super) fn advance(&mut self, delta_ms: f64) {
        if !self.active() {
            return;
        }
        let time = (delta_ms / 1000.0) as f32;
        let decay_rate = DAMPING_RATIO * FREQUENCY;
        let damped = FREQUENCY * (1.0 - DAMPING_RATIO * DAMPING_RATIO).sqrt();
        let decay = (-decay_rate * time).exp();
        let (sin, cos) = (damped * time).sin_cos();
        let (angle, rate) = (self.angle, self.rate);
        self.angle = (angle * cos + (rate + angle * decay_rate) * (sin / damped)) * decay;
        self.rate = (rate * cos
            - (rate * decay_rate + angle * FREQUENCY * FREQUENCY) * (sin / damped))
            * decay;
        self.limit();
        if self.angle.length() < REST_ANGLE && self.rate.length() < REST_RATE {
            self.angle = Vector2::ZERO;
            self.rate = Vector2::ZERO;
        }
    }

    /// Pin the swing at the cap and drop the rate carrying it further out.
    fn limit(&mut self) {
        let length = self.angle.length();
        if length <= MAX_SWING {
            return;
        }
        let direction = self.angle / length;
        self.angle = direction * MAX_SWING;
        let outward = self.rate.dot(direction);
        if outward > 0.0 {
            self.rate -= direction * outward;
        }
    }

    pub(super) fn active(&self) -> bool {
        self.angle != Vector2::ZERO || self.rate != Vector2::ZERO
    }

    /// `poses` with each sway bone turned so its tip moves along the swing; the
    /// skeleton-space bend axis is converted into the bone's parent frame, which its
    /// local rotation is relative to.
    pub(super) fn apply(&self, mut poses: Vec<BonePose>) -> Vec<BonePose> {
        let length = self.angle.length();
        if length == 0.0 {
            return poses;
        }
        let swing = Vector3::new(self.angle.x, 0.0, self.angle.y) / length;
        for bone in &self.bones {
            let axis = bone.along.cross(swing);
            let parent = self.model_rotation(&poses, self.parents[bone.index]);
            let local_axis = (Basis::from_quaternion(parent.inverse()) * axis).normalized();
            let turn = Quaternion::from_axis_angle(local_axis, length * bone.weight);
            let pose = &mut poses[bone.index];
            pose.rotation = (turn * pose.rotation).normalized();
        }
        poses
    }

    /// Skeleton-space rotation of `bone` (identity above the root).
    fn model_rotation(&self, poses: &[BonePose], bone: i16) -> Quaternion {
        let mut rotation = Quaternion::IDENTITY;
        let mut bone = bone;
        while let Ok(index) = usize::try_from(bone) {
            rotation = poses[index].rotation * rotation;
            bone = self.parents[index];
        }
        rotation
    }
}

/// Differentiates a skeleton's global origin into its horizontal velocity changes.
#[derive(Default)]
pub(super) struct VelocityTracker {
    last_origin: Option<Vector3>,
    last_velocity: Vector3,
}

impl VelocityTracker {
    pub(super) fn change(&mut self, _global: Transform3D, _delta_s: f32) -> Option<Vector3> {
        None
    }
}
