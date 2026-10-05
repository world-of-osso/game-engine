//! Portable action inventory, persisted binding grammar and matching semantics.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BindingKey {
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Space,
    Tab,
    Escape,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    PageUp,
    PageDown,
    NumLock,
    Home,
    End,
    Insert,
    Delete,
    Backspace,
    Enter,
    NumpadAdd,
    NumpadSubtract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingMouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// Physical input state. Modifiers are supplied separately because they are not bindable keys.
pub trait InputState {
    fn key_pressed(&self, key: BindingKey) -> bool;
    fn key_just_pressed(&self, key: BindingKey) -> bool;
    fn mouse_pressed(&self, button: BindingMouseButton) -> bool;
    fn mouse_just_pressed(&self, button: BindingMouseButton) -> bool;
    fn shift_held(&self) -> bool;
    fn ctrl_held(&self) -> bool;
}

const LETTER_KEYS: [(&str, BindingKey); 26] = [
    ("A", BindingKey::KeyA),
    ("B", BindingKey::KeyB),
    ("C", BindingKey::KeyC),
    ("D", BindingKey::KeyD),
    ("E", BindingKey::KeyE),
    ("F", BindingKey::KeyF),
    ("G", BindingKey::KeyG),
    ("H", BindingKey::KeyH),
    ("I", BindingKey::KeyI),
    ("J", BindingKey::KeyJ),
    ("K", BindingKey::KeyK),
    ("L", BindingKey::KeyL),
    ("M", BindingKey::KeyM),
    ("N", BindingKey::KeyN),
    ("O", BindingKey::KeyO),
    ("P", BindingKey::KeyP),
    ("Q", BindingKey::KeyQ),
    ("R", BindingKey::KeyR),
    ("S", BindingKey::KeyS),
    ("T", BindingKey::KeyT),
    ("U", BindingKey::KeyU),
    ("V", BindingKey::KeyV),
    ("W", BindingKey::KeyW),
    ("X", BindingKey::KeyX),
    ("Y", BindingKey::KeyY),
    ("Z", BindingKey::KeyZ),
];

const DIGIT_KEYS: [(&str, BindingKey); 10] = [
    ("0", BindingKey::Digit0),
    ("1", BindingKey::Digit1),
    ("2", BindingKey::Digit2),
    ("3", BindingKey::Digit3),
    ("4", BindingKey::Digit4),
    ("5", BindingKey::Digit5),
    ("6", BindingKey::Digit6),
    ("7", BindingKey::Digit7),
    ("8", BindingKey::Digit8),
    ("9", BindingKey::Digit9),
];

const FUNCTION_KEYS: [(&str, BindingKey); 12] = [
    ("F1", BindingKey::F1),
    ("F2", BindingKey::F2),
    ("F3", BindingKey::F3),
    ("F4", BindingKey::F4),
    ("F5", BindingKey::F5),
    ("F6", BindingKey::F6),
    ("F7", BindingKey::F7),
    ("F8", BindingKey::F8),
    ("F9", BindingKey::F9),
    ("F10", BindingKey::F10),
    ("F11", BindingKey::F11),
    ("F12", BindingKey::F12),
];

const NAMED_KEYS: [(&str, BindingKey); 22] = [
    ("Space", BindingKey::Space),
    ("Tab", BindingKey::Tab),
    ("Escape", BindingKey::Escape),
    ("Minus", BindingKey::Minus),
    ("Equal", BindingKey::Equal),
    ("BracketLeft", BindingKey::BracketLeft),
    ("BracketRight", BindingKey::BracketRight),
    ("ArrowLeft", BindingKey::ArrowLeft),
    ("ArrowRight", BindingKey::ArrowRight),
    ("ArrowUp", BindingKey::ArrowUp),
    ("ArrowDown", BindingKey::ArrowDown),
    ("PageUp", BindingKey::PageUp),
    ("PageDown", BindingKey::PageDown),
    ("NumLock", BindingKey::NumLock),
    ("Home", BindingKey::Home),
    ("End", BindingKey::End),
    ("Insert", BindingKey::Insert),
    ("Delete", BindingKey::Delete),
    ("Backspace", BindingKey::Backspace),
    ("Enter", BindingKey::Enter),
    ("NumpadAdd", BindingKey::NumpadAdd),
    ("NumpadSubtract", BindingKey::NumpadSubtract),
];

struct InputActionMeta {
    key: &'static str,
    label: &'static str,
    section: BindingSection,
    default_binding: Option<InputBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Copy)]
pub enum InputAction {
    MoveForward,
    MoveBackward,
    StrafeLeft,
    StrafeRight,
    Jump,
    SitOrStand,
    RunToggle,
    AutoRun,
    TurnLeft,
    TurnRight,
    PitchUp,
    PitchDown,
    ZoomIn,
    ZoomOut,
    TargetNearest,
    TargetPreviousEnemy,
    AssistTarget,
    TargetSelf,
    InteractTarget,
    RaidTarget1,
    RaidTarget2,
    RaidTarget3,
    RaidTarget4,
    RaidTarget5,
    RaidTarget6,
    RaidTarget7,
    RaidTarget8,
    RaidTargetNone,
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
    /// Retail `MULTIACTIONBAR1BUTTON1..12`: Action Bar 2 (`MultiBarBottomLeft`).
    MultiActionBar1Button1,
    MultiActionBar1Button2,
    MultiActionBar1Button3,
    MultiActionBar1Button4,
    MultiActionBar1Button5,
    MultiActionBar1Button6,
    MultiActionBar1Button7,
    MultiActionBar1Button8,
    MultiActionBar1Button9,
    MultiActionBar1Button10,
    MultiActionBar1Button11,
    MultiActionBar1Button12,
    /// Retail `MULTIACTIONBAR2BUTTON1..12`: Action Bar 3 (`MultiBarBottomRight`).
    MultiActionBar2Button1,
    MultiActionBar2Button2,
    MultiActionBar2Button3,
    MultiActionBar2Button4,
    MultiActionBar2Button5,
    MultiActionBar2Button6,
    MultiActionBar2Button7,
    MultiActionBar2Button8,
    MultiActionBar2Button9,
    MultiActionBar2Button10,
    MultiActionBar2Button11,
    MultiActionBar2Button12,
    /// Retail `BONUSACTIONBUTTON1..10`: the pet action bar buttons.
    PetActionSlot1,
    PetActionSlot2,
    PetActionSlot3,
    PetActionSlot4,
    PetActionSlot5,
    PetActionSlot6,
    PetActionSlot7,
    PetActionSlot8,
    PetActionSlot9,
    PetActionSlot10,
    ToggleMute,
    ToggleCharacter,
    ToggleSpellbook,
    ToggleProfessions,
    ToggleAchievements,
    ToggleTalents,
    ToggleSpecialization,
    ToggleEncounterJournal,
    ToggleSocial,
    ToggleLootRules,
    ToggleQuestLog,
    ToggleWorldMap,
    ToggleFramerate,
    MinimapZoomIn,
    MinimapZoomOut,
    OpenAllBags,
    ToggleBackpack,
    ToggleBag1,
    ToggleBag2,
    ToggleBag3,
    ToggleBag4,
}

