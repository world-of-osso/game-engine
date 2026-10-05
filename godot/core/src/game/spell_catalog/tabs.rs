//! Spellbook tab grouping from ChrClasses, ChrSpecialization, SpecializationSpells,
//! SkillLine and SkillLineAbility.
//!
//! Rule: a spell of the active non-Initial spec (its `SpecializationSpells` rows or
//! mastery) goes on the spec tab; else a spell of any class skill line (`SkillLine`
//! category 7 named like a `ChrClasses` row) goes on the class tab; else General.
//!
//! Future spells: the class line's auto-learned `SkillLineAbility` rows
//! (AcquireMethod 1, 2, 4, as the server grants them) and the active spec's
//! `SpecializationSpells`, each with its `SpellLevels.SpellLevel` (DifficultyID 0).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::csv_records::CsvTable;

/// `SkillLine.CategoryID` of class (and pet/mount) skill lines.
const SKILL_CATEGORY_CLASS: u32 = 7;
/// `ChrSpecialization.OrderIndex` of the pre-level-10 "Initial" spec.
const INITIAL_SPEC_ORDER: u32 = 4;
/// SkillLineAbilityAcquireMethod AutomaticSkillRank, AutomaticCharLevel and
/// LearnedOrAutomaticCharLevel (TrinityCore `DBCEnums.h`).
const AUTO_LEARN_METHODS: [u32; 3] = [1, 2, 4];

