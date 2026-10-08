//! Momentum sway on the HumanMale HD model: frame-rate independence, the swing cap,
//! the stop kick's direction and an untouched pose without velocity changes.
use super::AnimationState;
use super::momentum::{MAX_SWING, MomentumSway, VelocityTracker};
use super::npc_pose_tests::{human_male_hd, human_male_model, pose_distance};
use godot::builtin::{Basis, Transform3D, Vector3};

const STAND: u16 = 0;
/// Human run speed (yd/s).
const RUN_SPEED: f32 = 7.0;
/// Human walk speed (yd/s).
const WALK_SPEED: f32 = 2.5;
const FRAME_MS: f64 = 1000.0 / 60.0;
/// M2 key bones (wowdev.wiki M2 KeyBone): ArmL/ArmR shoulder joints, ThumbL/ThumbR.
const ARM_L: i32 = 0;
const ARM_R: i32 = 1;
const THUMB_L: i32 = 17;
const THUMB_R: i32 = 12;

/// Skeleton-space velocity at `frame` of a `fps` walk: forward from t = 0, turned to
/// the left at 1/6 s, stopped at 1/3 s (boundaries fall on frames at 30 and 144 fps).
/// It stays below the cap, where the spring is linear.
fn scripted_velocity(frame: usize, fps: usize) -> Vector3 {
    match frame * 6 / fps {
        0 => Vector3::new(WALK_SPEED, 0.0, 0.0),
        1 => Vector3::new(0.0, 0.0, -WALK_SPEED),
        _ => Vector3::ZERO,
    }
}

fn sway_after_half_second(fps: usize) -> MomentumSway {
    let mut sway = MomentumSway::new(human_male_model());
    let mut previous = Vector3::ZERO;
    for frame in 0..fps / 2 {
        let velocity = scripted_velocity(frame, fps);
        sway.kick(velocity - previous);
        previous = velocity;
        sway.advance(1000.0 / fps as f64);
    }
    sway
}

#[test]
fn spring_state_is_the_same_at_30_and_144_fps() {
    let slow = sway_after_half_second(30);
    let fast = sway_after_half_second(144);
    // Still swinging 1/6 s after the stop.
    assert!(
        slow.angle.length() > 1.0_f32.to_radians(),
        "{:?}",
        slow.angle
    );
    assert!(
        (slow.angle - fast.angle).length() < 1e-5,
        "{:?} vs {:?}",
        slow.angle,
        fast.angle
    );
    assert!(
        (slow.rate - fast.rate).length() < 1e-4,
        "{:?} vs {:?}",
        slow.rate,
        fast.rate
    );
}

fn bone_with_key(key_bone_id: i32) -> usize {
    human_male_model()
        .bones
        .iter()
        .position(|bone| bone.key_bone_id == key_bone_id)
        .expect("HumanMale HD key bone")
}

fn local_angle(a: Transform3D, b: Transform3D) -> f32 {
    let a = a.basis.get_quaternion();
    let b = b.basis.get_quaternion();
    2.0 * a.dot(b).abs().min(1.0).acos()
}

#[test]
fn swing_never_exceeds_the_cap_under_huge_velocity_changes() {
    let mut swaying = human_male_hd();
    let mut still = human_male_hd();
    let shoulder = bone_with_key(ARM_L);
    let mut largest = 0.0f32;
    for frame in 0..120 {
        // A 3000 yd/s jolt (a teleport), alternating direction every 20 frames.
        let sign = if (frame / 20) % 2 == 0 { 1.0 } else { -1.0 };
        swaying.add_velocity_change(Vector3::new(3000.0 * sign, 0.0, -1000.0));
        swaying.advance(FRAME_MS).unwrap();
        still.advance(FRAME_MS).unwrap();
        let swing = swaying.momentum.angle.length();
        assert!(swing <= MAX_SWING + 1e-6, "frame {frame}: {swing}");
        let bone = local_angle(swaying.poses()[shoulder], still.poses()[shoulder]);
        assert!(bone <= MAX_SWING + 1e-4, "frame {frame}: shoulder {bone}");
        largest = largest.max(swing);
    }
    assert!(
        largest > MAX_SWING - 1e-4,
        "never reached the cap: {largest}"
    );
}

