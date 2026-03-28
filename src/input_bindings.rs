use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(
    Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Copy,
)]
pub enum InputAction {
    MoveForward,
    MoveBackward,
    StrafeLeft,
    StrafeRight,
    Jump,
    RunToggle,
    AutoRun,
    TurnLeft,
    TurnRight,
    PitchUp,
    PitchDown,
    ZoomIn,
    ZoomOut,
    TargetNearest,
    TargetSelf,
    ActionSlot1,
    ActionSlot2,
    ActionSlot3,
    ActionSlot4,
    ActionSlot5,
    ActionSlot6,
    ActionSlot7,
    ActionSlot8,
    ActionSlot9,
    ActionSlot10,
    ActionSlot11,
    ActionSlot12,
    ToggleMute,
}

impl InputAction {
    pub const ALL: [Self; 28] = [
        Self::MoveForward,
        Self::MoveBackward,
        Self::StrafeLeft,
        Self::StrafeRight,
        Self::Jump,
        Self::RunToggle,
        Self::AutoRun,
        Self::TurnLeft,
        Self::TurnRight,
        Self::PitchUp,
        Self::PitchDown,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::TargetNearest,
        Self::TargetSelf,
        Self::ActionSlot1,
        Self::ActionSlot2,
        Self::ActionSlot3,
        Self::ActionSlot4,
        Self::ActionSlot5,
        Self::ActionSlot6,
        Self::ActionSlot7,
        Self::ActionSlot8,
        Self::ActionSlot9,
        Self::ActionSlot10,
        Self::ActionSlot11,
        Self::ActionSlot12,
        Self::ToggleMute,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::MoveForward => "move_forward",
            Self::MoveBackward => "move_backward",
            Self::StrafeLeft => "strafe_left",
            Self::StrafeRight => "strafe_right",
            Self::Jump => "jump",
            Self::RunToggle => "run_toggle",
            Self::AutoRun => "auto_run",
            Self::TurnLeft => "turn_left",
            Self::TurnRight => "turn_right",
            Self::PitchUp => "pitch_up",
            Self::PitchDown => "pitch_down",
            Self::ZoomIn => "zoom_in",
            Self::ZoomOut => "zoom_out",
            Self::TargetNearest => "target_nearest",
            Self::TargetSelf => "target_self",
            Self::ActionSlot1 => "action_slot_1",
            Self::ActionSlot2 => "action_slot_2",
            Self::ActionSlot3 => "action_slot_3",
            Self::ActionSlot4 => "action_slot_4",
            Self::ActionSlot5 => "action_slot_5",
            Self::ActionSlot6 => "action_slot_6",
            Self::ActionSlot7 => "action_slot_7",
            Self::ActionSlot8 => "action_slot_8",
            Self::ActionSlot9 => "action_slot_9",
            Self::ActionSlot10 => "action_slot_10",
            Self::ActionSlot11 => "action_slot_11",
            Self::ActionSlot12 => "action_slot_12",
            Self::ToggleMute => "toggle_mute",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "move_forward" => Self::MoveForward,
            "move_backward" => Self::MoveBackward,
            "strafe_left" => Self::StrafeLeft,
            "strafe_right" => Self::StrafeRight,
            "jump" => Self::Jump,
            "run_toggle" => Self::RunToggle,
            "auto_run" => Self::AutoRun,
            "turn_left" => Self::TurnLeft,
            "turn_right" => Self::TurnRight,
            "pitch_up" => Self::PitchUp,
            "pitch_down" => Self::PitchDown,
            "zoom_in" => Self::ZoomIn,
            "zoom_out" => Self::ZoomOut,
            "target_nearest" => Self::TargetNearest,
            "target_self" => Self::TargetSelf,
            "action_slot_1" => Self::ActionSlot1,
            "action_slot_2" => Self::ActionSlot2,
            "action_slot_3" => Self::ActionSlot3,
            "action_slot_4" => Self::ActionSlot4,
            "action_slot_5" => Self::ActionSlot5,
            "action_slot_6" => Self::ActionSlot6,
            "action_slot_7" => Self::ActionSlot7,
            "action_slot_8" => Self::ActionSlot8,
            "action_slot_9" => Self::ActionSlot9,
            "action_slot_10" => Self::ActionSlot10,
            "action_slot_11" => Self::ActionSlot11,
            "action_slot_12" => Self::ActionSlot12,
            "toggle_mute" => Self::ToggleMute,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MoveForward => "Move Forward",
            Self::MoveBackward => "Move Backward",
            Self::StrafeLeft => "Strafe Left",
            Self::StrafeRight => "Strafe Right",
            Self::Jump => "Jump",
            Self::RunToggle => "Run / Walk Toggle",
            Self::AutoRun => "Auto-Run",
            Self::TurnLeft => "Turn Left",
            Self::TurnRight => "Turn Right",
            Self::PitchUp => "Pitch Up",
            Self::PitchDown => "Pitch Down",
            Self::ZoomIn => "Zoom In",
            Self::ZoomOut => "Zoom Out",
            Self::TargetNearest => "Target Nearest",
            Self::TargetSelf => "Target Self",
            Self::ActionSlot1 => "Action Button 1",
            Self::ActionSlot2 => "Action Button 2",
            Self::ActionSlot3 => "Action Button 3",
            Self::ActionSlot4 => "Action Button 4",
            Self::ActionSlot5 => "Action Button 5",
            Self::ActionSlot6 => "Action Button 6",
            Self::ActionSlot7 => "Action Button 7",
            Self::ActionSlot8 => "Action Button 8",
            Self::ActionSlot9 => "Action Button 9",
            Self::ActionSlot10 => "Action Button 10",
            Self::ActionSlot11 => "Action Button 11",
            Self::ActionSlot12 => "Action Button 12",
            Self::ToggleMute => "Toggle Mute",
        }
    }

    pub fn section(self) -> BindingSection {
        match self {
            Self::MoveForward
            | Self::MoveBackward
            | Self::StrafeLeft
            | Self::StrafeRight
            | Self::Jump
            | Self::RunToggle
            | Self::AutoRun => BindingSection::Movement,
            Self::TurnLeft
            | Self::TurnRight
            | Self::PitchUp
            | Self::PitchDown
            | Self::ZoomIn
            | Self::ZoomOut => BindingSection::Camera,
            Self::TargetNearest | Self::TargetSelf => BindingSection::Targeting,
            Self::ActionSlot1
            | Self::ActionSlot2
            | Self::ActionSlot3
            | Self::ActionSlot4
            | Self::ActionSlot5
            | Self::ActionSlot6
            | Self::ActionSlot7
            | Self::ActionSlot8
            | Self::ActionSlot9
            | Self::ActionSlot10
            | Self::ActionSlot11
            | Self::ActionSlot12 => BindingSection::ActionBar,
            Self::ToggleMute => BindingSection::Audio,
        }
    }

    pub fn default_binding(self) -> Option<InputBinding> {
        match self {
            Self::MoveForward => Some(InputBinding::Keyboard(KeyCode::KeyW)),
            Self::MoveBackward => Some(InputBinding::Keyboard(KeyCode::KeyS)),
            Self::StrafeLeft => Some(InputBinding::Keyboard(KeyCode::KeyA)),
            Self::StrafeRight => Some(InputBinding::Keyboard(KeyCode::KeyD)),
            Self::Jump => Some(InputBinding::Keyboard(KeyCode::Space)),
            Self::RunToggle => Some(InputBinding::Keyboard(KeyCode::KeyZ)),
            Self::AutoRun => None,
            Self::TurnLeft => Some(InputBinding::Keyboard(KeyCode::ArrowLeft)),
            Self::TurnRight => Some(InputBinding::Keyboard(KeyCode::ArrowRight)),
            Self::PitchUp => Some(InputBinding::Keyboard(KeyCode::ArrowUp)),
            Self::PitchDown => Some(InputBinding::Keyboard(KeyCode::ArrowDown)),
            Self::ZoomIn => Some(InputBinding::Keyboard(KeyCode::PageUp)),
            Self::ZoomOut => Some(InputBinding::Keyboard(KeyCode::PageDown)),
            Self::TargetNearest => Some(InputBinding::Keyboard(KeyCode::Tab)),
            Self::TargetSelf => Some(InputBinding::Keyboard(KeyCode::F1)),
            Self::ActionSlot1 => Some(InputBinding::Keyboard(KeyCode::Digit1)),
            Self::ActionSlot2 => Some(InputBinding::Keyboard(KeyCode::Digit2)),
            Self::ActionSlot3 => Some(InputBinding::Keyboard(KeyCode::Digit3)),
            Self::ActionSlot4 => Some(InputBinding::Keyboard(KeyCode::Digit4)),
            Self::ActionSlot5 => Some(InputBinding::Keyboard(KeyCode::Digit5)),
            Self::ActionSlot6 => Some(InputBinding::Keyboard(KeyCode::Digit6)),
            Self::ActionSlot7 => Some(InputBinding::Keyboard(KeyCode::Digit7)),
            Self::ActionSlot8 => Some(InputBinding::Keyboard(KeyCode::Digit8)),
            Self::ActionSlot9 => Some(InputBinding::Keyboard(KeyCode::Digit9)),
            Self::ActionSlot10 => Some(InputBinding::Keyboard(KeyCode::Digit0)),
            Self::ActionSlot11 => Some(InputBinding::Keyboard(KeyCode::Minus)),
            Self::ActionSlot12 => Some(InputBinding::Keyboard(KeyCode::Equal)),
            Self::ToggleMute => Some(InputBinding::Keyboard(KeyCode::KeyM)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Copy)]
pub enum BindingSection {
    Movement,
    Camera,
    Targeting,
    ActionBar,
    Audio,
}

impl BindingSection {
    pub const ALL: [Self; 5] = [
        Self::Movement,
        Self::Camera,
        Self::Targeting,
        Self::ActionBar,
        Self::Audio,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Movement => "movement",
            Self::Camera => "camera",
            Self::Targeting => "targeting",
            Self::ActionBar => "action_bar",
            Self::Audio => "audio",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "movement" => Self::Movement,
            "camera" => Self::Camera,
            "targeting" => Self::Targeting,
            "action_bar" => Self::ActionBar,
            "audio" => Self::Audio,
            _ => return None,
        })
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Movement => "Movement",
            Self::Camera => "Camera",
            Self::Targeting => "Targeting",
            Self::ActionBar => "Action Bar",
            Self::Audio => "Audio",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum InputBinding {
    Keyboard(KeyCode),
    Mouse(MouseButton),
}

impl InputBinding {
    pub fn pressed(
        self,
        keys: &ButtonInput<KeyCode>,
        mouse_buttons: &ButtonInput<MouseButton>,
    ) -> bool {
        match self {
            Self::Keyboard(key) => keys.pressed(key),
            Self::Mouse(button) => mouse_buttons.pressed(button),
        }
    }

    pub fn just_pressed(
        self,
        keys: &ButtonInput<KeyCode>,
        mouse_buttons: &ButtonInput<MouseButton>,
    ) -> bool {
        match self {
            Self::Keyboard(key) => keys.just_pressed(key),
            Self::Mouse(button) => mouse_buttons.just_pressed(button),
        }
    }

    pub fn display(self) -> String {
        match self {
            Self::Keyboard(key) => key_display(key),
            Self::Mouse(button) => mouse_button_display(button),
        }
    }
}

impl Serialize for InputBinding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&binding_token(*self))
    }
}

