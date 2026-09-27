//! Collision detection using terrain heightmap, WMO floors + Bevy mesh raycasting.
//!
//! The player stands on the shared ground rule (`shared::ground`,
//! docs/specs/wmo-floor-collision.md): the highest walkable terrain or WMO
//! floor within step reach of the feet. Gravity and ground snapping follow it.

use std::collections::HashSet;

use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::prelude::*;
use shared::ground::{Ground, STEP_UP_HEIGHT, Surface, WmoCollision};
use shared::movement::{GRAVITY, GROUND_SNAP_THRESHOLD, MAX_SLOPE_ANGLE};

use game_engine::player_physics_data::{self, GroundSample, GroundState, VerticalState};

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

/// Floor collision of a spawned WMO root, placed by the root's transform.
#[derive(Component)]
pub struct WmoFloors(pub WmoCollision);

/// What supports a character's feet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroundProbe {
    /// The terrain tile under the feet has not loaded yet.
    Unloaded,
    /// Nothing within step reach: the character falls.
    Unsupported,
    Supported(Ground),
}

/// Terrain and WMO floors a character can stand on.
pub struct WorldGround<'a> {
    terrain: Option<&'a TerrainHeightmap>,
    wmos: Vec<&'a WmoCollision>,
}

impl<'a> WorldGround<'a> {
    pub fn new(terrain: Option<&'a TerrainHeightmap>, floors: &'a Query<&WmoFloors>) -> Self {
        Self {
            terrain,
            wmos: floors.iter().map(|floors| &floors.0).collect(),
        }
    }

    pub fn from_parts(terrain: Option<&'a TerrainHeightmap>, wmos: Vec<&'a WmoCollision>) -> Self {
        Self { terrain, wmos }
    }

    pub fn probe(&self, feet: Vec3) -> GroundProbe {
        if self.terrain.is_some_and(TerrainHeightmap::is_wmo_only) {
            return match shared::ground::ground_at(feet, None, self.wmos.iter().copied()) {
                Some(ground) => GroundProbe::Supported(ground),
                None => GroundProbe::Unsupported,
            };
        }
        let Some(terrain) = self
            .terrain
            .filter(|terrain| terrain.has_tile_at(feet.x, feet.z))
        else {
            return GroundProbe::Unloaded;
        };
        let terrain_y = terrain.height_at(feet.x, feet.z);
        match shared::ground::ground_at(feet, terrain_y, self.wmos.iter().copied()) {
            Some(ground) => GroundProbe::Supported(ground),
            None => GroundProbe::Unsupported,
        }
    }

    pub fn terrain(&self) -> Option<&'a TerrainHeightmap> {
        self.terrain
    }

    /// Height of the supporting surface, if any.
    pub fn height_at(&self, feet: Vec3) -> Option<f32> {
        match self.probe(feet) {
            GroundProbe::Supported(ground) => Some(ground.height),
            GroundProbe::Unloaded | GroundProbe::Unsupported => None,
        }
    }
}

/// Check whether the player stands on its ground.
fn update_grounded(
    terrain: Option<Res<TerrainHeightmap>>,
    floors: Query<&WmoFloors>,
    mut query: Query<(&Transform, &mut CharacterPhysics), With<Player>>,
) {
    let ground = WorldGround::new(terrain.as_deref(), &floors);
    for (tf, mut physics) in query.iter_mut() {
        physics.grounded = player_physics_data::update_grounded(
            tf.translation.y,
            physics_ground_state(ground.probe(tf.translation)),
            GROUND_SNAP_THRESHOLD,
        );
    }
}

/// Apply gravity when airborne, snap to ground when close.
fn apply_gravity_and_ground_snap(
    time: Res<Time>,
    terrain: Option<Res<TerrainHeightmap>>,
    floors: Query<&WmoFloors>,
    mut query: Query<(&mut Transform, &mut CharacterPhysics), With<Player>>,
) {
    let dt = time.delta_secs();
    let ground = WorldGround::new(terrain.as_deref(), &floors);
    for (mut tf, mut physics) in query.iter_mut() {
        let result = player_physics_data::apply_gravity_and_ground_snap(
            VerticalState {
                y: tf.translation.y,
                vertical_velocity: physics.vertical_velocity,
                grounded: physics.grounded,
            },
            physics_ground_state(ground.probe(tf.translation)),
            dt,
            GRAVITY,
        );
        tf.translation.y = result.y;
        physics.vertical_velocity = result.vertical_velocity;
        physics.grounded = result.grounded;
    }
}

fn physics_ground_state(probe: GroundProbe) -> GroundState {
    match probe {
        GroundProbe::Unloaded => GroundState::Unloaded,
        GroundProbe::Unsupported => GroundState::Unsupported,
        GroundProbe::Supported(ground) => GroundState::Supported(ground.height),
    }
}

