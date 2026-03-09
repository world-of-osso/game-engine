use std::collections::HashSet;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::prelude::*;

use crate::game_state::GameState;
use crate::sky::SkyDome;
use crate::terrain_heightmap::TerrainHeightmap;

/// Recursively collect all descendant entities into the set.
fn collect_descendants(entity: Entity, children_q: &Query<&Children>, out: &mut HashSet<Entity>) {
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            out.insert(child);
            collect_descendants(child, children_q, out);
        }
    }
}

pub struct WowCameraPlugin;

impl Plugin for WowCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (camera_input, cursor_grab, player_movement, camera_follow)
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

/// Marker for the player entity the camera orbits around.
#[derive(Component)]
pub struct Player;

/// Movement direction relative to the character's facing.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    #[default]
    None,
    Forward,
    Backward,
    Left,
    Right,
}

/// Total jump duration in seconds (up + down).
const JUMP_DURATION: f32 = 0.8;
/// Peak height of jump arc in world units.
const JUMP_HEIGHT: f32 = 2.5;
/// Base Y position (ground level) for the player.
const GROUND_Y: f32 = 0.0;

/// Signals current movement direction, run/walk toggle, and jump state.
#[derive(Component)]
pub struct MovementState {
    pub direction: MoveDirection,
    pub running: bool,
    pub jumping: bool,
    /// Elapsed time into the jump arc (seconds).
    pub jump_elapsed: f32,
}

impl Default for MovementState {
    fn default() -> Self {
        Self {
            direction: MoveDirection::None,
            running: true, // WoW defaults to running
            jumping: false,
            jump_elapsed: 0.0,
        }
    }
}

/// Character facing yaw (radians). RMB rotates this; the model entity rotation follows.
#[derive(Component)]
pub struct CharacterFacing {
    pub yaw: f32,
}

impl Default for CharacterFacing {
    fn default() -> Self {
        Self { yaw: 0.0 }
    }
}

#[derive(Component)]
pub struct WowCamera {
    pub pitch: f32,
    pub yaw: f32,
    pub distance: f32,
    pub target_distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    /// How fast the camera follows the player position (lerp speed).
    pub follow_speed: f32,
    /// How fast the camera zooms toward target_distance (lerp speed).
    pub zoom_speed: f32,
    /// Whether the camera is currently pulled in due to collision.
    pub collided: bool,
}

impl Default for WowCamera {
    fn default() -> Self {
        Self {
            pitch: -0.3,
            yaw: 0.0,
            distance: 15.0,
            target_distance: 15.0,
            min_distance: 2.0,
            max_distance: 40.0,
            follow_speed: 10.0,
            zoom_speed: 8.0,
            collided: false,
        }
    }
}

pub(crate) fn spawn_wow_camera(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Camera3d::default(),
            Transform::default(),
            WowCamera::default(),
        ))
        .id()
}

const SENSITIVITY: f32 = 0.01;
const WALK_SPEED: f32 = 2.5; // M2 Walk movespeed (2.5 yards/sec)
const RUN_SPEED: f32 = 7.0; // M2 Run movespeed (7.0 yards/sec)
const ZOOM_STEP: f32 = 2.0;
const KEY_ROTATE_SPEED: f32 = 2.5; // radians/sec for arrow key rotation
const KEY_ZOOM_SPEED: f32 = 15.0; // units/sec for page up/down zoom
const COLLISION_OFFSET: f32 = 0.3;
const EYE_HEIGHT: f32 = 1.8;
/// Speed at which camera recovers (lerps back out) after collision clears.
const COLLISION_RECOVERY_SPEED: f32 = 5.0;
const PITCH_LIMIT: f32 = 88.0_f32 * std::f32::consts::PI / 180.0;

