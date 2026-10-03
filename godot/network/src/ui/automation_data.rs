//! Logical UI automation actions and original key/state parsing, shared by both hosts.
pub use bevy::input::keyboard::KeyCode;

use crate::game_state_enum::GameState;

#[derive(Debug, Clone, PartialEq)]
pub enum UiAutomationAction {
    ClickFrame(String),
    /// Right mouse button on a frame; InWorld only.
    RightClickFrame(String),
    /// Left click with Shift held (Retail `SPLITSTACK`); InWorld only.
    ShiftClickFrame(String),
    TypeText(String),
    PressKey(KeyChord),
    WaitForState(GameState, f32),
    WaitForFrame(String, f32),
    /// Pause the script for this many seconds.
    Wait(f32),
    DumpTree,
    DumpUiTree,
}

/// A key press with held modifiers, e.g. `Shift+M`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChord {
    pub modifiers: Vec<KeyCode>,
    pub key: KeyCode,
}

impl From<KeyCode> for KeyChord {
    fn from(key: KeyCode) -> Self {
        Self {
            modifiers: Vec::new(),
            key,
        }
    }
}

impl KeyChord {
    /// The key for screens whose automation handlers do not model modifiers.
    pub fn unmodified_key(&self) -> Result<KeyCode, String> {
        if self.modifiers.is_empty() {
            Ok(self.key)
        } else {
            Err(format!(
                "modifier chord {self:?} is only supported in InWorld"
            ))
        }
    }
}

/// Parse `"M"`, `"F10"`, `"Escape"`, or `"Shift+M"` / `"Ctrl+Alt+1"` into a chord.
pub fn parse_key_chord(value: &str) -> Result<KeyChord, String> {
    let mut tokens: Vec<&str> = value.split('+').map(str::trim).collect();
    let key_token = tokens.pop().unwrap_or_default();
    let key = parse_automation_key(key_token)
        .ok_or_else(|| format!("unsupported automation key '{value}'"))?;
    let modifiers = tokens
        .into_iter()
        .map(|token| {
            parse_modifier(token)
                .ok_or_else(|| format!("unsupported modifier '{token}' in key '{value}'"))
        })
        .collect::<Result<_, _>>()?;
    Ok(KeyChord { modifiers, key })
}

fn parse_automation_key(token: &str) -> Option<KeyCode> {
    if token.eq_ignore_ascii_case("esc") {
        return Some(KeyCode::Escape);
    }
    crate::input_bindings_data::parse_key_name(token).map(Into::into)
}

fn parse_modifier(token: &str) -> Option<KeyCode> {
    match token.to_ascii_lowercase().as_str() {
        "shift" => Some(KeyCode::ShiftLeft),
        "ctrl" | "control" => Some(KeyCode::ControlLeft),
        "alt" => Some(KeyCode::AltLeft),
        _ => None,
    }
}

impl UiAutomationAction {
    pub fn is_input_action(&self) -> bool {
        matches!(
            self,
            Self::ClickFrame(_)
                | Self::RightClickFrame(_)
                | Self::ShiftClickFrame(_)
                | Self::TypeText(_)
                | Self::PressKey(_)
        )
    }
}

pub fn parse_state(value: &str) -> Result<GameState, String> {
    match value {
        "Login" | "login" => Ok(GameState::Login),
        "Connecting" | "connecting" => Ok(GameState::Connecting),
        "CharSelect" | "charselect" => Ok(GameState::CharSelect),
        "Loading" | "loading" => Ok(GameState::Loading),
        "InWorld" | "inworld" => Ok(GameState::InWorld),
        other => Err(format!("unknown game state '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_chords_preserve_modifier_order_duplicates_and_aliases() {
        assert_eq!(
            parse_key_chord(" control + Shift + CTRL + alt + Digit1 ").unwrap(),
            KeyChord {
                modifiers: vec![
                    KeyCode::ControlLeft,
                    KeyCode::ShiftLeft,
                    KeyCode::ControlLeft,
                    KeyCode::AltLeft,
                ],
                key: KeyCode::Digit1,
            }
        );
        assert_eq!(parse_key_chord("ESC").unwrap(), KeyCode::Escape.into());
        assert_eq!(parse_key_chord("KeyM").unwrap(), KeyCode::KeyM.into());
        assert_eq!(parse_key_chord("f12").unwrap(), KeyCode::F12.into());
    }

    #[test]
    fn key_and_state_errors_preserve_original_messages() {
        assert_eq!(
            parse_key_chord("Hyper+M"),
            Err("unsupported modifier 'Hyper' in key 'Hyper+M'".into())
        );
        assert_eq!(
            parse_key_chord("Shift+"),
            Err("unsupported automation key 'Shift+'".into())
        );
        assert_eq!(
            parse_key_chord("keyM"),
            Err("unsupported automation key 'keyM'".into())
        );
        assert_eq!(
            parse_state("LOGIN"),
            Err("unknown game state 'LOGIN'".into())
        );
    }

    #[test]
    fn unmodified_key_rejects_chords_for_non_world_screens() {
        assert_eq!(
            KeyChord::from(KeyCode::Enter).unmodified_key(),
            Ok(KeyCode::Enter)
        );
        let chord = parse_key_chord("Shift+M").unwrap();
        assert_eq!(
            chord.unmodified_key(),
            Err("modifier chord KeyChord { modifiers: [ShiftLeft], key: KeyM } is only supported in InWorld".into())
        );
    }
}
