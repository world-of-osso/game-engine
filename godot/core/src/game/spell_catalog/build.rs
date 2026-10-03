//! Joins the spell DB2 CSV exports into [`CatalogSpell`] rows.
//!
//! Per-difficulty tables contribute only their `DifficultyID = 0` row.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use super::csv_records::CsvTable;
use super::{
    CatalogEffect, CatalogSpell, SpellAutoAttack, SpellCharges, SpellCooldown, SpellPowerCost,
    SpellRange,
};

/// Source CSVs, also the cache key inputs.
pub(super) const SOURCE_TABLES: &[&str] = &[
    "SpellName",
    "Spell",
    "SpellMisc",
    "SpellEffect",
    "SpellPower",
    "SpellCastTimes",
    "SpellRange",
    "SpellDuration",
    "SpellRadius",
    "SpellCooldowns",
    "SpellCategories",
    "SpellCategory",
    "SpellAuraOptions",
];

type SpellMap = HashMap<u32, CatalogSpell>;

/// `SpellEffect.Effect` SCHOOL_DAMAGE and HEAL, and `EffectAura` PERIODIC_DAMAGE and
/// PERIODIC_HEAL: their points always add the power coefficient term. Other effects
/// (e.g. Bloodlust's haste, 30 points with a 0.25 coefficient) count as scaled only
/// when their stored points are 0.
const DAMAGE_OR_HEAL_EFFECTS: [u16; 2] = [2, 10];
const DAMAGE_OR_HEAL_AURAS: [u16; 2] = [3, 8];

/// `SpellMisc.Attributes_0` bit of passive spells.
const SPELL_ATTR0_PASSIVE: i64 = 0x40;
/// `SpellMisc.Attributes_0` SPELL_ATTR0_DO_NOT_DISPLAY: hidden in the spellbook,
/// aura icons and combat log.
const SPELL_ATTR0_DO_NOT_DISPLAY: i64 = 0x80;
/// `SpellMisc.Attributes_1` SPELL_ATTR1_INITIATES_COMBAT_ENABLES_AUTO_ATTACK.
const SPELL_ATTR1_INITIATES_COMBAT_ENABLES_AUTO_ATTACK: i64 = 0x200;
/// `SpellMisc.Attributes_2` SPELL_ATTR2_INITIATE_COMBAT_POST_CAST_ENABLES_AUTO_ATTACK.
const SPELL_ATTR2_INITIATE_COMBAT_POST_CAST_ENABLES_AUTO_ATTACK: i64 = 0x0010_0000;

pub(super) fn build_spells(dir: &Path) -> Result<Vec<CatalogSpell>, String> {
    let mut spells = load_names(dir)?;
    apply_text(dir, &mut spells)?;
    apply_misc(dir, &mut spells)?;
    apply_effects(dir, &mut spells)?;
    apply_powers(dir, &mut spells)?;
    apply_cooldowns(dir, &mut spells)?;
    apply_charges(dir, &mut spells)?;
    apply_aura_options(dir, &mut spells)?;
    let mut sorted: Vec<CatalogSpell> = spells.into_values().collect();
    sorted.sort_unstable_by_key(|spell| spell.id);
    Ok(sorted)
}

struct Row<'r> {
    table: &'r CsvTable,
    record: &'r [Cow<'r, str>],
    columns: &'r [usize],
}

impl Row<'_> {
    fn text(&self, column: usize) -> Result<&str, String> {
        self.record
            .get(self.columns[column])
            .map(|field| field.as_ref())
            .ok_or_else(|| format!("{} has a short record", self.table.path().display()))
    }

    fn get<T: FromStr>(&self, column: usize) -> Result<T, String> {
        let raw = self.text(column)?;
        raw.parse().map_err(|_| {
            format!(
                "{} column {}: bad value {raw:?}",
                self.table.path().display(),
                self.columns[column]
            )
        })
    }

    fn is_base_difficulty(&self, column: usize) -> Result<bool, String> {
        Ok(self.get::<u32>(column)? == 0)
    }
}

fn for_each_row(
    dir: &Path,
    table: &str,
    columns: &[&str],
    mut visit: impl FnMut(&Row) -> Result<(), String>,
) -> Result<(), String> {
    let table = CsvTable::read(&dir.join(format!("{table}.csv")))?;
    let columns = columns
        .iter()
        .map(|name| table.column(name))
        .collect::<Result<Vec<_>, _>>()?;
    for record in table.records() {
        visit(&Row {
            table: &table,
            record: &record,
            columns: &columns,
        })?;
    }
    Ok(())
}