impl InputAction {
    /// `BONUSACTIONBUTTON1..10`, pet bar button 1..10.
    pub const PET_ACTION_SLOTS: [Self; 10] = [
        Self::PetActionSlot1,
        Self::PetActionSlot2,
        Self::PetActionSlot3,
        Self::PetActionSlot4,
        Self::PetActionSlot5,
        Self::PetActionSlot6,
        Self::PetActionSlot7,
        Self::PetActionSlot8,
        Self::PetActionSlot9,
        Self::PetActionSlot10,
    ];

    /// `ACTIONBUTTON1..12`, main bar button 1..12.
    pub const ACTION_BAR_BUTTONS: [Self; 12] = [
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
    ];

    /// `MULTIACTIONBAR1BUTTON1..12`, Action Bar 2 button 1..12.
    pub const MULTI_ACTION_BAR_1: [Self; 12] = [
        Self::MultiActionBar1Button1,
        Self::MultiActionBar1Button2,
        Self::MultiActionBar1Button3,
        Self::MultiActionBar1Button4,
        Self::MultiActionBar1Button5,
        Self::MultiActionBar1Button6,
        Self::MultiActionBar1Button7,
        Self::MultiActionBar1Button8,
        Self::MultiActionBar1Button9,
        Self::MultiActionBar1Button10,
        Self::MultiActionBar1Button11,
        Self::MultiActionBar1Button12,
    ];

    /// `MULTIACTIONBAR2BUTTON1..12`, Action Bar 3 button 1..12.
    pub const MULTI_ACTION_BAR_2: [Self; 12] = [
        Self::MultiActionBar2Button1,
        Self::MultiActionBar2Button2,
        Self::MultiActionBar2Button3,
        Self::MultiActionBar2Button4,
        Self::MultiActionBar2Button5,
        Self::MultiActionBar2Button6,
        Self::MultiActionBar2Button7,
        Self::MultiActionBar2Button8,
        Self::MultiActionBar2Button9,
        Self::MultiActionBar2Button10,
        Self::MultiActionBar2Button11,
        Self::MultiActionBar2Button12,
    ];

    pub const ALL: [Self; 95] = [
        Self::MoveForward,
        Self::MoveBackward,
        Self::StrafeLeft,
        Self::StrafeRight,
        Self::Jump,
        Self::SitOrStand,
        Self::RunToggle,
        Self::AutoRun,
        Self::TurnLeft,
        Self::TurnRight,
        Self::PitchUp,
        Self::PitchDown,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::TargetNearest,
        Self::TargetPreviousEnemy,
        Self::AssistTarget,
        Self::TargetSelf,
        Self::InteractTarget,
        Self::RaidTarget1,
        Self::RaidTarget2,
        Self::RaidTarget3,
        Self::RaidTarget4,
        Self::RaidTarget5,
        Self::RaidTarget6,
        Self::RaidTarget7,
        Self::RaidTarget8,
        Self::RaidTargetNone,
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
        Self::MultiActionBar1Button1,
        Self::MultiActionBar1Button2,
        Self::MultiActionBar1Button3,
        Self::MultiActionBar1Button4,
        Self::MultiActionBar1Button5,
        Self::MultiActionBar1Button6,
        Self::MultiActionBar1Button7,
        Self::MultiActionBar1Button8,
        Self::MultiActionBar1Button9,
        Self::MultiActionBar1Button10,
        Self::MultiActionBar1Button11,
        Self::MultiActionBar1Button12,
        Self::MultiActionBar2Button1,
        Self::MultiActionBar2Button2,
        Self::MultiActionBar2Button3,
        Self::MultiActionBar2Button4,
        Self::MultiActionBar2Button5,
        Self::MultiActionBar2Button6,
        Self::MultiActionBar2Button7,
        Self::MultiActionBar2Button8,
        Self::MultiActionBar2Button9,
        Self::MultiActionBar2Button10,
        Self::MultiActionBar2Button11,
        Self::MultiActionBar2Button12,
        Self::PetActionSlot1,
        Self::PetActionSlot2,
        Self::PetActionSlot3,
        Self::PetActionSlot4,
        Self::PetActionSlot5,
        Self::PetActionSlot6,
        Self::PetActionSlot7,
        Self::PetActionSlot8,
        Self::PetActionSlot9,
        Self::PetActionSlot10,
        Self::ToggleMute,
        Self::ToggleCharacter,
        Self::ToggleSpellbook,
        Self::ToggleProfessions,
        Self::ToggleAchievements,
        Self::ToggleTalents,
        Self::ToggleSpecialization,
        Self::ToggleEncounterJournal,
        Self::ToggleSocial,
        Self::ToggleLootRules,
        Self::ToggleQuestLog,
        Self::ToggleWorldMap,
        Self::ToggleFramerate,
        Self::MinimapZoomIn,
        Self::MinimapZoomOut,
        Self::OpenAllBags,
        Self::ToggleBackpack,
        Self::ToggleBag1,
        Self::ToggleBag2,
        Self::ToggleBag3,
        Self::ToggleBag4,
    ];

