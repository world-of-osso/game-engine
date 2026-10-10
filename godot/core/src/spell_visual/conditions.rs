use super::*;

impl SpellVisualCatalog {
    pub(super) fn read_specializations(&mut self, dir: &Path) -> Result<(), String> {
        let table = Table::read(dir, "ChrSpecialization")?;
        self.spec_order_indices = table
            .ints(["ID", "OrderIndex"])?
            .into_iter()
            .map(|[id, index]| (id as u32, index as u8))
            .collect();
        Ok(())
    }

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

const HANDLED: [&str; 20] = [
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
    "AuraSpellLogic",
    "AuraSpellID_0",
    "AuraSpellID_1",
    "AuraSpellID_2",
    "AuraSpellID_3",
    "AuraStacks_0",
    "AuraStacks_1",
    "AuraStacks_2",
    "AuraStacks_3",
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
    indices: &[usize; 20],
    others: &[(usize, i64)],
) -> (u32, PlayerCondition) {
    let value = |slot: usize| condition_cell(row, indices[slot]);
    let race_mask = (value(4) as u32 as u64) | ((value(5) as u32 as u64) << 32);
    // Support one authored aura requirement only. Multi-operand logic remains
    // unavailable until its semantics can be established from local sources.
    const INVERT_FIRST_AURA: i64 = 1 << 16;
    let aura_logic = value(11);
    let unsupported_aura = !matches!(aura_logic, 0 | INVERT_FIRST_AURA)
        || (13..=15).any(|slot| value(slot) != 0)
        || (17..=19).any(|slot| value(slot) != 0);
    let condition = PlayerCondition {
        flags: value(2),
        class_mask: value(3),
        race_mask,
        gender: value(6),
        min_level: value(7),
        max_level: value(8),
        spec_index: value(9),
        weapon_subclass_mask: value(10),
        aura_spell: value(12) as u32,
        aura_stacks: value(16) as u32,
        aura_inverted: aura_logic == INVERT_FIRST_AURA,
        unsupported: unsupported_aura
            || others
                .iter()
                .any(|&(index, unset)| condition_cell(row, index) != unset),
    };
    (value(0) as u32, condition)
}
