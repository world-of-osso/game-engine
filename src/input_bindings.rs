//! Bevy resource and physical-input adapter for portable bindings.
pub use crate::input_bindings_data::{
    BindingKey, BindingMouseButton, BindingSection, InputAction, InputBinding, InputBindingsData,
    InputState, actions_for_section, binding_token, key_display, parse_binding_token,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(transparent)]
pub struct InputBindings(pub InputBindingsData);
impl Deref for InputBindings {
    type Target = InputBindingsData;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for InputBindings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl InputBindings {
    pub fn is_pressed(
        &self,
        action: InputAction,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        self.0.is_pressed(action, &BevyInput { keys, mouse })
    }
    pub fn is_just_pressed(
        &self,
        action: InputAction,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        self.0.is_just_pressed(action, &BevyInput { keys, mouse })
    }
}

struct BevyInput<'a> {
    keys: &'a ButtonInput<KeyCode>,
    mouse: &'a ButtonInput<MouseButton>,
}
impl InputState for BevyInput<'_> {
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

#[path = "input_bindings_bevy_data.rs"]
mod bevy_data;

impl From<MouseButton> for BindingMouseButton {
    fn from(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => Self::Left,
            MouseButton::Right => Self::Right,
            MouseButton::Middle => Self::Middle,
            MouseButton::Back => Self::Back,
            MouseButton::Forward => Self::Forward,
            MouseButton::Other(id) => Self::Other(id),
        }
    }
}
impl From<BindingMouseButton> for MouseButton {
    fn from(button: BindingMouseButton) -> Self {
        match button {
            BindingMouseButton::Left => Self::Left,
            BindingMouseButton::Right => Self::Right,
            BindingMouseButton::Middle => Self::Middle,
            BindingMouseButton::Back => Self::Back,
            BindingMouseButton::Forward => Self::Forward,
            BindingMouseButton::Other(id) => Self::Other(id),
        }
    }
}

/// Modifier-only capture is deliberately ignored; unsupported physical keys are errors.
pub fn captured_keyboard_binding(
    key: KeyCode,
    keys: &ButtonInput<KeyCode>,
) -> Result<Option<InputBinding>, KeyCode> {
    if matches!(
        key,
        KeyCode::ShiftLeft | KeyCode::ShiftRight | KeyCode::ControlLeft | KeyCode::ControlRight
    ) {
        return Ok(None);
    }
    let key = BindingKey::try_from(key)?;
    Ok(Some(
        if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
            InputBinding::CtrlKeyboard(key)
        } else if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            InputBinding::ShiftKeyboard(key)
        } else {
            InputBinding::Keyboard(key)
        },
    ))
}

pub fn parse_key_name(token: &str) -> Option<KeyCode> {
    crate::input_bindings_data::parse_key_name(token).map(Into::into)
}
pub fn key_alpha_numeric_label(key: KeyCode) -> Option<&'static str> {
    BindingKey::try_from(key)
        .ok()
        .and_then(crate::input_bindings_data::key_alpha_numeric_label)
}

#[cfg(test)]
#[path = "../tests/unit/input_bindings_tests.rs"]
mod tests;
