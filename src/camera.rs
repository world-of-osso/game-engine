use std::collections::HashSet;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::prelude::*;

use crate::collision::{self, CharacterPhysics};
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
        app.init_resource::<crate::client_options::CameraOptions>();
        app.add_systems(
            Update,
            (sync_camera_options, camera_input, cursor_grab, player_movement, camera_follow)
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

/// Base Y position (ground level) for the player (when no terrain is loaded).
const GROUND_Y: f32 = 0.0;

/// Signals current movement direction, run/walk toggle, and jump state.
#[derive(Component)]
pub struct MovementState {
    pub direction: MoveDirection,
    pub running: bool,
    pub jumping: bool,
}

impl Default for MovementState {
    fn default() -> Self {
        Self {
            direction: MoveDirection::None,
            running: true, // WoW defaults to running
            jumping: false,
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
        Self {
            yaw: std::f32::consts::PI,
        }
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
const LANDING_EPSILON: f32 = 0.05;

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
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::game_menu_screen::UiModalOpen>>,
    options: Res<crate::client_options::CameraOptions>,
    mut camera_q: Query<&mut WowCamera>,
    mut facing_q: Query<&mut CharacterFacing, With<Player>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    let Ok(mut cam) = camera_q.single_mut() else {
        return;
    };

    let rmb = mouse_buttons.pressed(MouseButton::Right);
    let lmb = mouse_buttons.pressed(MouseButton::Left);
    let delta = mouse_motion.delta;
    let dt = time.delta_secs();

    if rmb {
        // RMB: character snaps to face camera direction, then both rotate together
        let yaw_delta = -delta.x * options.look_sensitivity;
        cam.yaw += yaw_delta;
        cam.pitch += camera_pitch_delta(delta.y, &options);
        cam.pitch = cam.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        if let Ok(mut facing) = facing_q.single_mut() {
            facing.yaw = cam.yaw + std::f32::consts::PI;
        }
    } else if lmb {
        // LMB: orbit camera only (character doesn't turn)
        cam.yaw -= delta.x * options.look_sensitivity;
        cam.pitch += camera_pitch_delta(delta.y, &options);
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

fn camera_pitch_delta(mouse_delta_y: f32, options: &crate::client_options::CameraOptions) -> f32 {
    let sign = if options.invert_y { 1.0 } else { -1.0 };
    mouse_delta_y * options.look_sensitivity * sign
}

fn sync_camera_options(
    options: Res<crate::client_options::CameraOptions>,
    mut camera_q: Query<&mut WowCamera>,
) {
    if !options.is_changed() {
        return;
    }
    for mut camera in &mut camera_q {
        camera.follow_speed = options.follow_speed;
        camera.zoom_speed = options.zoom_speed;
        camera.min_distance = options.min_distance;
        camera.max_distance = options.max_distance.max(options.min_distance + 1.0);
        camera.target_distance = camera
            .target_distance
            .clamp(camera.min_distance, camera.max_distance);
        camera.distance = camera.distance.clamp(camera.min_distance, camera.max_distance);
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

fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    terrain: Option<Res<TerrainHeightmap>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut player_q: Query<
        (
            &mut Transform,
            &mut MovementState,
            &CharacterFacing,
            &mut CharacterPhysics,
        ),
        With<Player>,
    >,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        return;
    }
    let Ok((mut transform, mut movement, facing, mut physics)) = player_q.single_mut() else {
        return;
    };

    let (direction, anim_dir) = compute_movement_input(&keys, &mouse_buttons, facing);
    movement.direction = anim_dir;

    if keys.just_pressed(KeyCode::KeyZ) {
        movement.running = !movement.running;
    }

    let speed = if movement.running {
        RUN_SPEED
    } else {
        WALK_SPEED
    };
    apply_horizontal_movement(
        &mut transform,
        &mut movement,
        &mut physics,
        &keys,
        direction,
        speed,
        time.delta_secs(),
        terrain.as_deref(),
    );

    transform.rotation = Quat::from_rotation_y(facing.yaw - std::f32::consts::FRAC_PI_2);
}

#[allow(clippy::too_many_arguments)]
fn apply_horizontal_movement(
    transform: &mut Transform,
    movement: &mut MovementState,
    physics: &mut CharacterPhysics,
    keys: &ButtonInput<KeyCode>,
    direction: Vec3,
    speed: f32,
    dt: f32,
    terrain: Option<&TerrainHeightmap>,
) {
    if direction.length_squared() > 0.0 {
        let dir = direction.normalize();
        let proposed = transform.translation + dir * speed * dt;
        transform.translation = match terrain {
            Some(t) => collision::validate_movement_slope(
                transform.translation,
                proposed,
                t,
                physics.grounded && !movement.jumping,
            ),
            None => proposed,
        };
    }

    if keys.just_pressed(KeyCode::Space) && physics.grounded && !movement.jumping {
        movement.jumping = true;
        physics.vertical_velocity = collision::JUMP_IMPULSE;
    }
    if movement.jumping
        && physics.vertical_velocity <= 0.0
        && should_end_jump(transform, physics, terrain)
    {
        movement.jumping = false;
    }
}

fn should_end_jump(
    transform: &Transform,
    physics: &CharacterPhysics,
    terrain: Option<&TerrainHeightmap>,
) -> bool {
    if !physics.grounded {
        return false;
    }

    match terrain.and_then(|t| t.height_at(transform.translation.x, transform.translation.z)) {
        Some(ground_y) => transform.translation.y <= ground_y + LANDING_EPSILON,
        None => true,
    }
}

fn cursor_grab(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    let Ok(mut cursor) = cursor_q.single_mut() else {
        return;
    };
    if !crate::networking::gameplay_input_allowed(reconnect) {
        cursor.grab_mode = bevy::window::CursorGrabMode::None;
        cursor.visible = true;
        return;
    }
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

/// Build the set of entities excluded from camera collision (player + children + sky).
fn build_collision_excluded_set(
    player_entity: Entity,
    children_q: &Query<&Children>,
    sky_q: &Query<Entity, With<SkyDome>>,
) -> HashSet<Entity> {
    let mut excluded = HashSet::new();
    excluded.insert(player_entity);
    collect_descendants(player_entity, children_q, &mut excluded);
    for e in sky_q.iter() {
        excluded.insert(e);
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
    excluded: HashSet<Entity>,
    dt: f32,
) -> f32 {
    let intended_pos = eye_target - orbit_dir * cam.distance;
    let ray_dir = (intended_pos - eye_target).normalize_or_zero();
    if ray_dir.length_squared() == 0.0 {
        return cam.distance;
    }
    let ray = Ray3d::new(eye_target, Dir3::new(ray_dir).unwrap());
    let filter = |entity: Entity| !excluded.contains(&entity);
    let settings = MeshRayCastSettings::default().with_filter(&filter);
    let hits = ray_cast.cast_ray(ray, &settings);
    let closest_hit = hits.first().map(|(_, hit)| hit.distance);
    let adjusted = collision_adjusted_distance(cam.distance, closest_hit);
    if adjusted < cam.distance {
        cam.collided = true;
        adjusted
    } else if cam.collided {
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
}

#[allow(clippy::type_complexity)]
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
    let zoom_t = (cam.zoom_speed * dt).min(1.0);
    cam.distance = cam.distance.lerp(cam.target_distance, zoom_t);
    let follow_t = (cam.follow_speed * dt).min(1.0);
    let eye_target = player_tf.translation + Vec3::Y * EYE_HEIGHT;
    let rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let orbit_dir = rotation * Vec3::NEG_Z;
    let excluded = build_collision_excluded_set(player_entity, &children_q, &sky_q);
    let effective_distance = compute_effective_distance(
        &mut cam,
        &cam_tf,
        eye_target,
        orbit_dir,
        &mut ray_cast,
        excluded,
        dt,
    );
    let mut pos = eye_target - orbit_dir * effective_distance;
    let cam_ground = terrain
        .as_ref()
        .and_then(|t| t.height_at(pos.x, pos.z))
        .unwrap_or(GROUND_Y);
    pos.y = pos.y.max(cam_ground + 0.5);
    cam_tf.translation = cam_tf.translation.lerp(pos, follow_t);
    cam_tf.look_at(eye_target, Vec3::Y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain_heightmap::TerrainHeightmap;

    fn jump_heightmap() -> TerrainHeightmap {
        let data = std::fs::read("data/terrain/azeroth_32_48.adt")
            .expect("expected test ADT data/terrain/azeroth_32_48.adt");
        let adt =
            crate::asset::adt::load_adt_for_tile(&data, 32, 48).expect("expected ADT to parse");
        let mut heightmap = TerrainHeightmap::default();
        heightmap.insert_tile(32, 48, &adt);
        heightmap
    }

    fn jump_fixture(heightmap: &TerrainHeightmap) -> (Transform, MovementState, CharacterPhysics) {
        let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
        let ground_y = heightmap
            .height_at(bx, bz)
            .expect("expected terrain at sample position");
        let transform = Transform::from_xyz(bx, ground_y + 0.2, bz);
        let movement = MovementState {
            direction: MoveDirection::None,
            running: true,
            jumping: true,
        };
        let physics = CharacterPhysics {
            vertical_velocity: -1.0,
            grounded: true,
        };
        (transform, movement, physics)
    }

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

    #[test]
    fn jump_state_stays_active_until_player_reaches_ground() {
        let heightmap = jump_heightmap();
        let (mut transform, mut movement, mut physics) = jump_fixture(&heightmap);
        let keys = ButtonInput::<KeyCode>::default();

        apply_horizontal_movement(
            &mut transform,
            &mut movement,
            &mut physics,
            &keys,
            Vec3::ZERO,
            0.0,
            0.0,
            Some(&heightmap),
        );

        assert!(
            movement.jumping,
            "jumping should stay active until the player actually reaches the ground"
        );
    }
}