fn apply_keyboard_camera(
    keys: &ButtonInput<KeyCode>,
    dt: f32,
    cam: &mut WowCamera,
    facing_q: &mut Query<&mut CharacterFacing, With<Player>>,
) {
    // Arrow keys: rotate camera + character facing (like RMB drag)
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::ArrowRight) {
        let sign = if keys.pressed(KeyCode::ArrowLeft) {
            1.0
        } else {
            -1.0
        };
        let yaw_delta = sign * KEY_ROTATE_SPEED * dt;
        cam.yaw += yaw_delta;
        if let Ok(mut facing) = facing_q.single_mut() {
            facing.yaw += yaw_delta;
        }
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::ArrowDown) {
        let sign = if keys.pressed(KeyCode::ArrowUp) {
            1.0
        } else {
            -1.0
        };
        cam.pitch = (cam.pitch + sign * KEY_ROTATE_SPEED * dt).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }
    // Page Up/Down: zoom in/out
    if keys.pressed(KeyCode::PageUp) || keys.pressed(KeyCode::PageDown) {
        let sign = if keys.pressed(KeyCode::PageUp) {
            -1.0
        } else {
            1.0
        };
        cam.target_distance = (cam.target_distance + sign * KEY_ZOOM_SPEED * dt)
            .clamp(cam.min_distance, cam.max_distance);
    }
}

fn camera_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut camera_q: Query<&mut WowCamera>,
    mut facing_q: Query<&mut CharacterFacing, With<Player>>,
) {
    let Ok(mut cam) = camera_q.single_mut() else {
        return;
    };

    let rmb = mouse_buttons.pressed(MouseButton::Right);
    let lmb = mouse_buttons.pressed(MouseButton::Left);
    let delta = mouse_motion.delta;
    let dt = time.delta_secs();

    if rmb {
        // RMB: character snaps to face camera direction, then both rotate together
        let yaw_delta = -delta.x * SENSITIVITY;
        cam.yaw += yaw_delta;
        cam.pitch -= delta.y * SENSITIVITY;
        cam.pitch = cam.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        if let Ok(mut facing) = facing_q.single_mut() {
            facing.yaw = cam.yaw + std::f32::consts::PI;
        }
    } else if lmb {
        // LMB: orbit camera only (character doesn't turn)
        cam.yaw -= delta.x * SENSITIVITY;
        cam.pitch -= delta.y * SENSITIVITY;
        cam.pitch = cam.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    apply_keyboard_camera(&keys, dt, &mut cam, &mut facing_q);

    if mouse_scroll.delta.y != 0.0 {
        cam.target_distance -= mouse_scroll.delta.y * ZOOM_STEP;
        cam.target_distance = cam
            .target_distance
            .clamp(cam.min_distance, cam.max_distance);
    }
}

/// Compute movement direction vector and animation direction from input.
fn compute_movement_input(
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    facing: &CharacterFacing,
) -> (Vec3, MoveDirection) {
    let forward = Vec3::new(facing.yaw.sin(), 0.0, facing.yaw.cos());
    let right = Vec3::new(-forward.z, 0.0, forward.x);

    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction += right;
    }
    if mouse_buttons.pressed(MouseButton::Left) && mouse_buttons.pressed(MouseButton::Right) {
        direction += forward;
    }

    let fwd = keys.pressed(KeyCode::KeyW)
        || (mouse_buttons.pressed(MouseButton::Left) && mouse_buttons.pressed(MouseButton::Right));
    let anim_dir = if fwd {
        MoveDirection::Forward
    } else if keys.pressed(KeyCode::KeyS) {
        MoveDirection::Backward
    } else if keys.pressed(KeyCode::KeyA) {
        MoveDirection::Left
    } else if keys.pressed(KeyCode::KeyD) {
        MoveDirection::Right
    } else {
        MoveDirection::None
    };

    (direction, anim_dir)
}

/// Update jump arc height or land when duration expires.
fn update_jump_arc(
    movement: &mut MovementState,
    transform: &mut Transform,
    dt: f32,
    ground_y: f32,
) {
    movement.jump_elapsed += dt;
    if movement.jump_elapsed >= JUMP_DURATION {
        transform.translation.y = ground_y;
        movement.jumping = false;
    } else {
        let t = movement.jump_elapsed / JUMP_DURATION;
        transform.translation.y = ground_y + JUMP_HEIGHT * 4.0 * t * (1.0 - t);
    }
}

fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    terrain: Option<Res<TerrainHeightmap>>,
    mut player_q: Query<(&mut Transform, &mut MovementState, &CharacterFacing), With<Player>>,
) {
    let Ok((mut transform, mut movement, facing)) = player_q.single_mut() else {
        return;
    };

    let (direction, anim_dir) = compute_movement_input(&keys, &mouse_buttons, &facing);
    movement.direction = anim_dir;

    if keys.just_pressed(KeyCode::KeyZ) {
        movement.running = !movement.running;
    }

    if keys.just_pressed(KeyCode::Space) && !movement.jumping {
        movement.jumping = true;
        movement.jump_elapsed = 0.0;
    }

    let speed = if movement.running {
        RUN_SPEED
    } else {
        WALK_SPEED
    };
    if direction.length_squared() > 0.0 {
        let dir = direction.normalize();
        transform.translation += dir * speed * time.delta_secs();
    }

    let ground_y = terrain
        .as_ref()
        .and_then(|t| t.height_at(transform.translation.x, transform.translation.z))
        .unwrap_or(GROUND_Y);

    if movement.jumping {
        update_jump_arc(&mut movement, &mut transform, time.delta_secs(), ground_y);
    } else {
        transform.translation.y = ground_y;
    }

    transform.rotation = Quat::from_rotation_y(facing.yaw - std::f32::consts::FRAC_PI_2);
}

