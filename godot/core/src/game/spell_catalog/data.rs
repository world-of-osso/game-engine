//! Engine-free spell catalog data: spell rows, spellbook tab index and
//! description rendering, shared by the Bevy and Godot clients.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{build, cache, render, tabs};

pub use super::tabs::{PrimaryStat, SpecTabInfo, SpellbookTabIndex, SpellbookTabKind};

pub const SPELL_DB2_BUILD: &str = "12.1.0.69933";

/// Range in yards. Index 0 is the hostile range, index 1 the friendly range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpellRange {
    pub min_yd: [f32; 2],
    pub max_yd: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpellCooldown {
    pub recovery_ms: u32,
    pub category_recovery_ms: u32,
    /// Global cooldown triggered by the cast (`StartRecoveryTime`).
    pub gcd_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpellCharges {
    pub max_charges: u32,
    pub recovery_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpellPowerCost {
    /// Raw `SpellPower.PowerType` DB value (0 mana, 1 rage, 3 energy, ...).
    pub power_type: i8,
    pub flat: i32,
    pub pct: f32,
    /// Cost row only applies while this aura is active (0 = always).
    pub required_aura_spell_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogEffect {
    pub index: u8,
    pub effect: u16,
    pub aura: u16,
    pub base_points: f32,
    pub aura_period_ms: u32,
    /// Raw DB value; a few rows hold negative counts.
    pub chain_targets: i32,
    pub radius_yd: f32,
    /// `EffectBonusCoefficient` and `BonusCoefficientFromAP` when the points scale with
    /// the caster's spell and attack power (see `build::DAMAGE_OR_HEAL_EFFECTS`), else 0.
    pub spell_power_coefficient: f32,
    pub attack_power_coefficient: f32,
    /// Points scale with level (`ScalingClass` + `Coefficient`), from ExpectedStat data
    /// the client does not have.
    pub level_scaled: bool,
}

/// Whether casting the spell starts the caster's auto-attack on its target, a client
/// rule: the client sends the attack request (TrinityCore SharedDefines.h:482
/// `SPELL_ATTR1_INITIATES_COMBAT_ENABLES_AUTO_ATTACK` "(client only) Caster will begin
/// auto-attacking the target on cast", :530 `SPELL_ATTR2_INITIATE_COMBAT_POST_CAST_ENABLES_AUTO_ATTACK`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellAutoAttack {
    #[default]
    None,
    /// `SpellMisc.Attributes_1 & 0x200`: on cast (melee abilities such as Slam).
    OnCast,
    /// `SpellMisc.Attributes_2 & 0x100000` alone: once the cast completes (Smite).
    PostCast,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CatalogSpell {
    pub id: u32,
    pub name: Box<str>,
    pub subtext: Box<str>,
    /// Raw `Spell.Description_lang`; see [`SpellCatalog::render_description`].
    pub description: Box<str>,
    pub aura_description: Box<str>,
    /// Raw `SpellDescriptionVariables.Variables`: `$name=...` lines for `$<name>` tokens.
    pub description_variables: Box<str>,
    pub icon_fdid: u32,
    /// Raw DB value; one row holds -1.
    pub active_icon_fdid: i32,
    pub school_mask: u32,
    /// `SpellMisc.Attributes_0 & 0x40`.
    pub passive: bool,
    /// `SpellMisc.Attributes_0 & 0x80` (SPELL_ATTR0_DO_NOT_DISPLAY).
    pub hidden: bool,
    pub auto_attack: SpellAutoAttack,
    pub cast_time_ms: i32,
    pub range: SpellRange,
    /// `SpellDuration.Duration`; 0 = no duration, negative = until cancelled.
    pub duration_ms: i32,
    pub max_duration_ms: i32,
    pub cooldown: SpellCooldown,
    pub charges: Option<SpellCharges>,
    pub max_stacks: u32,
    /// Raw DB value; -1 on a few rows.
    pub proc_charges: i32,
    pub proc_chance: u32,
    pub powers: Box<[SpellPowerCost]>,
    pub effects: Box<[CatalogEffect]>,
    /// `SpellShapeshiftForm.BonusActionBar` of the form its MOD_SHAPESHIFT effect
    /// applies (Retail `GetBonusBarOffset` while the aura is up); 0 for none.
    pub bonus_bar: u8,
}

impl CatalogSpell {
    pub fn effect(&self, index: u8) -> Option<&CatalogEffect> {
        self.effects.iter().find(|effect| effect.index == index)
    }
}

/// Spells sorted by id.
#[derive(Debug, Default)]
pub struct SpellCatalogData {
    pub(crate) spells: Vec<CatalogSpell>,
    pub tabs: SpellbookTabIndex,
}

impl SpellCatalogData {
    fn from_sorted(mut spells: Vec<CatalogSpell>, tabs: SpellbookTabIndex) -> Self {
        // Deserialized Vecs grow by doubling; ~50 MB of slack otherwise.
        spells.shrink_to_fit();
        Self { spells, tabs }
    }

    /// Catalog from unsorted rows, for fixtures.
    pub fn from_parts(mut spells: Vec<CatalogSpell>, tabs: SpellbookTabIndex) -> Self {
        spells.sort_unstable_by_key(|spell| spell.id);
        Self::from_sorted(spells, tabs)
    }

    pub fn len(&self) -> usize {
        self.spells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<&CatalogSpell> {
        let index = self
            .spells
            .binary_search_by_key(&id, |spell| spell.id)
            .ok()?;
        Some(&self.spells[index])
    }

    pub fn render_description(&self, id: u32, ctx: &SpellTextContext) -> Option<String> {
        let spell = self.get(id)?;
        Some(render::render_spell_text(
            &spell.description,
            spell,
            self,
            ctx,
        ))
    }

    pub fn render_aura_description(&self, id: u32, ctx: &SpellTextContext) -> Option<String> {
        let spell = self.get(id)?;
        Some(render::render_spell_text(
            &spell.aura_description,
            spell,
            self,
            ctx,
        ))
    }
}

/// The viewing player's state that description tokens read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellTextContext {
    pub known_spells: Vec<u32>,
    /// Spell ids of the auras on the player.
    pub auras: Vec<u32>,
    /// Active `ChrSpecialization` id.
    pub spec_id: Option<u32>,
    /// The player's replicated spell and attack power; `None` before they arrive.
    pub caster_power: Option<CasterPower>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CasterPower {
    pub spell_power: f32,
    pub attack_power: f32,
}

pub struct SpellCatalogPaths {
    pub source_dir: PathBuf,
    pub cache_path: PathBuf,
}

impl SpellCatalogPaths {
    pub fn for_data_dir(data_dir: &Path) -> Self {
        Self {
            source_dir: data_dir.join("db2").join(SPELL_DB2_BUILD),
            cache_path: data_dir
                .join("cache")
                .join(format!("spell_catalog-{SPELL_DB2_BUILD}.bin")),
        }
    }
}

/// Loads from the cache when fresh, otherwise rebuilds from CSV and rewrites the cache.
/// The spellbook tab index is small and always rebuilt from CSV.
pub fn load_spell_catalog(paths: &SpellCatalogPaths) -> Result<SpellCatalogData, String> {
    let tabs = tabs::load_tab_index(&paths.source_dir)?;
    let key = cache::cache_key(&paths.source_dir)?;
    let spells = crate::db2_cache::load_or_build(&paths.cache_path, &key, || {
        build::build_spells(&paths.source_dir)
    })?;
    Ok(SpellCatalogData::from_sorted(spells, tabs))
}
