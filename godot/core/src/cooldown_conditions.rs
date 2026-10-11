//! The PlayerCondition predicates actually referenced by local CooldownSet data.
//! Primary semantics: TrinityCore ConditionMgr::IsPlayerMeetingCondition and
//! PlayerConditionLogic. Unimplemented fields remain explicit errors, not false.

use crate::spell_catalog::csv_records::CsvTable;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub struct ConditionActor<'a> {
    /// ChrRaces.PlayableRaceBit, not race ID minus one (allied/product races differ).
    pub race_bit: u32,
    pub class: u8,
    pub spec_index: u32,
    pub known: &'a [u32],
    pub auras: &'a BTreeMap<u32, u32>,
}

#[derive(Clone, Debug)]
struct ConditionRow {
    race_mask: u64,
    class_mask: u32,
    spec_index: i32,
    spells: [u32; 4],
    spell_logic: u32,
    aura_spells: [u32; 4],
    aura_stacks: [u32; 4],
    aura_logic: u32,
    unsupported: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct CooldownConditions {
    rows: BTreeMap<u32, ConditionRow>,
}

impl CooldownConditions {
    pub fn load(dir: &Path, needed: &BTreeSet<u32>) -> Result<Self, String> {
        let table = CsvTable::read(&dir.join("PlayerCondition.csv"))?;
        let id_column = table.column("ID")?;
        let mut rows = BTreeMap::new();
        for record in table.records() {
            let id = record
                .get(id_column)
                .ok_or("PlayerCondition: missing ID")?
                .parse()
                .map_err(|error| format!("PlayerCondition ID: {error}"))?;
            if !needed.contains(&id) {
                continue;
            }
            let cells = table
                .headers()
                .iter()
                .cloned()
                .zip(record.iter().map(|cell| cell.to_string()))
                .collect();
            rows.insert(id, read_condition(&cells)?);
        }
        for id in needed.iter().filter(|id| **id != 0) {
            if !rows.contains_key(id) {
                return Err(format!("PlayerCondition {id}: absent authored row"));
            }
        }
        Ok(Self { rows })
    }

    pub fn evaluate(&self, id: u32, actor: &ConditionActor<'_>) -> Result<bool, String> {
        if id == 0 {
            return Ok(true);
        }
        let row = self
            .rows
            .get(&id)
            .ok_or_else(|| format!("PlayerCondition {id}: not loaded"))?;
        if !row.unsupported.is_empty() {
            return Err(format!(
                "PlayerCondition {id}: unsupported {}",
                row.unsupported.join(", ")
            ));
        }
        if !matches_mask(row.race_mask, actor.race_bit) {
            return Ok(false);
        }
        let class_bit = u32::from(actor.class)
            .checked_sub(1)
            .and_then(|index| 1u32.checked_shl(index));
        if row.class_mask != 0 && !class_bit.is_some_and(|bit| row.class_mask & bit != 0) {
            return Ok(false);
        }
        if row.spec_index >= 0 && row.spec_index as u32 != actor.spec_index {
            return Ok(false);
        }
        let spells = row
            .spells
            .map(|spell| spell == 0 || actor.known.contains(&spell));
        if row.spells[0] != 0 && !condition_logic(row.spell_logic, spells) {
            return Ok(false);
        }
        let auras = std::array::from_fn(|index| {
            aura_requirement(row.aura_spells[index], row.aura_stacks[index], actor)
        });
        Ok(row.aura_spells[0] == 0 || condition_logic(row.aura_logic, auras))
    }
}

fn matches_mask(mask: u64, race_bit: u32) -> bool {
    mask == 0
        || 1u64
            .checked_shl(race_bit)
            .is_some_and(|bit| mask & bit != 0)
}

fn aura_requirement(spell: u32, stacks: u32, actor: &ConditionActor<'_>) -> bool {
    if spell == 0 {
        return true;
    }
    let held = actor.auras.get(&spell).copied().unwrap_or(0);
    if stacks == 0 {
        held > 0
    } else {
        held >= stacks
    }
}

fn condition_logic(logic: u32, values: [bool; 4]) -> bool {
    let mask = values
        .iter()
        .enumerate()
        .fold(0u32, |mask, (index, value)| {
            mask | (u32::from(*value) << index)
        });
    let mask = mask ^ (logic >> 16);
    let mut result = mask & 1;
    for index in 1..4 {
        match (logic >> (2 * (index - 1))) & 3 {
            1 => result &= (mask >> index) & 1,
            2 => result |= (mask >> index) & 1,
            _ => {}
        }
    }
    result != 0
}

type Cells = BTreeMap<String, String>;

fn number<T: std::str::FromStr>(cells: &Cells, field: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    cells
        .get(field)
        .ok_or_else(|| format!("PlayerCondition: missing {field}"))?
        .parse()
        .map_err(|error| format!("PlayerCondition {field}: {error}"))
}

fn four(cells: &Cells, prefix: &str) -> Result<[u32; 4], String> {
    let values = (0..4)
        .map(|index| number(cells, &format!("{prefix}_{index}")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(values.try_into().expect("four fields requested"))
}

fn read_condition(cells: &Cells) -> Result<ConditionRow, String> {
    let unsupported = cells
        .iter()
        .filter(|(name, value)| {
            !supported_field(name) && !matches!(value.as_str(), "0" | "-1" | "")
        })
        .map(|(name, _)| name.clone())
        .collect();
    let mut row = ConditionRow {
        // CSV exposes two signed 32-bit words of one 64-bit mask (as the existing
        // spell_visual PlayerCondition importer does), not two 64-bit words.
        race_mask: (number::<i64>(cells, "RaceMasks_0")? as u32 as u64)
            | ((number::<i64>(cells, "RaceMasks_1")? as u32 as u64) << 32),
        class_mask: number(cells, "ClassMask")?,
        spec_index: number(cells, "ChrSpecializationIndex")?,
        spells: four(cells, "SpellID")?,
        spell_logic: number(cells, "SpellLogic")?,
        aura_spells: four(cells, "AuraSpellID")?,
        aura_stacks: four(cells, "AuraStacks")?,
        aura_logic: number(cells, "AuraSpellLogic")?,
        unsupported,
    };
    // ClientExecutable selects the execution context. WithinOrAboveRecord only
    // changes ContentTuning bounds (ConditionMgr::IsPlayerMeetingCondition);
    // nonzero ContentTuningID is already rejected as an unsupported field.
    if number::<u32>(cells, "Flags")? & !(0x01 | 0x20) != 0 {
        row.unsupported.push("Flags".into());
    }
    Ok(row)
}

fn supported_field(name: &str) -> bool {
    matches!(
        name,
        "ID" | "Flags"
            | "RaceMasks_0"
            | "RaceMasks_1"
            | "ClassMask"
            | "ChrSpecializationIndex"
            | "SpellLogic"
            | "AuraSpellLogic"
    ) || ["SpellID_", "AuraSpellID_", "AuraStacks_"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}