impl<'de> Deserialize<'de> for InputBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let token = String::deserialize(deserializer)?;
        parse_binding_token(&token).map_err(serde::de::Error::custom)
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputBindings {
    bindings: BTreeMap<InputAction, Option<InputBinding>>,
}

impl Default for InputBindings {
    fn default() -> Self {
        let bindings = InputAction::ALL
            .into_iter()
            .map(|action| (action, action.default_binding()))
            .collect();
        Self { bindings }
    }
}

impl InputBindings {
    pub fn binding(&self, action: InputAction) -> Option<InputBinding> {
        self.bindings.get(&action).copied().flatten()
    }

    pub fn is_pressed(
        &self,
        action: InputAction,
        keys: &ButtonInput<KeyCode>,
        mouse_buttons: &ButtonInput<MouseButton>,
    ) -> bool {
        self.binding(action)
            .is_some_and(|binding| binding.pressed(keys, mouse_buttons))
    }

    pub fn is_just_pressed(
        &self,
        action: InputAction,
        keys: &ButtonInput<KeyCode>,
        mouse_buttons: &ButtonInput<MouseButton>,
    ) -> bool {
        self.binding(action)
            .is_some_and(|binding| binding.just_pressed(keys, mouse_buttons))
    }

    pub fn assign(&mut self, action: InputAction, binding: InputBinding) {
        for existing in InputAction::ALL {
            if existing != action && self.binding(existing) == Some(binding) {
                self.bindings.insert(existing, None);
            }
        }
        self.bindings.insert(action, Some(binding));
    }

