use shared::components::{PowerEntry, PowerType, UnitPowers};

use crate::status::{SecondaryResourceKindEntry, power_display_modifier};

/// Primary power bar in displayed units.
#[derive(Clone, Debug, PartialEq)]
pub struct PowerBarState {
    pub power: PowerType,
    pub current: i32,
    pub max: i32,
}

impl PowerBarState {
    /// The unit's first bar-type power; pip resources are shown as `SecondaryResourceEntry`.
    pub fn primary(powers: &UnitPowers) -> Option<Self> {
        powers
            .entries
            .iter()
            .find(|entry| !is_pip_power(entry.power))
            .map(Self::from_entry)
    }

    fn from_entry(entry: &PowerEntry) -> Self {
        let modifier = power_display_modifier(entry.power);
        Self {
            power: entry.power,
            current: entry.current / modifier,
            max: entry.max / modifier,
        }
    }

    pub fn text(&self) -> String {
        format!("{} / {}", self.current, self.max)
    }
}

fn is_pip_power(power: PowerType) -> bool {
    matches!(
        power,
        PowerType::ComboPoints
            | PowerType::HolyPower
            | PowerType::Chi
            | PowerType::Essence
            | PowerType::SoulShards
            | PowerType::ArcaneCharges
            | PowerType::Runes
    )
}

/// Retail `PowerBarColor` (Blizzard_UnitFrame `UnitFrame.lua`).
pub fn power_bar_color(power: PowerType) -> &'static str {
    match power {
        PowerType::Mana => "0.0,0.0,1.0,1.0",
        PowerType::Rage => "1.0,0.0,0.0,1.0",
        PowerType::Focus => "1.0,0.5,0.25,1.0",
        PowerType::Energy => "1.0,1.0,0.0,1.0",
        PowerType::RunicPower => "0.0,0.82,1.0,1.0",
        PowerType::LunarPower => "0.3,0.52,0.9,1.0",
        PowerType::Maelstrom => "0.0,0.5,1.0,1.0",
        PowerType::Insanity => "0.4,0.0,0.8,1.0",
        PowerType::Fury => "0.788,0.259,0.992,1.0",
        PowerType::Pain => "1.0,0.612,0.0,1.0",
        _ => "0.5,0.5,0.5,1.0",
    }
}

/// Lit/unlit pip colours; lit follows `PowerBarColor` where Retail defines one.
pub(super) fn pip_color(kind: &SecondaryResourceKindEntry, lit: bool) -> &'static str {
    match (kind, lit) {
        (SecondaryResourceKindEntry::ComboPoints, true) => "1.0,0.96,0.41,1.0",
        (SecondaryResourceKindEntry::HolyPower, true) => "0.95,0.9,0.6,1.0",
        (SecondaryResourceKindEntry::Chi, true) => "0.71,1.0,0.92,1.0",
        (SecondaryResourceKindEntry::SoulShards, true) => "0.5,0.32,0.55,1.0",
        (SecondaryResourceKindEntry::ArcaneCharges, true) => "0.1,0.1,0.98,1.0",
        (SecondaryResourceKindEntry::Runes, true) => "0.5,0.5,0.5,1.0",
        (SecondaryResourceKindEntry::Essence, true) => "0.24,0.78,1.0,1.0",
        (_, false) => "0.10,0.10,0.11,0.95",
    }
}
