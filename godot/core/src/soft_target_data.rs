//! Retail soft-target CVars shared by the Bevy and Godot clients' options files.

use serde::{Deserialize, Serialize};

/// `SoftTargetInteractArc`: "0 = No yaw arc allowance, must be directly in front. 1 = Must
/// be in front yaw arc. 2 = Can be anywhere in targeting area." (Wow.exe CVar help).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum SoftTargetArc {
    #[default]
    DirectlyInFront,
    InFront,
    Anywhere,
}

impl TryFrom<u8> for SoftTargetArc {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, String> {
        match value {
            0 => Ok(Self::DirectlyInFront),
            1 => Ok(Self::InFront),
            2 => Ok(Self::Anywhere),
            _ => Err(format!("softTargetInteractArc {value} is not 0, 1 or 2")),
        }
    }
}

impl From<SoftTargetArc> for u8 {
    fn from(arc: SoftTargetArc) -> u8 {
        arc as u8
    }
}

/// Retail soft interact CVars, defaulting as Retail does (wow-ui-sim `cvars.yaml:1316-1330`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoftTargetOptions {
    #[serde(rename = "softTargetInteractArc")]
    pub interact_arc: SoftTargetArc,
    /// `SoftTargetInteractRange`, yards.
    #[serde(rename = "softTargetInteractRange")]
    pub interact_range: f32,
    #[serde(rename = "softTargetIconEnemy")]
    pub icon_enemy: bool,
    #[serde(rename = "softTargetIconInteract")]
    pub icon_interact: bool,
    #[serde(rename = "softTargetIconGameObject")]
    pub icon_game_object: bool,
    #[serde(rename = "softTargetLowPriorityIcons")]
    pub low_priority_icons: bool,
}

impl Default for SoftTargetOptions {
    fn default() -> Self {
        Self {
            interact_arc: SoftTargetArc::DirectlyInFront,
            interact_range: 10.0,
            icon_enemy: false,
            icon_interact: true,
            icon_game_object: false,
            low_priority_icons: false,
        }
    }
}

impl SoftTargetOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.interact_range.is_finite() && self.interact_range >= 0.0 {
            Ok(())
        } else {
            Err(format!(
                "softTargetInteractRange {} is not a distance",
                self.interact_range
            ))
        }
    }

    /// The Accessibility "Interact Key Icons" dropdown value (`GetValue`,
    /// Blizzard_SettingsDefinitions_Frame/Accessibility.lua:176-190).
    pub fn interact_key_icons(&self) -> InteractKeyIcons {
        let icons = [
            self.icon_enemy,
            self.icon_interact,
            self.icon_game_object,
            self.low_priority_icons,
        ];
        if icons.iter().all(|on| *on) {
            InteractKeyIcons::ShowAll
        } else if icons.iter().all(|on| !*on) {
            InteractKeyIcons::ShowNone
        } else {
            InteractKeyIcons::Default
        }
    }

    /// The dropdown's `SetValue` (Accessibility.lua:192-208).
    pub fn set_interact_key_icons(&mut self, value: InteractKeyIcons) {
        let (enemy, interact, game_object, low_priority) = match value {
            InteractKeyIcons::Default => (false, true, false, false),
            InteractKeyIcons::ShowAll => (true, true, true, true),
            InteractKeyIcons::ShowNone => (false, false, false, false),
        };
        self.icon_enemy = enemy;
        self.icon_interact = interact;
        self.icon_game_object = game_object;
        self.low_priority_icons = low_priority;
    }
}

/// "Interact Key Icons" (`INTERACT_ICONS_OPTION`) choices, Retail values 1-3
/// (Accessibility.lua:210-216, labels from GlobalStrings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractKeyIcons {
    /// `INTERACT_ICONS_DEFAULT` "NPCs Only (Default)".
    Default,
    /// `INTERACT_ICONS_SHOW_ALL` "Show All".
    ShowAll,
    /// `INTERACT_ICONS_SHOW_NONE` "Show None".
    ShowNone,
}

impl InteractKeyIcons {
    pub const ALL: [Self; 3] = [Self::Default, Self::ShowAll, Self::ShowNone];

    /// The Retail dropdown value.
    pub fn value(self) -> u8 {
        match self {
            Self::Default => 1,
            Self::ShowAll => 2,
            Self::ShowNone => 3,
        }
    }

    pub fn from_value(value: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|choice| choice.value() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "NPCs Only (Default)",
            Self::ShowAll => "Show All",
            Self::ShowNone => "Show None",
        }
    }
}
