//! Bevy-free horizontal proposal and vertical ground transitions.

use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroundState {
    Unloaded,
    Unsupported,
    Supported(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalState {
    pub y: f32,
    pub vertical_velocity: f32,
    pub grounded: bool,
}

pub fn build_proposed_ground_movement(
    current: Vec3,
    direction: Vec3,
    speed: f32,
    dt: f32,
) -> Option<Vec3> {
    if direction.length_squared() == 0.0 {
        return None;
    }
    Some(current + direction.normalize() * speed * dt)
}

pub fn update_grounded(y: f32, ground: GroundState, snap_threshold: f32) -> bool {
    match ground {
        GroundState::Supported(height) => (y - height).abs() < snap_threshold,
        GroundState::Unsupported => false,
        GroundState::Unloaded => true,
    }
}

pub fn apply_gravity_and_ground_snap(
    mut state: VerticalState,
    ground: GroundState,
    dt: f32,
    gravity: f32,
) -> VerticalState {
    let ground_y = match ground {
        GroundState::Supported(height) => Some(height),
        GroundState::Unsupported => None,
        GroundState::Unloaded => {
            state.vertical_velocity = 0.0;
            return state;
        }
    };

    if state.grounded && state.vertical_velocity <= 0.0 {
        if let Some(ground_y) = ground_y {
            state.y = ground_y;
        }
        state.vertical_velocity = 0.0;
    } else {
        state.vertical_velocity -= gravity * dt;
        state.y += state.vertical_velocity * dt;
        if let Some(ground_y) = ground_y
            && state.y <= ground_y
        {
            state.y = ground_y;
            state.vertical_velocity = 0.0;
            state.grounded = true;
        }
    }
    state
}
