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
