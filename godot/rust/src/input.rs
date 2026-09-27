//! Retained physical input for native gameplay bindings.

use std::collections::HashSet;

use game_engine_core::input_bindings_data::{BindingKey, BindingMouseButton, InputState};

#[derive(Default)]
pub(crate) struct PhysicalInput {
    keys: HashSet<BindingKey>,
    pressed_keys: HashSet<BindingKey>,
    buttons: HashSet<BindingMouseButton>,
    pressed_buttons: HashSet<BindingMouseButton>,
    shift: bool,
    ctrl: bool,
    motion: [f32; 2],
    scroll: f32,
}

impl PhysicalInput {
    pub fn set_key(&mut self, key: BindingKey, pressed: bool) {
        if pressed {
            if self.keys.insert(key) {
                self.pressed_keys.insert(key);
            }
        } else {
            self.keys.remove(&key);
        }
    }

    pub fn set_mouse(&mut self, button: BindingMouseButton, pressed: bool) {
        if pressed {
            if self.buttons.insert(button) {
                self.pressed_buttons.insert(button);
            }
        } else {
            self.buttons.remove(&button);
        }
    }

    pub fn set_modifiers(&mut self, shift: bool, ctrl: bool) {
        self.shift = shift;
        self.ctrl = ctrl;
    }

    pub fn add_motion(&mut self, x: f32, y: f32) {
        self.motion[0] += x;
        self.motion[1] += y;
    }

    pub fn add_scroll(&mut self, delta: f32) {
        self.scroll += delta;
    }

    pub fn motion(&self) -> [f32; 2] {
        self.motion
    }

    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    pub fn finish_frame(&mut self) {
        self.pressed_keys.clear();
        self.pressed_buttons.clear();
        self.motion = [0.0; 2];
        self.scroll = 0.0;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl InputState for PhysicalInput {
    fn key_pressed(&self, key: BindingKey) -> bool {
        self.keys.contains(&key)
    }

    fn key_just_pressed(&self, key: BindingKey) -> bool {
        self.pressed_keys.contains(&key)
    }

    fn mouse_pressed(&self, button: BindingMouseButton) -> bool {
        self.buttons.contains(&button)
    }

    fn mouse_just_pressed(&self, button: BindingMouseButton) -> bool {
        self.pressed_buttons.contains(&button)
    }

    fn shift_held(&self) -> bool {
        self.shift
    }

    fn ctrl_held(&self) -> bool {
        self.ctrl
    }
}

#[cfg(test)]
mod tests {
    use super::PhysicalInput;
    use game_engine_core::input_bindings_data::{BindingKey, BindingMouseButton, InputState};

    #[test]
    fn key_edges_last_one_frame_but_held_keys_survive() {
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        assert!(input.key_pressed(BindingKey::KeyW));
        assert!(input.key_just_pressed(BindingKey::KeyW));
        input.finish_frame();
        assert!(input.key_pressed(BindingKey::KeyW));
        assert!(!input.key_just_pressed(BindingKey::KeyW));
        input.set_key(BindingKey::KeyW, true);
        assert!(!input.key_just_pressed(BindingKey::KeyW));
        input.set_key(BindingKey::KeyW, false);
        assert!(!input.key_pressed(BindingKey::KeyW));
        input.set_key(BindingKey::KeyW, true);
        assert!(input.key_just_pressed(BindingKey::KeyW));
    }

    #[test]
    fn mouse_edges_and_motion_are_consumed_without_releasing_buttons() {
        let mut input = PhysicalInput::default();
        input.set_mouse(BindingMouseButton::Right, true);
        input.add_motion(3.0, -2.0);
        input.add_motion(-1.0, 5.0);
        input.add_scroll(2.0);
        input.add_scroll(-0.5);
        assert!(input.mouse_just_pressed(BindingMouseButton::Right));
        assert_eq!(input.motion(), [2.0, 3.0]);
        assert_eq!(input.scroll(), 1.5);
        input.finish_frame();
        assert!(input.mouse_pressed(BindingMouseButton::Right));
        assert!(!input.mouse_just_pressed(BindingMouseButton::Right));
        assert_eq!(input.motion(), [0.0, 0.0]);
        assert_eq!(input.scroll(), 0.0);
        input.set_mouse(BindingMouseButton::Right, false);
        assert!(!input.mouse_pressed(BindingMouseButton::Right));
    }

    #[test]
    fn focus_loss_clears_held_input_modifiers_and_pending_motion() {
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::Space, true);
        input.set_mouse(BindingMouseButton::Left, true);
        input.set_modifiers(true, true);
        input.add_motion(8.0, 9.0);
        input.add_scroll(1.0);
        assert!(input.shift_held());
        assert!(input.ctrl_held());
        input.clear();
        assert!(!input.key_pressed(BindingKey::Space));
        assert!(!input.key_just_pressed(BindingKey::Space));
        assert!(!input.mouse_pressed(BindingMouseButton::Left));
        assert!(!input.mouse_just_pressed(BindingMouseButton::Left));
        assert!(!input.shift_held());
        assert!(!input.ctrl_held());
        assert_eq!(input.motion(), [0.0, 0.0]);
        assert_eq!(input.scroll(), 0.0);
    }
}
