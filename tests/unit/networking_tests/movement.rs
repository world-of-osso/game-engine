use super::*;

#[test]
fn idle_produces_zero_direction() {
    let dir = movement_to_direction(&make_state(MoveDirection::None), &make_facing(0.0));
    assert_eq!(dir, [0.0, 0.0, 0.0]);
}

#[test]
fn forward_at_zero_yaw() {
    let dir = movement_to_direction(&make_state(MoveDirection::Forward), &make_facing(0.0));
    assert!(dir[0].abs() < 1e-6);
    assert_eq!(dir[1], 0.0);
    assert!((dir[2] - 1.0).abs() < 1e-6);
}

#[test]
fn forward_at_90_degrees() {
    let dir = movement_to_direction(&make_state(MoveDirection::Forward), &make_facing(FRAC_PI_2));
    assert!((dir[0] - 1.0).abs() < 1e-6);
    assert!(dir[2].abs() < 1e-6);
}

#[test]
fn backward_is_opposite_of_forward() {
    let facing = make_facing(0.5);
    let fwd = movement_to_direction(&make_state(MoveDirection::Forward), &facing);
    let bwd = movement_to_direction(&make_state(MoveDirection::Backward), &facing);
    assert!((fwd[0] + bwd[0]).abs() < 1e-6);
    assert!((fwd[2] + bwd[2]).abs() < 1e-6);
}

#[test]
fn left_is_perpendicular_to_forward() {
    let facing = make_facing(0.0);
    let fwd = movement_to_direction(&make_state(MoveDirection::Forward), &facing);
    let left = movement_to_direction(&make_state(MoveDirection::Left), &facing);
    let dot = fwd[0] * left[0] + fwd[2] * left[2];
    assert!(dot.abs() < 1e-6);
}

#[test]
fn right_is_opposite_of_left() {
    let facing = make_facing(PI / 3.0);
    let left = movement_to_direction(&make_state(MoveDirection::Left), &facing);
    let right = movement_to_direction(&make_state(MoveDirection::Right), &facing);
    assert!((left[0] + right[0]).abs() < 1e-6);
    assert!((left[2] + right[2]).abs() < 1e-6);
}

#[test]
fn direction_is_unit_length() {
    for dir in [
        MoveDirection::Forward,
        MoveDirection::Backward,
        MoveDirection::Left,
        MoveDirection::Right,
    ] {
        let d = movement_to_direction(&make_state(dir), &make_facing(1.23));
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-6,
            "direction {dir:?} has length {len}"
        );
    }
}

#[test]
fn y_component_always_zero() {
    for yaw in [0.0, FRAC_PI_2, PI, -PI] {
        for dir in [
            MoveDirection::Forward,
            MoveDirection::Backward,
            MoveDirection::Left,
            MoveDirection::Right,
        ] {
            let d = movement_to_direction(&make_state(dir), &make_facing(yaw));
            assert_eq!(d[1], 0.0);
        }
    }
}