    pub fn key(self) -> &'static str {
        self.meta().key
    }

    pub fn from_key(key: &str) -> Option<Self> {
        movement_action_from_key(key)
            .or_else(|| camera_action_from_key(key))
            .or_else(|| targeting_action_from_key(key))
            .or_else(|| action_slot_from_key(key))
            .or_else(|| multi_action_bar_from_key(key))
            .or_else(|| pet_action_slot_from_key(key))
            .or_else(|| audio_action_from_key(key))
            .or_else(|| interface_action_from_key(key))
            .or_else(|| bag_action_from_key(key))
    }

    pub fn label(self) -> &'static str {
        self.meta().label
    }

    pub fn section(self) -> BindingSection {
        self.meta().section
    }

    /// The raid target icon a `RAIDTARGET1..8` binding assigns, 0 for `RAIDTARGETNONE`.
    pub fn raid_target_icon(self) -> Option<u8> {
        Some(match self {
            Self::RaidTarget1 => 1,
            Self::RaidTarget2 => 2,
            Self::RaidTarget3 => 3,
            Self::RaidTarget4 => 4,
            Self::RaidTarget5 => 5,
            Self::RaidTarget6 => 6,
            Self::RaidTarget7 => 7,
            Self::RaidTarget8 => 8,
            Self::RaidTargetNone => 0,
            _ => return None,
        })
    }

    pub fn default_binding(self) -> Option<InputBinding> {
        self.meta().default_binding
    }

    /// Pet bar button index (0-based) a `BONUSACTIONBUTTON` binding presses.
    pub fn pet_action_slot(self) -> Option<usize> {
        Self::PET_ACTION_SLOTS
            .iter()
            .position(|action| *action == self)
    }

    fn meta(self) -> InputActionMeta {
        if let Some(meta) = self.action_slot_meta() {
            return meta;
        }
        if let Some(meta) = self.multi_action_bar_meta() {
            return meta;
        }
        if let Some(meta) = self.pet_action_slot_meta() {
            return meta;
        }
        if let Some(meta) = self.interface_meta() {
            return meta;
        }
        if let Some(meta) = self.bag_meta() {
            return meta;
        }
        self.non_action_slot_meta()
    }

    fn interface_meta(self) -> Option<InputActionMeta> {
        let (key, label, binding) = match self {
            Self::ToggleCharacter => (
                "toggle_character",
                "Character Info",
                Some(keyboard(BindingKey::KeyC)),
            ),
            Self::ToggleSpellbook => (
                "toggle_spellbook",
                "Spellbook",
                Some(keyboard(BindingKey::KeyP)),
            ),
            Self::ToggleProfessions => (
                "toggle_professions",
                "Professions",
                Some(keyboard(BindingKey::KeyK)),
            ),
            Self::ToggleAchievements => (
                "toggle_achievements",
                "Achievements",
                Some(keyboard(BindingKey::KeyY)),
            ),
            Self::ToggleTalents => (
                "toggle_talents",
                "Talents",
                Some(keyboard(BindingKey::KeyN)),
            ),
            // Retail has no separate specialization default (Bindings_Standard.xml:1244).
            Self::ToggleSpecialization => ("toggle_specialization", "Specialization", None),
            Self::ToggleEncounterJournal => (
                "toggle_encounter_journal",
                "Adventure Guide",
                Some(keyboard(BindingKey::KeyJ)),
            ),
            Self::ToggleSocial => ("toggle_social", "Social", Some(keyboard(BindingKey::KeyO))),
            Self::ToggleLootRules => ("toggle_loot_rules", "Loot Rules", None),
            // Retail TOGGLEQUESTLOG default binding.
            Self::ToggleQuestLog => (
                "toggle_quest_log",
                "Quest Log",
                Some(keyboard(BindingKey::KeyL)),
            ),
            Self::ToggleWorldMap => (
                "toggle_world_map",
                "World Map",
                Some(keyboard(BindingKey::KeyM)),
            ),
            // Retail `TOGGLEFPS` (`Bindings_Standard.xml`: `FramerateFrame:Toggle()`), Ctrl+R.
            Self::ToggleFramerate => (
                "toggle_framerate",
                "Toggle Framerate Display",
                Some(InputBinding::CtrlKeyboard(BindingKey::KeyR)),
            ),
            // Retail `MINIMAPZOOMIN`/`MINIMAPZOOMOUT` (`Bindings_Standard.xml:1378-1383`):
            // `Minimap_ZoomIn()`/`Minimap_ZoomOut()`, Num Pad +/-.
            Self::MinimapZoomIn => (
                "minimap_zoom_in",
                "Minimap Zoom In",
                Some(keyboard(BindingKey::NumpadAdd)),
            ),
            Self::MinimapZoomOut => (
                "minimap_zoom_out",
                "Minimap Zoom Out",
                Some(keyboard(BindingKey::NumpadSubtract)),
            ),
            _ => return None,
        };
        Some(input_action_meta(
            key,
            label,
            BindingSection::Interface,
            binding,
        ))
    }

    /// Retail `Bindings_Standard.xml:1203-1223` bag bindings with the client's default
    /// keys (the user's Retail `bindings-cache.wtf` unbinds SHIFT-B and F8-F11 from
    /// them). Single-slot model: TOGGLEBACKPACK's second Retail key, F12, is not bound.
    fn bag_meta(self) -> Option<InputActionMeta> {
        let (key, label, binding) = match self {
            Self::OpenAllBags => ("open_all_bags", "Open All Bags", keyboard(BindingKey::KeyB)),
            Self::ToggleBackpack => (
                "toggle_backpack",
                "Toggle Backpack",
                InputBinding::ShiftKeyboard(BindingKey::KeyB),
            ),
            Self::ToggleBag1 => ("toggle_bag_1", "Toggle Bag 1", keyboard(BindingKey::F8)),
            Self::ToggleBag2 => ("toggle_bag_2", "Toggle Bag 2", keyboard(BindingKey::F9)),
            Self::ToggleBag3 => ("toggle_bag_3", "Toggle Bag 3", keyboard(BindingKey::F10)),
            Self::ToggleBag4 => ("toggle_bag_4", "Toggle Bag 4", keyboard(BindingKey::F11)),
            _ => return None,
        };
        Some(input_action_meta(
            key,
            label,
            BindingSection::Bags,
            Some(binding),
        ))
    }

    /// Container id a `TOGGLEBAGn` binding toggles: `ToggleBag(5 - n)`
    /// (`Bindings_Standard.xml:1209-1220`), so F8..F11 run left to right along the bag bar.
    pub fn toggled_bag(self) -> Option<usize> {
        Some(match self {
            Self::ToggleBag1 => 4,
            Self::ToggleBag2 => 3,
            Self::ToggleBag3 => 2,
            Self::ToggleBag4 => 1,
            _ => return None,
        })
    }

    fn non_action_slot_meta(self) -> InputActionMeta {
        match self {
            Self::MoveForward => movement_meta("move_forward", "Move Forward", BindingKey::KeyW),
            Self::MoveBackward => movement_meta("move_backward", "Move Backward", BindingKey::KeyS),
            Self::StrafeLeft => movement_meta("strafe_left", "Strafe Left", BindingKey::KeyA),
            Self::StrafeRight => movement_meta("strafe_right", "Strafe Right", BindingKey::KeyD),
            Self::Jump => movement_meta("jump", "Jump", BindingKey::Space),
            // Retail `SITORSTAND` (`BINDING_NAME_SITORSTAND`): descends while swimming.
            Self::SitOrStand => movement_meta("sit_or_stand", "Sit/Move Down", BindingKey::KeyX),
            Self::RunToggle => movement_meta("run_toggle", "Run / Walk Toggle", BindingKey::KeyZ),
            Self::AutoRun => movement_meta("auto_run", "Auto-Run", BindingKey::NumLock),
            Self::TurnLeft => camera_meta("turn_left", "Turn Left", BindingKey::ArrowLeft),
            Self::TurnRight => camera_meta("turn_right", "Turn Right", BindingKey::ArrowRight),
            Self::PitchUp => camera_meta("pitch_up", "Pitch Up", BindingKey::ArrowUp),
            Self::PitchDown => camera_meta("pitch_down", "Pitch Down", BindingKey::ArrowDown),
            Self::ZoomIn => camera_meta("zoom_in", "Zoom In", BindingKey::PageUp),
            Self::ZoomOut => camera_meta("zoom_out", "Zoom Out", BindingKey::PageDown),
            Self::TargetNearest => {
                targeting_meta("target_nearest", "Target Nearest", BindingKey::Tab)
            }
            // `TARGETPREVIOUSENEMY`: `TargetNearestEnemy(true)` (Bindings_Standard.xml:998).
            Self::TargetPreviousEnemy => input_action_meta(
                "target_previous_enemy",
                "Target Previous Enemy",
                BindingSection::Targeting,
                Some(InputBinding::ShiftKeyboard(BindingKey::Tab)),
            ),
            // `ASSISTTARGET`: `AssistUnit("target")` (Bindings_Standard.xml:1174).
            Self::AssistTarget => {
                targeting_meta("assist_target", "Assist Target", BindingKey::KeyF)
            }
            Self::TargetSelf => targeting_meta("target_self", "Target Self", BindingKey::F1),
            // `INTERACTTARGET`: `InteractUnit("anyinteract")` (Bindings_Standard.xml:1171).
            // Unbound by default: Retail's interact key tutorial warns when no key is
            // assigned (Blizzard_Tutorials_Frame_Tutorials.lua:94-99).
            Self::InteractTarget => input_action_meta(
                "interact_target",
                "Interact With Target",
                BindingSection::Targeting,
                None,
            ),
            // `RAIDTARGET1..8`: `SetRaidTargetIcon("target", n)`; `RAIDTARGETNONE`:
            // `SetRaidTarget("target", 0)` (Bindings_Standard.xml:1573-1599), unbound.
            Self::RaidTarget1 => raid_target_meta("raid_target_1", "Assign Star to Target"),
            Self::RaidTarget2 => raid_target_meta("raid_target_2", "Assign Circle to Target"),
            Self::RaidTarget3 => raid_target_meta("raid_target_3", "Assign Diamond to Target"),
            Self::RaidTarget4 => raid_target_meta("raid_target_4", "Assign Triangle to Target"),
            Self::RaidTarget5 => raid_target_meta("raid_target_5", "Assign Moon to Target"),
            Self::RaidTarget6 => raid_target_meta("raid_target_6", "Assign Square to Target"),
            Self::RaidTarget7 => raid_target_meta("raid_target_7", "Assign Cross to Target"),
            Self::RaidTarget8 => raid_target_meta("raid_target_8", "Assign Skull to Target"),
            Self::RaidTargetNone => {
                raid_target_meta("raid_target_none", "Clear Target Marker Icon")
            }
            Self::ToggleMute => input_action_meta(
                "toggle_mute",
                "Toggle Mute",
                BindingSection::Audio,
                Some(InputBinding::CtrlKeyboard(BindingKey::KeyS)),
            ),
            Self::ToggleCharacter
            | Self::ToggleSpellbook
            | Self::ToggleProfessions
            | Self::ToggleAchievements
            | Self::ToggleTalents
            | Self::ToggleSpecialization
            | Self::ToggleEncounterJournal
            | Self::ToggleSocial
            | Self::ToggleLootRules
            | Self::ToggleQuestLog
            | Self::ToggleWorldMap
            | Self::ToggleFramerate
            | Self::MinimapZoomIn
            | Self::MinimapZoomOut => unreachable!("panel toggles handled by interface_meta"),
            Self::OpenAllBags
            | Self::ToggleBackpack
            | Self::ToggleBag1
            | Self::ToggleBag2
            | Self::ToggleBag3
            | Self::ToggleBag4 => unreachable!("bag toggles handled by bag_meta"),
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
            | Self::ActionSlot12 => unreachable!("action slots handled by action_slot_meta"),
            Self::PetActionSlot1
            | Self::PetActionSlot2
            | Self::PetActionSlot3
            | Self::PetActionSlot4
            | Self::PetActionSlot5
            | Self::PetActionSlot6
            | Self::PetActionSlot7
            | Self::PetActionSlot8
            | Self::PetActionSlot9
            | Self::PetActionSlot10 => {
                unreachable!("pet action slots handled by pet_action_slot_meta")
            }
            Self::MultiActionBar1Button1
            | Self::MultiActionBar1Button2
            | Self::MultiActionBar1Button3
            | Self::MultiActionBar1Button4
            | Self::MultiActionBar1Button5
            | Self::MultiActionBar1Button6
            | Self::MultiActionBar1Button7
            | Self::MultiActionBar1Button8
            | Self::MultiActionBar1Button9
            | Self::MultiActionBar1Button10
            | Self::MultiActionBar1Button11
            | Self::MultiActionBar1Button12
            | Self::MultiActionBar2Button1
            | Self::MultiActionBar2Button2
            | Self::MultiActionBar2Button3
            | Self::MultiActionBar2Button4
            | Self::MultiActionBar2Button5
            | Self::MultiActionBar2Button6
            | Self::MultiActionBar2Button7
            | Self::MultiActionBar2Button8
            | Self::MultiActionBar2Button9
            | Self::MultiActionBar2Button10
            | Self::MultiActionBar2Button11
            | Self::MultiActionBar2Button12 => {
                unreachable!("extra action bar buttons handled by multi_action_bar_meta")
            }
        }
    }

    /// Retail `MULTIACTIONBAR1BUTTONn` / `MULTIACTIONBAR2BUTTONn` (`Bindings_Standard.xml:397-563`,
    /// categories `BINDING_HEADER_ACTIONBAR2` "Action Bar 2" / `BINDING_HEADER_ACTIONBAR3`
    /// "Action Bar 3", names `BINDING_NAME_MULTIACTIONBAR1BUTTONn` "Action Bar 2 Button n").
    /// Retail ships them unbound: the bindings XML carries no keys.
    fn multi_action_bar_meta(self) -> Option<InputActionMeta> {
        const BAR_2: [(&str, &str); 12] = [
            ("multi_action_bar_1_button_1", "Action Bar 2 Button 1"),
            ("multi_action_bar_1_button_2", "Action Bar 2 Button 2"),
            ("multi_action_bar_1_button_3", "Action Bar 2 Button 3"),
            ("multi_action_bar_1_button_4", "Action Bar 2 Button 4"),
            ("multi_action_bar_1_button_5", "Action Bar 2 Button 5"),
            ("multi_action_bar_1_button_6", "Action Bar 2 Button 6"),
            ("multi_action_bar_1_button_7", "Action Bar 2 Button 7"),
            ("multi_action_bar_1_button_8", "Action Bar 2 Button 8"),
            ("multi_action_bar_1_button_9", "Action Bar 2 Button 9"),
            ("multi_action_bar_1_button_10", "Action Bar 2 Button 10"),
            ("multi_action_bar_1_button_11", "Action Bar 2 Button 11"),
            ("multi_action_bar_1_button_12", "Action Bar 2 Button 12"),
        ];
        const BAR_3: [(&str, &str); 12] = [
            ("multi_action_bar_2_button_1", "Action Bar 3 Button 1"),
            ("multi_action_bar_2_button_2", "Action Bar 3 Button 2"),
            ("multi_action_bar_2_button_3", "Action Bar 3 Button 3"),
            ("multi_action_bar_2_button_4", "Action Bar 3 Button 4"),
            ("multi_action_bar_2_button_5", "Action Bar 3 Button 5"),
            ("multi_action_bar_2_button_6", "Action Bar 3 Button 6"),
            ("multi_action_bar_2_button_7", "Action Bar 3 Button 7"),
            ("multi_action_bar_2_button_8", "Action Bar 3 Button 8"),
            ("multi_action_bar_2_button_9", "Action Bar 3 Button 9"),
            ("multi_action_bar_2_button_10", "Action Bar 3 Button 10"),
            ("multi_action_bar_2_button_11", "Action Bar 3 Button 11"),
            ("multi_action_bar_2_button_12", "Action Bar 3 Button 12"),
        ];
        let (bar, index) = self.multi_action_bar()?;
        let (names, section) = if bar == 1 {
            (&BAR_2, BindingSection::ActionBar2)
        } else {
            (&BAR_3, BindingSection::ActionBar3)
        };
        let (key, label) = names[index];
        Some(input_action_meta(key, label, section, None))
    }

    /// `(n, button index)` of a `MULTIACTIONBAR<n>BUTTON` binding, button index 0-based.
    pub fn multi_action_bar(self) -> Option<(u8, usize)> {
        let in_bar = |bar: &[Self; 12]| bar.iter().position(|action| *action == self);
        in_bar(&Self::MULTI_ACTION_BAR_1)
            .map(|index| (1, index))
            .or_else(|| in_bar(&Self::MULTI_ACTION_BAR_2).map(|index| (2, index)))
    }

    /// Retail `BONUSACTIONBUTTON1..10` (`Bindings_Standard.xml:272-361`,
    /// `BINDING_HEADER_ACTIONBAR`, `BINDING_NAME_BONUSACTIONBUTTONn` "Pet Action Button n")
    /// on Ctrl-1..Ctrl-9, Ctrl-0 (`~/Repos/worldofwhatever/DefaultBindings.wtf:53-62`,
    /// `bind CTRL-1 BONUSACTIONBUTTON1`; the bindings XML carries no keys).
    fn pet_action_slot_meta(self) -> Option<InputActionMeta> {
        const KEYS: [(&str, &str, BindingKey); 10] = [
            (
                "pet_action_slot_1",
                "Pet Action Button 1",
                BindingKey::Digit1,
            ),
            (
                "pet_action_slot_2",
                "Pet Action Button 2",
                BindingKey::Digit2,
            ),
            (
                "pet_action_slot_3",
                "Pet Action Button 3",
                BindingKey::Digit3,
            ),
            (
                "pet_action_slot_4",
                "Pet Action Button 4",
                BindingKey::Digit4,
            ),
            (
                "pet_action_slot_5",
                "Pet Action Button 5",
                BindingKey::Digit5,
            ),
            (
                "pet_action_slot_6",
                "Pet Action Button 6",
                BindingKey::Digit6,
            ),
            (
                "pet_action_slot_7",
                "Pet Action Button 7",
                BindingKey::Digit7,
            ),
            (
                "pet_action_slot_8",
                "Pet Action Button 8",
                BindingKey::Digit8,
            ),
            (
                "pet_action_slot_9",
                "Pet Action Button 9",
                BindingKey::Digit9,
            ),
            (
                "pet_action_slot_10",
                "Pet Action Button 10",
                BindingKey::Digit0,
            ),
        ];
        let (key, label, digit) = KEYS[self.pet_action_slot()?];
        Some(input_action_meta(
            key,
            label,
            BindingSection::ActionBar,
            Some(InputBinding::CtrlKeyboard(digit)),
        ))
    }

    fn action_slot_meta(self) -> Option<InputActionMeta> {
        let (slot, key) = match self {
            Self::ActionSlot1 => (1, BindingKey::Digit1),
            Self::ActionSlot2 => (2, BindingKey::Digit2),
            Self::ActionSlot3 => (3, BindingKey::Digit3),
            Self::ActionSlot4 => (4, BindingKey::Digit4),
            Self::ActionSlot5 => (5, BindingKey::Digit5),
            Self::ActionSlot6 => (6, BindingKey::Digit6),
            Self::ActionSlot7 => (7, BindingKey::Digit7),
            Self::ActionSlot8 => (8, BindingKey::Digit8),
            Self::ActionSlot9 => (9, BindingKey::Digit9),
            Self::ActionSlot10 => (10, BindingKey::Digit0),
            Self::ActionSlot11 => (11, BindingKey::Minus),
            Self::ActionSlot12 => (12, BindingKey::Equal),
            _ => return None,
        };
        Some(action_slot_meta(slot, self, key))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Copy)]
