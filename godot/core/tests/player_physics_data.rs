use game_engine_core::player_physics_data::{
    GroundSample, GroundState, VerticalState, apply_gravity_and_ground_snap,
    build_proposed_ground_movement, clamp_movement_to_walls, is_walkable_slope, update_grounded,
    validate_movement_slope,
};
use glam::Vec3;

const GRAVITY: f32 = 19.6;

#[test]
fn toyfx2_feather_fall_caps_prediction_and_removal_restores_gravity() {
    let mut state = VerticalState {
        y: 100.0,
        vertical_velocity: -40.0,
        grounded: false,
    };
    for _ in 0..120 {
        let previous = state.y;
        state = apply_gravity_and_ground_snap(
            state,
            GroundState::Supported(0.0),
            1.0 / 60.0,
            GRAVITY,
            Some(7.0),
        );
        assert_eq!(state.vertical_velocity, -7.0);
        assert!((previous - state.y - 7.0 / 60.0).abs() < 0.001);
    }
    let uncapped =
        apply_gravity_and_ground_snap(state, GroundState::Supported(0.0), 0.1, GRAVITY, None);
    assert!(uncapped.vertical_velocity < -7.0);
    let jumping = apply_gravity_and_ground_snap(
        VerticalState {
            vertical_velocity: 9.0,
            ..state
        },
        GroundState::Supported(0.0),
        0.1,
        GRAVITY,
        Some(7.0),
    );
    assert!(jumping.vertical_velocity > 0.0);
}
const SNAP: f32 = 0.3;
const MAX_SLOPE_ANGLE: f32 = std::f32::consts::FRAC_PI_4;
const STEP_UP_HEIGHT: f32 = 1.6;

fn terrain(height: f32) -> Option<GroundSample> {
    Some(GroundSample {
        height,
        is_terrain: true,
    })
}

fn wmo(height: f32) -> Option<GroundSample> {
    Some(GroundSample {
        height,
        is_terrain: false,
    })
}

fn slope_move(
    current: Vec3,
    proposed: Vec3,
    origin: Option<GroundSample>,
    target: Option<GroundSample>,
    snap: bool,
) -> Vec3 {
    validate_movement_slope(
        current,
        proposed,
        origin,
        target,
        snap,
        MAX_SLOPE_ANGLE,
        STEP_UP_HEIGHT,
    )
}

#[test]
fn missing_target_preserves_proposal_even_when_snap_requested() {
    let current = Vec3::new(0.0, 4.0, 0.0);
    let proposed = Vec3::new(1.0, 5.0, 0.0);
    assert_eq!(
        slope_move(current, proposed, terrain(4.0), None, true),
        proposed
    );
}

#[test]
fn only_terrain_to_terrain_checks_slope_in_both_directions() {
    let current = Vec3::new(0.0, 4.0, 0.0);
    let proposed = Vec3::new(1.0, 4.0, 0.0);
    assert_eq!(
        slope_move(current, proposed, terrain(4.0), terrain(6.0), false),
        current
    );
    assert_eq!(
        slope_move(current, proposed, terrain(4.0), terrain(2.0), false),
        current
    );
    assert_eq!(
        slope_move(current, proposed, wmo(4.0), terrain(6.0), false),
        proposed
    );
    assert_eq!(
        slope_move(current, proposed, terrain(4.0), wmo(6.0), false),
        proposed
    );
    assert_eq!(
        slope_move(current, proposed, None, terrain(6.0), false),
        proposed
    );
}

#[test]
fn short_horizontal_distance_is_walkable_even_with_large_height_difference() {
    assert!(is_walkable_slope(100.0, 0.0009, MAX_SLOPE_ANGLE));
    assert!(!is_walkable_slope(100.0, 0.001, MAX_SLOPE_ANGLE));
}

#[test]
fn slope_angle_limit_is_inclusive_and_uses_absolute_height_difference() {
    assert!(is_walkable_slope(1.0, 1.0, MAX_SLOPE_ANGLE));
    assert!(is_walkable_slope(-1.0, 1.0, MAX_SLOPE_ANGLE));
    assert!(!is_walkable_slope(1.01, 1.0, MAX_SLOPE_ANGLE));
}

#[test]
fn snapping_requires_target_within_step_below_current_height() {
    let current = Vec3::new(0.0, 4.0, 0.0);
    let proposed = Vec3::new(1.0, 8.0, 0.0);
    assert_eq!(
        slope_move(current, proposed, None, terrain(2.4), true),
        proposed.with_y(2.4)
    );
    assert_eq!(
        slope_move(current, proposed, None, terrain(2.39), true),
        proposed
    );
    assert_eq!(
        slope_move(current, proposed, None, terrain(5.0), true),
        proposed.with_y(5.0)
    );
    assert_eq!(
        slope_move(current, proposed, None, terrain(2.4), false),
        proposed
    );
}

