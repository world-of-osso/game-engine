//! Spellbook tab grouping from ChrClasses, ChrSpecialization, SpecializationSpells,
//! SkillLine and SkillLineAbility.
//!
//! Rule: a spell of the active non-Initial spec (its `SpecializationSpells` rows or
//! mastery) goes on the spec tab; else a spell of any class skill line (`SkillLine`
//! category 7 named like a `ChrClasses` row) goes on the class tab; else General.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::csv_records::CsvTable;

/// `SkillLine.CategoryID` of class (and pet/mount) skill lines.
const SKILL_CATEGORY_CLASS: u32 = 7;
/// `ChrSpecialization.OrderIndex` of the pre-level-10 "Initial" spec.
const INITIAL_SPEC_ORDER: u32 = 4;

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
    pub initial: bool,
    pub spells: HashSet<u32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellbookTabIndex {
    pub class_names: HashMap<u32, String>,
    pub specs: HashMap<u32, SpecTabInfo>,
    /// Spell id → class id of the class skill line teaching it.
    pub class_spells: HashMap<u32, u32>,
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
    let class_spells = load_class_spells(dir, &class_names)?;
    Ok(SpellbookTabIndex {
        class_names,
        specs,
        class_spells,
    })
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
    ];
    let mut specs = HashMap::new();
    for_each_record(dir, "ChrSpecialization", &columns, |row| {
        let table = "ChrSpecialization";
        let mastery = [parse_u32(table, row[4])?, parse_u32(table, row[5])?];
        let spec = SpecTabInfo {
            name: row[1].to_string(),
            class_id: parse_u32(table, row[2])?,
            initial: parse_u32(table, row[3])? == INITIAL_SPEC_ORDER,
            spells: mastery.into_iter().filter(|&id| id != 0).collect(),
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

fn load_class_spells(
    dir: &Path,
    class_names: &HashMap<u32, String>,
) -> Result<HashMap<u32, u32>, String> {
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
    for_each_record(dir, "SkillLineAbility", &["SkillLine", "Spell"], |row| {
        let table = "SkillLineAbility";
        if let Some(&class_id) = class_lines.get(&parse_u32(table, row[0])?) {
            class_spells.insert(parse_u32(table, row[1])?, class_id);
        }
        Ok(())
    })?;
    Ok(class_spells)
}