pub enum BindingSection {
    Movement,
    Camera,
    Targeting,
    ActionBar,
    /// `BINDING_HEADER_ACTIONBAR2`.
    ActionBar2,
    /// `BINDING_HEADER_ACTIONBAR3`.
    ActionBar3,
    Audio,
    Interface,
    Bags,
}

impl BindingSection {
    pub const ALL: [Self; 9] = [
        Self::Movement,
        Self::Camera,
        Self::Targeting,
        Self::ActionBar,
        Self::ActionBar2,
        Self::ActionBar3,
        Self::Audio,
        Self::Interface,
        Self::Bags,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Movement => "movement",
            Self::Camera => "camera",
            Self::Targeting => "targeting",
            Self::ActionBar => "action_bar",
            Self::ActionBar2 => "action_bar_2",
            Self::ActionBar3 => "action_bar_3",
            Self::Audio => "audio",
            Self::Interface => "interface",
            Self::Bags => "bags",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "movement" => Self::Movement,
            "camera" => Self::Camera,
            "targeting" => Self::Targeting,
            "action_bar" => Self::ActionBar,
            "action_bar_2" => Self::ActionBar2,
            "action_bar_3" => Self::ActionBar3,
            "audio" => Self::Audio,
            "interface" => Self::Interface,
            "bags" => Self::Bags,
            _ => return None,
        })
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Movement => "Movement",
            Self::Camera => "Camera",
            Self::Targeting => "Targeting",
            Self::ActionBar => "Action Bar",
            Self::ActionBar2 => "Action Bar 2",
            Self::ActionBar3 => "Action Bar 3",
            Self::Audio => "Audio",
            Self::Interface => "Interface",
            Self::Bags => "Bags",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum InputBinding {
    Keyboard(BindingKey),
    /// Key pressed while either Shift is held.
    ShiftKeyboard(BindingKey),
    /// Key pressed while either Ctrl is held.
    CtrlKeyboard(BindingKey),
    Mouse(BindingMouseButton),
}

