use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;

use crate::terrain::TerrainHeightmap;

pub struct WowCameraPlugin;

impl Plugin for WowCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (camera_input, cursor_grab, player_movement, camera_follow).chain());
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
        }
    }
}

const SENSITIVITY: f32 = 0.01;
const WALK_SPEED: f32 = 2.5; // M2 Walk movespeed (2.5 yards/sec)
const RUN_SPEED: f32 = 7.0; // M2 Run movespeed (7.0 yards/sec)
const ZOOM_STEP: f32 = 2.0;
const KEY_ROTATE_SPEED: f32 = 2.5; // radians/sec for arrow key rotation
const KEY_ZOOM_SPEED: f32 = 15.0; // units/sec for page up/down zoom
const ZOOM_LERP_SPEED: f32 = 10.0;
const PITCH_LIMIT: f32 = 88.0_f32 * std::f32::consts::PI / 180.0;

fn apply_keyboard_camera(
    keys: &ButtonInput<KeyCode>,
    dt: f32,
    cam: &mut WowCamera,
    facing_q: &mut Query<&mut CharacterFacing, With<Player>>,
) {
    // Arrow keys: rotate camera + character facing (like RMB drag)
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::ArrowRight) {
        let sign = if keys.pressed(KeyCode::ArrowLeft) { 1.0 } else { -1.0 };
        let yaw_delta = sign * KEY_ROTATE_SPEED * dt;
        cam.yaw += yaw_delta;
        if let Ok(mut facing) = facing_q.single_mut() {
            facing.yaw += yaw_delta;
        }
    }
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::ArrowDown) {
        let sign = if keys.pressed(KeyCode::ArrowUp) { 1.0 } else { -1.0 };
        cam.pitch = (cam.pitch + sign * KEY_ROTATE_SPEED * dt).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }
    // Page Up/Down: zoom in/out
    if keys.pressed(KeyCode::PageUp) || keys.pressed(KeyCode::PageDown) {
        let sign = if keys.pressed(KeyCode::PageUp) { -1.0 } else { 1.0 };
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
        cam.target_distance = cam.target_distance.clamp(cam.min_distance, cam.max_distance);
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
    if keys.pressed(KeyCode::KeyW) { direction += forward; }
    if keys.pressed(KeyCode::KeyS) { direction -= forward; }
    if keys.pressed(KeyCode::KeyA) { direction -= right; }
    if keys.pressed(KeyCode::KeyD) { direction += right; }
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
fn update_jump_arc(movement: &mut MovementState, transform: &mut Transform, dt: f32, ground_y: f32) {
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
    mut player_q: Query<
        (&mut Transform, &mut MovementState, &CharacterFacing),
        With<Player>,
    >,
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

    let speed = if movement.running { RUN_SPEED } else { WALK_SPEED };
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
    let Ok(mut cursor) = cursor_q.single_mut() else { return };
    let held = mouse_buttons.pressed(MouseButton::Left) || mouse_buttons.pressed(MouseButton::Right);
    if held {
        cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
        cursor.visible = false;
    } else {
        cursor.grab_mode = bevy::window::CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn camera_follow(
    time: Res<Time>,
    terrain: Option<Res<TerrainHeightmap>>,
    player_q: Query<&Transform, (With<Player>, Without<WowCamera>)>,
    mut camera_q: Query<(&mut WowCamera, &mut Transform), Without<Player>>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let Ok((mut cam, mut cam_tf)) = camera_q.single_mut() else {
        return;
    };

    // Smooth zoom
    let dt = time.delta_secs();
    cam.distance = cam.distance.lerp(cam.target_distance, ZOOM_LERP_SPEED * dt);

    // Orbit offset from yaw/pitch
    let rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let offset = rotation * Vec3::new(0.0, 0.0, cam.distance);

    let eye_target = player_tf.translation + Vec3::Y * 1.5;
    let mut pos = eye_target + offset;
    let cam_ground = terrain
        .as_ref()
        .and_then(|t| t.height_at(pos.x, pos.z))
        .unwrap_or(GROUND_Y);
    pos.y = pos.y.max(cam_ground + 0.5);
    cam_tf.translation = pos;
    cam_tf.look_at(eye_target, Vec3::Y);
}
