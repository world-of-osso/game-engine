//! Unit power display rules (pip resources, display modifiers), engine-free so the
//! Godot unit frames share them.

use serde::{Deserialize, Serialize};
use shared::components::{PowerType, UnitPowers, UnitRunes};

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
}

/// The Retail player class resource bar a class has (`class` KeyValue of each bar).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClassBar {
    /// `RogueComboPointBarFrame` (RogueComboPointBar.xml:228-249).
    RogueComboPoints,
    /// `DruidComboPointBarFrame` (DruidComboPointBar.xml:91-113).
    DruidComboPoints,
    /// `MonkHarmonyBarFrame` (MonkHarmonyBar.xml:104-129).
    Chi,
    /// `WarlockPowerFrame` (ShardBar.xml:150-173).
    SoulShards,
    /// `EssencePlayerFrame` (EssenceFramePlayer.xml:280-304).
    Essence,
    /// `RuneFrame` (RuneFrame.xml:186-243).
    Runes,
    /// `PaladinPowerBarFrame` (PaladinPowerBar.xml:98-241).
    HolyPower,
    /// `MageArcaneChargesFrame` (MageArcaneChargesBar.xml:115-136).
    ArcaneCharges,
}

/// `ChrClasses` ids.
const CLASS_PALADIN: u8 = 2;
const CLASS_ROGUE: u8 = 4;
const CLASS_DEATH_KNIGHT: u8 = 6;
const CLASS_MAGE: u8 = 8;
const CLASS_WARLOCK: u8 = 9;
const CLASS_MONK: u8 = 10;
const CLASS_DRUID: u8 = 11;
const CLASS_EVOKER: u8 = 13;

impl ClassBar {
    pub fn for_class(class: u8) -> Option<Self> {
        Some(match class {
            CLASS_ROGUE => Self::RogueComboPoints,
            CLASS_DRUID => Self::DruidComboPoints,
            CLASS_MONK => Self::Chi,
            CLASS_WARLOCK => Self::SoulShards,
            CLASS_EVOKER => Self::Essence,
            CLASS_DEATH_KNIGHT => Self::Runes,
            CLASS_PALADIN => Self::HolyPower,
            CLASS_MAGE => Self::ArcaneCharges,
            _ => return None,
        })
    }

    /// The bar's `powerType` KeyValue.
    pub fn power(self) -> PowerType {
        match self {
            Self::RogueComboPoints | Self::DruidComboPoints => PowerType::ComboPoints,
            Self::Chi => PowerType::Chi,
            Self::SoulShards => PowerType::SoulShards,
            Self::Essence => PowerType::Essence,
            Self::Runes => PowerType::Runes,
            Self::HolyPower => PowerType::HolyPower,
            Self::ArcaneCharges => PowerType::ArcaneCharges,
        }
    }

    /// `spec` KeyValue: Windwalker 269 (MonkHarmonyBar.xml:116), Arcane 62
    /// (MageArcaneChargesBar.xml:127).
    fn required_spec(self) -> Option<u32> {
        match self {
            Self::Chi => Some(269),
            Self::ArcaneCharges => Some(62),
            _ => None,
        }
    }

    /// `requiredShownLevel` KeyValue: soul shards from 10 (ShardBar.xml:161).
    fn required_level(self) -> u8 {
        match self {
            Self::SoulShards => 10,
            _ => 0,
        }
    }

    /// Retail `ClassPowerBar:Setup` spec gate (ClassPowerBar.lua:82-83), the druid
    /// `shouldShowBarFunc` cat-form gate (DruidComboPointBar.lua:3-12: display power
    /// Energy) and `requiredShownLevel` (ClassResourceBarTemplate.lua:101-105).
    fn shown(self, player: &ClassBarPlayer, primary: Option<PowerType>) -> bool {
        let spec = self
            .required_spec()
            .is_none_or(|required| player.spec == Some(required));
        let form = self != Self::DruidComboPoints || primary == Some(PowerType::Energy);
        spec && form && player.level >= self.required_level()
    }
}

/// What gates the player's class bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassBarPlayer {
    pub class: u8,
    pub spec: Option<u32>,
    pub level: u8,
    pub in_combat: bool,
}

/// Received resource timing and charged-point state. Times use the animator's monotonic
/// seconds clock; `partial` uses thousandths and charged indices are 1-based.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassBarDynamics {
    pub partial: u16,
    pub regen_per_sec: f32,
    pub received_at: f64,
    pub charged_points: Vec<u8>,
    pub runes: Option<ClassBarRunes>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClassBarRunes {
    pub duration_ms: u32,
    pub ready_in_ms: Vec<u32>,
}

/// The player's shown class bar and its power.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassBarResource {
    pub bar: ClassBar,
    pub current: u8,
    pub max: u8,
    /// Power in tenths of a displayed unit (soul shard fragments).
    pub tenths: u16,
    pub dynamics: ClassBarDynamics,
    pub spec: Option<u32>,
    pub in_combat: bool,
}

impl ClassBarResource {
    /// The class's bar when Retail shows it, with its power from `powers`.
    /// Received timing: `partial` (thousandths) and the regen rate in displayed units per
    /// second from the bar's `PowerEntry`, the 1-based charged points, and, for runes, each
    /// rune's `UnitRunes` recharge. `received_at` is stamped by the animator.
    pub fn for_player(
        powers: &UnitPowers,
        runes: Option<&UnitRunes>,
        player: &ClassBarPlayer,
    ) -> Option<Self> {
        let bar = ClassBar::for_class(player.class)?;
        let primary = powers.entries.first().map(|entry| entry.power);
        if !bar.shown(player, primary) {
            return None;
        }
        let entry = powers
            .entries
            .iter()
            .find(|entry| entry.power == bar.power())?;
        let modifier = power_display_modifier(entry.power);
        let whole = |raw: i32| (raw / modifier).clamp(0, u8::MAX as i32) as u8;
        Some(Self {
            bar,
            current: whole(entry.current),
            max: whole(entry.max),
            tenths: (entry.current * 10 / modifier).clamp(0, u16::MAX as i32) as u16,
            dynamics: ClassBarDynamics {
                partial: entry.partial,
                regen_per_sec: entry.regen_per_sec / modifier as f32,
                received_at: 0.0,
                charged_points: powers.charged_points.clone(),
                runes: runes
                    .filter(|_| bar == ClassBar::Runes)
                    .map(|runes| ClassBarRunes {
                        duration_ms: runes.duration_ms,
                        ready_in_ms: runes.ready_in_ms.clone(),
                    }),
            },
            spec: player.spec,
            in_combat: player.in_combat,
        })
    }
}
