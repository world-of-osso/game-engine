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
    SpellRange, SpellbookListing,
};

/// Source CSVs, also the cache key inputs.
pub(super) const SOURCE_TABLES: &[&str] = &[
    "SpellName",
    "Spell",
    "SpellMisc",
    "SpellEffect",
    "SpellTargetRestrictions",
    "SpellPower",
    "SpellCastTimes",
    "SpellRange",
    "SpellDuration",
    "SpellRadius",
    "SpellCooldowns",
    "SpellCategories",
    "SpellCategory",
    "SpellAuraOptions",
    "SpellShapeshiftForm",
    "SpellDescriptionVariables",
    "SpellXDescriptionVariables",
];

type SpellMap = HashMap<u32, CatalogSpell>;

const SPELL_EFFECT_COLUMNS: &[&str] = &[
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
    "EffectMiscValue_0",
    "ImplicitTarget_0",
    "ImplicitTarget_1",
];

#[derive(Default)]
struct SpellEffectRows {
    effects: HashMap<u32, Vec<CatalogEffect>>,
    bonus_bars: HashMap<u32, u8>,
}

/// `SpellEffect.Effect` SCHOOL_DAMAGE and HEAL, and `EffectAura` PERIODIC_DAMAGE and
/// PERIODIC_HEAL: their points always add the power coefficient term. Other effects
/// (e.g. Bloodlust's haste, 30 points with a 0.25 coefficient) count as scaled only
/// when their stored points are 0.
const DAMAGE_OR_HEAL_EFFECTS: [u16; 2] = [2, 10];
const DAMAGE_OR_HEAL_AURAS: [u16; 2] = [3, 8];
/// INFERRED: TrinityCore a352b1fa `SpellInfo.cpp:256,264,279,282` maps these IDs to destination-area units.
const DESTINATION_AREA_TARGETS: [u16; 4] = [8, 16, 31, 34];
/// INFERRED: 12.1.5.70448 player-spell rows use Effects 3 and 32 for area radius on DEST_DEST.
const DESTINATION_AREA_EFFECTS: [u16; 2] = [3, 32];
const TARGET_DEST_DEST: u16 = 87;
/// `EffectAura` MOD_SHAPESHIFT; `EffectMiscValue_0` is the `SpellShapeshiftForm` id.
const AURA_MOD_SHAPESHIFT: u16 = 36;

/// `SpellMisc.Attributes_0` bit of passive spells.
const SPELL_ATTR0_PASSIVE: i64 = 0x40;
/// `SpellMisc.Attributes_0` SPELL_ATTR0_DO_NOT_DISPLAY: hidden in the spellbook,
/// aura icons and combat log.
const SPELL_ATTR0_DO_NOT_DISPLAY: i64 = 0x80;
/// `SpellMisc.Attributes_4` SPELL_ATTR4_NOT_IN_SPELLBOOK.
const SPELL_ATTR4_NOT_IN_SPELLBOOK: i64 = 0x8000;
/// `SpellMisc.Attributes_7` SPELL_ATTR7_ONLY_IN_SPELLBOOK_UNTIL_LEARNED.
const SPELL_ATTR7_ONLY_IN_SPELLBOOK_UNTIL_LEARNED: i64 = 0x0001_0000;
/// `SpellMisc.Attributes_8` SPELL_ATTR8_NOT_IN_SPELLBOOK_UNTIL_LEARNED.
const SPELL_ATTR8_NOT_IN_SPELLBOOK_UNTIL_LEARNED: i64 = 0x2000;
/// `SpellMisc.Attributes_1` SPELL_ATTR1_INITIATES_COMBAT_ENABLES_AUTO_ATTACK.
const SPELL_ATTR1_INITIATES_COMBAT_ENABLES_AUTO_ATTACK: i64 = 0x200;
/// `SpellMisc.Attributes_2` SPELL_ATTR2_INITIATE_COMBAT_POST_CAST_ENABLES_AUTO_ATTACK.
const SPELL_ATTR2_INITIATE_COMBAT_POST_CAST_ENABLES_AUTO_ATTACK: i64 = 0x0010_0000;