fn step(state: VerticalState, ground: GroundState, dt: f32) -> VerticalState {
    let grounded = update_grounded(state.y, ground, SNAP);
    apply_gravity_and_ground_snap(
        VerticalState { grounded, ..state },
        ground,
        dt,
        GRAVITY,
        None,
    )
}

#[test]
fn proposal_normalizes_entire_direction_before_displacement() {
    assert_eq!(
        build_proposed_ground_movement(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(3.0, 0.0, 4.0),
            10.0,
            0.5
        ),
        Some(Vec3::new(4.0, 2.0, 7.0))
    );
    assert_eq!(
        build_proposed_ground_movement(Vec3::new(1.0, 2.0, 3.0), Vec3::ZERO, 10.0, 0.5),
        None
    );
}

#[test]
fn supported_ground_uses_strict_snap_threshold() {
    assert!(update_grounded(2.29, GroundState::Supported(2.0), SNAP));
    assert!(!update_grounded(SNAP, GroundState::Supported(0.0), SNAP));
    assert!(!update_grounded(0.0, GroundState::Unsupported, SNAP));
    assert!(update_grounded(0.0, GroundState::Unloaded, SNAP));
}

#[test]
fn unsupported_ground_falls_from_rest() {
    let result = step(
        VerticalState {
            y: 2.0,
            vertical_velocity: 0.0,
            grounded: true,
        },
        GroundState::Unsupported,
        0.1,
    );
    assert!((result.vertical_velocity + 1.96).abs() < 1e-6);
    assert!((result.y - 1.804).abs() < 1e-6);
    assert!(!result.grounded);
}

#[test]
fn unloaded_ground_freezes_height_and_clears_velocity() {
    assert_eq!(
        step(
            VerticalState {
                y: 2.0,
                vertical_velocity: -3.0,
                grounded: false
            },
            GroundState::Unloaded,
            0.1
        ),
        VerticalState {
            y: 2.0,
            vertical_velocity: 0.0,
            grounded: true
        }
    );
}

#[test]
fn grounded_nonpositive_velocity_snaps_to_supported_height() {
    assert_eq!(
        step(
            VerticalState {
                y: 2.1,
                vertical_velocity: -2.0,
                grounded: false
            },
            GroundState::Supported(2.0),
            0.1
        ),
        VerticalState {
            y: 2.0,
            vertical_velocity: 0.0,
            grounded: true
        }
    );
}

#[test]
fn upward_jump_integrates_even_when_grounded() {
    let result = step(
        VerticalState {
            y: 2.0,
            vertical_velocity: 7.0,
            grounded: true,
        },
        GroundState::Supported(2.0),
        0.1,
    );
    assert!((result.vertical_velocity - 5.04).abs() < 1e-6);
    assert!((result.y - 2.504).abs() < 1e-6);
    assert!(result.grounded);
}

#[test]
fn falling_across_supported_height_clamps_and_lands() {
    assert_eq!(
        step(
            VerticalState {
                y: 2.4,
                vertical_velocity: -4.0,
                grounded: false
            },
            GroundState::Supported(2.0),
            0.1
        ),
        VerticalState {
            y: 2.0,
            vertical_velocity: 0.0,
            grounded: true
        }
    );
}

/// The original `clamp_movement_against_wmo_meshes`: one horizontal ray 0.6 yd above the feet
/// along the move; a wall before the destination (plus a 0.05 yd margin) stops the move 0.05 yd
/// short of it, keeping the proposed height; no slide.
#[test]
fn wall_before_destination_stops_the_move_short_of_it() {
    let current = Vec3::new(1.0, 5.0, 2.0);
    let proposed = Vec3::new(5.0, 5.5, 2.0);
    let mut rays = Vec::new();
    let clamped = clamp_movement_to_walls(current, proposed, |origin, direction, length| {
        rays.push((origin, direction, length));
        Some(2.0)
    });
    assert_eq!(rays, vec![(Vec3::new(1.0, 5.6, 2.0), Vec3::X, 4.05)]);
    assert!(
        (clamped - Vec3::new(2.95, 5.5, 2.0)).length() < 1e-5,
        "{clamped}"
    );

    for hit in [None, Some(4.05), Some(10.0)] {
        assert_eq!(
            clamp_movement_to_walls(current, proposed, |_, _, _| hit),
            proposed,
            "{hit:?}"
        );
    }
    assert_eq!(
        clamp_movement_to_walls(current, current.with_y(9.0), |_, _, _| Some(0.0)),
        current.with_y(9.0),
        "a vertical move casts nothing"
    );
    let touching = clamp_movement_to_walls(current, proposed, |_, _, _| Some(0.01));
    assert_eq!(touching, Vec3::new(1.0, 5.5, 2.0));
}