fn cursor_grab(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    let Ok(mut cursor) = cursor_q.single_mut() else {
        return;
    };
    let held =
        mouse_buttons.pressed(MouseButton::Left) || mouse_buttons.pressed(MouseButton::Right);
    if held {
        cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
        cursor.visible = false;
    } else {
        cursor.grab_mode = bevy::window::CursorGrabMode::None;
        cursor.visible = true;
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

fn camera_follow(
    time: Res<Time>,
    terrain: Option<Res<TerrainHeightmap>>,
    player_q: Query<(Entity, &Transform), (With<Player>, Without<WowCamera>)>,
    mut camera_q: Query<(&mut WowCamera, &mut Transform), Without<Player>>,
    mut ray_cast: MeshRayCast,
    sky_q: Query<Entity, With<SkyDome>>,
    children_q: Query<&Children>,
) {
    let Ok((player_entity, player_tf)) = player_q.single() else {
        return;
    };
    let Ok((mut cam, mut cam_tf)) = camera_q.single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    // Smooth zoom: lerp actual distance toward target
    let zoom_t = (cam.zoom_speed * dt).min(1.0);
    cam.distance = cam.distance.lerp(cam.target_distance, zoom_t);

    // Smooth follow: lerp camera focus toward player position
    let follow_t = (cam.follow_speed * dt).min(1.0);
    let eye_target = player_tf.translation + Vec3::Y * EYE_HEIGHT;

    // Orbit offset from yaw/pitch
    let rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let orbit_dir = rotation * Vec3::NEG_Z;
    let intended_pos = eye_target - orbit_dir * cam.distance;

    // Collision: raycast from player eye to intended camera position
    let ray_dir = (intended_pos - eye_target).normalize_or_zero();
    let effective_distance = if ray_dir.length_squared() > 0.0 {
        let ray = Ray3d::new(eye_target, Dir3::new(ray_dir).unwrap());
        // Exclude player model (+ children), sky dome from collision
        let mut excluded = HashSet::new();
        excluded.insert(player_entity);
        collect_descendants(player_entity, &children_q, &mut excluded);
        for e in sky_q.iter() {
            excluded.insert(e);
        }
        let filter = |entity: Entity| !excluded.contains(&entity);
        let settings = MeshRayCastSettings::default().with_filter(&filter);
        let hits = ray_cast.cast_ray(ray, &settings);
        let closest_hit = hits.first().map(|(_, hit)| hit.distance);
        let adjusted = collision_adjusted_distance(cam.distance, closest_hit);
        if adjusted < cam.distance {
            cam.collided = true;
            adjusted // snap in immediately
        } else if cam.collided {
            // Collision cleared — lerp back out smoothly
            let recovery_t = (COLLISION_RECOVERY_SPEED * dt).min(1.0);
            let recovered = cam_tf
                .translation
                .distance(eye_target)
                .lerp(cam.distance, recovery_t);
            if (recovered - cam.distance).abs() < 0.05 {
                cam.collided = false;
            }
            recovered
        } else {
            cam.distance
        }
    } else {
        cam.distance
    };

    let mut pos = eye_target - orbit_dir * effective_distance;

    // Clamp camera above terrain
    let cam_ground = terrain
        .as_ref()
        .and_then(|t| t.height_at(pos.x, pos.z))
        .unwrap_or(GROUND_Y);
    pos.y = pos.y.max(cam_ground + 0.5);

    // Smooth follow for final position
    cam_tf.translation = cam_tf.translation.lerp(pos, follow_t);
    cam_tf.look_at(eye_target, Vec3::Y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_follow_lerps() {
        // Given current pos far from target, lerp should move partway
        let current = Vec3::new(0.0, 5.0, 10.0);
        let target = Vec3::new(10.0, 5.0, 10.0);
        let follow_speed: f32 = 10.0;
        let dt: f32 = 0.016; // ~60fps
        let t = (follow_speed * dt).min(1.0);
        let result = current.lerp(target, t);

        // Should move toward target but not reach it in one frame
        assert!(result.x > current.x, "should move toward target");
        assert!(result.x < target.x, "should not reach target in one frame");
        assert!(
            (result.x - 1.6).abs() < 0.1,
            "expected ~1.6, got {}",
            result.x
        );
    }

    #[test]
    fn test_zoom_interpolation() {
        // When target_distance differs from distance, distance should move toward it
        let mut distance: f32 = 15.0;
        let target_distance: f32 = 5.0;
        let zoom_speed: f32 = 8.0;
        let dt: f32 = 0.016;

        // Simulate a few frames
        for _ in 0..10 {
            let t = (zoom_speed * dt).min(1.0);
            distance = distance.lerp(target_distance, t);
        }

        assert!(distance < 15.0, "distance should decrease toward target");
        assert!(distance > 5.0, "should not reach target in 10 frames");
        // After 10 frames at 8.0 speed, should be noticeably closer
        assert!(
            distance < 10.0,
            "expected significant progress, got {}",
            distance
        );
    }

    #[test]
    fn test_collision_pulls_camera_forward() {
        // Hit closer than intended -> clamp
        let intended = 15.0;
        let hit = Some(8.0);
        let result = collision_adjusted_distance(intended, hit);
        assert!(
            (result - 7.7).abs() < 0.01,
            "expected 8.0 - 0.3 = 7.7, got {}",
            result
        );

        // No hit -> keep intended
        let result_no_hit = collision_adjusted_distance(intended, None);
        assert_eq!(result_no_hit, intended);

        // Hit farther than intended -> keep intended
        let result_far = collision_adjusted_distance(intended, Some(20.0));
        assert_eq!(result_far, intended);

        // Very close hit -> clamp to minimum 0.5
        let result_close = collision_adjusted_distance(15.0, Some(0.2));
        assert!(
            (result_close - 0.5).abs() < 0.01,
            "should clamp to 0.5, got {}",
            result_close
        );
    }

    #[test]
    fn test_collision_recovery_lerps_back() {
        // Simulate recovery: camera was at 5.0 (collided), target is 15.0
        let current_dist: f32 = 5.0;
        let target_dist: f32 = 15.0;
        let recovery_speed: f32 = 5.0;
        let dt: f32 = 0.016;
        let recovery_t = (recovery_speed * dt).min(1.0);
        let recovered = current_dist.lerp(target_dist, recovery_t);

        // Should move toward target but not snap
        assert!(recovered > current_dist, "should move outward");
        assert!(recovered < target_dist, "should not snap to target");
        assert!(
            (recovered - 5.8).abs() < 0.5,
            "expected gradual recovery, got {}",
            recovered
        );
    }
}