impl InputBinding {
    pub fn pressed(self, state: &impl InputState) -> bool {
        match self {
            Self::Keyboard(key) => state.key_pressed(key),
            Self::ShiftKeyboard(key) => state.shift_held() && state.key_pressed(key),
            Self::CtrlKeyboard(key) => state.ctrl_held() && state.key_pressed(key),
            Self::Mouse(button) => state.mouse_pressed(button),
        }
    }

    pub fn just_pressed(self, state: &impl InputState) -> bool {
        match self {
            Self::Keyboard(key) => state.key_just_pressed(key),
            Self::ShiftKeyboard(key) => state.shift_held() && state.key_just_pressed(key),
            Self::CtrlKeyboard(key) => state.ctrl_held() && state.key_just_pressed(key),
            Self::Mouse(button) => state.mouse_just_pressed(button),
        }
    }

    /// Action button hotkey label, `GetBindingText(key, 1)` (`ActionButton.lua:491`):
    /// modifiers abbreviated to `CTRL_KEY_TEXT_ABBR` "c" and `SHIFT_KEY_TEXT_ABBR` "s"
    /// (GlobalStrings); the key keeps its `KEY_<name>` text, since Retail defines
    /// `KEY_ABBR_<name>` only for gamepad buttons (`SharedConstants.lua:56-90`).
    pub fn hotkey_text(self) -> String {
        match self {
            Self::Keyboard(key) => key_display(key),
            Self::ShiftKeyboard(key) => format!("s-{}", key_display(key)),
            Self::CtrlKeyboard(key) => format!("c-{}", key_display(key)),
            Self::Mouse(button) => mouse_button_display(button),
        }
    }

    /// The Retail binding key `GetBindingKey` returns: uppercase, never localised,
    /// modifier first ("CTRL-1", "SHIFT-]", "BUTTON3", "SPACE").
    pub fn binding_key_name(self) -> String {
        match self {
            Self::Keyboard(key) => key_binding_name(key),
            Self::ShiftKeyboard(key) => format!("SHIFT-{}", key_binding_name(key)),
            Self::CtrlKeyboard(key) => format!("CTRL-{}", key_binding_name(key)),
            Self::Mouse(button) => mouse_button_binding_name(button),
        }
    }

    /// FlareUI's clean keybind label (`ShortenKey`, FlareUI `Modules/ActionBars.lua:201-222`):
    /// the raw binding key with `BUTTONn` -> `Bn`, then `KEY_SHORT`'s replacements in order
    /// ("CTRL-1" -> "C1", "SHIFT-BUTTON3" -> "SB3", "BACKSPACE" -> "BS").
    pub fn flare_hotkey_text(self) -> String {
        let name = self.binding_key_name();
        let mut text = match name.split_once("BUTTON") {
            Some((head, digits)) if digits.chars().all(|c| c.is_ascii_digit()) => {
                format!("{head}B{digits}")
            }
            _ => name,
        };
        for (long, short) in FLARE_KEY_SHORT {
            text = text.replace(long, short);
        }
        text
    }

