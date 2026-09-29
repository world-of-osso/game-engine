//! Missile flight at `SpellMisc.Speed`.
use super::missile_step;
use godot::builtin::Vector3;

const FRAME: f32 = 1.0 / 60.0;

/// Frames a missile takes from `start` to `goal` at `speed` yd/s, 60 fps.
fn frames_to_arrive(start: Vector3, goal: Vector3, speed: f32) -> usize {
    let mut position = start;
    for frame in 1..10_000 {
        match missile_step(position, goal, speed * FRAME) {
            Some(next) => position = next,
            None => return frame,
        }
    }
    panic!("the missile never arrives");
}

/// Frostbolt (35 yd/s) over 6.9 yards takes 0.197 s: 12 frames at 60 fps, the last
/// one partial; 13.9 yards takes 0.397 s, 24 frames.
#[test]
fn travel_time_is_distance_over_speed() {
    let start = Vector3::new(1.0, 2.0, 3.0);
    let direction = Vector3::new(2.0, 1.0, -2.0).normalized();
    assert_eq!(frames_to_arrive(start, start + direction * 6.9, 35.0), 12);
    assert_eq!(frames_to_arrive(start, start + direction * 13.9, 35.0), 24);
}

/// Each step closes `step` yards along the straight line to the goal.
#[test]
fn a_step_moves_straight_at_the_goal() {
    let start = Vector3::new(0.0, 1.0, 0.0);
    let goal = Vector3::new(3.0, 1.0, 4.0);
    let next = missile_step(start, goal, 1.0).unwrap();
    assert!((next - Vector3::new(0.6, 1.0, 0.8)).length() < 1e-5);
    assert_eq!(missile_step(start, goal, 5.0), None);
}