fn load_names(dir: &Path) -> Result<SpellMap, String> {
    let mut spells = SpellMap::new();
    for_each_row(dir, "SpellName", &["ID", "Name_lang"], |row| {
        let id = row.get(0)?;
        let name = row.text(1)?.into();
        spells.insert(
            id,
            CatalogSpell {
                id,
                name,
                ..Default::default()
            },
        );
        Ok(())
    })?;
    Ok(spells)
}

fn apply_text(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    let columns = [
        "ID",
        "NameSubtext_lang",
        "Description_lang",
        "AuraDescription_lang",
    ];
    for_each_row(dir, "Spell", &columns, |row| {
        if let Some(spell) = spells.get_mut(&row.get(0)?) {
            spell.subtext = row.text(1)?.into();
            spell.description = row.text(2)?.into();
            spell.aura_description = row.text(3)?.into();
        }
        Ok(())
    })
}

fn load_id_map<T>(
    dir: &Path,
    table: &str,
    columns: &[&str],
    value: impl Fn(&Row) -> Result<T, String>,
) -> Result<HashMap<u32, T>, String> {
    let mut map = HashMap::new();
    for_each_row(dir, table, columns, |row| {
        map.insert(row.get(0)?, value(row)?);
        Ok(())
    })?;
    Ok(map)
}

fn apply_misc(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    let cast_times = load_id_map(dir, "SpellCastTimes", &["ID", "Base"], |row| row.get(1))?;
    let durations = load_id_map(
        dir,
        "SpellDuration",
        &["ID", "Duration", "MaxDuration"],
        |row| Ok((row.get(1)?, row.get(2)?)),
    )?;
    let range_columns = ["ID", "RangeMin_0", "RangeMin_1", "RangeMax_0", "RangeMax_1"];
    let ranges = load_id_map(dir, "SpellRange", &range_columns, |row| {
        Ok(SpellRange {
            min_yd: [row.get(1)?, row.get(2)?],
            max_yd: [row.get(3)?, row.get(4)?],
        })
    })?;
    let columns = [
        "SpellID",
        "DifficultyID",
        "CastingTimeIndex",
        "DurationIndex",
        "RangeIndex",
        "SchoolMask",
        "SpellIconFileDataID",
        "ActiveIconFileDataID",
        "Attributes_0",
        "Attributes_1",
        "Attributes_2",
    ];
    for_each_row(dir, "SpellMisc", &columns, |row| {
        if !row.is_base_difficulty(1)? {
            return Ok(());
        }
        let Some(spell) = spells.get_mut(&row.get(0)?) else {
            return Ok(());
        };
        spell.cast_time_ms = cast_times.get(&row.get(2)?).copied().unwrap_or(0);
        (spell.duration_ms, spell.max_duration_ms) =
            durations.get(&row.get(3)?).copied().unwrap_or((0, 0));
        spell.range = ranges.get(&row.get(4)?).copied().unwrap_or_default();
        spell.school_mask = row.get(5)?;
        spell.icon_fdid = row.get(6)?;
        spell.active_icon_fdid = row.get(7)?;
        let attributes = row.get::<i64>(8)?;
        spell.passive = attributes & SPELL_ATTR0_PASSIVE != 0;
        spell.hidden = attributes & SPELL_ATTR0_DO_NOT_DISPLAY != 0;
        spell.auto_attack = auto_attack(row.get(9)?, row.get(10)?);
        Ok(())
    })
}

fn auto_attack(attributes_1: i64, attributes_2: i64) -> SpellAutoAttack {
    if attributes_1 & SPELL_ATTR1_INITIATES_COMBAT_ENABLES_AUTO_ATTACK != 0 {
        SpellAutoAttack::OnCast
    } else if attributes_2 & SPELL_ATTR2_INITIATE_COMBAT_POST_CAST_ENABLES_AUTO_ATTACK != 0 {
        SpellAutoAttack::PostCast
    } else {
        SpellAutoAttack::None
    }
}

