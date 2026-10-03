use super::*;

#[test]
fn next_step_clips_to_requested_duration() {
    let mut movement = ScriptedMovement::default();
    movement.start(0.25, Some(90.0)).unwrap();

    let first = movement.next_step(0.1).unwrap();
    let second = movement.next_step(0.1).unwrap();
    let last = movement.next_step(0.1).unwrap();

    assert_eq!(first.duration_secs, 0.1);
    assert_eq!(second.duration_secs, 0.1);
    assert!((last.duration_secs - 0.05).abs() < f32::EPSILON);
    assert_eq!(first.facing_yaw, Some(std::f32::consts::FRAC_PI_2));
    assert!(movement.next_step(0.1).is_none());
}

#[test]
fn stop_discards_remaining_scripted_movement() {
    let mut movement = ScriptedMovement::default();
    movement.start(1.0, None).unwrap();
    movement.stop();

    assert!(movement.next_step(0.1).is_none());
}

#[test]
fn invalid_start_preserves_active_scripted_movement() {
    let mut movement = ScriptedMovement::default();
    movement.start(1.0, Some(450.0)).unwrap();

    assert!(movement.start(0.0, None).is_err());
    assert!(movement.start(60.1, None).is_err());
    assert!(movement.start(f32::NAN, None).is_err());
    assert!(movement.start(1.0, Some(f32::INFINITY)).is_err());

    let step = movement.next_step(0.1).unwrap();
    assert!((step.duration_secs - 0.1).abs() < f32::EPSILON);
    assert_eq!(step.facing_yaw, Some(std::f32::consts::FRAC_PI_2));
}
