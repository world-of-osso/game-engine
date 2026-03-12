//! Collision detection using terrain heightmap + Bevy mesh raycasting.
//!
//! Player movement is validated against terrain slope and height.
//! Gravity and ground snapping replace the old hardcoded Y assignment.

use bevy::prelude::*;
use shared::movement::{GRAVITY, GROUND_SNAP_THRESHOLD, MAX_SLOPE_ANGLE};

pub use shared::movement::JUMP_IMPULSE;

use crate::camera::Player;
use crate::game_state::GameState;
use crate::terrain_heightmap::TerrainHeightmap;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_grounded, apply_gravity_and_ground_snap)
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

/// Tracks vertical velocity and grounded state for a character.
#[derive(Component)]
pub struct CharacterPhysics {
    pub vertical_velocity: f32,
    pub grounded: bool,
}

impl Default for CharacterPhysics {
    fn default() -> Self {
        Self {
            vertical_velocity: 0.0,
            grounded: true,
        }
    }
}

/// Check whether the player is on walkable ground based on terrain height.
fn update_grounded(
    terrain: Option<Res<TerrainHeightmap>>,
    mut query: Query<(&Transform, &mut CharacterPhysics), With<Player>>,
) {
    let Some(terrain) = terrain.as_ref() else {
        return;
    };
    for (tf, mut physics) in query.iter_mut() {
        let ground = terrain.height_at(tf.translation.x, tf.translation.z);
        physics.grounded = match ground {
            Some(h) => (tf.translation.y - h).abs() < GROUND_SNAP_THRESHOLD,
            None => false,
        };
    }
}

/// Apply gravity when airborne, snap to ground when close.
fn apply_gravity_and_ground_snap(
    time: Res<Time>,
    terrain: Option<Res<TerrainHeightmap>>,
    mut query: Query<(&mut Transform, &mut CharacterPhysics), With<Player>>,
) {
    let dt = time.delta_secs();
    for (mut tf, mut physics) in query.iter_mut() {
        let ground_y = terrain
            .as_ref()
            .and_then(|t| t.height_at(tf.translation.x, tf.translation.z))
            .unwrap_or(0.0);

        if physics.grounded && physics.vertical_velocity <= 0.0 {
            tf.translation.y = ground_y;
            physics.vertical_velocity = 0.0;
        } else {
            physics.vertical_velocity -= GRAVITY * dt;
            tf.translation.y += physics.vertical_velocity * dt;
            clamp_to_ground(&mut tf, &mut physics, ground_y);
        }
    }
}

fn clamp_to_ground(tf: &mut Transform, physics: &mut CharacterPhysics, ground_y: f32) {
    if tf.translation.y <= ground_y {
        tf.translation.y = ground_y;
        physics.vertical_velocity = 0.0;
        physics.grounded = true;
    }
}

/// Check if terrain slope between two positions is walkable.
/// Returns true if the slope angle is within MAX_SLOPE_ANGLE.
pub fn is_walkable_slope(height_diff: f32, horizontal_dist: f32) -> bool {
    if horizontal_dist < 0.001 {
        return true;
    }
    let slope = (height_diff / horizontal_dist).abs().atan();
    slope <= MAX_SLOPE_ANGLE
}

/// Validate a proposed movement against terrain slope.
/// Returns the clamped position if slope is too steep, or the proposed position if walkable.
pub fn validate_movement_slope(current: Vec3, proposed: Vec3, terrain: &TerrainHeightmap) -> Vec3 {
    let Some(proposed_height) = terrain.height_at(proposed.x, proposed.z) else {
        return proposed;
    };
    let current_height = terrain.height_at(current.x, current.z).unwrap_or(current.y);
    let horizontal = Vec2::new(proposed.x - current.x, proposed.z - current.z).length();
    let height_diff = proposed_height - current_height;

    if is_walkable_slope(height_diff, horizontal) {
        proposed
    } else {
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_terrain_is_walkable() {
        assert!(is_walkable_slope(0.0, 10.0));
    }

    #[test]
    fn gentle_slope_is_walkable() {
        // 30° slope: height = tan(30°) * dist ≈ 0.577
        assert!(is_walkable_slope(0.577, 1.0));
    }

    #[test]
    fn steep_slope_is_rejected() {
        // 60° slope: height = tan(60°) * dist ≈ 1.732
        assert!(!is_walkable_slope(1.732, 1.0));
    }

    #[test]
    fn vertical_wall_is_rejected() {
        assert!(!is_walkable_slope(10.0, 0.1));
    }
}
