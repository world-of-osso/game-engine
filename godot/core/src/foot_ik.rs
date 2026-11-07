//! Two-bone foot IK for character skeletons and the leg chains it solves.
//!
//! Retail stairs are drawn with steps over a collision-only ramp, and carpets are drawn
//! above the collision floor, so an animated foot can sink into or float over the visible
//! surface. Movement and the collision capsule stay untouched: this module only computes
//! how to rotate the upper and lower leg so the foot pivot reaches a target, and how far
//! the pelvis would have to drop to reach it.
//!
//! The rig is found from the model itself: attachment points 47 and 48 are the two feet
//! (checked against every local playable character M2), their lowest common ancestor is the
//! pelvis, and the ancestors just under it are the thigh and knee.

use glam::{Quat, Vec3};

/// Attachment lookup ids of the two feet, in model bone space.
pub const FOOT_ATTACHMENTS: [u32; 2] = [47, 48];
/// A leg shorter than this, or a chain with fewer bones, is not a leg rig.
pub const MIN_LEG_BONE: f32 = 0.05;
/// Pivots must descend by at least this much per joint, so a mis-detected chain is rejected.
pub const MIN_DESCENT: f32 = 0.02;
/// The most a solved knee may bend past the animation's own knee angle (30 degrees).
pub const MAX_KNEE_EXTRA_BEND: f32 = std::f32::consts::PI / 6.0;

/// One leg's bone indices in the model skeleton.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegChain {
    /// Thigh (hip pivot).
    pub upper: usize,
    /// Knee (parent of the shin's lower joints).
    pub lower: usize,
    /// Foot attachment bone; the end effector.
    pub end: usize,
    /// Bind-pose height of the foot pivot above the model origin, in model units.
    pub end_height: f32,
}

/// The two detected legs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegRig {
    /// Lowest common ancestor of both feet; lowering it lowers the whole body.
    pub pelvis: usize,
    pub legs: [LegChain; 2],
}

/// Finds the two leg chains from the model's bones and attachment points.
///
/// `attachment_bones` holds the bone index each foot attachment is parented to, in the
/// same order as [`FOOT_ATTACHMENTS`]. A model without both feet, or without a shared
/// pelvis above them, has no rig and keeps its authored pose.
pub fn detect_legs(
    parents: &[i16],
    pivots: &[[f32; 3]],
    attachment_bones: [Option<u16>; 2],
) -> Option<LegRig> {
    let feet = [
        usize::from(attachment_bones[0]?),
        usize::from(attachment_bones[1]?),
    ];
    if feet[0] == feet[1] || feet.iter().any(|&foot| foot >= parents.len()) {
        return None;
    }
    let path = |mut bone: usize| -> Vec<usize> {
        let mut path = vec![bone];
        while let Some(parent) = parents.get(bone).copied().filter(|parent| *parent >= 0) {
            bone = parent as usize;
            path.push(bone);
            if path.len() > parents.len() {
                break;
            }
        }
        path
    };
    let first = path(feet[0]);
    let second = path(feet[1]);
    let pelvis = first.iter().find(|bone| second.contains(bone)).copied()?;
    if pelvis == feet[0] || pelvis == feet[1] {
        return None;
    }
    let leg = |foot: usize, path: &[usize]| -> Option<LegChain> {
        // Chain from the foot up to (excluding) the pelvis: foot, ... ankle ..., knee, thigh.
        let chain: Vec<usize> = path
            .iter()
            .copied()
            .take_while(|bone| *bone != pelvis)
            .collect();
        if chain.len() < 3 {
            return None;
        }
        let upper = *chain.last()?;
        let lower = chain[chain.len() - 2];
        let end = foot;
        let height = |bone: usize| pivots.get(bone).map(|pivot| pivot[1]);
        let (upper_y, lower_y, end_y) = (height(upper)?, height(lower)?, height(end)?);
        if upper_y - lower_y < MIN_DESCENT || lower_y - end_y < MIN_DESCENT {
            return None;
        }
        let length = |a: usize, b: usize| {
            let (a, b) = (pivots.get(a)?, pivots.get(b)?);
            Some(Vec3::new(a[0] - b[0], a[1] - b[1], a[2] - b[2]).length())
        };
        if length(upper, lower)? < MIN_LEG_BONE || length(lower, end)? < MIN_LEG_BONE {
            return None;
        }
        Some(LegChain {
            upper,
            lower,
            end,
            end_height: end_y,
        })
    };
    Some(LegRig {
        pelvis,
        legs: [leg(feet[0], &first)?, leg(feet[1], &second)?],
    })
}

