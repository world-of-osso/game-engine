//! Retained physical input for native gameplay bindings.

use std::collections::HashSet;

use game_engine_core::input_bindings_data::{BindingKey, BindingMouseButton, InputState};

/// Where in Godot's input pipeline an event was observed.
#[derive(Clone, Copy)]
enum InputStage {
    /// `_input`, before any control sees the event.
    BeforeGui,
    /// `_unhandled_input`, after no control consumed it.
    Unconsumed,
}

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
    /// Viewport position of the latest mouse event.
    pointer: [f32; 2],
}

impl PhysicalInput {
    /// Cursor presentation observes mouse positions even when gameplay input is modal.
    pub fn capture_pointer(&mut self, event: &godot::obj::Gd<godot::classes::InputEvent>) {
        use godot::classes::{InputEventMouseButton, InputEventMouseMotion};
        if let Ok(mouse) = event.clone().try_cast::<InputEventMouseButton>() {
            self.pointer = mouse.get_position().to_array();
        } else if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            self.pointer = motion.get_position().to_array();
        }
    }

    pub fn capture(&mut self, event: &godot::obj::Gd<godot::classes::InputEvent>) {
        self.capture_pointer(event);
        use godot::classes::{
            InputEventKey, InputEventMouseButton, InputEventMouseMotion, InputEventWithModifiers,
        };
        if let Ok(modifiers) = event.clone().try_cast::<InputEventWithModifiers>() {
            self.set_modifiers(modifiers.is_shift_pressed(), modifiers.is_ctrl_pressed());
        }
        if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
            if let Some(binding) = crate::input_keys::binding_key(key.get_physical_keycode()) {
                self.set_key(binding, key.is_pressed());
            }
        } else if let Ok(mouse) = event.clone().try_cast::<InputEventMouseButton>() {
            self.capture_mouse(&mouse);
        } else if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            let relative = motion.get_relative();
            self.add_motion(relative.x, relative.y);
        }
    }

    /// After GUI handling: a button press no control consumed reaches gameplay.
    pub fn capture_unconsumed_press(&mut self, event: &godot::obj::Gd<godot::classes::InputEvent>) {
        let Ok(mouse) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseButton>()
        else {
            return;
        };
        if let Some(binding) = crate::input_keys::binding_mouse_button(mouse.get_button_index()) {
            self.mouse_button(binding, mouse.is_pressed(), InputStage::Unconsumed);
        }
    }

    fn capture_mouse(&mut self, mouse: &godot::obj::Gd<godot::classes::InputEventMouseButton>) {
        use godot::global::MouseButton;
        let button = mouse.get_button_index();
        if let Some(binding) = crate::input_keys::binding_mouse_button(button) {
            self.mouse_button(binding, mouse.is_pressed(), InputStage::BeforeGui);
        } else if mouse.is_pressed() {
            match button {
                MouseButton::WHEEL_UP => self.add_scroll(mouse.get_factor()),
                MouseButton::WHEEL_DOWN => self.add_scroll(-mouse.get_factor()),
                _ => {}
            }
        }
    }

    /// A press waits for GUI handling, so one a frame consumed never holds a gameplay
    /// button (no camera drag from UI); a release always arrives before the GUI.
    fn mouse_button(&mut self, button: BindingMouseButton, pressed: bool, stage: InputStage) {
        match (pressed, stage) {
            (true, InputStage::Unconsumed) | (false, InputStage::BeforeGui) => {
                self.set_mouse(button, pressed)
            }
            (true, InputStage::BeforeGui) | (false, InputStage::Unconsumed) => {}
        }
    }

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

    pub fn pointer(&self) -> [f32; 2] {
        self.pointer
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

    /// A keyboard-owning picker stops movement without moving the item cursor.
    pub fn clear_gameplay(&mut self) {
        let pointer = self.pointer;
        self.clear();
        self.pointer = pointer;
    }

    pub fn gameplay_state(&self, keyboard_enabled: bool) -> GameplayInputState<'_> {
        GameplayInputState {
            physical: self,
            keyboard_enabled,
        }
    }
}

pub(crate) struct GameplayInputState<'a> {
    physical: &'a PhysicalInput,
    keyboard_enabled: bool,
}

impl InputState for GameplayInputState<'_> {
    fn key_pressed(&self, key: BindingKey) -> bool {
        self.keyboard_enabled && self.physical.key_pressed(key)
    }
    fn key_just_pressed(&self, key: BindingKey) -> bool {
        self.keyboard_enabled && self.physical.key_just_pressed(key)
    }
    fn mouse_pressed(&self, button: BindingMouseButton) -> bool {
        self.physical.mouse_pressed(button)
    }
    fn mouse_just_pressed(&self, button: BindingMouseButton) -> bool {
        self.physical.mouse_just_pressed(button)
    }
    fn shift_held(&self) -> bool {
        self.keyboard_enabled && self.physical.shift_held()
    }
    fn ctrl_held(&self) -> bool {
        self.keyboard_enabled && self.physical.ctrl_held()
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
    use super::{InputStage, PhysicalInput};
    use game_engine_core::input_bindings_data::{BindingKey, BindingMouseButton, InputState};

    #[test]
    fn keyboard_suppression_preserves_mouse_look_and_physical_held_keys() {
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        input.set_mouse(BindingMouseButton::Right, true);
        input.set_modifiers(true, true);
        let suppressed = input.gameplay_state(false);
        assert!(!suppressed.key_pressed(BindingKey::KeyW));
        assert!(!suppressed.key_just_pressed(BindingKey::KeyW));
        assert!(!suppressed.shift_held());
        assert!(!suppressed.ctrl_held());
        assert!(suppressed.mouse_pressed(BindingMouseButton::Right));
        assert!(suppressed.mouse_just_pressed(BindingMouseButton::Right));
        let enabled = input.gameplay_state(true);
        assert!(enabled.key_pressed(BindingKey::KeyW));
        assert!(enabled.key_just_pressed(BindingKey::KeyW));
        assert!(enabled.shift_held());
        assert!(enabled.ctrl_held());
    }

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
    fn press_consumed_by_ui_never_holds_a_gameplay_button_during_its_drag() {
        let mut input = PhysicalInput::default();
        // Press on a bag slot: seen before the GUI, then consumed by the control.
        input.mouse_button(BindingMouseButton::Left, true, InputStage::BeforeGui);
        input.add_motion(40.0, -12.0);
        assert!(!input.mouse_pressed(BindingMouseButton::Left));
        assert!(!input.mouse_just_pressed(BindingMouseButton::Left));
        input.mouse_button(BindingMouseButton::Left, false, InputStage::BeforeGui);
        assert!(!input.mouse_pressed(BindingMouseButton::Left));
    }

    #[test]
    fn unconsumed_world_press_holds_until_its_release_even_over_ui() {
        let mut input = PhysicalInput::default();
        input.mouse_button(BindingMouseButton::Right, true, InputStage::BeforeGui);
        input.mouse_button(BindingMouseButton::Right, true, InputStage::Unconsumed);
        assert!(input.mouse_just_pressed(BindingMouseButton::Right));
        input.finish_frame();
        // The release arrives before the GUI, even when it lands on a frame.
        input.mouse_button(BindingMouseButton::Right, false, InputStage::BeforeGui);
        assert!(!input.mouse_pressed(BindingMouseButton::Right));
        // A release seen again after the GUI does not re-press.
        input.mouse_button(BindingMouseButton::Right, false, InputStage::Unconsumed);
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
