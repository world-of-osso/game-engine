//! Group sizes backing DifficultyUtil.GetMaxPlayers, separate from DB2 display names.
use crate::csv_util::read_db2_rows;
use std::{collections::BTreeMap, path::Path};

pub(super) fn read_max_players(data: &Path) -> Result<BTreeMap<u32, u32>, String> {
    let mut sizes = BTreeMap::new();
    read_db2_rows(&data.join("db2/active/Difficulty.csv"), |row| {
        sizes.insert(row.parse("ID")?, row.parse("MaxPlayers")?);
        Ok(())
    })?;
    Ok(sizes)
}