/// An auto-learned class spell and the level it is learned at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeveledSpell {
    pub spell_id: u32,
    /// `SpellLevels.SpellLevel`; 0 without a row.
    pub level: u32,
    /// `SkillLineAbility.RaceMasks_0/1`; all zero allows every race.
    pub race_masks: [u32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellbookTabKind {
    General,
    Class,
    Spec,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpecTabInfo {
    pub name: String,
    pub class_id: u32,
    /// `ChrSpecialization.OrderIndex`: 0 for the class's first spec.
    pub order_index: u32,
    pub initial: bool,
    pub spells: HashSet<u32>,
    /// `ChrSpecialization.PrimaryStatPriority`; see [`SpecTabInfo::primary_stat`].
    pub primary_stat_priority: u32,
    /// `ChrSpecialization.SpellIconFileID`.
    pub icon_fdid: u32,
    /// `ChrSpecialization.Role`: tank 0, healer 1, damage 2.
    pub role: u32,
    pub description: String,
}

/// The stat a specialization's gear and character sheet favour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimaryStat {
    Strength,
    Agility,
    Intellect,
}

impl SpecTabInfo {
    /// `Player::GetPrimaryStat` (TrinityCore a352b1fa StatSystem.cpp:297-314):
    /// `PrimaryStatPriority` 4 and up Strength, 2-3 Agility, else Intellect.
    pub fn primary_stat(&self) -> PrimaryStat {
        match self.primary_stat_priority {
            4.. => PrimaryStat::Strength,
            2 | 3 => PrimaryStat::Agility,
            _ => PrimaryStat::Intellect,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellbookTabIndex {
    pub class_names: HashMap<u32, String>,
    pub specs: HashMap<u32, SpecTabInfo>,
    /// Spell id → class id of the class skill line teaching it.
    pub class_spells: HashMap<u32, u32>,
    /// Class id → its auto-learned class-line spells, by level then spell id.
    pub class_progression: HashMap<u32, Vec<LeveledSpell>>,
    /// `SpellLevels.SpellLevel` of spec spells.
    pub spell_levels: HashMap<u32, u32>,
    /// `ChrRaces.PlayableRaceBit` by race id.
    pub race_bits: HashMap<u32, u32>,
}

impl SpellbookTabIndex {
    pub fn classify(&self, spell_id: u32, spec_id: Option<u32>) -> SpellbookTabKind {
        let spec = spec_id.and_then(|id| self.specs.get(&id));
        if let Some(spec) = spec.filter(|spec| spec.spells.contains(&spell_id)) {
            return if spec.initial {
                SpellbookTabKind::Class
            } else {
                SpellbookTabKind::Spec
            };
        }
        if self.class_spells.contains_key(&spell_id) {
            return SpellbookTabKind::Class;
        }
        SpellbookTabKind::General
    }

    /// Class tab title: the spec's class, else the class of the first class spell.
    pub fn class_name(&self, spec_id: Option<u32>, known: &[u32]) -> Option<&str> {
        let class_id = spec_id
            .and_then(|id| self.specs.get(&id))
            .map(|spec| spec.class_id)
            .or_else(|| {
                known
                    .iter()
                    .find_map(|id| self.class_spells.get(id).copied())
            })?;
        self.class_names.get(&class_id).map(String::as_str)
    }

    /// Spells the player learns later: the class's auto-learned spells and the spec's
    /// spells above `level` that the race may learn, by level.
    pub fn future_spells(
        &self,
        class_id: u32,
        race_id: u32,
        spec_id: Option<u32>,
        level: u32,
    ) -> Vec<LeveledSpell> {
        let race_allows = |masks: [u32; 2]| {
            masks == [0, 0]
                || self.race_bits.get(&race_id).is_some_and(|&bit| {
                    let mask = u64::from(masks[0]) | (u64::from(masks[1]) << 32);
                    bit < 64 && mask & (1 << bit) != 0
                })
        };
        let class = self.class_progression.get(&class_id).into_iter().flatten();
        let spec = spec_id
            .and_then(|id| self.specs.get(&id))
            .into_iter()
            .flat_map(|spec| &spec.spells)
            .map(|&spell_id| LeveledSpell {
                spell_id,
                level: self.spell_levels.get(&spell_id).copied().unwrap_or(0),
                race_masks: [0, 0],
            });
        let mut future: Vec<LeveledSpell> = class
            .copied()
            .chain(spec)
            .filter(|spell| spell.level > level && race_allows(spell.race_masks))
            .collect();
        future.sort_by_key(|spell| (spell.level, spell.spell_id));
        future.dedup_by_key(|spell| spell.spell_id);
        future
    }

    /// Spec tab title; `None` for the Initial spec, which has no tab.
    pub fn spec_name(&self, spec_id: Option<u32>) -> Option<&str> {
        let spec = self.specs.get(&spec_id?)?;
        (!spec.initial).then_some(spec.name.as_str())
    }
}

pub(super) fn load_tab_index(dir: &Path) -> Result<SpellbookTabIndex, String> {
    let class_names = load_class_names(dir)?;
    let mut specs = load_specs(dir)?;
    add_spec_spells(dir, &mut specs)?;
    let (class_spells, class_abilities) = load_class_spells(dir, &class_names)?;
    let spell_levels = load_spell_levels(dir)?;
    let level_of = |spell_id: &u32| spell_levels.get(spell_id).copied().unwrap_or(0);
    let mut class_progression: HashMap<u32, Vec<LeveledSpell>> = HashMap::new();
    for ability in class_abilities {
        class_progression
            .entry(ability.class_id)
            .or_default()
            .push(LeveledSpell {
                spell_id: ability.spell_id,
                level: level_of(&ability.spell_id),
                race_masks: ability.race_masks,
            });
    }
    for spells in class_progression.values_mut() {
        spells.sort_by_key(|spell| (spell.level, spell.spell_id));
        spells.dedup_by_key(|spell| spell.spell_id);
    }
    let spec_levels = specs
        .values()
        .flat_map(|spec| &spec.spells)
        .map(|spell_id| (*spell_id, level_of(spell_id)))
        .collect();
    Ok(SpellbookTabIndex {
        class_names,
        specs,
        class_spells,
        class_progression,
        spell_levels: spec_levels,
        race_bits: load_race_bits(dir)?,
    })
}

/// An auto-learned `SkillLineAbility` row of a class line.
struct ClassAbility {
    class_id: u32,
    spell_id: u32,
    race_masks: [u32; 2],
}

fn load_spell_levels(dir: &Path) -> Result<HashMap<u32, u32>, String> {
    let mut levels = HashMap::new();
    let columns = ["SpellID", "DifficultyID", "SpellLevel"];
    for_each_record(dir, "SpellLevels", &columns, |row| {
        let table = "SpellLevels";
        if parse_u32(table, row[1])? == 0 {
            levels.insert(parse_u32(table, row[0])?, parse_u32(table, row[2])?);
        }
        Ok(())
    })?;
    Ok(levels)
}

fn load_race_bits(dir: &Path) -> Result<HashMap<u32, u32>, String> {
    let mut bits = HashMap::new();
    for_each_record(dir, "ChrRaces", &["ID", "PlayableRaceBit"], |row| {
        let bit: i64 = row[1]
            .parse()
            .map_err(|_| format!("ChrRaces: bad integer {:?}", row[1]))?;
        if bit >= 0 {
            bits.insert(parse_u32("ChrRaces", row[0])?, bit as u32);
        }
        Ok(())
    })?;
    Ok(bits)
}

fn for_each_record(
    dir: &Path,
    table: &str,
    columns: &[&str],
    mut visit: impl FnMut(&[&str]) -> Result<(), String>,
) -> Result<(), String> {
    let table = CsvTable::read(&dir.join(format!("{table}.csv")))?;
    let indices = columns
        .iter()
        .map(|name| table.column(name))
        .collect::<Result<Vec<_>, _>>()?;
    for record in table.records() {
        let fields = indices
            .iter()
            .map(|&index| record.get(index).map(|field| field.as_ref()))
            .collect::<Option<Vec<&str>>>()
            .ok_or_else(|| format!("{} has a short record", table.path().display()))?;
        visit(&fields)?;
    }
    Ok(())
}

fn parse_u32(table: &str, raw: &str) -> Result<u32, String> {
    raw.parse()
        .map_err(|_| format!("{table}: bad integer {raw:?}"))
}

fn load_class_names(dir: &Path) -> Result<HashMap<u32, String>, String> {
    let mut names = HashMap::new();
    for_each_record(dir, "ChrClasses", &["ID", "Name_lang"], |row| {
        names.insert(parse_u32("ChrClasses", row[0])?, row[1].to_string());
        Ok(())
    })?;
    Ok(names)
}

fn load_specs(dir: &Path) -> Result<HashMap<u32, SpecTabInfo>, String> {
    let columns = [
        "ID",
        "Name_lang",
        "ClassID",
        "OrderIndex",
        "MasterySpellID_0",
        "MasterySpellID_1",
        "PrimaryStatPriority",
        "SpellIconFileID",
        "Role",
        "Description_lang",
    ];
    let mut specs = HashMap::new();
    for_each_record(dir, "ChrSpecialization", &columns, |row| {
        let table = "ChrSpecialization";
        let mastery = [parse_u32(table, row[4])?, parse_u32(table, row[5])?];
        let order_index = parse_u32(table, row[3])?;
        let spec = SpecTabInfo {
            name: row[1].to_string(),
            class_id: parse_u32(table, row[2])?,
            order_index,
            initial: order_index == INITIAL_SPEC_ORDER,
            spells: mastery.into_iter().filter(|&id| id != 0).collect(),
            primary_stat_priority: parse_u32(table, row[6])?,
            icon_fdid: parse_u32(table, row[7])?,
            role: parse_u32(table, row[8])?,
            description: row[9].to_string(),
        };
        specs.insert(parse_u32(table, row[0])?, spec);
        Ok(())
    })?;
    Ok(specs)
}

fn add_spec_spells(dir: &Path, specs: &mut HashMap<u32, SpecTabInfo>) -> Result<(), String> {
    for_each_record(dir, "SpecializationSpells", &["SpecID", "SpellID"], |row| {
        let table = "SpecializationSpells";
        if let Some(spec) = specs.get_mut(&parse_u32(table, row[0])?) {
            spec.spells.insert(parse_u32(table, row[1])?);
        }
        Ok(())
    })
}

/// Spell → class of every class-line spell, and the class lines' auto-learned rows.
fn load_class_spells(
    dir: &Path,
    class_names: &HashMap<u32, String>,
) -> Result<(HashMap<u32, u32>, Vec<ClassAbility>), String> {
    let class_by_name: HashMap<&str, u32> = class_names
        .iter()
        .map(|(&id, name)| (name.as_str(), id))
        .collect();
    let mut class_lines = HashMap::new();
    let columns = ["ID", "CategoryID", "DisplayName_lang"];
    for_each_record(dir, "SkillLine", &columns, |row| {
        if parse_u32("SkillLine", row[1])? == SKILL_CATEGORY_CLASS
            && let Some(&class_id) = class_by_name.get(row[2])
        {
            class_lines.insert(parse_u32("SkillLine", row[0])?, class_id);
        }
        Ok(())
    })?;
    let mut class_spells = HashMap::new();
    let mut abilities = Vec::new();
    let columns = [
        "SkillLine",
        "Spell",
        "AcquireMethod",
        "ClassMask",
        "RaceMasks_0",
        "RaceMasks_1",
    ];
    for_each_record(dir, "SkillLineAbility", &columns, |row| {
        let table = "SkillLineAbility";
        let Some(&class_id) = class_lines.get(&parse_u32(table, row[0])?) else {
            return Ok(());
        };
        let spell_id = parse_u32(table, row[1])?;
        class_spells.insert(spell_id, class_id);
        let class_mask = parse_mask(table, row[3])?;
        let class_bit = 1u32.checked_shl(class_id.saturating_sub(1)).unwrap_or(0);
        if AUTO_LEARN_METHODS.contains(&parse_u32(table, row[2])?)
            && (class_mask == 0 || class_mask & class_bit != 0)
        {
            abilities.push(ClassAbility {
                class_id,
                spell_id,
                race_masks: [parse_mask(table, row[4])?, parse_mask(table, row[5])?],
            });
        }
        Ok(())
    })?;
    Ok((class_spells, abilities))
}

/// A signed 32-bit DB2 mask column as its bits.
fn parse_mask(table: &str, raw: &str) -> Result<u32, String> {
    raw.parse::<i64>()
        .map(|value| value as u32)
        .map_err(|_| format!("{table}: bad mask {raw:?}"))
}
