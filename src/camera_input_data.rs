//! Bevy-free in-world camera input calculation.

use crate::camera_control_data::{CameraState, PITCH_LIMIT_DEGREES};
use crate::input_bindings_data::{BindingMouseButton, InputAction, InputBindingsData, InputState};

const KEY_ROTATE_SPEED: f32 = 2.5;
const KEY_ZOOM_SPEED: f32 = 15.0;
const ZOOM_STEP: f32 = 2.0;
const PITCH_LIMIT: f32 = PITCH_LIMIT_DEGREES * std::f32::consts::PI / 180.0;

#[derive(Debug, Clone, Copy)]
pub struct CameraInput {
    pub delta_x: f32,
    pub delta_y: f32,
    pub scroll_y: f32,
    pub dt: f32,
    pub look_sensitivity: f32,
    pub invert_y: bool,
}

/// Apply mouse motion, then bound keyboard actions, then wheel zoom.
/// `facing` is absent when there is not exactly one player character.
pub fn apply_camera_input(
    camera: &mut CameraState,
    facing: Option<f32>,
    bindings: &InputBindingsData,
    state: &impl InputState,
    input: CameraInput,
) -> Option<f32> {
    let mut facing = facing;
    let right = state.mouse_pressed(BindingMouseButton::Right);
    let left = state.mouse_pressed(BindingMouseButton::Left);
    if right || left {
        camera.yaw -= input.delta_x * input.look_sensitivity;
        let sign = if input.invert_y { 1.0 } else { -1.0 };
        camera.pitch = (camera.pitch + input.delta_y * input.look_sensitivity * sign)
            .clamp(-PITCH_LIMIT, PITCH_LIMIT);
        if right {
            facing = facing.map(|_| camera.yaw + std::f32::consts::PI);
        }
    }

    if let Some(sign) = pressed_axis_sign(
        bindings,
        state,
        InputAction::TurnLeft,
        InputAction::TurnRight,
    ) {
        let delta = sign * KEY_ROTATE_SPEED * input.dt;
        camera.yaw += delta;
        facing = facing.map(|yaw| yaw + delta);
    }
    if let Some(sign) = pressed_axis_sign(
        bindings,
        state,
        InputAction::PitchUp,
        InputAction::PitchDown,
    ) {
        camera.pitch =
            (camera.pitch + sign * KEY_ROTATE_SPEED * input.dt).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }
    if let Some(sign) =
        pressed_axis_sign(bindings, state, InputAction::ZoomOut, InputAction::ZoomIn)
    {
        camera.target_distance = (camera.target_distance + sign * KEY_ZOOM_SPEED * input.dt)
            .clamp(camera.min_distance, camera.max_distance);
    }
    if input.scroll_y != 0.0 {
        camera.target_distance = (camera.target_distance - input.scroll_y * ZOOM_STEP)
            .clamp(camera.min_distance, camera.max_distance);
    }
    facing
}

fn pressed_axis_sign(
    bindings: &InputBindingsData,
    state: &impl InputState,
    positive: InputAction,
    negative: InputAction,
) -> Option<f32> {
    if bindings.is_pressed(positive, state) {
        Some(1.0)
    } else if bindings.is_pressed(negative, state) {
        Some(-1.0)
    } else {
        None
    }
}