/// Current world-space joint positions of one leg.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegJoints {
    pub upper: Vec3,
    pub lower: Vec3,
    pub end: Vec3,
}

/// The solved rotations for one leg.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegSolution {
    /// World-space rotation to apply about the thigh's pivot.
    pub upper_rotation: Quat,
    /// World-space rotation to apply about the knee, after the thigh's.
    pub lower_rotation: Quat,
    /// Where the foot pivot ends up.
    pub end: Vec3,
    /// Distance past the leg's full reach; the pelvis must drop this much to touch.
    pub overreach: f32,
    /// Whether the target was inside the leg's reach.
    pub reachable: bool,
}

impl LegSolution {
    pub fn identity(leg: LegJoints) -> Self {
        Self {
            upper_rotation: Quat::IDENTITY,
            lower_rotation: Quat::IDENTITY,
            end: leg.end,
            overreach: 0.0,
            reachable: true,
        }
    }
}

/// Solves the two-bone chain `upper -> lower -> end` toward `target`.
///
/// `pole` is the point the knee should bend toward (the animated knee itself, so the
/// authored bend plane is preserved). `weight` blends the target back toward the animated
/// foot, so a partly faded solve cannot snap the leg. The result preserves both bone
/// lengths; an unreachable target stops at full extension and reports the overreach.
///
/// `max_knee_extra` bounds the knee's bend to that many radians past the animated pose's
/// own flexion. A target that would bend the knee further stops at the limit and misses it.
pub fn solve_two_bone(
    leg: LegJoints,
    target: Vec3,
    pole: Vec3,
    weight: f32,
    max_knee_extra: f32,
) -> LegSolution {
    let weight = weight.clamp(0.0, 1.0);
    if weight <= 0.0 {
        return LegSolution::identity(leg);
    }
    let blended = leg.end.lerp(target, weight);
    let upper_length = (leg.lower - leg.upper).length();
    let lower_length = (leg.end - leg.lower).length();
    if upper_length <= MIN_LEG_BONE || lower_length <= MIN_LEG_BONE {
        return LegSolution::identity(leg);
    }
    let to_target = blended - leg.upper;
    let distance = to_target.length();
    if distance <= f32::EPSILON {
        return LegSolution::identity(leg);
    }
    let direction = to_target / distance;
    let full_reach = upper_length + lower_length;
    // The leg can only shorten so far before its knee is bent past the authored angle by
    // more than the limit; a closer target then leaves the foot short of it.
    let knee_min_reach = minimum_reach(
        (leg.upper - leg.lower).normalize_or_zero(),
        (leg.end - leg.lower).normalize_or_zero(),
        upper_length,
        lower_length,
        max_knee_extra,
    );
    let minimum_reach = knee_min_reach.max((upper_length - lower_length).abs() + 1e-4);
    let reach = distance.clamp(minimum_reach, full_reach - 1e-4);
    let overreach = (distance - full_reach).max(0.0);
    // The foot ends `reach` along the target direction: short of a target past the leg's
    // reach, and past a target that would bend the knee beyond its limit.
    let goal = leg.upper + direction * reach;
    // Distance from the thigh pivot to the knee along the target direction.
    let along =
        (upper_length * upper_length - lower_length * lower_length + reach * reach) / (2.0 * reach);
    let height = (upper_length * upper_length - along * along)
        .max(0.0)
        .sqrt();
    let pole_direction = {
        let offset = pole - leg.upper;
        let projected = offset - direction * offset.dot(direction);
        projected.normalize_or_zero()
    };
    let knee = leg.upper + direction * along + pole_direction * height;
    let upper_rotation = rotation_between(leg.lower - leg.upper, knee - leg.upper);
    let bent_lower = knee + upper_rotation * (leg.end - leg.lower);
    let second = rotation_between(bent_lower - knee, goal - knee);
    // The shin's rotation happens after the thigh's, so its own delta is the composition.
    let lower_rotation = second * upper_rotation;
    LegSolution {
        upper_rotation,
        lower_rotation,
        end: knee + second * (bent_lower - knee),
        overreach,
        reachable: distance <= full_reach + 1e-4 && distance >= knee_min_reach - 1e-4,
    }
}

