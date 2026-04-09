use std::collections::HashSet;

use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::prelude::*;

use crate::sky::SkyDome;
use crate::taxi::TaxiCameraTarget;
use crate::terrain_heightmap::TerrainHeightmap;

use super::{COLLISION_OFFSET, COLLISION_RECOVERY_SPEED, EYE_HEIGHT, GROUND_Y, Player, WowCamera};

type FollowPlayerQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Transform), (With<Player>, Without<WowCamera>)>;
type FollowTaxiQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Transform), (With<TaxiCameraTarget>, Without<WowCamera>)>;
type FollowCameraQuery<'w, 's> =
    Query<'w, 's, (&'static mut WowCamera, &'static mut Transform), Without<Player>>;

const TERRAIN_COLLISION_STEPS: usize = 24;
const TERRAIN_COLLISION_CLEARANCE: f32 = 0.2;

/// Recursively collect all descendant entities into the set.
fn collect_descendants(entity: Entity, children_q: &Query<&Children>, out: &mut HashSet<Entity>) {
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            out.insert(child);
            collect_descendants(child, children_q, out);
        }
    }
}

/// Compute camera distance clamped by a collision hit.
/// Returns the adjusted distance if hit is closer than intended, otherwise the intended distance.
fn collision_adjusted_distance(intended_distance: f32, hit_distance: Option<f32>) -> f32 {
    match hit_distance {
        Some(hit) if hit < intended_distance => (hit - COLLISION_OFFSET).max(0.5),
        _ => intended_distance,
    }
}

fn terrain_adjusted_distance(
    intended_distance: f32,
    eye_target: Vec3,
    orbit_dir: Vec3,
    mut terrain_height_at: impl FnMut(f32, f32) -> Option<f32>,
) -> f32 {
    if intended_distance <= 0.0 || orbit_dir.length_squared() == 0.0 {
        return intended_distance;
    }

    let intended_pos = eye_target - orbit_dir * intended_distance;
    for step in 1..=TERRAIN_COLLISION_STEPS {
        let t = step as f32 / TERRAIN_COLLISION_STEPS as f32;
        let sample = eye_target.lerp(intended_pos, t);
        let Some(terrain_y) = terrain_height_at(sample.x, sample.z) else {
            continue;
        };
        if terrain_y + TERRAIN_COLLISION_CLEARANCE <= sample.y {
            continue;
        }

        let blocked_distance = eye_target.distance(sample);
        return (blocked_distance - COLLISION_OFFSET).max(0.5);
    }

    intended_distance
}

/// Build the set of entities excluded from camera collision (player + children + sky).
fn build_collision_excluded_set(
    player_entity: Entity,
    children_q: &Query<&Children>,
    sky_q: &Query<Entity, With<SkyDome>>,
) -> HashSet<Entity> {
    let mut excluded = HashSet::new();
    excluded.insert(player_entity);
    collect_descendants(player_entity, children_q, &mut excluded);
    for entity in sky_q.iter() {
        excluded.insert(entity);
    }
    excluded
}

/// Compute effective camera distance accounting for mesh collision and recovery.
fn compute_effective_distance(
    cam: &mut WowCamera,
    cam_tf: &Transform,
    eye_target: Vec3,
    orbit_dir: Vec3,
    ray_cast: &mut MeshRayCast,
    terrain: Option<&TerrainHeightmap>,
    excluded: HashSet<Entity>,
    dt: f32,
) -> f32 {
    let terrain_distance = terrain
        .map(|terrain| {
            terrain_adjusted_distance(cam.distance, eye_target, orbit_dir, |x, z| {
                terrain.height_at(x, z)
            })
        })
        .unwrap_or(cam.distance);
    let intended_pos = eye_target - orbit_dir * terrain_distance;
    let ray_dir = (intended_pos - eye_target).normalize_or_zero();
    if ray_dir.length_squared() == 0.0 {
        return terrain_distance;
    }

    let ray = Ray3d::new(eye_target, Dir3::new(ray_dir).unwrap());
    let filter = |entity: Entity| !excluded.contains(&entity);
    let settings = MeshRayCastSettings::default().with_filter(&filter);
    let hits = ray_cast.cast_ray(ray, &settings);
    let closest_hit = hits.first().map(|(_, hit)| hit.distance);
    let adjusted = collision_adjusted_distance(terrain_distance, closest_hit);
    if adjusted < cam.distance {
        cam.collided = true;
        return adjusted;
    }

    if cam.collided {
        let recovery_t = (COLLISION_RECOVERY_SPEED * dt).min(1.0);
        let recovered = cam_tf
            .translation
            .distance(eye_target)
            .lerp(cam.distance, recovery_t);
        if (recovered - cam.distance).abs() < 0.05 {
            cam.collided = false;
        }
        return recovered;
    }

    cam.distance
}

fn follow_target(
    taxi_q: &FollowTaxiQuery<'_, '_>,
    player_q: &FollowPlayerQuery<'_, '_>,
) -> Option<(Entity, Vec3)> {
    if let Ok((entity, transform)) = taxi_q.single() {
        return Some((entity, transform.translation));
    }
    let Ok((entity, transform)) = player_q.single() else {
        return None;
    };
    Some((entity, transform.translation))
}