    pub fn display(self) -> String {
        match self {
            Self::Keyboard(key) => key_display(key),
            Self::ShiftKeyboard(key) => format!("Shift-{}", key_display(key)),
            Self::CtrlKeyboard(key) => format!("Ctrl-{}", key_display(key)),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "SavedInputBindings")]
pub struct InputBindingsData {
    bindings: BTreeMap<InputAction, Option<InputBinding>>,
}

/// Persisted form. Saved files store every action, so a saved value equal to a
/// retired default is treated as "never customized".
#[derive(Deserialize)]
struct SavedInputBindings {
    bindings: BTreeMap<InputAction, Option<InputBinding>>,
}

/// Defaults that shipped and were later changed: (action, old default).
const RETIRED_DEFAULTS: [(InputAction, InputBinding); 1] = [(
    InputAction::ToggleMute,
    InputBinding::Keyboard(BindingKey::KeyM),
)];

impl From<SavedInputBindings> for InputBindingsData {
    /// Explicit saved bindings win. Actions missing from the file or still on a
    /// retired default get the current default unless an explicit binding owns it.
    fn from(saved: SavedInputBindings) -> Self {
        let mut bindings: BTreeMap<_, _> = saved
            .bindings
            .into_iter()
            .filter(|(action, binding)| !is_retired_default(*action, *binding))
            .collect();
        for action in InputAction::ALL {
            if bindings.contains_key(&action) {
                continue;
            }
            let default = action
                .default_binding()
                .filter(|binding| !bindings.values().any(|owned| *owned == Some(*binding)));
            bindings.insert(action, default);
        }
        Self { bindings }
    }
}

fn is_retired_default(action: InputAction, binding: Option<InputBinding>) -> bool {
    RETIRED_DEFAULTS
        .iter()
        .any(|(retired_action, retired)| *retired_action == action && binding == Some(*retired))
}

impl Default for InputBindingsData {
    fn default() -> Self {
        let bindings = InputAction::ALL
            .into_iter()
            .map(|action| (action, action.default_binding()))
            .collect();
        Self { bindings }
    }
}

impl InputBindingsData {
    pub fn binding(&self, action: InputAction) -> Option<InputBinding> {
        self.bindings.get(&action).copied().flatten()
    }

    pub fn is_pressed(&self, action: InputAction, state: &impl InputState) -> bool {
        self.binding(action).is_some_and(|binding| {
            binding.pressed(state) && !self.shadowed_by_modified_binding(binding, state)
        })
    }

    pub fn is_just_pressed(&self, action: InputAction, state: &impl InputState) -> bool {
        self.binding(action).is_some_and(|binding| {
            binding.just_pressed(state) && !self.shadowed_by_modified_binding(binding, state)
        })
    }

    fn shadowed_by_modified_binding(&self, binding: InputBinding, state: &impl InputState) -> bool {
        let InputBinding::Keyboard(key) = binding else {
            return false;
        };
        let owned = |modified| self.bindings.values().any(|b| *b == Some(modified));
        (state.shift_held() && owned(InputBinding::ShiftKeyboard(key)))
            || (state.ctrl_held() && owned(InputBinding::CtrlKeyboard(key)))
    }

