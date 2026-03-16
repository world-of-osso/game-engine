//! Collision detection using terrain heightmap + Bevy mesh raycasting.
//!
//! Player movement is validated against terrain slope and height.
//! Gravity and ground snapping replace the old hardcoded Y assignment.

use bevy::prelude::*;
use shared::movement::{GRAVITY, GROUND_SNAP_THRESHOLD, MAX_SLOPE_ANGLE};

use crate::camera::Player;
use crate::game_state::GameState;
use crate::terrain_heightmap::TerrainHeightmap;

/// Upward jump velocity in yards/sec.
///
/// The previous value (`9.0`) produced a visibly floaty arc with a peak just
/// over 2 yards at the current gravity. Lowering this keeps jumps grounded
/// closer to the in-game feel.
pub const JUMP_IMPULSE: f32 = 7.0;

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
    for (tf, mut physics) in query.iter_mut() {
        let ground = terrain
            .as_ref()
            .and_then(|t| t.height_at(tf.translation.x, tf.translation.z));
        physics.grounded = match ground {
            Some(h) => (tf.translation.y - h).abs() < GROUND_SNAP_THRESHOLD,
            // No terrain data yet — treat as grounded to prevent falling through the world.
            None => true,
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
            .and_then(|t| t.height_at(tf.translation.x, tf.translation.z));

        // No terrain loaded yet — freeze vertical position to prevent falling through the world.
        let Some(ground_y) = ground_y else {
            physics.vertical_velocity = 0.0;
            continue;
        };

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
pub fn validate_movement_slope(
    current: Vec3,
    proposed: Vec3,
    terrain: &TerrainHeightmap,
    snap_to_ground: bool,
) -> Vec3 {
    let Some(proposed_height) = terrain.height_at(proposed.x, proposed.z) else {
        return proposed;
    };
    let current_height = terrain.height_at(current.x, current.z).unwrap_or(current.y);
    let horizontal = Vec2::new(proposed.x - current.x, proposed.z - current.z).length();
    let height_diff = proposed_height - current_height;

    if is_walkable_slope(height_diff, horizontal) {
        if snap_to_ground {
            proposed.with_y(proposed_height)
        } else {
            proposed
        }
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

    #[test]
    fn jump_apex_stays_under_one_and_a_half_yards() {
        let apex = JUMP_IMPULSE.powi(2) / (2.0 * GRAVITY);
        assert!(apex < 1.5, "jump apex too high: {apex}");
        assert!(apex > 1.0, "jump apex too low: {apex}");
    }

    #[test]
    fn walkable_movement_snaps_to_sampled_ground_height() {
        let data = std::fs::read("data/terrain/azeroth_32_48.adt")
            .expect("expected test ADT data/terrain/azeroth_32_48.adt");
        let adt =
            crate::asset::adt::load_adt_for_tile(&data, 32, 48).expect("expected ADT to parse");
        let mut heightmap = crate::terrain_heightmap::TerrainHeightmap::default();
        heightmap.insert_tile(32, 48, &adt);

        let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
        let current_y = heightmap
            .height_at(bx, bz)
            .expect("expected terrain at sample position");
        let current = Vec3::new(bx, current_y, bz);

        let mut target = None;
        for dx in [-0.75, -0.5, -0.25, 0.25, 0.5, 0.75] {
            for dz in [-0.75, -0.5, -0.25, 0.25, 0.5, 0.75] {
                let proposed_height = heightmap.height_at(bx + dx, bz + dz);
                let Some(proposed_y) = proposed_height else {
                    continue;
                };
                let horizontal = Vec2::new(dx, dz).length();
                if horizontal < 0.001 {
                    continue;
                }
                let height_diff = proposed_y - current_y;
                if height_diff.abs() > 0.01 && is_walkable_slope(height_diff, horizontal) {
                    target = Some((bx + dx, bz + dz, proposed_y));
                    break;
                }
            }
            if target.is_some() {
                break;
            }
        }

        let (target_x, target_z, target_y) =
            target.expect("expected a nearby walkable point with a different terrain height");
        let moved = validate_movement_slope(
            current,
            Vec3::new(target_x, current_y, target_z),
            &heightmap,
            true,
        );

        assert!(
            (moved.y - target_y).abs() < 0.001,
            "walkable movement should follow terrain, got y={} terrain_y={target_y}",
            moved.y
        );
    }

    #[test]
    fn airborne_movement_keeps_vertical_position() {
        let data = std::fs::read("data/terrain/azeroth_32_48.adt")
            .expect("expected test ADT data/terrain/azeroth_32_48.adt");
        let adt =
            crate::asset::adt::load_adt_for_tile(&data, 32, 48).expect("expected ADT to parse");
        let mut heightmap = crate::terrain_heightmap::TerrainHeightmap::default();
        heightmap.insert_tile(32, 48, &adt);

        let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
        let ground_y = heightmap
            .height_at(bx, bz)
            .expect("expected terrain at sample position");
        let current = Vec3::new(bx, ground_y + 1.0, bz);
        let proposed = Vec3::new(bx + 0.25, ground_y + 1.0, bz + 0.25);

        let moved = validate_movement_slope(current, proposed, &heightmap, false);

        assert!(
            (moved.y - proposed.y).abs() < 0.001,
            "airborne movement should preserve vertical position, got y={} proposed_y={}",
            moved.y,
            proposed.y
        );
    }
}
