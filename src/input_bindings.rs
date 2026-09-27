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

impl TryFrom<KeyCode> for BindingKey {
    type Error = KeyCode;
    fn try_from(key: KeyCode) -> Result<Self, Self::Error> {
        match key {
            KeyCode::KeyA => Ok(Self::KeyA),
            KeyCode::KeyB => Ok(Self::KeyB),
            KeyCode::KeyC => Ok(Self::KeyC),
            KeyCode::KeyD => Ok(Self::KeyD),
            KeyCode::KeyE => Ok(Self::KeyE),
            KeyCode::KeyF => Ok(Self::KeyF),
            KeyCode::KeyG => Ok(Self::KeyG),
            KeyCode::KeyH => Ok(Self::KeyH),
            KeyCode::KeyI => Ok(Self::KeyI),
            KeyCode::KeyJ => Ok(Self::KeyJ),
            KeyCode::KeyK => Ok(Self::KeyK),
            KeyCode::KeyL => Ok(Self::KeyL),
            KeyCode::KeyM => Ok(Self::KeyM),
            KeyCode::KeyN => Ok(Self::KeyN),
            KeyCode::KeyO => Ok(Self::KeyO),
            KeyCode::KeyP => Ok(Self::KeyP),
            KeyCode::KeyQ => Ok(Self::KeyQ),
            KeyCode::KeyR => Ok(Self::KeyR),
            KeyCode::KeyS => Ok(Self::KeyS),
            KeyCode::KeyT => Ok(Self::KeyT),
            KeyCode::KeyU => Ok(Self::KeyU),
            KeyCode::KeyV => Ok(Self::KeyV),
            KeyCode::KeyW => Ok(Self::KeyW),
            KeyCode::KeyX => Ok(Self::KeyX),
            KeyCode::KeyY => Ok(Self::KeyY),
            KeyCode::KeyZ => Ok(Self::KeyZ),
            KeyCode::Digit0 => Ok(Self::Digit0),
            KeyCode::Digit1 => Ok(Self::Digit1),
            KeyCode::Digit2 => Ok(Self::Digit2),
            KeyCode::Digit3 => Ok(Self::Digit3),
            KeyCode::Digit4 => Ok(Self::Digit4),
            KeyCode::Digit5 => Ok(Self::Digit5),
            KeyCode::Digit6 => Ok(Self::Digit6),
            KeyCode::Digit7 => Ok(Self::Digit7),
            KeyCode::Digit8 => Ok(Self::Digit8),
            KeyCode::Digit9 => Ok(Self::Digit9),
            KeyCode::F1 => Ok(Self::F1),
            KeyCode::F2 => Ok(Self::F2),
            KeyCode::F3 => Ok(Self::F3),
            KeyCode::F4 => Ok(Self::F4),
            KeyCode::F5 => Ok(Self::F5),
            KeyCode::F6 => Ok(Self::F6),
            KeyCode::F7 => Ok(Self::F7),
            KeyCode::F8 => Ok(Self::F8),
            KeyCode::F9 => Ok(Self::F9),
            KeyCode::F10 => Ok(Self::F10),
            KeyCode::F11 => Ok(Self::F11),
            KeyCode::F12 => Ok(Self::F12),
            KeyCode::Space => Ok(Self::Space),
            KeyCode::Tab => Ok(Self::Tab),
            KeyCode::Escape => Ok(Self::Escape),
            KeyCode::Minus => Ok(Self::Minus),
            KeyCode::Equal => Ok(Self::Equal),
            KeyCode::BracketLeft => Ok(Self::BracketLeft),
            KeyCode::BracketRight => Ok(Self::BracketRight),
            KeyCode::ArrowLeft => Ok(Self::ArrowLeft),
            KeyCode::ArrowRight => Ok(Self::ArrowRight),
            KeyCode::ArrowUp => Ok(Self::ArrowUp),
            KeyCode::ArrowDown => Ok(Self::ArrowDown),
            KeyCode::PageUp => Ok(Self::PageUp),
            KeyCode::PageDown => Ok(Self::PageDown),
            KeyCode::NumLock => Ok(Self::NumLock),
            KeyCode::Home => Ok(Self::Home),
            KeyCode::End => Ok(Self::End),
            KeyCode::Insert => Ok(Self::Insert),
            KeyCode::Delete => Ok(Self::Delete),
            KeyCode::Backspace => Ok(Self::Backspace),
            KeyCode::Enter => Ok(Self::Enter),
            _ => Err(key),
        }
    }
}
impl From<BindingKey> for KeyCode {
    fn from(key: BindingKey) -> Self {
        match key {
            BindingKey::KeyA => Self::KeyA,
            BindingKey::KeyB => Self::KeyB,
            BindingKey::KeyC => Self::KeyC,
            BindingKey::KeyD => Self::KeyD,
            BindingKey::KeyE => Self::KeyE,
            BindingKey::KeyF => Self::KeyF,
            BindingKey::KeyG => Self::KeyG,
            BindingKey::KeyH => Self::KeyH,
            BindingKey::KeyI => Self::KeyI,
            BindingKey::KeyJ => Self::KeyJ,
            BindingKey::KeyK => Self::KeyK,
            BindingKey::KeyL => Self::KeyL,
            BindingKey::KeyM => Self::KeyM,
            BindingKey::KeyN => Self::KeyN,
            BindingKey::KeyO => Self::KeyO,
            BindingKey::KeyP => Self::KeyP,
            BindingKey::KeyQ => Self::KeyQ,
            BindingKey::KeyR => Self::KeyR,
            BindingKey::KeyS => Self::KeyS,
            BindingKey::KeyT => Self::KeyT,
            BindingKey::KeyU => Self::KeyU,
            BindingKey::KeyV => Self::KeyV,
            BindingKey::KeyW => Self::KeyW,
            BindingKey::KeyX => Self::KeyX,
            BindingKey::KeyY => Self::KeyY,
            BindingKey::KeyZ => Self::KeyZ,
            BindingKey::Digit0 => Self::Digit0,
            BindingKey::Digit1 => Self::Digit1,
            BindingKey::Digit2 => Self::Digit2,
            BindingKey::Digit3 => Self::Digit3,
            BindingKey::Digit4 => Self::Digit4,
            BindingKey::Digit5 => Self::Digit5,
            BindingKey::Digit6 => Self::Digit6,
            BindingKey::Digit7 => Self::Digit7,
            BindingKey::Digit8 => Self::Digit8,
            BindingKey::Digit9 => Self::Digit9,
            BindingKey::F1 => Self::F1,
            BindingKey::F2 => Self::F2,
            BindingKey::F3 => Self::F3,
            BindingKey::F4 => Self::F4,
            BindingKey::F5 => Self::F5,
            BindingKey::F6 => Self::F6,
            BindingKey::F7 => Self::F7,
            BindingKey::F8 => Self::F8,
            BindingKey::F9 => Self::F9,
            BindingKey::F10 => Self::F10,
            BindingKey::F11 => Self::F11,
            BindingKey::F12 => Self::F12,
            BindingKey::Space => Self::Space,
            BindingKey::Tab => Self::Tab,
            BindingKey::Escape => Self::Escape,
            BindingKey::Minus => Self::Minus,
            BindingKey::Equal => Self::Equal,
            BindingKey::BracketLeft => Self::BracketLeft,
            BindingKey::BracketRight => Self::BracketRight,
            BindingKey::ArrowLeft => Self::ArrowLeft,
            BindingKey::ArrowRight => Self::ArrowRight,
            BindingKey::ArrowUp => Self::ArrowUp,
            BindingKey::ArrowDown => Self::ArrowDown,
            BindingKey::PageUp => Self::PageUp,
            BindingKey::PageDown => Self::PageDown,
            BindingKey::NumLock => Self::NumLock,
            BindingKey::Home => Self::Home,
            BindingKey::End => Self::End,
            BindingKey::Insert => Self::Insert,
            BindingKey::Delete => Self::Delete,
            BindingKey::Backspace => Self::Backspace,
            BindingKey::Enter => Self::Enter,
        }
    }
}
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