    /// Binds `binding` to `action`, unbinding it from whichever action held it; returns that
    /// action (Retail `KeybindListener:UnbindKey`, `Blizzard_Keybindings.lua:121-139`).
    pub fn assign(&mut self, action: InputAction, binding: InputBinding) -> Option<InputAction> {
        let mut unbound = None;
        for existing in InputAction::ALL {
            if existing != action && self.binding(existing) == Some(binding) {
                self.bindings.insert(existing, None);
                unbound = Some(existing);
            }
        }
        self.bindings.insert(action, Some(binding));
        unbound
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

fn keyboard(key: BindingKey) -> InputBinding {
    InputBinding::Keyboard(key)
}

fn action_slot_meta(slot: u8, _action: InputAction, default_key: BindingKey) -> InputActionMeta {
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

fn movement_meta(
    key: &'static str,
    label: &'static str,
    default_key: BindingKey,
) -> InputActionMeta {
    input_action_meta(
        key,
        label,
        BindingSection::Movement,
        Some(InputBinding::Keyboard(default_key)),
    )
}

fn camera_meta(key: &'static str, label: &'static str, default_key: BindingKey) -> InputActionMeta {
    input_action_meta(
        key,
        label,
        BindingSection::Camera,
        Some(InputBinding::Keyboard(default_key)),
    )
}

fn targeting_meta(
    key: &'static str,
    label: &'static str,
    default_key: BindingKey,
) -> InputActionMeta {
    input_action_meta(
        key,
        label,
        BindingSection::Targeting,
        Some(InputBinding::Keyboard(default_key)),
    )
}

/// Retail files these under `BINDING_HEADER_RAID_TARGET` ("Target Markers").
fn raid_target_meta(key: &'static str, label: &'static str) -> InputActionMeta {
    input_action_meta(key, label, BindingSection::Targeting, None)
}

fn movement_action_from_key(key: &str) -> Option<InputAction> {
    Some(match key {
        "move_forward" => InputAction::MoveForward,
        "move_backward" => InputAction::MoveBackward,
        "strafe_left" => InputAction::StrafeLeft,
        "strafe_right" => InputAction::StrafeRight,
        "jump" => InputAction::Jump,
        "sit_or_stand" => InputAction::SitOrStand,
        "run_toggle" => InputAction::RunToggle,
        "auto_run" => InputAction::AutoRun,
        _ => return None,
    })
}

fn camera_action_from_key(key: &str) -> Option<InputAction> {
    Some(match key {
        "turn_left" => InputAction::TurnLeft,
        "turn_right" => InputAction::TurnRight,
        "pitch_up" => InputAction::PitchUp,
        "pitch_down" => InputAction::PitchDown,
        "zoom_in" => InputAction::ZoomIn,
        "zoom_out" => InputAction::ZoomOut,
        _ => return None,
    })
}

fn targeting_action_from_key(key: &str) -> Option<InputAction> {
    Some(match key {
        "target_nearest" => InputAction::TargetNearest,
        "target_previous_enemy" => InputAction::TargetPreviousEnemy,
        "assist_target" => InputAction::AssistTarget,
        "target_self" => InputAction::TargetSelf,
        "interact_target" => InputAction::InteractTarget,
        "raid_target_1" => InputAction::RaidTarget1,
        "raid_target_2" => InputAction::RaidTarget2,
        "raid_target_3" => InputAction::RaidTarget3,
        "raid_target_4" => InputAction::RaidTarget4,
        "raid_target_5" => InputAction::RaidTarget5,
        "raid_target_6" => InputAction::RaidTarget6,
        "raid_target_7" => InputAction::RaidTarget7,
        "raid_target_8" => InputAction::RaidTarget8,
        "raid_target_none" => InputAction::RaidTargetNone,
        _ => return None,
    })
}

fn action_slot_from_key(key: &str) -> Option<InputAction> {
    Some(match key {
        "action_slot_1" => InputAction::ActionSlot1,
        "action_slot_2" => InputAction::ActionSlot2,
        "action_slot_3" => InputAction::ActionSlot3,
        "action_slot_4" => InputAction::ActionSlot4,
        "action_slot_5" => InputAction::ActionSlot5,
        "action_slot_6" => InputAction::ActionSlot6,
        "action_slot_7" => InputAction::ActionSlot7,
        "action_slot_8" => InputAction::ActionSlot8,
        "action_slot_9" => InputAction::ActionSlot9,
        "action_slot_10" => InputAction::ActionSlot10,
        "action_slot_11" => InputAction::ActionSlot11,
        "action_slot_12" => InputAction::ActionSlot12,
        _ => return None,
    })
}

fn multi_action_bar_from_key(key: &str) -> Option<InputAction> {
    InputAction::MULTI_ACTION_BAR_1
        .into_iter()
        .chain(InputAction::MULTI_ACTION_BAR_2)
        .find(|action| action.key() == key)
}

fn pet_action_slot_from_key(key: &str) -> Option<InputAction> {
    InputAction::PET_ACTION_SLOTS
        .into_iter()
        .find(|action| action.key() == key)
}

fn interface_action_from_key(key: &str) -> Option<InputAction> {
    Some(match key {
        "toggle_character" => InputAction::ToggleCharacter,
        "toggle_spellbook" => InputAction::ToggleSpellbook,
        "toggle_professions" => InputAction::ToggleProfessions,
        "toggle_achievements" => InputAction::ToggleAchievements,
        "toggle_talents" => InputAction::ToggleTalents,
        "toggle_specialization" => InputAction::ToggleSpecialization,
        "toggle_encounter_journal" => InputAction::ToggleEncounterJournal,
        "toggle_social" => InputAction::ToggleSocial,
        "toggle_loot_rules" => InputAction::ToggleLootRules,
        "toggle_quest_log" => InputAction::ToggleQuestLog,
        "toggle_world_map" => InputAction::ToggleWorldMap,
        "toggle_framerate" => InputAction::ToggleFramerate,
        "minimap_zoom_in" => InputAction::MinimapZoomIn,
        "minimap_zoom_out" => InputAction::MinimapZoomOut,
        _ => return None,
    })
}

fn bag_action_from_key(key: &str) -> Option<InputAction> {
    Some(match key {
        "open_all_bags" => InputAction::OpenAllBags,
        "toggle_backpack" => InputAction::ToggleBackpack,
        "toggle_bag_1" => InputAction::ToggleBag1,
        "toggle_bag_2" => InputAction::ToggleBag2,
        "toggle_bag_3" => InputAction::ToggleBag3,
        "toggle_bag_4" => InputAction::ToggleBag4,
        _ => return None,
    })
}

fn audio_action_from_key(key: &str) -> Option<InputAction> {
    match key {
        "toggle_mute" => Some(InputAction::ToggleMute),
        _ => None,
    }
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
        BindingSection::Movement => movement_section_actions(),
        BindingSection::Camera => camera_section_actions(),
        BindingSection::Targeting => targeting_section_actions(),
        BindingSection::ActionBar => action_bar_section_actions(),
        BindingSection::ActionBar2 => &InputAction::MULTI_ACTION_BAR_1,
        BindingSection::ActionBar3 => &InputAction::MULTI_ACTION_BAR_2,
        BindingSection::Audio => audio_section_actions(),
        BindingSection::Interface => interface_section_actions(),
        BindingSection::Bags => bag_section_actions(),
    }
}

fn movement_section_actions() -> &'static [InputAction] {
    &[
        InputAction::MoveForward,
        InputAction::MoveBackward,
        InputAction::StrafeLeft,
        InputAction::StrafeRight,
        InputAction::Jump,
        InputAction::SitOrStand,
        InputAction::RunToggle,
        InputAction::AutoRun,
    ]
}

fn camera_section_actions() -> &'static [InputAction] {
    &[
        InputAction::TurnLeft,
        InputAction::TurnRight,
        InputAction::PitchUp,
        InputAction::PitchDown,
        InputAction::ZoomIn,
        InputAction::ZoomOut,
    ]
}

fn targeting_section_actions() -> &'static [InputAction] {
    &[
        InputAction::TargetNearest,
        InputAction::TargetPreviousEnemy,
        InputAction::AssistTarget,
        InputAction::TargetSelf,
        InputAction::InteractTarget,
        InputAction::RaidTarget1,
        InputAction::RaidTarget2,
        InputAction::RaidTarget3,
        InputAction::RaidTarget4,
        InputAction::RaidTarget5,
        InputAction::RaidTarget6,
        InputAction::RaidTarget7,
        InputAction::RaidTarget8,
        InputAction::RaidTargetNone,
    ]
}

fn bag_section_actions() -> &'static [InputAction] {
    &[
        InputAction::OpenAllBags,
        InputAction::ToggleBackpack,
        InputAction::ToggleBag1,
        InputAction::ToggleBag2,
        InputAction::ToggleBag3,
        InputAction::ToggleBag4,
    ]
}

fn action_bar_section_actions() -> &'static [InputAction] {
    &[
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
        InputAction::PetActionSlot1,
        InputAction::PetActionSlot2,
        InputAction::PetActionSlot3,
        InputAction::PetActionSlot4,
        InputAction::PetActionSlot5,
        InputAction::PetActionSlot6,
        InputAction::PetActionSlot7,
        InputAction::PetActionSlot8,
        InputAction::PetActionSlot9,
        InputAction::PetActionSlot10,
    ]
}

fn audio_section_actions() -> &'static [InputAction] {
    &[InputAction::ToggleMute]
}

fn interface_section_actions() -> &'static [InputAction] {
    &[
        InputAction::ToggleCharacter,
        InputAction::ToggleSpellbook,
        InputAction::ToggleProfessions,
        InputAction::ToggleAchievements,
        InputAction::ToggleTalents,
        InputAction::ToggleSpecialization,
        InputAction::ToggleEncounterJournal,
        InputAction::ToggleSocial,
        InputAction::ToggleLootRules,
        InputAction::ToggleQuestLog,
        InputAction::ToggleWorldMap,
        InputAction::ToggleFramerate,
        InputAction::MinimapZoomIn,
        InputAction::MinimapZoomOut,
    ]
}

pub fn binding_token(binding: InputBinding) -> String {
    match binding {
        InputBinding::Keyboard(key) => format!("key:{key:?}"),
        InputBinding::ShiftKeyboard(key) => format!("shift+key:{key:?}"),
        InputBinding::CtrlKeyboard(key) => format!("ctrl+key:{key:?}"),
        InputBinding::Mouse(button) => format!("mouse:{button:?}"),
    }
}

