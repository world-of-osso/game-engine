//! Bevy-free original player movement input decisions.

use crate::input_bindings_data::{BindingMouseButton, InputAction, InputBindingsData, InputState};
use glam::Vec3;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    #[default]
    None,
    Forward,
    Backward,
    Left,
    Right,
}

/// Return the unnormalized world-space direction and animation direction.
/// Forward binding and both mouse buttons deliberately contribute separately.
pub fn compute_movement_input(
    bindings: &InputBindingsData,
    state: &impl InputState,
    autorun: bool,
    scripted_forward: bool,
    facing_yaw: f32,
) -> ([f32; 3], MoveDirection) {
    let forward = Vec3::new(facing_yaw.sin(), 0.0, facing_yaw.cos());
    let right = Vec3::new(-forward.z, 0.0, forward.x);
    let both_mouse = state.mouse_pressed(BindingMouseButton::Left)
        && state.mouse_pressed(BindingMouseButton::Right);
    let forward_pressed = bindings.is_pressed(InputAction::MoveForward, state);
    let backward_pressed = bindings.is_pressed(InputAction::MoveBackward, state);
    let left_pressed = bindings.is_pressed(InputAction::StrafeLeft, state);
    let right_pressed = bindings.is_pressed(InputAction::StrafeRight, state);

    let mut direction = Vec3::ZERO;
    if forward_pressed || autorun || scripted_forward {
        direction += forward;
    }
    if backward_pressed {
        direction -= forward;
    }
    if left_pressed {
        direction -= right;
    }
    if right_pressed {
        direction += right;
    }
    if both_mouse {
        direction += forward;
    }

    let anim_dir = if forward_pressed || autorun || scripted_forward || both_mouse {
        MoveDirection::Forward
    } else if backward_pressed {
        MoveDirection::Backward
    } else if left_pressed {
        MoveDirection::Left
    } else if right_pressed {
        MoveDirection::Right
    } else {
        MoveDirection::None
    };
    (direction.to_array(), anim_dir)
}

/// Apply the original autorun/run-toggle ordering without changing jump state.
pub fn sync_movement_toggles(
    bindings: &InputBindingsData,
    state: &impl InputState,
    mut autorun: bool,
    mut running: bool,
) -> (bool, bool) {
    if bindings.is_just_pressed(InputAction::AutoRun, state) {
        autorun = !autorun;
    }
    if bindings.is_pressed(InputAction::MoveBackward, state) {
        autorun = false;
    }
    if bindings.is_just_pressed(InputAction::RunToggle, state) {
        running = !running;
    }
    (autorun, running)
}

pub fn has_manual_movement_override(bindings: &InputBindingsData, state: &impl InputState) -> bool {
    bindings.is_pressed(InputAction::MoveForward, state)
        || bindings.is_pressed(InputAction::MoveBackward, state)
        || bindings.is_pressed(InputAction::StrafeLeft, state)
        || bindings.is_pressed(InputAction::StrafeRight, state)
        || bindings.is_just_pressed(InputAction::Jump, state)
        || bindings.is_just_pressed(InputAction::AutoRun, state)
        || (state.mouse_pressed(BindingMouseButton::Left)
            && state.mouse_pressed(BindingMouseButton::Right))
}

/// Encode the animation direction, rather than the diagonal prediction vector, for the wire.
pub fn movement_to_direction(direction: MoveDirection, facing_yaw: f32) -> [f32; 3] {
    let forward = [facing_yaw.sin(), 0.0, facing_yaw.cos()];
    let right = [-forward[2], 0.0, forward[0]];
    let mut result = [0.0; 3];
    let contribution = match direction {
        MoveDirection::Forward => forward,
        MoveDirection::Backward => [-forward[0], 0.0, -forward[2]],
        MoveDirection::Left => [-right[0], 0.0, -right[2]],
        MoveDirection::Right => right,
        MoveDirection::None => [0.0; 3],
    };
    result[0] += contribution[0];
    result[2] += contribution[2];
    result
}

pub fn movement_speed_multiplier(direction: MoveDirection) -> f32 {
    match direction {
        MoveDirection::Backward => 0.6, // shared::movement::BACKPEDAL_MULTIPLIER
        MoveDirection::Left | MoveDirection::Right => 0.8, // shared::movement::STRAFE_MULTIPLIER
        MoveDirection::None | MoveDirection::Forward => 1.0,
    }
}
