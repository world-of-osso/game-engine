use super::*;

pub(super) fn apply_keyboard_camera(
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    bindings: &InputBindings,
    dt: f32,
    cam: &mut WowCamera,
    facing_q: &mut Query<&mut CharacterFacing, With<Player>>,
) {
    apply_keyboard_yaw(keys, mouse_buttons, bindings, dt, cam, facing_q);
    apply_keyboard_pitch(keys, mouse_buttons, bindings, dt, cam);
    apply_keyboard_zoom(keys, mouse_buttons, bindings, dt, cam);
}

fn apply_keyboard_yaw(
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    bindings: &InputBindings,
    dt: f32,
    cam: &mut WowCamera,
    facing_q: &mut Query<&mut CharacterFacing, With<Player>>,
) {
    let Some(sign) = pressed_axis_sign(
        bindings,
        keys,
        mouse_buttons,
        InputAction::TurnLeft,
        InputAction::TurnRight,
    ) else {
        return;
    };
    let yaw_delta = sign * KEY_ROTATE_SPEED * dt;
    cam.yaw += yaw_delta;
    if let Ok(mut facing) = facing_q.single_mut() {
        facing.yaw += yaw_delta;
    }
}

fn apply_keyboard_pitch(
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    bindings: &InputBindings,
    dt: f32,
    cam: &mut WowCamera,
) {
    let Some(sign) = pressed_axis_sign(
        bindings,
        keys,
        mouse_buttons,
        InputAction::PitchUp,
        InputAction::PitchDown,
    ) else {
        return;
    };
    cam.pitch = (cam.pitch + sign * KEY_ROTATE_SPEED * dt).clamp(-PITCH_LIMIT, PITCH_LIMIT);
}

fn apply_keyboard_zoom(
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    bindings: &InputBindings,
    dt: f32,
    cam: &mut WowCamera,
) {
    let Some(sign) = pressed_axis_sign(
        bindings,
        keys,
        mouse_buttons,
        InputAction::ZoomOut,
        InputAction::ZoomIn,
    ) else {
        return;
    };
    cam.target_distance = (cam.target_distance + sign * KEY_ZOOM_SPEED * dt)
        .clamp(cam.min_distance, cam.max_distance);
}

fn pressed_axis_sign(
    bindings: &InputBindings,
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    positive: InputAction,
    negative: InputAction,
) -> Option<f32> {
    if bindings.is_pressed(positive, keys, mouse_buttons) {
        Some(1.0)
    } else if bindings.is_pressed(negative, keys, mouse_buttons) {
        Some(-1.0)
    } else {
        None
    }
}

pub(super) fn camera_pitch_delta(
    mouse_delta_y: f32,
    options: &crate::client_options::CameraOptions,
) -> f32 {
    let sign = if options.invert_y { 1.0 } else { -1.0 };
    mouse_delta_y * options.look_sensitivity * sign
}