    pub fn clear(&mut self, action: InputAction) {
        self.bindings.insert(action, None);
    }

    pub fn reset_section(&mut self, section: BindingSection) {
        for action in actions_for_section(section) {
            self.bindings.insert(*action, action.default_binding());
        }
    }
}

pub fn actions_for_section(section: BindingSection) -> &'static [InputAction] {
    match section {
        BindingSection::Movement => &[
            InputAction::MoveForward,
            InputAction::MoveBackward,
            InputAction::StrafeLeft,
            InputAction::StrafeRight,
            InputAction::Jump,
            InputAction::RunToggle,
            InputAction::AutoRun,
        ],
        BindingSection::Camera => &[
            InputAction::TurnLeft,
            InputAction::TurnRight,
            InputAction::PitchUp,
            InputAction::PitchDown,
            InputAction::ZoomIn,
            InputAction::ZoomOut,
        ],
        BindingSection::Targeting => &[InputAction::TargetNearest, InputAction::TargetSelf],
        BindingSection::ActionBar => &[
            InputAction::ActionSlot1,
            InputAction::ActionSlot2,
            InputAction::ActionSlot3,
            InputAction::ActionSlot4,
            InputAction::ActionSlot5,
            InputAction::ActionSlot6,
            InputAction::ActionSlot7,
            InputAction::ActionSlot8,
            InputAction::ActionSlot9,
            InputAction::ActionSlot10,
            InputAction::ActionSlot11,
            InputAction::ActionSlot12,
        ],
        BindingSection::Audio => &[InputAction::ToggleMute],
    }
}

