//! Group sizes backing DifficultyUtil.GetMaxPlayers, separate from DB2 display names.
use crate::csv_util::read_db2_rows;
use std::{collections::BTreeMap, path::Path};

pub(super) fn read_max_players(cache: &Path) -> Result<BTreeMap<u32, u32>, String> {
    let data = cache
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| format!("Journal cache has no data root: {}", cache.display()))?;
    let mut sizes = BTreeMap::new();
    read_db2_rows(&data.join("db2/active/Difficulty.csv"), |row| {
        sizes.insert(row.parse("ID")?, row.parse("MaxPlayers")?);
        Ok(())
    })?;
    Ok(sizes)
}
