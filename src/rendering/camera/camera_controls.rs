use super::*;
use game_engine::input_bindings::{BindingKey, BindingMouseButton, InputState};

/// Bevy physical input adapter; key/button conversion remains in input_bindings.
pub(super) struct CameraInputState<'a> {
    pub keys: &'a ButtonInput<KeyCode>,
    pub mouse: &'a ButtonInput<MouseButton>,
}

impl InputState for CameraInputState<'_> {
    fn key_pressed(&self, key: BindingKey) -> bool {
        self.keys.pressed(key.into())
    }
    fn key_just_pressed(&self, key: BindingKey) -> bool {
        self.keys.just_pressed(key.into())
    }
    fn mouse_pressed(&self, button: BindingMouseButton) -> bool {
        self.mouse.pressed(button.into())
    }
    fn mouse_just_pressed(&self, button: BindingMouseButton) -> bool {
        self.mouse.just_pressed(button.into())
    }
    fn shift_held(&self) -> bool {
        self.keys.pressed(KeyCode::ShiftLeft) || self.keys.pressed(KeyCode::ShiftRight)
    }
    fn ctrl_held(&self) -> bool {
        self.keys.pressed(KeyCode::ControlLeft) || self.keys.pressed(KeyCode::ControlRight)
    }
}
