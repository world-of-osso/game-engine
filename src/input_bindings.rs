use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const LETTER_KEYS: [(&str, KeyCode); 26] = [
    ("A", KeyCode::KeyA),
    ("B", KeyCode::KeyB),
    ("C", KeyCode::KeyC),
    ("D", KeyCode::KeyD),
    ("E", KeyCode::KeyE),
    ("F", KeyCode::KeyF),
    ("G", KeyCode::KeyG),
    ("H", KeyCode::KeyH),
    ("I", KeyCode::KeyI),
    ("J", KeyCode::KeyJ),
    ("K", KeyCode::KeyK),
    ("L", KeyCode::KeyL),
    ("M", KeyCode::KeyM),
    ("N", KeyCode::KeyN),
    ("O", KeyCode::KeyO),
    ("P", KeyCode::KeyP),
    ("Q", KeyCode::KeyQ),
    ("R", KeyCode::KeyR),
    ("S", KeyCode::KeyS),
    ("T", KeyCode::KeyT),
    ("U", KeyCode::KeyU),
    ("V", KeyCode::KeyV),
    ("W", KeyCode::KeyW),
    ("X", KeyCode::KeyX),
    ("Y", KeyCode::KeyY),
    ("Z", KeyCode::KeyZ),
];

const DIGIT_KEYS: [(&str, KeyCode); 10] = [
    ("0", KeyCode::Digit0),
    ("1", KeyCode::Digit1),
    ("2", KeyCode::Digit2),
    ("3", KeyCode::Digit3),
    ("4", KeyCode::Digit4),
    ("5", KeyCode::Digit5),
    ("6", KeyCode::Digit6),
    ("7", KeyCode::Digit7),
    ("8", KeyCode::Digit8),
    ("9", KeyCode::Digit9),
];

const FUNCTION_KEYS: [(&str, KeyCode); 12] = [
    ("F1", KeyCode::F1),
    ("F2", KeyCode::F2),
    ("F3", KeyCode::F3),
    ("F4", KeyCode::F4),
    ("F5", KeyCode::F5),
    ("F6", KeyCode::F6),
    ("F7", KeyCode::F7),
    ("F8", KeyCode::F8),
    ("F9", KeyCode::F9),
    ("F10", KeyCode::F10),
    ("F11", KeyCode::F11),
    ("F12", KeyCode::F12),
];

