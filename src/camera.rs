use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;

pub struct WowCameraPlugin;

impl Plugin for WowCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (camera_input, player_movement, camera_follow).chain());
    }
}

/// Marker for the player entity the camera orbits around.
#[derive(Component)]
pub struct Player;

/// Signals whether the player is currently moving (WASD/mouse).
#[derive(Component, Default)]
pub struct MovementState {
    pub moving: bool,
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

const SENSITIVITY: f32 = 0.003;
const MOVE_SPEED: f32 = 10.0;
const ZOOM_STEP: f32 = 2.0;
const ZOOM_LERP_SPEED: f32 = 10.0;
const PITCH_LIMIT: f32 = 88.0_f32 * std::f32::consts::PI / 180.0;

fn camera_input(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut camera_q: Query<&mut WowCamera>,
) {
    let Ok(mut cam) = camera_q.single_mut() else {
        return;
    };

    let rmb = mouse_buttons.pressed(MouseButton::Right);
    let lmb = mouse_buttons.pressed(MouseButton::Left);

    if rmb || lmb {
        let delta = mouse_motion.delta;
        cam.yaw -= delta.x * SENSITIVITY;
        cam.pitch -= delta.y * SENSITIVITY;
        cam.pitch = cam.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    if mouse_scroll.delta.y != 0.0 {
        cam.target_distance -= mouse_scroll.delta.y * ZOOM_STEP;
        cam.target_distance = cam.target_distance.clamp(cam.min_distance, cam.max_distance);
    }
}

fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    camera_q: Query<&WowCamera>,
    mut player_q: Query<(&mut Transform, &mut MovementState), With<Player>>,
) {
    let Ok(cam) = camera_q.single() else {
        return;
    };
    let Ok((mut transform, mut movement)) = player_q.single_mut() else {
        return;
    };

    let mut direction = Vec3::ZERO;

    // WASD relative to camera yaw (horizontal plane only)
    let forward = Vec3::new(-cam.yaw.sin(), 0.0, -cam.yaw.cos());
    let right = Vec3::new(forward.z, 0.0, -forward.x);

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

    // Both mouse buttons = move forward
    if mouse_buttons.pressed(MouseButton::Left) && mouse_buttons.pressed(MouseButton::Right) {
        direction += forward;
    }

    movement.moving = direction.length_squared() > 0.0;
    if movement.moving {
        direction = direction.normalize();
        transform.translation += direction * MOVE_SPEED * time.delta_secs();
    }
}

fn camera_follow(
    time: Res<Time>,
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
    cam_tf.translation = eye_target + offset;
    cam_tf.look_at(eye_target, Vec3::Y);
}