pub(super) fn camera_follow(
    time: Res<Time>,
    terrain: Option<Res<TerrainHeightmap>>,
    taxi_q: FollowTaxiQuery<'_, '_>,
    player_q: FollowPlayerQuery<'_, '_>,
    mut camera_q: FollowCameraQuery<'_, '_>,
    mut ray_cast: MeshRayCast,
    sky_q: Query<Entity, With<SkyDome>>,
    children_q: Query<&Children>,
) {
    let Some((player_entity, target_translation)) = follow_target(&taxi_q, &player_q) else {
        return;
    };
    let Ok((mut cam, mut cam_tf)) = camera_q.single_mut() else {
        return;
    };

    let dt = time.delta_secs();
    let zoom_t = (cam.zoom_speed * dt).min(1.0);
    cam.distance = cam.distance.lerp(cam.target_distance, zoom_t);
    let follow_t = (cam.follow_speed * dt).min(1.0);
    let eye_target = target_translation + Vec3::Y * EYE_HEIGHT;
    let rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let orbit_dir = rotation * Vec3::NEG_Z;
    let excluded = build_collision_excluded_set(player_entity, &children_q, &sky_q);
    let effective_distance = compute_effective_distance(
        &mut cam,
        &cam_tf,
        eye_target,
        orbit_dir,
        &mut ray_cast,
        terrain.as_deref(),
        excluded,
        dt,
    );
    let mut pos = eye_target - orbit_dir * effective_distance;
    let cam_ground = terrain
        .as_ref()
        .and_then(|heightmap| heightmap.height_at(pos.x, pos.z))
        .unwrap_or(GROUND_Y);
    pos.y = pos.y.max(cam_ground + 0.5);
    cam_tf.translation = cam_tf.translation.lerp(pos, follow_t);
    cam_tf.look_at(eye_target, Vec3::Y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_follow_lerps() {
        let current = Vec3::new(0.0, 5.0, 10.0);
        let target = Vec3::new(10.0, 5.0, 10.0);
        let follow_speed: f32 = 10.0;
        let dt: f32 = 0.016;
        let t = (follow_speed * dt).min(1.0);
        let result = current.lerp(target, t);

        assert!(result.x > current.x, "should move toward target");
        assert!(result.x < target.x, "should not reach target in one frame");
        assert!(
            (result.x - 1.6).abs() < 0.1,
            "expected ~1.6, got {}",
            result.x
        );
    }

    #[test]
    fn test_collision_pulls_camera_forward() {
        let intended = 15.0;
        let hit = Some(8.0);
        let result = collision_adjusted_distance(intended, hit);
        assert!(
            (result - 7.7).abs() < 0.01,
            "expected 8.0 - 0.3 = 7.7, got {}",
            result
        );

        let result_no_hit = collision_adjusted_distance(intended, None);
        assert_eq!(result_no_hit, intended);

        let result_far = collision_adjusted_distance(intended, Some(20.0));
        assert_eq!(result_far, intended);

        let result_close = collision_adjusted_distance(15.0, Some(0.2));
        assert!(
            (result_close - 0.5).abs() < 0.01,
            "should clamp to 0.5, got {}",
            result_close
        );
    }

    #[test]
    fn test_collision_recovery_lerps_back() {
        let current_dist: f32 = 5.0;
        let target_dist: f32 = 15.0;
        let recovery_speed: f32 = 5.0;
        let dt: f32 = 0.016;
        let recovery_t = (recovery_speed * dt).min(1.0);
        let recovered = current_dist.lerp(target_dist, recovery_t);

        assert!(recovered > current_dist, "should move outward");
        assert!(recovered < target_dist, "should not snap to target");
        assert!(
            (recovered - 5.8).abs() < 0.5,
            "expected gradual recovery, got {}",
            recovered
        );
    }

    #[test]
    fn terrain_occlusion_keeps_full_distance_when_segment_is_clear() {
        let eye_target = Vec3::new(0.0, 2.0, 0.0);
        let orbit_dir = Vec3::NEG_Z;
        let intended = 12.0;

        let adjusted = terrain_adjusted_distance(intended, eye_target, orbit_dir, |_, _| Some(0.0));

        assert_eq!(adjusted, intended);
    }

    #[test]
    fn terrain_occlusion_pulls_camera_forward_when_hill_blocks_view() {
        let eye_target = Vec3::new(0.0, 2.0, 0.0);
        let orbit_dir = Vec3::NEG_Z;
        let intended = 12.0;

        let adjusted = terrain_adjusted_distance(intended, eye_target, orbit_dir, |x, z| {
            if x.abs() < 0.5 && z > 5.0 && z < 7.5 {
                Some(2.4)
            } else {
                Some(0.0)
            }
        });

        assert!(adjusted < intended, "hill should pull camera forward");
        assert!(
            adjusted > 0.5,
            "camera should still keep a minimum distance"
        );
    }
}
