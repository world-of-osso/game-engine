//! Bevy-free horizontal proposal and vertical ground transitions.

use glam::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundSample {
    pub height: f32,
    pub is_terrain: bool,
}

pub fn is_walkable_slope(height_diff: f32, horizontal_dist: f32, max_slope_angle: f32) -> bool {
    if horizontal_dist < 0.001 {
        return true;
    }
    let slope = (height_diff / horizontal_dist).abs().atan();
    slope <= max_slope_angle
}

/// Decide whether to block or snap a movement using already-probed ground.
/// The caller probes the target at the proposed XZ and the current Y.
pub fn validate_movement_slope(
    current: Vec3,
    proposed: Vec3,
    origin: Option<GroundSample>,
    target: Option<GroundSample>,
    snap_to_ground: bool,
    max_slope_angle: f32,
    step_up_height: f32,
) -> Vec3 {
    let Some(target) = target else {
        return proposed;
    };
    if let Some(origin) = origin
        && origin.is_terrain
        && target.is_terrain
    {
        let horizontal = Vec2::new(proposed.x - current.x, proposed.z - current.z).length();
        if !is_walkable_slope(target.height - origin.height, horizontal, max_slope_angle) {
            return current;
        }
    }
    if snap_to_ground && target.height >= current.y - step_up_height {
        proposed.with_y(target.height)
    } else {
        proposed
    }
}

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

/// Height above the feet of the wall ray (original `WMO_COLLISION_RAY_HEIGHT`).
pub const WALL_RAY_HEIGHT: f32 = 0.6;
/// Distance a move stops short of a wall (original `WMO_COLLISION_MARGIN`).
pub const WALL_MARGIN: f32 = 0.05;

/// Stop a horizontal move short of the first wall along it, keeping the proposed height; no
/// slide. `wall_hit(origin, direction, length)` is the distance to the first wall on the ray.
pub fn clamp_movement_to_walls(
    current: Vec3,
    proposed: Vec3,
    wall_hit: impl FnOnce(Vec3, Vec3, f32) -> Option<f32>,
) -> Vec3 {
    let movement = Vec3::new(proposed.x - current.x, 0.0, proposed.z - current.z);
    let distance = movement.length();
    if distance <= f32::EPSILON {
        return proposed;
    }
    let direction = movement / distance;
    let reach = distance + WALL_MARGIN;
    let Some(hit) = wall_hit(current + Vec3::Y * WALL_RAY_HEIGHT, direction, reach) else {
        return proposed;
    };
    if hit >= reach {
        return proposed;
    }
    let clamped = current + direction * (hit - WALL_MARGIN).max(0.0);
    Vec3::new(clamped.x, proposed.y, clamped.z)
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