struct InputActionMeta {
    key: &'static str,
    label: &'static str,
    section: BindingSection,
    default_binding: Option<InputBinding>,
}

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
        self.meta().key
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
        self.meta().label
    }

    pub fn section(self) -> BindingSection {
        self.meta().section
    }

    pub fn default_binding(self) -> Option<InputBinding> {
        self.meta().default_binding
    }

    fn meta(self) -> InputActionMeta {
        match self {
            Self::MoveForward => movement_meta("move_forward", "Move Forward", KeyCode::KeyW),
            Self::MoveBackward => movement_meta("move_backward", "Move Backward", KeyCode::KeyS),
            Self::StrafeLeft => movement_meta("strafe_left", "Strafe Left", KeyCode::KeyA),
            Self::StrafeRight => movement_meta("strafe_right", "Strafe Right", KeyCode::KeyD),
            Self::Jump => movement_meta("jump", "Jump", KeyCode::Space),
            Self::RunToggle => movement_meta("run_toggle", "Run / Walk Toggle", KeyCode::KeyZ),
            Self::AutoRun => movement_meta_without_default("auto_run", "Auto-Run"),
            Self::TurnLeft => camera_meta("turn_left", "Turn Left", KeyCode::ArrowLeft),
            Self::TurnRight => camera_meta("turn_right", "Turn Right", KeyCode::ArrowRight),
            Self::PitchUp => camera_meta("pitch_up", "Pitch Up", KeyCode::ArrowUp),
            Self::PitchDown => camera_meta("pitch_down", "Pitch Down", KeyCode::ArrowDown),
            Self::ZoomIn => camera_meta("zoom_in", "Zoom In", KeyCode::PageUp),
            Self::ZoomOut => camera_meta("zoom_out", "Zoom Out", KeyCode::PageDown),
            Self::TargetNearest => targeting_meta("target_nearest", "Target Nearest", KeyCode::Tab),
            Self::TargetSelf => targeting_meta("target_self", "Target Self", KeyCode::F1),
            Self::ActionSlot1 => action_slot_meta(1, Self::ActionSlot1, KeyCode::Digit1),
            Self::ActionSlot2 => action_slot_meta(2, Self::ActionSlot2, KeyCode::Digit2),
            Self::ActionSlot3 => action_slot_meta(3, Self::ActionSlot3, KeyCode::Digit3),
            Self::ActionSlot4 => action_slot_meta(4, Self::ActionSlot4, KeyCode::Digit4),
            Self::ActionSlot5 => action_slot_meta(5, Self::ActionSlot5, KeyCode::Digit5),
            Self::ActionSlot6 => action_slot_meta(6, Self::ActionSlot6, KeyCode::Digit6),
            Self::ActionSlot7 => action_slot_meta(7, Self::ActionSlot7, KeyCode::Digit7),
            Self::ActionSlot8 => action_slot_meta(8, Self::ActionSlot8, KeyCode::Digit8),
            Self::ActionSlot9 => action_slot_meta(9, Self::ActionSlot9, KeyCode::Digit9),
            Self::ActionSlot10 => action_slot_meta(10, Self::ActionSlot10, KeyCode::Digit0),
            Self::ActionSlot11 => action_slot_meta(11, Self::ActionSlot11, KeyCode::Minus),
            Self::ActionSlot12 => action_slot_meta(12, Self::ActionSlot12, KeyCode::Equal),
            Self::ToggleMute => audio_meta("toggle_mute", "Toggle Mute", KeyCode::KeyM),
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

fn action_slot_meta(slot: u8, _action: InputAction, default_key: KeyCode) -> InputActionMeta {
    let (key, label) = match slot {
        1 => ("action_slot_1", "Action Button 1"),
        2 => ("action_slot_2", "Action Button 2"),
        3 => ("action_slot_3", "Action Button 3"),
        4 => ("action_slot_4", "Action Button 4"),
        5 => ("action_slot_5", "Action Button 5"),
        6 => ("action_slot_6", "Action Button 6"),
        7 => ("action_slot_7", "Action Button 7"),
        8 => ("action_slot_8", "Action Button 8"),
        9 => ("action_slot_9", "Action Button 9"),
        10 => ("action_slot_10", "Action Button 10"),
        11 => ("action_slot_11", "Action Button 11"),
        12 => ("action_slot_12", "Action Button 12"),
        _ => unreachable!("unsupported action slot"),
    };
    InputActionMeta {
        key,
        label,
        section: BindingSection::ActionBar,
        default_binding: Some(InputBinding::Keyboard(default_key)),
    }
}

fn movement_meta(key: &'static str, label: &'static str, default_key: KeyCode) -> InputActionMeta {
    input_action_meta(
        key,
        label,
        BindingSection::Movement,
        Some(InputBinding::Keyboard(default_key)),
    )
}

fn movement_meta_without_default(key: &'static str, label: &'static str) -> InputActionMeta {
    input_action_meta(key, label, BindingSection::Movement, None)
}

fn camera_meta(key: &'static str, label: &'static str, default_key: KeyCode) -> InputActionMeta {
    input_action_meta(
        key,
        label,
        BindingSection::Camera,
        Some(InputBinding::Keyboard(default_key)),
    )
}

fn targeting_meta(key: &'static str, label: &'static str, default_key: KeyCode) -> InputActionMeta {
    input_action_meta(
        key,
        label,
        BindingSection::Targeting,
        Some(InputBinding::Keyboard(default_key)),
    )
}

fn audio_meta(key: &'static str, label: &'static str, default_key: KeyCode) -> InputActionMeta {
    input_action_meta(
        key,
        label,
        BindingSection::Audio,
        Some(InputBinding::Keyboard(default_key)),
    )
}

fn input_action_meta(
    key: &'static str,
    label: &'static str,
    section: BindingSection,
    default_binding: Option<InputBinding>,
) -> InputActionMeta {
    InputActionMeta {
        key,
        label,
        section,
        default_binding,
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
    LETTER_KEYS
        .iter()
        .chain(DIGIT_KEYS.iter())
        .find_map(|(label, code)| (*code == key).then_some(*label))
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
    let token = token.strip_prefix("Key")?;
    lookup_named_key(token, &LETTER_KEYS)
}

fn parse_digit_key(token: &str) -> Option<KeyCode> {
    let token = token.strip_prefix("Digit")?;
    lookup_named_key(token, &DIGIT_KEYS)
}

fn parse_function_key(token: &str) -> Option<KeyCode> {
    lookup_named_key(token, &FUNCTION_KEYS)
}

fn lookup_named_key(token: &str, entries: &[(&str, KeyCode)]) -> Option<KeyCode> {
    entries
        .iter()
        .find_map(|(name, code)| (*name == token).then_some(*code))
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
