//! Unit power display rules (pip resources, display modifiers), engine-free so the
//! Godot unit frames share them.

use serde::{Deserialize, Serialize};
use shared::components::{PowerType, UnitPowers};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecondaryResourceKindEntry {
    ComboPoints,
    HolyPower,
    Chi,
    Essence,
    SoulShards,
    ArcaneCharges,
    Runes,
}

impl SecondaryResourceKindEntry {
    fn from_power(power: PowerType) -> Option<Self> {
        Some(match power {
            PowerType::ComboPoints => Self::ComboPoints,
            PowerType::HolyPower => Self::HolyPower,
            PowerType::Chi => Self::Chi,
            PowerType::Essence => Self::Essence,
            PowerType::SoulShards => Self::SoulShards,
            PowerType::ArcaneCharges => Self::ArcaneCharges,
            PowerType::Runes => Self::Runes,
            _ => return None,
        })
    }

    /// `ChrSpecialization` id a Retail bar's `spec` KeyValue requires: Arcane 62
    /// (`SPEC_MAGE_ARCANE`, MageArcaneChargesBar.xml:127) and Windwalker 269
    /// (`SPEC_MONK_WINDWALKER`, MonkHarmonyBar.xml:116). The other bars show for the
    /// whole class.
    fn required_spec(&self) -> Option<u32> {
        match self {
            Self::ArcaneCharges => Some(62),
            Self::Chi => Some(269),
            _ => None,
        }
    }
}

/// Raw `UnitPowers` units per displayed unit: Retail `PowerType.csv` `DisplayModifier`
/// (Rage, Runic Power, Soul Shards, Lunar Power, Pain ÷10; Insanity ÷100; others ÷1).
pub fn power_display_modifier(power: PowerType) -> i32 {
    match power {
        PowerType::Rage
        | PowerType::RunicPower
        | PowerType::SoulShards
        | PowerType::LunarPower
        | PowerType::Pain => 10,
        PowerType::Insanity => 100,
        _ => 1,
    }
}

/// Pip resource (combo points, holy power, ...) in whole displayed units.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecondaryResourceEntry {
    pub kind: SecondaryResourceKindEntry,
    pub current: u8,
    pub max: u8,
}

impl SecondaryResourceEntry {
    /// First pip-type power of `powers`; partial units (soul shard tenths) round down.
    pub fn from_unit_powers(powers: &UnitPowers) -> Option<Self> {
        powers.entries.iter().find_map(|entry| {
            let kind = SecondaryResourceKindEntry::from_power(entry.power)?;
            let modifier = power_display_modifier(entry.power);
            let whole = |raw: i32| (raw / modifier).clamp(0, u8::MAX as i32) as u8;
            Some(Self {
                kind,
                current: whole(entry.current),
                max: whole(entry.max),
            })
        })
    }

    /// Retail `ClassPowerBar:Setup` (ClassPowerBar.lua:82-83): shown when the bar has no
    /// spec requirement or `spec` is the required one.
    pub fn shown_for_spec(&self, spec: Option<u32>) -> bool {
        self.kind
            .required_spec()
            .is_none_or(|required| spec == Some(required))
    }
}
