use super::*;

impl SpellVisualCatalog {
    pub(super) fn read_conditions(&mut self, dir: &Path) -> Result<(), String> {
        let referenced: std::collections::HashSet<u32> = self
            .spells
            .values()
            .flatten()
            .map(|choice| choice.caster_condition)
            .filter(|&id| id != 0)
            .collect();
        let table = Table::read(dir, "PlayerCondition")?;
        let id = table.column("ID")?;
        let wanted: Vec<&Vec<String>> = table
            .rows
            .iter()
            .filter(|row| {
                row.get(id)
                    .and_then(|cell| cell.parse::<u32>().ok())
                    .is_some_and(|id| referenced.contains(&id))
            })
            .collect();
        let indices = table.indices(HANDLED)?;
        let others = unhandled_columns(&table);
        for row in wanted {
            let (id, condition) = parse_condition(row, &indices, &others);
            self.conditions.insert(id, condition);
        }
        Ok(())
    }
}

const HANDLED: [&str; 11] = [
    "ID",
    "Failure_description_lang",
    "Flags",
    "ClassMask",
    "RaceMasks_0",
    "RaceMasks_1",
    "Gender",
    "MinLevel",
    "MaxLevel",
    "ChrSpecializationIndex",
    "WeaponSubclassMask",
];
// Columns whose "no requirement" value is -1; every other column's is 0.
const UNSET_MINUS_ONE: [&str; 7] = [
    "NativeGender",
    "MinExpansionLevel",
    "MaxExpansionLevel",
    "ChrSpecializationRole",
    "PowerType",
    "MaxExpansionTier",
    "MinExpansionTier",
];

fn unhandled_columns(table: &Table) -> Vec<(usize, i64)> {
    table
        .columns
        .iter()
        .filter(|(name, _)| !HANDLED.contains(&name.as_str()) && !name.ends_with("Logic"))
        .map(|(name, &index)| {
            let unset = if UNSET_MINUS_ONE.contains(&name.as_str()) {
                -1
            } else {
                0
            };
            (index, unset)
        })
        .collect()
}

fn condition_cell(row: &[String], index: usize) -> i64 {
    row.get(index)
        .and_then(|cell| parse_number(cell))
        .unwrap_or(0)
}

fn parse_condition(
    row: &[String],
    indices: &[usize; 11],
    others: &[(usize, i64)],
) -> (u32, PlayerCondition) {
    let value = |slot: usize| condition_cell(row, indices[slot]);
    let race_mask = (value(4) as u32 as u64) | ((value(5) as u32 as u64) << 32);
    let condition = PlayerCondition {
        flags: value(2),
        class_mask: value(3),
        race_mask,
        gender: value(6),
        min_level: value(7),
        max_level: value(8),
        spec_index: value(9),
        weapon_subclass_mask: value(10),
        unsupported: others
            .iter()
            .any(|&(index, unset)| condition_cell(row, index) != unset),
    };
    (value(0) as u32, condition)
}