pub(super) fn build_spells(dir: &Path) -> Result<Vec<CatalogSpell>, String> {
    let mut spells = load_names(dir)?;
    apply_text(dir, &mut spells)?;
    apply_description_variables(dir, &mut spells)?;
    apply_misc(dir, &mut spells)?;
    apply_ground_target_restrictions(dir, &mut spells)?;
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

/// The `SpellDescriptionVariables.Variables` text a `SpellXDescriptionVariables` row
/// links to the spell: the `$name=...` definitions its `$<name>` tokens read. Build
/// 12.1.0.69933 links spell 66109 to row 1, which the client table does not have.
fn apply_description_variables(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    let variables = load_id_map(
        dir,
        "SpellDescriptionVariables",
        &["ID", "Variables"],
        |row| Ok(Box::<str>::from(row.text(1)?)),
    )?;
    let columns = ["SpellID", "SpellDescriptionVariablesID"];
    for_each_row(dir, "SpellXDescriptionVariables", &columns, |row| {
        let (spell_id, variables_id): (u32, u32) = (row.get(0)?, row.get(1)?);
        let Some(text) = variables.get(&variables_id) else {
            log::warn!("spell {spell_id} links missing SpellDescriptionVariables {variables_id}");
            return Ok(());
        };
        if let Some(spell) = spells.get_mut(&spell_id) {
            spell.description_variables = text.clone();
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
        "Attributes_4",
        "Attributes_7",
        "Attributes_8",
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
        spell.range_id = row.get(4)?;
        spell.range = ranges.get(&spell.range_id).copied().unwrap_or_default();
        spell.school_mask = row.get(5)?;
        spell.icon_fdid = row.get(6)?;
        spell.active_icon_fdid = row.get(7)?;
        let attributes = row.get::<i64>(8)?;
        spell.passive = attributes & SPELL_ATTR0_PASSIVE != 0;
        spell.spellbook = spellbook_listing(attributes, row.get(11)?, row.get(12)?, row.get(13)?);
        spell.auto_attack = auto_attack(row.get(9)?, row.get(10)?);
        Ok(())
    })
}

fn spellbook_listing(
    attributes_0: i64,
    attributes_4: i64,
    attributes_7: i64,
    attributes_8: i64,
) -> SpellbookListing {
    let until_learned = attributes_7 & SPELL_ATTR7_ONLY_IN_SPELLBOOK_UNTIL_LEARNED != 0;
    let once_learned = attributes_8 & SPELL_ATTR8_NOT_IN_SPELLBOOK_UNTIL_LEARNED != 0;
    if attributes_0 & SPELL_ATTR0_DO_NOT_DISPLAY != 0
        || attributes_4 & SPELL_ATTR4_NOT_IN_SPELLBOOK != 0
        || (until_learned && once_learned)
    {
        SpellbookListing::Never
    } else if until_learned {
        SpellbookListing::UntilLearned
    } else if once_learned {
        SpellbookListing::OnceLearned
    } else {
        SpellbookListing::Always
    }
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

fn ground_area_radius_yd(effect: u16, targets: [u16; 2], radii: [f32; 2]) -> Option<f32> {
    let destination_effect_has_area = DESTINATION_AREA_EFFECTS.contains(&effect);
    targets
        .into_iter()
        .zip(radii)
        .filter_map(|(target, radius)| {
            let target_selects_area = DESTINATION_AREA_TARGETS.contains(&target)
                || (destination_effect_has_area && target == TARGET_DEST_DEST);
            (target_selects_area && radius > 0.0).then_some(radius)
        })
        .reduce(f32::max)
}

fn apply_effects(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    let radii = load_id_map(dir, "SpellRadius", &["ID", "Radius"], |row| row.get(1))?;
    let form_bonus_bars = load_id_map(
        dir,
        "SpellShapeshiftForm",
        &["ID", "BonusActionBar"],
        |row| row.get::<u8>(1),
    )?;
    let mut parsed = SpellEffectRows::default();
    for_each_row(dir, "SpellEffect", SPELL_EFFECT_COLUMNS, |row| {
        if row.is_base_difficulty(1)? {
            parse_spell_effect(row, &radii, &form_bonus_bars, spells, &mut parsed)?;
        }
        Ok(())
    })?;
    assign_spell_effects(parsed.effects, spells);
    assign_bonus_bars(parsed.bonus_bars, spells);
    Ok(())
}

fn parse_spell_effect(
    row: &Row,
    radii: &HashMap<u32, f32>,
    form_bonus_bars: &HashMap<u32, u8>,
    spells: &mut SpellMap,
    parsed: &mut SpellEffectRows,
) -> Result<(), String> {
    let spell_id = row.get(0)?;
    let effect = row.get(3)?;
    let targets = [row.get(15)?, row.get(16)?];
    let effect_radii =
        [row.get(8)?, row.get(9)?].map(|index| radii.get(&index).copied().unwrap_or(0.0_f32));
    update_ground_target_radius(spells, spell_id, effect, targets, effect_radii);
    if row.get::<u16>(4)? == AURA_MOD_SHAPESHIFT
        && let Some(&bonus_bar) = form_bonus_bars.get(&row.get(14)?)
    {
        parsed.bonus_bars.insert(spell_id, bonus_bar);
    }
    parsed
        .effects
        .entry(spell_id)
        .or_default()
        .push(catalog_effect_from_row(row, effect, effect_radii)?);
    Ok(())
}

fn catalog_effect_from_row(
    row: &Row,
    effect: u16,
    effect_radii: [f32; 2],
) -> Result<CatalogEffect, String> {
    let aura = row.get::<u16>(4)?;
    let base_points: f32 = row.get(5)?;
    let (sp_coefficient, ap_coefficient): (f32, f32) = (row.get(10)?, row.get(11)?);
    let has_power_coefficient = sp_coefficient != 0.0 || ap_coefficient != 0.0;
    let damage_or_heal =
        DAMAGE_OR_HEAL_EFFECTS.contains(&effect) || DAMAGE_OR_HEAL_AURAS.contains(&aura);
    let (spell_power_coefficient, attack_power_coefficient) =
        if has_power_coefficient && (damage_or_heal || base_points == 0.0) {
            (sp_coefficient, ap_coefficient)
        } else {
            (0.0, 0.0)
        };
    let radius_yd = if effect_radii[0] > 0.0 {
        effect_radii[0]
    } else {
        effect_radii[1]
    };
    Ok(CatalogEffect {
        index: row.get(2)?,
        effect,
        aura,
        base_points,
        aura_period_ms: row.get(6)?,
        chain_targets: row.get(7)?,
        radius_yd,
        spell_power_coefficient,
        attack_power_coefficient,
        level_scaled: row.get::<i32>(12)? != 0 && row.get::<f32>(13)? != 0.0,
    })
}

fn update_ground_target_radius(
    spells: &mut SpellMap,
    spell_id: u32,
    effect: u16,
    targets: [u16; 2],
    radii: [f32; 2],
) {
    let Some(radius) = ground_area_radius_yd(effect, targets, radii) else {
        return;
    };
    let Some(spell) = spells
        .get_mut(&spell_id)
        .filter(|spell| spell.ground_targeted)
    else {
        return;
    };
    spell.ground_target_radius = Some(
        spell
            .ground_target_radius
            .map_or(radius, |current| current.max(radius)),
    );
}

fn assign_spell_effects(effects: HashMap<u32, Vec<CatalogEffect>>, spells: &mut SpellMap) {
    for (spell_id, mut list) in effects {
        if let Some(spell) = spells.get_mut(&spell_id) {
            list.sort_unstable_by_key(|effect| effect.index);
            spell.effects = list.into_boxed_slice();
        }
    }
}

fn assign_bonus_bars(bonus_bars: HashMap<u32, u8>, spells: &mut SpellMap) {
    for (spell_id, bonus_bar) in bonus_bars {
        if let Some(spell) = spells.get_mut(&spell_id) {
            spell.bonus_bar = bonus_bar;
        }
    }
}

/// INFERRED: `Targets & 0x40` is `TARGET_FLAG_DEST_LOCATION` (TrinityCore a352b1fa,
/// `SpellDefines.h:318`) and requires an explicit destination. `DEST_DEST` alone is not
/// enough, because area-trigger effects can derive their destination from the caster.
fn apply_ground_target_restrictions(dir: &Path, spells: &mut SpellMap) -> Result<(), String> {
    const TARGET_FLAG_DEST_LOCATION: u32 = 0x40;
    let columns = ["SpellID", "DifficultyID", "Targets"];
    for_each_row(dir, "SpellTargetRestrictions", &columns, |row| {
        if !row.is_base_difficulty(1)? {
            return Ok(());
        }
        if row.get::<u32>(2)? & TARGET_FLAG_DEST_LOCATION != 0
            && let Some(spell) = spells.get_mut(&row.get(0)?)
        {
            spell.ground_targeted = true;
        }
        Ok(())
    })
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