fn binding_token(binding: InputBinding) -> String {
    match binding {
        InputBinding::Keyboard(key) => format!("key:{key:?}"),
        InputBinding::Mouse(button) => format!("mouse:{button:?}"),
    }
}

fn parse_binding_token(token: &str) -> Result<InputBinding, String> {
    if let Some(key) = token.strip_prefix("key:") {
        return parse_key_code(key)
            .map(InputBinding::Keyboard)
            .ok_or_else(|| format!("unsupported key binding token '{token}'"));
    }
    if let Some(button) = token.strip_prefix("mouse:") {
        return parse_mouse_button(button)
            .map(InputBinding::Mouse)
            .ok_or_else(|| format!("unsupported mouse binding token '{token}'"));
    }
    Err(format!("invalid binding token '{token}'"))
}

fn key_display(key: KeyCode) -> String {
    key_short_label(key)
        .map(str::to_string)
        .unwrap_or_else(|| format!("{key:?}"))
}

fn key_short_label(key: KeyCode) -> Option<&'static str> {
    match key {
        KeyCode::Space => Some("Space"),
        KeyCode::Tab => Some("Tab"),
        KeyCode::Escape => Some("Escape"),
        KeyCode::Minus => Some("-"),
        KeyCode::Equal => Some("="),
        KeyCode::BracketLeft => Some("["),
        KeyCode::BracketRight => Some("]"),
        KeyCode::ArrowLeft => Some("Left Arrow"),
        KeyCode::ArrowRight => Some("Right Arrow"),
        KeyCode::ArrowUp => Some("Up Arrow"),
        KeyCode::ArrowDown => Some("Down Arrow"),
        KeyCode::PageUp => Some("Page Up"),
        KeyCode::PageDown => Some("Page Down"),
        _ => key_alpha_numeric_label(key),
    }
}

