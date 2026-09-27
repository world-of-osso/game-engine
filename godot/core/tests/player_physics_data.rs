use game_engine_core::player_physics_data::{
    GroundState, VerticalState, apply_gravity_and_ground_snap, build_proposed_ground_movement,
    update_grounded,
};
use glam::Vec3;

const GRAVITY: f32 = 19.6;
const SNAP: f32 = 0.3;

fn step(state: VerticalState, ground: GroundState, dt: f32) -> VerticalState {
    let grounded = update_grounded(state.y, ground, SNAP);
    apply_gravity_and_ground_snap(VerticalState { grounded, ..state }, ground, dt, GRAVITY)
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