fn apply_effects(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    let radii = load_id_map(dir, "SpellRadius", &["ID", "Radius"], |row| row.get(1))?;
    let radius = |index: u32| radii.get(&index).copied().unwrap_or(0.0_f32);
    let columns = [
        "SpellID",
        "DifficultyID",
        "EffectIndex",
        "Effect",
        "EffectAura",
        "EffectBasePointsF",
        "EffectAuraPeriod",
        "EffectChainTargets",
        "EffectRadiusIndex_0",
        "EffectRadiusIndex_1",
        "EffectBonusCoefficient",
        "BonusCoefficientFromAP",
        "ScalingClass",
        "Coefficient",
    ];
    let mut effects: HashMap<u32, Vec<CatalogEffect>> = HashMap::new();
    for_each_row(dir, "SpellEffect", &columns, |row| {
        if !row.is_base_difficulty(1)? {
            return Ok(());
        }
        let primary_radius = radius(row.get(8)?);
        let radius_yd = if primary_radius > 0.0 {
            primary_radius
        } else {
            radius(row.get(9)?)
        };
        let base_points: f32 = row.get(5)?;
        let (sp_coefficient, ap_coefficient): (f32, f32) = (row.get(10)?, row.get(11)?);
        let has_power_coefficient = sp_coefficient != 0.0 || ap_coefficient != 0.0;
        let damage_or_heal = DAMAGE_OR_HEAL_EFFECTS.contains(&row.get(3)?)
            || DAMAGE_OR_HEAL_AURAS.contains(&row.get(4)?);
        let spell_power_scaled = has_power_coefficient && (damage_or_heal || base_points == 0.0);
        let level_scaled = row.get::<i32>(12)? != 0 && row.get::<f32>(13)? != 0.0;
        effects.entry(row.get(0)?).or_default().push(CatalogEffect {
            index: row.get(2)?,
            effect: row.get(3)?,
            aura: row.get(4)?,
            base_points,
            aura_period_ms: row.get(6)?,
            chain_targets: row.get(7)?,
            radius_yd,
            spell_power_coefficient: if spell_power_scaled {
                sp_coefficient
            } else {
                0.0
            },
            attack_power_coefficient: if spell_power_scaled {
                ap_coefficient
            } else {
                0.0
            },
            level_scaled,
        });
        Ok(())
    })?;
    for (id, mut list) in effects {
        if let Some(spell) = spells.get_mut(&id) {
            list.sort_unstable_by_key(|effect| effect.index);
            spell.effects = list.into_boxed_slice();
        }
    }
    Ok(())
}

fn apply_powers(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    let columns = [
        "SpellID",
        "OrderIndex",
        "PowerType",
        "ManaCost",
        "PowerCostPct",
        "RequiredAuraSpellID",
    ];
    let mut powers: HashMap<u32, Vec<(u32, SpellPowerCost)>> = HashMap::new();
    for_each_row(dir, "SpellPower", &columns, |row| {
        let cost = SpellPowerCost {
            power_type: row.get(2)?,
            flat: row.get(3)?,
            pct: row.get(4)?,
            required_aura_spell_id: row.get(5)?,
        };
        powers
            .entry(row.get(0)?)
            .or_default()
            .push((row.get(1)?, cost));
        Ok(())
    })?;
    for (id, mut list) in powers {
        if let Some(spell) = spells.get_mut(&id) {
            list.sort_by_key(|(order, _)| *order);
            spell.powers = list.into_iter().map(|(_, cost)| cost).collect();
        }
    }
    Ok(())
}

fn apply_cooldowns(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    let columns = [
        "SpellID",
        "DifficultyID",
        "RecoveryTime",
        "CategoryRecoveryTime",
        "StartRecoveryTime",
    ];
    for_each_row(dir, "SpellCooldowns", &columns, |row| {
        if !row.is_base_difficulty(1)? {
            return Ok(());
        }
        if let Some(spell) = spells.get_mut(&row.get(0)?) {
            spell.cooldown = SpellCooldown {
                recovery_ms: row.get(2)?,
                category_recovery_ms: row.get(3)?,
                gcd_ms: row.get(4)?,
            };
        }
        Ok(())
    })
}

fn apply_charges(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    let category_columns = ["ID", "MaxCharges", "ChargeRecoveryTime"];
    let categories = load_id_map(dir, "SpellCategory", &category_columns, |row| {
        Ok(SpellCharges {
            max_charges: row.get(1)?,
            recovery_ms: row.get(2)?,
        })
    })?;
    let columns = ["SpellID", "DifficultyID", "ChargeCategory"];
    for_each_row(dir, "SpellCategories", &columns, |row| {
        if !row.is_base_difficulty(1)? {
            return Ok(());
        }
        let Some(spell) = spells.get_mut(&row.get(0)?) else {
            return Ok(());
        };
        spell.charges = categories
            .get(&row.get(2)?)
            .copied()
            .filter(|charges| charges.max_charges > 0);
        Ok(())
    })
}

fn apply_aura_options(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    let columns = [
        "SpellID",
        "DifficultyID",
        "CumulativeAura",
        "ProcChance",
        "ProcCharges",
    ];
    for_each_row(dir, "SpellAuraOptions", &columns, |row| {
        if !row.is_base_difficulty(1)? {
            return Ok(());
        }
        if let Some(spell) = spells.get_mut(&row.get(0)?) {
            spell.max_stacks = row.get(2)?;
            spell.proc_chance = row.get(3)?;
            spell.proc_charges = row.get(4)?;
        }
        Ok(())
    })
}
