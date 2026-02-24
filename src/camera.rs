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

/// Movement direction relative to the character's facing.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
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
const GROUND_Y: f32 = 0.5;

/// Signals current movement direction and jump state.
#[derive(Component, Default)]
pub struct MovementState {
    pub direction: MoveDirection,
    pub jumping: bool,
    /// Elapsed time into the jump arc (seconds).
    pub jump_elapsed: f32,
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
const MOVE_SPEED: f32 = 2.5; // Match M2 Walk movespeed (2.5 yards/sec)
const ZOOM_STEP: f32 = 2.0;
const ZOOM_LERP_SPEED: f32 = 10.0;
const PITCH_LIMIT: f32 = 88.0_f32 * std::f32::consts::PI / 180.0;

fn camera_input(
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

    if rmb {
        // RMB: rotate character facing + camera follows
        let yaw_delta = -delta.x * SENSITIVITY;
        cam.yaw += yaw_delta;
        cam.pitch -= delta.y * SENSITIVITY;
        cam.pitch = cam.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        if let Ok(mut facing) = facing_q.single_mut() {
            facing.yaw += yaw_delta;
        }
    } else if lmb {
        // LMB: orbit camera only (character doesn't turn)
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
    mut player_q: Query<
        (&mut Transform, &mut MovementState, &CharacterFacing),
        With<Player>,
    >,
) {
    let Ok((mut transform, mut movement, facing)) = player_q.single_mut() else {
        return;
    };

    // Movement relative to character facing (not camera)
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
    let back = keys.pressed(KeyCode::KeyS);
    let left = keys.pressed(KeyCode::KeyA);
    let right_key = keys.pressed(KeyCode::KeyD);

    movement.direction = if fwd {
        MoveDirection::Forward
    } else if back {
        MoveDirection::Backward
    } else if left {
        MoveDirection::Left
    } else if right_key {
        MoveDirection::Right
    } else {
        MoveDirection::None
    };

    if keys.just_pressed(KeyCode::Space) && !movement.jumping {
        movement.jumping = true;
        movement.jump_elapsed = 0.0;
    }

    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
        transform.translation += direction * MOVE_SPEED * time.delta_secs();
    }

    // Parabolic jump arc
    if movement.jumping {
        movement.jump_elapsed += time.delta_secs();
        if movement.jump_elapsed >= JUMP_DURATION {
            // Land: snap to ground, clear jumping so animation system transitions to JumpEnd
            transform.translation.y = GROUND_Y;
            movement.jumping = false;
        } else {
            // t in [0,1], parabola peaks at t=0.5
            let t = movement.jump_elapsed / JUMP_DURATION;
            transform.translation.y = GROUND_Y + JUMP_HEIGHT * 4.0 * t * (1.0 - t);
        }
    }

    // Rotate model to match character facing (base -PI/2 for WoW→Bevy model orientation)
    transform.rotation = Quat::from_rotation_y(facing.yaw - std::f32::consts::FRAC_PI_2);
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