fn key_alpha_numeric_label(key: KeyCode) -> Option<&'static str> {
    match key {
        KeyCode::KeyA => Some("A"),
        KeyCode::KeyB => Some("B"),
        KeyCode::KeyC => Some("C"),
        KeyCode::KeyD => Some("D"),
        KeyCode::KeyE => Some("E"),
        KeyCode::KeyF => Some("F"),
        KeyCode::KeyG => Some("G"),
        KeyCode::KeyH => Some("H"),
        KeyCode::KeyI => Some("I"),
        KeyCode::KeyJ => Some("J"),
        KeyCode::KeyK => Some("K"),
        KeyCode::KeyL => Some("L"),
        KeyCode::KeyM => Some("M"),
        KeyCode::KeyN => Some("N"),
        KeyCode::KeyO => Some("O"),
        KeyCode::KeyP => Some("P"),
        KeyCode::KeyQ => Some("Q"),
        KeyCode::KeyR => Some("R"),
        KeyCode::KeyS => Some("S"),
        KeyCode::KeyT => Some("T"),
        KeyCode::KeyU => Some("U"),
        KeyCode::KeyV => Some("V"),
        KeyCode::KeyW => Some("W"),
        KeyCode::KeyX => Some("X"),
        KeyCode::KeyY => Some("Y"),
        KeyCode::KeyZ => Some("Z"),
        KeyCode::Digit0 => Some("0"),
        KeyCode::Digit1 => Some("1"),
        KeyCode::Digit2 => Some("2"),
        KeyCode::Digit3 => Some("3"),
        KeyCode::Digit4 => Some("4"),
        KeyCode::Digit5 => Some("5"),
        KeyCode::Digit6 => Some("6"),
        KeyCode::Digit7 => Some("7"),
        KeyCode::Digit8 => Some("8"),
        KeyCode::Digit9 => Some("9"),
        _ => None,
    }
}

fn mouse_button_display(button: MouseButton) -> String {
    match button {
        MouseButton::Left => "Left Mouse".to_string(),
        MouseButton::Right => "Right Mouse".to_string(),
        MouseButton::Middle => "Middle Mouse".to_string(),
        MouseButton::Back => "Back Mouse".to_string(),
        MouseButton::Forward => "Forward Mouse".to_string(),
        MouseButton::Other(id) => format!("Mouse Button {id}"),
    }
}

fn parse_key_code(token: &str) -> Option<KeyCode> {
    parse_letter_key(token)
        .or_else(|| parse_digit_key(token))
        .or_else(|| parse_function_key(token))
        .or_else(|| parse_named_key(token))
}

fn parse_letter_key(token: &str) -> Option<KeyCode> {
    match token.strip_prefix("Key")? {
        "A" => Some(KeyCode::KeyA),
        "B" => Some(KeyCode::KeyB),
        "C" => Some(KeyCode::KeyC),
        "D" => Some(KeyCode::KeyD),
        "E" => Some(KeyCode::KeyE),
        "F" => Some(KeyCode::KeyF),
        "G" => Some(KeyCode::KeyG),
        "H" => Some(KeyCode::KeyH),
        "I" => Some(KeyCode::KeyI),
        "J" => Some(KeyCode::KeyJ),
        "K" => Some(KeyCode::KeyK),
        "L" => Some(KeyCode::KeyL),
        "M" => Some(KeyCode::KeyM),
        "N" => Some(KeyCode::KeyN),
        "O" => Some(KeyCode::KeyO),
        "P" => Some(KeyCode::KeyP),
        "Q" => Some(KeyCode::KeyQ),
        "R" => Some(KeyCode::KeyR),
        "S" => Some(KeyCode::KeyS),
        "T" => Some(KeyCode::KeyT),
        "U" => Some(KeyCode::KeyU),
        "V" => Some(KeyCode::KeyV),
        "W" => Some(KeyCode::KeyW),
        "X" => Some(KeyCode::KeyX),
        "Y" => Some(KeyCode::KeyY),
        "Z" => Some(KeyCode::KeyZ),
        _ => None,
    }
}