pub fn parse_binding_token(token: &str) -> Result<InputBinding, String> {
    if let Some(key) = token.strip_prefix("shift+key:") {
        return parse_key_code(key)
            .map(InputBinding::ShiftKeyboard)
            .ok_or_else(|| format!("unsupported key binding token '{token}'"));
    }
    if let Some(key) = token.strip_prefix("ctrl+key:") {
        return parse_key_code(key)
            .map(InputBinding::CtrlKeyboard)
            .ok_or_else(|| format!("unsupported key binding token '{token}'"));
    }
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

/// FlareUI `KEY_SHORT` (`Modules/ActionBars.lua:201-212`), applied in this order.
const FLARE_KEY_SHORT: [(&str, &str); 29] = [
    ("ALT-", "A"),
    ("CTRL-", "C"),
    ("SHIFT-", "S"),
    ("META-", "M"),
    ("NUMPAD", "N"),
    ("PLUS", "+"),
    ("MINUS", "-"),
    ("MULTIPLY", "*"),
    ("DIVIDE", "/"),
    ("BACKSPACE", "BS"),
    ("CAPSLOCK", "Cp"),
    ("CLEAR", "Cl"),
    ("DELETE", "Del"),
    ("MOUSEWHEELDOWN", "WD"),
    ("MOUSEWHEELUP", "WU"),
    ("NUMLOCK", "NL"),
    ("PAGEDOWN", "PD"),
    ("PAGEUP", "PU"),
    ("SCROLLLOCK", "SL"),
    ("SPACEBAR", "Sp"),
    ("SPACE", "Sp"),
    ("TAB", "Tb"),
    ("DOWNARROW", "Dn"),
    ("LEFTARROW", "Lf"),
    ("RIGHTARROW", "Rt"),
    ("UPARROW", "Up"),
    ("INSERT", "Ins"),
    ("HOME", "Hm"),
    ("END", "En"),
];

/// Retail binding key name of `key` (`GetBindingKey` spelling).
fn key_binding_name(key: BindingKey) -> String {
    let name = match key {
        BindingKey::Space => "SPACE",
        BindingKey::Tab => "TAB",
        BindingKey::Escape => "ESCAPE",
        BindingKey::Minus => "-",
        BindingKey::Equal => "=",
        BindingKey::BracketLeft => "[",
        BindingKey::BracketRight => "]",
        BindingKey::ArrowLeft => "LEFT",
        BindingKey::ArrowRight => "RIGHT",
        BindingKey::ArrowUp => "UP",
        BindingKey::ArrowDown => "DOWN",
        BindingKey::PageUp => "PAGEUP",
        BindingKey::PageDown => "PAGEDOWN",
        BindingKey::NumLock => "NUMLOCK",
        BindingKey::Home => "HOME",
        BindingKey::End => "END",
        BindingKey::Insert => "INSERT",
        BindingKey::Delete => "DELETE",
        BindingKey::Backspace => "BACKSPACE",
        BindingKey::Enter => "ENTER",
        BindingKey::NumpadAdd => "NUMPADPLUS",
        BindingKey::NumpadSubtract => "NUMPADMINUS",
        // Letters and digits as labelled; F1..F12 as their variant name.
        _ => {
            return key_alpha_numeric_label(key)
                .map(str::to_string)
                .unwrap_or_else(|| format!("{key:?}"));
        }
    };
    name.to_string()
}

/// Retail mouse binding key: `BUTTON1` left .. `BUTTON5` forward.
fn mouse_button_binding_name(button: BindingMouseButton) -> String {
    let number = match button {
        BindingMouseButton::Left => 1,
        BindingMouseButton::Right => 2,
        BindingMouseButton::Middle => 3,
        BindingMouseButton::Back => 4,
        BindingMouseButton::Forward => 5,
        BindingMouseButton::Other(id) => id,
    };
    format!("BUTTON{number}")
}

pub fn key_display(key: BindingKey) -> String {
    key_short_label(key)
        .map(str::to_string)
        .unwrap_or_else(|| format!("{key:?}"))
}

fn key_short_label(key: BindingKey) -> Option<&'static str> {
    match key {
        // GlobalStrings `KEY_SPACE`.
        BindingKey::Space => Some("Spacebar"),
        BindingKey::Tab => Some("Tab"),
        BindingKey::Escape => Some("Escape"),
        BindingKey::Minus => Some("-"),
        BindingKey::Equal => Some("="),
        BindingKey::BracketLeft => Some("["),
        BindingKey::BracketRight => Some("]"),
        BindingKey::ArrowLeft => Some("Left Arrow"),
        BindingKey::ArrowRight => Some("Right Arrow"),
        BindingKey::ArrowUp => Some("Up Arrow"),
        BindingKey::ArrowDown => Some("Down Arrow"),
        BindingKey::PageUp => Some("Page Up"),
        BindingKey::PageDown => Some("Page Down"),
        BindingKey::NumLock => Some("Num Lock"),
        BindingKey::NumpadAdd => Some("Num Pad +"),
        BindingKey::NumpadSubtract => Some("Num Pad -"),
        _ => key_alpha_numeric_label(key),
    }
}

/// Label of a letter or digit key ("A" for `KeyA`, "1" for `Digit1`).
pub fn key_alpha_numeric_label(key: BindingKey) -> Option<&'static str> {
    LETTER_KEYS
        .iter()
        .chain(DIGIT_KEYS.iter())
        .find_map(|(label, code)| (*code == key).then_some(*label))
}

fn mouse_button_display(button: BindingMouseButton) -> String {
    match button {
        // GlobalStrings `KEY_BUTTON1..5`.
        BindingMouseButton::Left => "Left Mouse Button".to_string(),
        BindingMouseButton::Right => "Right Mouse Button".to_string(),
        BindingMouseButton::Middle => "Middle Mouse".to_string(),
        BindingMouseButton::Back => "Mouse Button 4".to_string(),
        BindingMouseButton::Forward => "Mouse Button 5".to_string(),
        BindingMouseButton::Other(id) => format!("Mouse Button {id}"),
    }
}

/// Parse a human key name ("M", "KeyM", "1", "F10", "Escape"), ignoring ASCII case.
pub fn parse_key_name(token: &str) -> Option<BindingKey> {
    let token = token.strip_prefix("Key").unwrap_or(token);
    let token = token.strip_prefix("Digit").unwrap_or(token);
    LETTER_KEYS
        .iter()
        .chain(DIGIT_KEYS.iter())
        .chain(FUNCTION_KEYS.iter())
        .chain(NAMED_KEYS.iter())
        .find_map(|(name, code)| name.eq_ignore_ascii_case(token).then_some(*code))
}

fn parse_key_code(token: &str) -> Option<BindingKey> {
    parse_letter_key(token)
        .or_else(|| parse_digit_key(token))
        .or_else(|| parse_function_key(token))
        .or_else(|| parse_named_key(token))
}

fn parse_letter_key(token: &str) -> Option<BindingKey> {
    let token = token.strip_prefix("Key")?;
    lookup_named_key(token, &LETTER_KEYS)
}

fn parse_digit_key(token: &str) -> Option<BindingKey> {
    let token = token.strip_prefix("Digit")?;
    lookup_named_key(token, &DIGIT_KEYS)
}

fn parse_function_key(token: &str) -> Option<BindingKey> {
    lookup_named_key(token, &FUNCTION_KEYS)
}

fn lookup_named_key(token: &str, entries: &[(&str, BindingKey)]) -> Option<BindingKey> {
    entries
        .iter()
        .find_map(|(name, code)| (*name == token).then_some(*code))
}

fn parse_named_key(token: &str) -> Option<BindingKey> {
    lookup_named_key(token, &NAMED_KEYS)
}

fn parse_mouse_button(token: &str) -> Option<BindingMouseButton> {
    match token {
        "Left" => Some(BindingMouseButton::Left),
        "Right" => Some(BindingMouseButton::Right),
        "Middle" => Some(BindingMouseButton::Middle),
        "Back" => Some(BindingMouseButton::Back),
        "Forward" => Some(BindingMouseButton::Forward),
        _ => token
            .strip_prefix("Other(")?
            .strip_suffix(')')?
            .parse()
            .ok()
            .map(BindingMouseButton::Other),
    }
}