/// Check if terrain slope between two positions is walkable.
/// Returns true if the slope angle is within MAX_SLOPE_ANGLE.
pub fn is_walkable_slope(height_diff: f32, horizontal_dist: f32) -> bool {
    player_physics_data::is_walkable_slope(height_diff, horizontal_dist, MAX_SLOPE_ANGLE)
}

/// Validate a proposed movement against the ground at its destination.
/// Terrain to terrain must stay within the walkable slope; WMO floors are
/// walkable by their face normal. A grounded move snaps onto a destination
/// within step reach below and walks off a higher ledge. Returns `current`
/// for a blocked move.
pub fn validate_movement_slope(
    current: Vec3,
    proposed: Vec3,
    ground: &WorldGround,
    snap_to_ground: bool,
) -> Vec3 {
    let target = slope_ground_sample(ground.probe(proposed.with_y(current.y)));
    if target.is_none() {
        return proposed;
    }
    let origin = slope_ground_sample(ground.probe(current));
    player_physics_data::validate_movement_slope(
        current,
        proposed,
        origin,
        target,
        snap_to_ground,
        MAX_SLOPE_ANGLE,
        STEP_UP_HEIGHT,
    )
}

fn slope_ground_sample(probe: GroundProbe) -> Option<GroundSample> {
    match probe {
        GroundProbe::Supported(ground) => Some(GroundSample {
            height: ground.height,
            is_terrain: ground.surface == Surface::Terrain,
        }),
        GroundProbe::Unloaded | GroundProbe::Unsupported => None,
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

pub use game_engine::culling::compute_world_aabb;

/// Check proposed movement against authored doodad collision surfaces.
pub fn clamp_movement_against_doodad_colliders(
    current: Vec3,
    proposed: Vec3,
    colliders: &[&game_engine::culling::DoodadCollider],
) -> Vec3 {
    let movement = Vec3::new(proposed.x - current.x, 0.0, proposed.z - current.z);
    let distance = movement.length();
    if distance <= f32::EPSILON || colliders.is_empty() {
        return proposed;
    }
    let ray = Ray3d::new(
        current + Vec3::Y * WMO_COLLISION_RAY_HEIGHT,
        Dir3::new(movement / distance).expect("non-zero horizontal movement"),
    );
    let closest_hit = colliders
        .iter()
        .filter_map(|collider| collider.ray_hit(ray, distance + WMO_COLLISION_MARGIN))
        .reduce(f32::min);
    clamp_movement_to_hit(current, proposed, closest_hit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_wall(x: f32) -> game_engine::culling::DoodadCollider {
        let geometry = crate::asset::m2::M2CollisionMesh {
            bounds_min: [x, -1.0, 0.0],
            bounds_max: [x, 1.0, 2.0],
            vertices: vec![[x, -1.0, 0.0], [x, 1.0, 0.0], [x, 1.0, 2.0], [x, -1.0, 2.0]],
            indices: vec![0, 1, 2, 0, 2, 3],
        };
        game_engine::culling::DoodadCollider::new(
            std::sync::Arc::new(geometry),
            &Transform::default(),
        )
    }

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
            &WorldGround::from_parts(Some(&heightmap), Vec::new()),
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

        let ground = WorldGround::from_parts(Some(&heightmap), Vec::new());
        let moved = validate_movement_slope(current, proposed, &ground, false);

        assert!(
            (moved.y - proposed.y).abs() < 0.001,
            "airborne movement should preserve vertical position, got y={} proposed_y={}",
            moved.y,
            proposed.y
        );
    }

    #[test]
    fn a_wmo_only_map_has_no_unloaded_terrain_to_wait_for() {
        let feet = Vec3::new(56.68, -19.27, -0.62);
        let adt_map = crate::terrain_heightmap::TerrainHeightmap::default();
        assert_eq!(
            WorldGround::from_parts(Some(&adt_map), Vec::new()).probe(feet),
            GroundProbe::Unloaded
        );
        let mut wmo_map = crate::terrain_heightmap::TerrainHeightmap::default();
        wmo_map.set_wmo_only();
        // No WMO floor under the feet yet: the character falls instead of freezing.
        assert_eq!(
            WorldGround::from_parts(Some(&wmo_map), Vec::new()).probe(feet),
            GroundProbe::Unsupported
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
        let collider = test_wall(2.0);
        let clamped = clamp_movement_against_doodad_colliders(current, proposed, &[&collider]);
        assert!(clamped.x < proposed.x, "should be clamped before doodad");
        assert!(clamped.x < 2.0, "should stop before the box");
    }

    #[test]
    fn doodad_collider_no_block_when_path_clear() {
        let current = Vec3::new(0.0, 1.0, 0.0);
        let proposed = Vec3::new(1.0, 1.0, 0.0);
        let collider = test_wall(5.0);
        let clamped = clamp_movement_against_doodad_colliders(current, proposed, &[&collider]);
        assert_eq!(clamped, proposed);
    }
}