fn parse_digit_key(token: &str) -> Option<KeyCode> {
    match token.strip_prefix("Digit")? {
        "0" => Some(KeyCode::Digit0),
        "1" => Some(KeyCode::Digit1),
        "2" => Some(KeyCode::Digit2),
        "3" => Some(KeyCode::Digit3),
        "4" => Some(KeyCode::Digit4),
        "5" => Some(KeyCode::Digit5),
        "6" => Some(KeyCode::Digit6),
        "7" => Some(KeyCode::Digit7),
        "8" => Some(KeyCode::Digit8),
        "9" => Some(KeyCode::Digit9),
        _ => None,
    }
}

fn parse_function_key(token: &str) -> Option<KeyCode> {
    let number = token.strip_prefix('F')?.parse::<u8>().ok()?;
    match number {
        1 => Some(KeyCode::F1),
        2 => Some(KeyCode::F2),
        3 => Some(KeyCode::F3),
        4 => Some(KeyCode::F4),
        5 => Some(KeyCode::F5),
        6 => Some(KeyCode::F6),
        7 => Some(KeyCode::F7),
        8 => Some(KeyCode::F8),
        9 => Some(KeyCode::F9),
        10 => Some(KeyCode::F10),
        11 => Some(KeyCode::F11),
        12 => Some(KeyCode::F12),
        _ => None,
    }
}

fn parse_named_key(token: &str) -> Option<KeyCode> {
    match token {
        "Space" => Some(KeyCode::Space),
        "Tab" => Some(KeyCode::Tab),
        "Escape" => Some(KeyCode::Escape),
        "Minus" => Some(KeyCode::Minus),
        "Equal" => Some(KeyCode::Equal),
        "BracketLeft" => Some(KeyCode::BracketLeft),
        "BracketRight" => Some(KeyCode::BracketRight),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "PageUp" => Some(KeyCode::PageUp),
        "PageDown" => Some(KeyCode::PageDown),
        "Home" => Some(KeyCode::Home),
        "End" => Some(KeyCode::End),
        "Insert" => Some(KeyCode::Insert),
        "Delete" => Some(KeyCode::Delete),
        "Backspace" => Some(KeyCode::Backspace),
        "Enter" => Some(KeyCode::Enter),
        _ => None,
    }
}

fn parse_mouse_button(token: &str) -> Option<MouseButton> {
    match token {
        "Left" => Some(MouseButton::Left),
        "Right" => Some(MouseButton::Right),
        "Middle" => Some(MouseButton::Middle),
        "Back" => Some(MouseButton::Back),
        "Forward" => Some(MouseButton::Forward),
        _ => token
            .strip_prefix("Other(")?
            .strip_suffix(')')?
            .parse()
            .ok()
            .map(MouseButton::Other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_swaps_existing_owner() {
        let mut bindings = InputBindings::default();
        bindings.assign(InputAction::Jump, InputBinding::Keyboard(KeyCode::KeyW));

        assert_eq!(
            bindings.binding(InputAction::Jump),
            Some(InputBinding::Keyboard(KeyCode::KeyW))
        );
        assert_eq!(bindings.binding(InputAction::MoveForward), None);
    }

    #[test]
    fn reset_section_only_resets_selected_section() {
        let mut bindings = InputBindings::default();
        bindings.clear(InputAction::MoveForward);
        bindings.clear(InputAction::ToggleMute);

        bindings.reset_section(BindingSection::Movement);

        assert_eq!(
            bindings.binding(InputAction::MoveForward),
            Some(InputBinding::Keyboard(KeyCode::KeyW))
        );
        assert_eq!(bindings.binding(InputAction::ToggleMute), None);
    }
}
