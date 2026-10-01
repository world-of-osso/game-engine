//! Missile flight at `SpellMisc.Speed`, and where a late kit model joins its timeline.
use super::{EffectClips, Phase, missile_step, phase_at};
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

/// A kit model with a 1 s Stand start, a 0.5 s Decay end and particles living 0.3 s.
fn clips(end: bool) -> EffectClips {
    EffectClips {
        start: (0, 1.0),
        hold: Some(158),
        end: end.then_some((159, 0.5)),
        particle_tail: 0.3,
    }
}

fn close(phase: Phase, expected: Phase) -> bool {
    match (phase, expected) {
        (Phase::Start(a), Phase::Start(b))
        | (Phase::End(a), Phase::End(b))
        | (Phase::Tail(a), Phase::Tail(b)) => (a - b).abs() < 1e-5,
        (a, b) => a == b,
    }
}

/// An on-time model starts its start clip from its beginning.
#[test]
fn an_on_time_model_plays_its_start_clip_from_the_start() {
    let (phase, clip) = phase_at(&clips(true), false, 0.0).unwrap();
    assert!(close(phase, Phase::Start(1.0)));
    assert_eq!(clip, Some((0, false, 0.0)));
}

/// A one-shot model loaded 0.4 s late joins 0.4 s into its start clip with 0.6 s left;
/// 1.2 s late it is 0.2 s into its end clip; 1.6 s late only its particle tail is left.
#[test]
fn a_late_one_shot_joins_its_timeline_where_the_kit_is() {
    let (phase, clip) = phase_at(&clips(true), false, 0.4).unwrap();
    assert!(close(phase, Phase::Start(0.6)));
    assert_eq!(clip.map(|(id, looping, _)| (id, looping)), Some((0, false)));
    assert!((clip.unwrap().2 - 0.4).abs() < 1e-5);

    let (phase, clip) = phase_at(&clips(true), false, 1.2).unwrap();
    assert!(close(phase, Phase::End(0.3)));
    assert_eq!(
        clip.map(|(id, looping, _)| (id, looping)),
        Some((159, false))
    );
    assert!((clip.unwrap().2 - 0.2).abs() < 1e-5);

    let (phase, clip) = phase_at(&clips(true), false, 1.6).unwrap();
    assert!(close(phase, Phase::Tail(0.2)));
    assert_eq!(clip, None);
}

/// A one-shot loaded after its start, end and tail (1.8 s) is not shown; without an
/// end clip it is over after start and tail (1.3 s).
#[test]
fn a_one_shot_loaded_after_its_kit_ended_is_not_shown() {
    assert_eq!(phase_at(&clips(true), false, 1.81), None);
    assert_eq!(phase_at(&clips(true), false, 5.0), None);
    assert!(phase_at(&clips(false), false, 1.25).is_some());
    assert_eq!(phase_at(&clips(false), false, 1.31), None);
}

/// A held kit's model (a precast's hands) loaded 2.5 s into the cast loops its Hold
/// clip, 1.5 s into it, and never times out on its own.
#[test]
fn a_late_held_model_joins_its_hold_loop() {
    let (phase, clip) = phase_at(&clips(true), true, 2.5).unwrap();
    assert_eq!(phase, Phase::Hold);
    assert_eq!(
        clip.map(|(id, looping, _)| (id, looping)),
        Some((158, true))
    );
    assert!((clip.unwrap().2 - 1.5).abs() < 1e-5);
    let no_hold = EffectClips {
        hold: None,
        ..clips(true)
    };
    let (_, clip) = phase_at(&no_hold, true, 60.0).unwrap();
    assert_eq!(clip.map(|(id, looping, _)| (id, looping)), Some((0, true)));
}
