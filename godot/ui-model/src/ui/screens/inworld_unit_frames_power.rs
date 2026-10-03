use shared::components::{PowerEntry, PowerType, UnitPowers};

use crate::status::power_display_modifier;

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

    /// The unit's `power`, as `UnitPower(unit, power)` reads it.
    pub fn of(powers: &UnitPowers, power: PowerType) -> Option<Self> {
        powers
            .entries
            .iter()
            .find(|entry| entry.power == power)
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

/// `PowerBarColor` (PowerBarColorUtil.lua:18-33); others take mana's colour as Retail
/// does for tokens without an entry (CompactUnitFrame.lua:780-786).
pub fn power_bar_rgb(power: PowerType) -> [f32; 3] {
    match power {
        PowerType::Rage => [1.0, 0.0, 0.0],
        PowerType::Focus => [1.0, 0.5, 0.25],
        PowerType::Energy => [1.0, 1.0, 0.0],
        PowerType::RunicPower => [0.0, 0.82, 1.0],
        PowerType::LunarPower => [0.30, 0.52, 0.90],
        PowerType::Maelstrom => [0.0, 0.5, 1.0],
        PowerType::Insanity => [0.40, 0.0, 0.80],
        PowerType::Fury => [0.788, 0.259, 0.992],
        PowerType::Pain => [1.0, 156.0 / 255.0, 0.0],
        _ => [0.0, 0.0, 1.0],
    }
}