/// Hip-to-foot distance at which the knee is bent `max_knee_extra` radians past the
/// animated knee's own flexion. Shorter targets need a deeper bend than the limit allows.
fn minimum_reach(
    thigh: Vec3,
    shin: Vec3,
    upper_length: f32,
    lower_length: f32,
    max_knee_extra: f32,
) -> f32 {
    let authored_flexion = std::f32::consts::PI - thigh.dot(shin).clamp(-1.0, 1.0).acos();
    let flexion = (authored_flexion + max_knee_extra).clamp(0.0, std::f32::consts::PI);
    (upper_length * upper_length
        + lower_length * lower_length
        + 2.0 * upper_length * lower_length * flexion.cos())
    .max(0.0)
    .sqrt()
}

/// Shortest rotation taking `from` to `to`; identity for a zero or opposite direction.
fn rotation_between(from: Vec3, to: Vec3) -> Quat {
    let (from, to) = (from.normalize_or_zero(), to.normalize_or_zero());
    if from == Vec3::ZERO || to == Vec3::ZERO {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_arc(from, to)
}

/// The vertical offset the whole visible model takes for this frame's foot targets.
///
/// `deltas` holds target minus authored foot height for each leg in yards; a leg the
/// animation lifted has no entry and cannot pull the body down. Both planted feet on one
/// tread share their delta, so the body rises by it instead of the knees bending. The
/// offset is clamped to one step and moves only the visible skeleton: collision, movement
/// and the server position never change.
pub fn root_offset(deltas: [Option<f32>; 2], step: f32) -> f32 {
    let lowest = deltas
        .iter()
        .flatten()
        .copied()
        .fold(f32::INFINITY, f32::min);
    if !lowest.is_finite() {
        return 0.0;
    }
    lowest.clamp(-step, step)
}

/// How far a foot may drift, in yards, and still reuse the ground under it.
///
/// A foot that moved less than this sits over the same surface: an idle sway moves it by
/// millimetres, while a walking foot covers a hand's width per frame. Retire this tolerance
/// if a ground sample is ever reused across a tread edge that shows.
pub const STILL_GROUND_TOLERANCE: f32 = 0.001;

/// Whether a foot may reuse the ground sampled at `sampled` instead of querying again: it
/// did not move horizontally further than [`STILL_GROUND_TOLERANCE`], so it stands over
/// the same surface.
pub fn reuses_ground(sampled: (f32, f32), foot: (f32, f32)) -> bool {
    (foot.0 - sampled.0).abs() <= STILL_GROUND_TOLERANCE
        && (foot.1 - sampled.1).abs() <= STILL_GROUND_TOLERANCE
}

/// Whether a foot belongs to the supported (stance) half of the animation cycle.
/// `min` is the lower of the two animated feet; a foot lifted above it by `lift` is
/// swinging and is left alone.
pub fn is_stance(foot_y: f32, min_foot_y: f32, lift: f32) -> bool {
    foot_y <= min_foot_y + lift
}

/// The knee's bend in `solution`, in radians past straight: 0 is a straight leg.
pub fn knee_flexion(leg: LegJoints, solution: LegSolution) -> f32 {
    let knee = leg.upper + solution.upper_rotation * (leg.lower - leg.upper);
    let thigh = (leg.upper - knee).normalize_or_zero();
    let shin = (solution.end - knee).normalize_or_zero();
    if thigh == Vec3::ZERO || shin == Vec3::ZERO {
        return 0.0;
    }
    std::f32::consts::PI - thigh.dot(shin).clamp(-1.0, 1.0).acos()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Human-female-like skeleton: root, pelvis, two thighs/knees/ankles/feet.
    fn human_bones() -> (Vec<i16>, Vec<[f32; 3]>) {
        let parents = vec![
            -1, // 0 root
            0,  // 1 spine
            1,  // 2 pelvis
            2,  // 3 left thigh
            2,  // 4 right thigh
            3,  // 5 left knee
            4,  // 6 right knee
            5,  // 7 left ankle
            6,  // 8 right ankle
            7,  // 9 left foot
            8,  // 10 right foot
        ];
        // Engine axes: y up, x forward, z sideways (WoW y maps to -z).
        let pivots = vec![
            [0.0, 0.0, 0.0],
            [0.0, 1.22, 0.0],
            [0.02, 1.16, 0.0],
            [0.03, 1.09, 0.09],
            [0.03, 1.09, -0.09],
            [0.03, 0.59, 0.09],
            [0.03, 0.59, -0.09],
            [-0.06, 0.09, 0.09],
            [-0.06, 0.09, -0.09],
            [-0.06, 0.09, 0.09],
            [-0.06, 0.09, -0.09],
        ];
        (parents, pivots)
    }

    #[test]
    fn detects_both_legs_from_the_foot_attachments() {
        let (parents, pivots) = human_bones();
        let rig = detect_legs(&parents, &pivots, [Some(9), Some(10)]).expect("rig");
        assert_eq!(rig.legs[0].upper, 3);
        assert_eq!(rig.legs[0].lower, 5);
        assert_eq!(rig.legs[0].end, 9);
        assert_eq!(rig.legs[1].upper, 4);
        assert_eq!(rig.legs[1].lower, 6);
        assert_eq!(rig.legs[1].end, 10);
        assert!((rig.legs[0].end_height - 0.09).abs() < 1e-5);
    }

    #[test]
    fn a_skeleton_without_both_feet_has_no_rig() {
        let (parents, pivots) = human_bones();
        assert!(detect_legs(&parents, &pivots, [Some(9), None]).is_none());
        assert!(detect_legs(&parents, &pivots, [Some(9), Some(9)]).is_none());
        assert!(detect_legs(&parents, &pivots, [Some(9), Some(11)]).is_none());
    }

    fn joints() -> LegJoints {
        LegJoints {
            upper: Vec3::new(0.0, 1.0, 0.0),
            lower: Vec3::new(0.0, 0.5, 0.0),
            end: Vec3::new(0.0, 0.0, 0.0),
        }
    }

    fn assert_lengths(leg: LegJoints, solution: LegSolution) {
        let knee = leg.upper + solution.upper_rotation * (leg.lower - leg.upper);
        assert!(((leg.lower - leg.upper).length() - (knee - leg.upper).length()).abs() < 1e-4);
        assert!(
            ((leg.end - leg.lower).length() - (solution.end - knee).length()).abs() < 1e-4,
            "shin length changed"
        );
    }

    #[test]
    fn a_reachable_target_puts_the_foot_on_it_and_keeps_bone_lengths() {
        let leg = joints();
        // A nearly straight leg: inside full reach and no more than the extra bend limit.
        let target = Vec3::new(0.2, 0.05, 0.0);
        let solution = solve_two_bone(leg, target, leg.lower, 1.0, MAX_KNEE_EXTRA_BEND);
        assert!(solution.reachable);
        assert_eq!(solution.overreach, 0.0);
        assert!(solution.end.distance(target) < 1e-4, "{:?}", solution.end);
        assert_lengths(leg, solution);
    }

    #[test]
    fn a_weighted_solve_stops_between_the_animated_foot_and_the_target() {
        let leg = joints();
        let target = Vec3::new(0.3, 0.0, 0.0);
        let half = solve_two_bone(leg, target, leg.lower, 0.5, MAX_KNEE_EXTRA_BEND);
        assert!(half.end.x > 0.0 && half.end.x < 0.3);
        let none = solve_two_bone(leg, target, leg.lower, 0.0, MAX_KNEE_EXTRA_BEND);
        assert_eq!(none.upper_rotation, Quat::IDENTITY);
        assert_eq!(none.lower_rotation, Quat::IDENTITY);
        assert_eq!(none.end, leg.end);
    }

    #[test]
    fn an_unreachable_target_reports_the_overreach_and_stays_extended() {
        let leg = joints();
        let target = Vec3::new(0.0, -0.5, 0.0);
        let solution = solve_two_bone(leg, target, leg.lower, 1.0, MAX_KNEE_EXTRA_BEND);
        assert!(!solution.reachable);
        assert!((solution.overreach - 0.5).abs() < 1e-4);
        assert!((solution.end.y + 0.0).abs() < 1e-3);
        assert_lengths(leg, solution);
    }

    /// Two feet on one tread move the whole body up by the tread's delta, so the legs keep
    /// the animated pose instead of crouching to reach the surface.
    #[test]
    fn both_feet_on_one_tread_raise_the_body_with_the_leg_pose_unchanged() {
        let leg = joints();
        let offset = root_offset([Some(0.6), Some(0.6)], 1.6);
        assert!((offset - 0.6).abs() < 1e-5, "offset {offset}");
        let lifted = LegJoints {
            upper: leg.upper + Vec3::Y * offset,
            lower: leg.lower + Vec3::Y * offset,
            end: leg.end + Vec3::Y * offset,
        };
        // The tread is exactly the lifted foot: no residual target, no leg rotation.
        for foot in [leg.end, lifted.end] {
            let residual = foot - Vec3::Y * offset;
            let solution = solve_two_bone(lifted, residual, lifted.lower, 1.0, MAX_KNEE_EXTRA_BEND);
            assert!(
                solution.upper_rotation.angle_between(Quat::IDENTITY) < 1e-3,
                "thigh moved: {:?}",
                solution.upper_rotation
            );
            assert!(solution.lower_rotation.angle_between(Quat::IDENTITY) < 1e-3);
            assert!((solution.end - lifted.end).length() < 1e-3);
            assert_lengths(lifted, solution);
        }
    }

    #[test]
    fn a_lifted_swing_foot_does_not_pull_the_body_down() {
        assert_eq!(root_offset([Some(0.4), None], 1.6), 0.4);
        assert_eq!(root_offset([Some(0.4), Some(0.1)], 1.6), 0.1);
        assert_eq!(root_offset([None, None], 1.6), 0.0);
    }

    #[test]
    fn the_visual_root_offset_is_clamped_to_one_step() {
        assert!((root_offset([Some(4.0), Some(3.0)], 1.6) - 1.6).abs() < 1e-5);
        assert!((root_offset([Some(-4.0), Some(-3.0)], 1.6) + 1.6).abs() < 1e-5);
    }

    /// A knee may bend at most the extra angle it is given; a closer target then falls
    /// short of the foot instead of folding the leg.
    #[test]
    fn the_knee_bends_at_most_the_extra_angle_it_is_given() {
        let leg = joints();
        let target = Vec3::new(0.2, 0.85, 0.0);
        let solution = solve_two_bone(leg, target, leg.lower, 1.0, MAX_KNEE_EXTRA_BEND);
        let flexion = knee_flexion(leg, solution);
        assert!(
            (flexion - MAX_KNEE_EXTRA_BEND).abs() < 1e-3,
            "flexion {flexion} vs limit {MAX_KNEE_EXTRA_BEND}"
        );
        assert!(!solution.reachable);
        assert!(
            solution.end.distance(target) > 0.05,
            "foot reached the target"
        );
        assert_lengths(leg, solution);
    }

    #[test]
    fn the_pole_point_chooses_the_knee_side() {
        let leg = joints();
        let forward = solve_two_bone(
            leg,
            Vec3::new(0.0, 0.4, 0.0),
            leg.upper + Vec3::new(1.0, 0.0, 0.0),
            1.0,
            MAX_KNEE_EXTRA_BEND,
        );
        let backward = solve_two_bone(
            leg,
            Vec3::new(0.0, 0.4, 0.0),
            leg.upper + Vec3::new(-1.0, 0.0, 0.0),
            1.0,
            MAX_KNEE_EXTRA_BEND,
        );
        let knee =
            |solution: LegSolution| leg.upper + solution.upper_rotation * (leg.lower - leg.upper);
        assert!(knee(forward).x > 0.05);
        assert!(knee(backward).x < -0.05);
    }

    #[test]
    fn a_lifted_foot_is_not_in_stance() {
        assert!(is_stance(0.1, 0.1, 0.12));
        assert!(is_stance(0.2, 0.1, 0.12));
        assert!(!is_stance(0.35, 0.1, 0.12));
    }

    /// An idle foot sways by millimetres and keeps the ground it found; a walking foot
    /// covers more than the tolerance in one frame and asks again.
    #[test]
    fn only_a_foot_that_barely_moved_reuses_its_ground() {
        let sampled = (-8564.0, -778.0);
        assert!(reuses_ground(sampled, (sampled.0 + 0.0004, sampled.1 - 0.0003)));
        assert!(reuses_ground(sampled, sampled));
        // 0.7 yd/s at 120 FPS is ~0.006 yd; a run is ~0.058 yd.
        assert!(!reuses_ground(sampled, (sampled.0 + 0.006, sampled.1)));
        assert!(!reuses_ground(sampled, (sampled.0, sampled.1 + 0.058)));
    }
}
