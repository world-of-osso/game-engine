//! ReadyCheck.lua:31–48: current instance difficulty, else selected difficulty;
//! show its name only when Difficulty.db2 supplies a positive ToggleDifficultyID.
use std::{collections::BTreeMap, path::Path};

use crate::csv_util::read_db2_rows;

#[derive(Debug)]
struct Difficulty {
    name: String,
    toggle_id: u32,
}

#[derive(Debug)]
pub struct ReadyCheckDifficulties(BTreeMap<u32, Difficulty>);

impl ReadyCheckDifficulties {
    pub fn load(db2: &Path) -> Result<Self, String> {
        let mut rows = BTreeMap::new();
        read_db2_rows(&db2.join("Difficulty.csv"), |row| {
            rows.insert(
                row.parse("ID")?,
                Difficulty {
                    name: row.field("Name_lang")?.to_owned(),
                    toggle_id: row.parse("ToggleDifficultyID")?,
                },
            );
            Ok(())
        })?;
        Ok(Self(rows))
    }

    /// The toggle target gates the label; the label names the CURRENT difficulty.
    pub fn listener_name(
        &self,
        current: Option<u32>,
        selected: Option<u32>,
    ) -> Result<Option<String>, String> {
        let Some(id) = current.filter(|id| *id != 0).or(selected) else {
            return Ok(None);
        };
        let row = self
            .0
            .get(&id)
            .ok_or_else(|| format!("ReadyCheck Difficulty.csv missing difficulty {id}"))?;
        Ok((row.toggle_id > 0).then(|| row.name.clone()))
    }
}
