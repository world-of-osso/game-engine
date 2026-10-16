//! DungeonEncounter identifiers and sparse lockout bits, not journal boss order.
use crate::csv_util::read_db2_rows;
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DungeonEncounter {
    pub map: u32,
    /// Zero applies to every difficulty of this map.
    pub difficulty: u32,
    pub bit: u32,
}

pub(super) fn read_encounter_locks(data: &Path) -> Result<BTreeMap<u32, DungeonEncounter>, String> {
    let mut encounters = BTreeMap::new();
    read_db2_rows(&data.join("db2/active/DungeonEncounter.csv"), |row| {
        encounters.insert(
            row.parse("ID")?,
            DungeonEncounter {
                map: row.parse("MapID")?,
                difficulty: row.parse("DifficultyID")?,
                bit: row.parse("Bit")?,
            },
        );
        Ok(())
    })?;
    Ok(encounters)
}