fn model_transforms(player: &AnimationState) -> Vec<Transform3D> {
    let mut models: Vec<Transform3D> = Vec::new();
    for (index, local) in player.poses().into_iter().enumerate() {
        let parent = human_male_model().bones[index].parent_bone_id;
        let model = if parent < 0 {
            local
        } else {
            models[parent as usize] * local
        };
        models.push(model);
    }
    models
}

/// The child of `shoulder` on the path up from key bone `thumb`.
fn elbow_below(shoulder: usize, thumb: i32) -> usize {
    let bones = &human_male_model().bones;
    let mut bone = bone_with_key(thumb);
    while bones[bone].parent_bone_id as usize != shoulder {
        bone = bones[bone].parent_bone_id as usize;
    }
    bone
}

#[test]
fn stopping_swings_the_shoulders_forward_about_the_lateral_axis() {
    let mut swaying = human_male_hd();
    let mut still = human_male_hd();
    swaying.update_locomotion(STAND, false, false).unwrap();
    still.update_locomotion(STAND, false, false).unwrap();
    // A run stopped dead: 7 yd/s forward (model +X) lost in one frame.
    swaying.add_velocity_change(Vector3::new(-RUN_SPEED, 0.0, 0.0));
    for _ in 0..4 {
        swaying.advance(FRAME_MS).unwrap();
        still.advance(FRAME_MS).unwrap();
    }
    let swayed = model_transforms(&swaying);
    let resting = model_transforms(&still);
    for (arm, thumb) in [(ARM_L, THUMB_L), (ARM_R, THUMB_R)] {
        let shoulder = bone_with_key(arm);
        let elbow = elbow_below(shoulder, thumb);
        let upper_arm = |models: &[Transform3D]| models[elbow].origin - models[shoulder].origin;
        let (from, to) = (upper_arm(&resting), upper_arm(&swayed));
        let angle = from.angle_to(to).to_degrees();
        assert!((5.0..=12.0).contains(&angle), "arm {arm}: {angle}°");
        // The elbow swings forward (model +X) about the lateral axis; a bend axis left
        // in skeleton space would twist it sideways (model Z).
        let moved = to - from;
        assert!(moved.x > 0.02, "arm {arm}: elbow moved {moved:?}");
        assert!(
            moved.z.abs() < 0.25 * moved.x,
            "arm {arm}: elbow moved {moved:?}"
        );
    }
}

#[test]
fn zero_velocity_change_leaves_the_pose_unchanged() {
    let mut swaying = human_male_hd();
    let mut still = human_male_hd();
    for _ in 0..30 {
        swaying.add_velocity_change(Vector3::ZERO);
        swaying.advance(FRAME_MS).unwrap();
        still.advance(FRAME_MS).unwrap();
    }
    assert!(!swaying.momentum.active());
    assert_eq!(pose_distance(&swaying.poses(), &still.poses()), 0.0);
}

#[test]
fn a_stop_while_facing_world_north_kicks_skeleton_backwards() {
    // A 1.3-scaled unit turned so model +X (forward) points world -Z, running there
    // at 7 yd/s while climbing 1 yd/s, then stopping; frames are 1/60 s.
    let basis = Basis::from_axis_angle(Vector3::UP, std::f32::consts::FRAC_PI_2)
        .scaled(Vector3::splat(1.3));
    let step = 1.0 / 60.0;
    let mut tracker = VelocityTracker::default();
    let mut origin = Vector3::new(10.0, 5.0, 20.0);
    let mut changes = Vec::new();
    for frame in 0..5 {
        changes.push(tracker.change(Transform3D::new(basis, origin), step));
        if frame < 2 {
            origin += Vector3::new(0.0, 1.0, -RUN_SPEED) * step;
        }
    }
    assert_eq!(changes[0], None);
    let kick = |frame: usize| changes[frame].expect("tracked frame");
    // The start is the first measured speed, the stop its loss; no kicks in between
    // or after, and the climb never reaches the horizontal swing.
    assert!(
        (kick(1) - Vector3::new(RUN_SPEED, 0.0, 0.0)).length() < 1e-2,
        "{:?}",
        kick(1)
    );
    assert!(kick(2).length() < 1e-2, "{:?}", kick(2));
    assert!(
        (kick(3) - Vector3::new(-RUN_SPEED, 0.0, 0.0)).length() < 1e-2,
        "{:?}",
        kick(3)
    );
    assert!(kick(4).length() < 1e-6, "{:?}", kick(4));
}
