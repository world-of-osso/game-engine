//! Collision detection using terrain heightmap + Bevy mesh raycasting.
//!
//! Player movement is validated against terrain slope and height.
//! Gravity and ground snapping replace the old hardcoded Y assignment.

use std::collections::HashSet;

use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
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
const WMO_COLLISION_RAY_HEIGHT: f32 = 0.6;
const WMO_COLLISION_MARGIN: f32 = 0.05;

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

/// Marker for WMO batch meshes that should block player movement.
#[derive(Component)]
pub struct WmoCollisionMesh;

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

pub fn clamp_movement_against_wmo_meshes(
    current: Vec3,
    proposed: Vec3,
    ray_cast: &mut MeshRayCast,
    collision_meshes: &HashSet<Entity>,
) -> Vec3 {
    let movement = Vec3::new(proposed.x - current.x, 0.0, proposed.z - current.z);
    let distance = movement.length();
    if distance <= f32::EPSILON || collision_meshes.is_empty() {
        return proposed;
    }

    let direction = movement / distance;
    let ray = Ray3d::new(
        current + Vec3::Y * WMO_COLLISION_RAY_HEIGHT,
        Dir3::new(direction).expect("non-zero horizontal movement"),
    );
    let filter = |entity: Entity| collision_meshes.contains(&entity);
    let settings = MeshRayCastSettings::default().with_filter(&filter);
    let hit_distance = ray_cast
        .cast_ray(ray, &settings)
        .first()
        .map(|(_, hit)| hit.distance);

    clamp_movement_to_hit(current, proposed, hit_distance)
}

fn clamp_movement_to_hit(current: Vec3, proposed: Vec3, hit_distance: Option<f32>) -> Vec3 {
    let movement = Vec3::new(proposed.x - current.x, 0.0, proposed.z - current.z);
    let distance = movement.length();
    let Some(hit_distance) = hit_distance else {
        return proposed;
    };
    if hit_distance >= distance + WMO_COLLISION_MARGIN {
        return proposed;
    }

    let allowed_distance = (hit_distance - WMO_COLLISION_MARGIN).max(0.0);
    let direction = movement / distance.max(f32::EPSILON);
    let clamped = current + direction * allowed_distance;
    Vec3::new(clamped.x, proposed.y, clamped.z)
}

/// Compute a world-space AABB from M2 model-local bounding box corners
/// transformed by the doodad's world transform.
pub fn compute_world_aabb(
    local_min: [f32; 3],
    local_max: [f32; 3],
    transform: &Transform,
) -> (Vec3, Vec3) {
    let bevy_min = crate::asset::m2::wow_to_bevy(local_min[0], local_min[1], local_min[2]);
    let bevy_max = crate::asset::m2::wow_to_bevy(local_max[0], local_max[1], local_max[2]);
    let lo = Vec3::from(bevy_min).min(Vec3::from(bevy_max));
    let hi = Vec3::from(bevy_min).max(Vec3::from(bevy_max));
    let corners = [
        Vec3::new(lo.x, lo.y, lo.z),
        Vec3::new(hi.x, lo.y, lo.z),
        Vec3::new(lo.x, hi.y, lo.z),
        Vec3::new(hi.x, hi.y, lo.z),
        Vec3::new(lo.x, lo.y, hi.z),
        Vec3::new(hi.x, lo.y, hi.z),
        Vec3::new(lo.x, hi.y, hi.z),
        Vec3::new(hi.x, hi.y, hi.z),
    ];
    let mat = transform.compute_matrix();
    let mut world_min = Vec3::splat(f32::MAX);
    let mut world_max = Vec3::splat(f32::MIN);
    for corner in corners {
        let world = mat.transform_point3(corner);
        world_min = world_min.min(world);
        world_max = world_max.max(world);
    }
    (world_min, world_max)
}

/// Test if a horizontal ray intersects an AABB. Returns the hit distance
/// along the ray, or `None` if no intersection.
pub fn ray_aabb_intersect(
    origin: Vec3,
    direction: Vec3,
    aabb_min: Vec3,
    aabb_max: Vec3,
) -> Option<f32> {
    let inv_dir = Vec3::new(
        if direction.x.abs() > f32::EPSILON {
            1.0 / direction.x
        } else {
            f32::MAX
        },
        if direction.y.abs() > f32::EPSILON {
            1.0 / direction.y
        } else {
            f32::MAX
        },
        if direction.z.abs() > f32::EPSILON {
            1.0 / direction.z
        } else {
            f32::MAX
        },
    );
    let t1 = (aabb_min - origin) * inv_dir;
    let t2 = (aabb_max - origin) * inv_dir;
    let t_near = t1.min(t2);
    let t_far = t1.max(t2);
    let t_enter = t_near.x.max(t_near.y).max(t_near.z);
    let t_exit = t_far.x.min(t_far.y).min(t_far.z);
    if t_enter <= t_exit && t_exit >= 0.0 {
        Some(t_enter.max(0.0))
    } else {
        None
    }
}

