//! Pure in-world orbit, terrain and mesh collision, and follow calculations.

use glam::{EulerRot, FloatExt, Quat, Vec3};

use crate::camera_control_data::CameraState;

pub const EYE_HEIGHT: f32 = 1.8;
pub const COLLISION_OFFSET: f32 = 0.3;
pub const COLLISION_RECOVERY_SPEED: f32 = 5.0;
const TERRAIN_COLLISION_STEPS: usize = 24;
const TERRAIN_COLLISION_CLEARANCE: f32 = 0.2;

/// Camera's last followed player point and adopted movement epoch.
#[derive(Default)]
pub struct TeleportAnchor {
    previous: Option<(u32, Vec3)>,
}

impl TeleportAnchor {
    pub fn translate_camera(&mut self, current: Vec3, player: Vec3, epoch: Option<u32>) -> Vec3 {
        let Some(epoch) = epoch else {
            return current;
        };
        let translated = match self.previous {
            Some((previous_epoch, previous_player)) if previous_epoch != epoch => {
                current + player - previous_player
            }
            _ => current,
        };
        self.previous = Some((epoch, player));
        translated
    }
}

#[cfg(test)]
mod teleport_tests {
    use super::*;
    #[test]
    fn teleport_translates_camera_and_preserves_orbit_without_follow_lag() {
        let mut anchor = TeleportAnchor::default();
        let camera = Vec3::new(10.0, 5.0, 25.0);
        let player = Vec3::new(10.0, 2.0, 20.0);
        assert_eq!(anchor.translate_camera(camera, player, Some(4)), camera);
        let destination = Vec3::new(30.0, 2.0, 20.0);
        let shifted = anchor.translate_camera(camera, destination, Some(5));
        assert_eq!(shifted - destination, camera - player);
        assert_eq!(
            anchor.translate_camera(shifted, destination + Vec3::X, Some(5)),
            shifted
        );
    }
}

pub struct CameraPose {
    pub position: Vec3,
    pub eye_target: Vec3,
}

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
    terrain_height_at: &mut dyn FnMut(f32, f32) -> Option<f32>,
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

fn effective_distance(
    cam: &mut CameraState,
    eye_target: Vec3,
    orbit_dir: Vec3,
    terrain_height_at: Option<&mut dyn FnMut(f32, f32) -> Option<f32>>,
    mesh_hit_at: &mut impl FnMut(Vec3, Vec3) -> Option<f32>,
    dt: f32,
) -> f32 {
    let terrain_distance = terrain_height_at
        .map(|height_at| terrain_adjusted_distance(cam.distance, eye_target, orbit_dir, height_at))
        .unwrap_or(cam.distance);
    let intended_pos = eye_target - orbit_dir * terrain_distance;
    let ray_dir = (intended_pos - eye_target).normalize_or_zero();
    if ray_dir.length_squared() == 0.0 {
        return terrain_distance;
    }

    let closest_hit = mesh_hit_at(eye_target, ray_dir);
    let adjusted = collision_adjusted_distance(terrain_distance, closest_hit);
    if adjusted < cam.distance {
        cam.collision_distance = Some(adjusted);
        return adjusted;
    }

    let Some(pulled_in) = cam.collision_distance else {
        return cam.distance;
    };
    // Recover from the pulled-in orbit distance, not the smoothed camera's distance to the moving
    // eye: that includes follow lag, so a running player would hold recovery open indefinitely.
    let recovery_t = (COLLISION_RECOVERY_SPEED * dt).min(1.0);
    let recovered = pulled_in.lerp(cam.distance, recovery_t);
    cam.collision_distance = ((recovered - cam.distance).abs() >= 0.05).then_some(recovered);
    recovered
}

/// Advance the original camera calculation. Callers supply actual terrain heights, mesh ray
/// hits, and map-specific ground height; `None` ground means no floor (WMO-only maps).
/// The mesh-hit callback must perform a real ray cast; it is never replaced with a no-hit default.
pub fn follow_camera(
    cam: &mut CameraState,
    current_position: Vec3,
    target_translation: Vec3,
    dt: f32,
    terrain_height_at: Option<&mut dyn FnMut(f32, f32) -> Option<f32>>,
    mut mesh_hit_at: impl FnMut(Vec3, Vec3) -> Option<f32>,
    mut ground_at: impl FnMut(Vec3) -> Option<f32>,
) -> CameraPose {
    let zoom_t = (cam.zoom_speed * dt).min(1.0);
    cam.distance = cam.distance.lerp(cam.target_distance, zoom_t);
    let follow_t = (cam.follow_speed * dt).min(1.0);
    let eye_target = target_translation + Vec3::Y * EYE_HEIGHT;
    let rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let orbit_dir = rotation * Vec3::NEG_Z;
    let distance = effective_distance(
        cam,
        eye_target,
        orbit_dir,
        terrain_height_at,
        &mut mesh_hit_at,
        dt,
    );
    let mut pos = eye_target - orbit_dir * distance;
    if let Some(ground) = ground_at(pos) {
        pos.y = pos.y.max(ground + 0.5);
    }
    CameraPose {
        position: current_position.lerp(pos, follow_t),
        eye_target,
    }
}

/// Pull a smoothed camera position back in front of the first blocker between it and the eye.
/// `follow_camera` ray-checks only the target pose; smoothing moves the camera on a straight
/// line from its last pose, which can cut through a stair nose or a wall corner.
pub fn keep_in_sight(
    eye_target: Vec3,
    camera: Vec3,
    mut mesh_hit_at: impl FnMut(Vec3, Vec3) -> Option<f32>,
) -> Vec3 {
    let offset = camera - eye_target;
    let distance = offset.length();
    let direction = offset.normalize_or_zero();
    if direction == Vec3::ZERO {
        return camera;
    }
    match mesh_hit_at(eye_target, direction) {
        Some(hit) if hit < distance => {
            eye_target + direction * collision_adjusted_distance(distance, Some(hit))
        }
        _ => camera,
    }
}
