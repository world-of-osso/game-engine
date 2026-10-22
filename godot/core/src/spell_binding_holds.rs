//! Preserve the physical cast binding and actual sent spell until key/button-up.
use crate::input_bindings_data::{InputBinding, InputState};

#[derive(Default)]
pub struct SpellBindingHolds(Vec<(InputBinding, u32)>);

fn physical_binding(binding: InputBinding) -> InputBinding {
    match binding {
        InputBinding::ShiftKeyboard(key) | InputBinding::CtrlKeyboard(key) => {
            InputBinding::Keyboard(key)
        }
        binding => binding,
    }
}

impl SpellBindingHolds {
    pub fn record(&mut self, binding: InputBinding, spell: u32) {
        let physical = physical_binding(binding);
        self.0.retain(|(held, _)| *held != physical);
        self.0.push((physical, spell));
    }

    /// Physical key-up, independent of current modifiers or changed action assignments.
    pub fn take_released(&mut self, input: &impl InputState) -> Vec<u32> {
        let mut released = Vec::new();
        self.0.retain(|(binding, spell)| {
            let held = binding.pressed(input);
            if !held {
                released.push(*spell);
            }
            held
        });
        released
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_bindings_data::{BindingKey, BindingMouseButton};

    #[derive(Default)]
    struct Input {
        key: bool,
        mouse: bool,
        shift: bool,
    }

    impl InputState for Input {
        fn key_pressed(&self, key: BindingKey) -> bool {
            key == BindingKey::Digit1 && self.key
        }
        fn key_just_pressed(&self, _: BindingKey) -> bool {
            false
        }
        fn mouse_pressed(&self, button: BindingMouseButton) -> bool {
            button == BindingMouseButton::Middle && self.mouse
        }
        fn mouse_just_pressed(&self, _: BindingMouseButton) -> bool {
            false
        }
        fn shift_held(&self) -> bool {
            self.shift
        }
        fn ctrl_held(&self) -> bool {
            false
        }
    }

    #[test]
    fn release_uses_actual_sent_font_spell_once_after_physical_key_up() {
        let mut holds = SpellBindingHolds::default();
        holds.record(InputBinding::ShiftKeyboard(BindingKey::Digit1), 382266);
        assert!(
            holds
                .take_released(&Input {
                    key: true,
                    shift: true,
                    ..Default::default()
                })
                .is_empty()
        );
        // Modifier release alone is not release of the spell's physical cast key.
        assert!(
            holds
                .take_released(&Input {
                    key: true,
                    ..Default::default()
                })
                .is_empty()
        );
        assert_eq!(holds.take_released(&Input::default()), vec![382266]);
        assert!(holds.take_released(&Input::default()).is_empty());
    }

    #[test]
    fn mouse_release_and_world_exit_do_not_replay_previous_casts() {
        let mut holds = SpellBindingHolds::default();
        holds.record(InputBinding::Mouse(BindingMouseButton::Middle), 359073);
        assert!(
            holds
                .take_released(&Input {
                    mouse: true,
                    ..Default::default()
                })
                .is_empty()
        );
        assert_eq!(holds.take_released(&Input::default()), vec![359073]);
        holds.record(InputBinding::Keyboard(BindingKey::Digit1), 357208);
        holds.clear();
        assert!(holds.take_released(&Input::default()).is_empty());
    }
}