/// Check proposed movement against doodad AABB colliders.
/// Returns the clamped position if a doodad blocks the path.
pub fn clamp_movement_against_doodad_colliders(
    current: Vec3,
    proposed: Vec3,
    colliders: &[(Vec3, Vec3)],
) -> Vec3 {
    let movement = Vec3::new(proposed.x - current.x, 0.0, proposed.z - current.z);
    let distance = movement.length();
    if distance <= f32::EPSILON || colliders.is_empty() {
        return proposed;
    }
    let direction = movement / distance;
    let ray_origin = current + Vec3::Y * WMO_COLLISION_RAY_HEIGHT;

    let mut closest_hit = None;
    for &(aabb_min, aabb_max) in colliders {
        if let Some(t) = ray_aabb_intersect(ray_origin, direction, aabb_min, aabb_max) {
            closest_hit = Some(match closest_hit {
                Some(prev) => f32::min(prev, t),
                None => t,
            });
        }
    }
    clamp_movement_to_hit(current, proposed, closest_hit)
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

    #[test]
    fn wmo_hit_before_destination_clamps_horizontal_movement() {
        let current = Vec3::new(1.0, 5.0, 2.0);
        let proposed = Vec3::new(5.0, 5.0, 2.0);

        let clamped = clamp_movement_to_hit(current, proposed, Some(2.0));

        assert!(
            (clamped.x - 2.95).abs() < 0.001,
            "expected wall margin clamp"
        );
        assert_eq!(clamped.y, proposed.y);
        assert_eq!(clamped.z, proposed.z);
    }

    #[test]
    fn wmo_hit_past_destination_keeps_proposed_position() {
        let current = Vec3::new(1.0, 5.0, 2.0);
        let proposed = Vec3::new(5.0, 5.0, 2.0);

        assert_eq!(
            clamp_movement_to_hit(current, proposed, Some(10.0)),
            proposed
        );
        assert_eq!(clamp_movement_to_hit(current, proposed, None), proposed);
    }

    #[test]
    fn ray_aabb_hit_from_outside() {
        let origin = Vec3::new(0.0, 1.0, 0.0);
        let dir = Vec3::X;
        let hit = ray_aabb_intersect(
            origin,
            dir,
            Vec3::new(2.0, 0.0, -1.0),
            Vec3::new(4.0, 2.0, 1.0),
        );
        assert!(hit.is_some());
        let t = hit.unwrap();
        assert!((t - 2.0).abs() < 0.01, "expected hit at t≈2.0, got {t}");
    }

    #[test]
    fn ray_aabb_miss() {
        let origin = Vec3::new(0.0, 5.0, 0.0);
        let dir = Vec3::X;
        // Ray is above the box
        let hit = ray_aabb_intersect(
            origin,
            dir,
            Vec3::new(2.0, 0.0, -1.0),
            Vec3::new(4.0, 2.0, 1.0),
        );
        assert!(hit.is_none());
    }

    #[test]
    fn ray_aabb_origin_inside() {
        let origin = Vec3::new(3.0, 1.0, 0.0);
        let dir = Vec3::X;
        let hit = ray_aabb_intersect(
            origin,
            dir,
            Vec3::new(2.0, 0.0, -1.0),
            Vec3::new(4.0, 2.0, 1.0),
        );
        assert!(hit.is_some());
        assert!(
            (hit.unwrap() - 0.0).abs() < 0.01,
            "inside origin should hit at t≈0"
        );
    }

    #[test]
    fn compute_world_aabb_identity_transform() {
        let transform = Transform::default();
        let (wmin, wmax) = compute_world_aabb([0.0, -1.0, 2.0], [1.0, 1.0, 4.0], &transform);
        // wow_to_bevy: [x, y, z] -> [x, z, -y]
        // min: [0, -1, 2] -> [0, 2, 1]
        // max: [1, 1, 4] -> [1, 4, -1]
        // After min/max correction: min=[0, 2, -1], max=[1, 4, 1]
        assert!((wmin.x - 0.0).abs() < 0.01);
        assert!((wmax.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn compute_world_aabb_with_scale() {
        let transform = Transform::from_scale(Vec3::splat(2.0));
        let (wmin, wmax) = compute_world_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], &transform);
        let size = wmax - wmin;
        assert!(
            (size.x - 2.0).abs() < 0.01,
            "scaled size should be 2, got {}",
            size.x
        );
    }

    #[test]
    fn doodad_collider_blocks_movement() {
        let current = Vec3::new(0.0, 1.0, 0.0);
        let proposed = Vec3::new(5.0, 1.0, 0.0);
        let colliders = vec![(Vec3::new(2.0, 0.0, -1.0), Vec3::new(4.0, 2.0, 1.0))];
        let clamped = clamp_movement_against_doodad_colliders(current, proposed, &colliders);
        assert!(clamped.x < proposed.x, "should be clamped before doodad");
        assert!(clamped.x < 2.0, "should stop before the box");
    }

    #[test]
    fn doodad_collider_no_block_when_path_clear() {
        let current = Vec3::new(0.0, 1.0, 0.0);
        let proposed = Vec3::new(1.0, 1.0, 0.0);
        let colliders = vec![(Vec3::new(5.0, 0.0, -1.0), Vec3::new(7.0, 2.0, 1.0))];
        let clamped = clamp_movement_against_doodad_colliders(current, proposed, &colliders);
        assert_eq!(clamped, proposed);
    }
}
