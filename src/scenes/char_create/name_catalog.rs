use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::Resource;

const MAX_CHARACTER_NAME_BYTES: usize = 12;

/// Authored NameGen names indexed by the playable race and body-type IDs.
#[derive(Default)]
pub(super) struct NameCatalog {
    names: HashMap<(u8, u8), Vec<String>>,
}

#[derive(Resource)]
pub(super) struct NameCatalogResource(pub(super) Result<NameCatalog, String>);

impl NameCatalog {
    pub(super) fn load(path: &Path) -> Result<Self, String> {
        let csv = std::fs::read_to_string(path).map_err(|err| {
            format!(
                "Cannot read authored name catalog {}: {err}",
                path.display()
            )
        })?;
        Self::parse(&csv).map_err(|err| format!("Invalid {}: {err}", path.display()))
    }

    fn parse(csv: &str) -> Result<Self, String> {
        let mut lines = csv.lines();
        if lines.next() != Some("ID,Name,RaceID,Sex") {
            return Err("expected ID,Name,RaceID,Sex header".to_string());
        }
        let mut catalog = Self::default();
        for (index, line) in lines.enumerate() {
            let fields: Vec<_> = line.split(',').collect();
            let [id, name, race, sex] = fields.as_slice() else {
                return Err(format!("line {}: expected four columns", index + 2));
            };
            id.parse::<u32>()
                .map_err(|err| format!("line {}: invalid ID: {err}", index + 2))?;
            let race = race
                .parse::<u8>()
                .map_err(|err| format!("line {}: invalid RaceID: {err}", index + 2))?;
            let sex = sex
                .parse::<u8>()
                .map_err(|err| format!("line {}: invalid Sex: {err}", index + 2))?;
            // The creation server accepts only 2–12 ASCII letters. Skip incompatible
            // authored entries instead of truncating them into invented names.
            if (2..=MAX_CHARACTER_NAME_BYTES).contains(&name.len())
                && name.bytes().all(|byte| byte.is_ascii_alphabetic())
            {
                let names = catalog.names.entry((race, sex)).or_default();
                if !names.iter().any(|candidate| candidate == name) {
                    names.push((*name).to_owned());
                }
            }
        }
        if catalog.names.is_empty() {
            return Err("no valid authored names".to_string());
        }
        Ok(catalog)
    }

    pub(super) fn has_names(&self, race: u8, sex: u8) -> bool {
        self.names.contains_key(&(canonical_name_race(race), sex))
    }

    pub(super) fn pick_name(&self, race: u8, sex: u8, current: &str, seed: u64) -> Option<&str> {
        let names = self.names.get(&(canonical_name_race(race), sex))?;
        let current_index = names.iter().position(|name| name == current);
        let count = names.len() - usize::from(current_index.is_some());
        if count == 0 {
            return None;
        }
        let mut index = (seed % count as u64) as usize;
        if current_index.is_some_and(|previous| index >= previous) {
            index += 1;
        }
        Some(&names[index])
    }
}

fn canonical_name_race(race: u8) -> u8 {
    // ChrRaces 12.1.0.69875: playable 25/26 have NeutralRaceID 24;
    // NameGen records the neutral 24, not either faction alias.
    match race {
        25 | 26 => 24,
        _ => race,
    }
}

#[cfg(test)]
#[path = "name_catalog_tests.rs"]
mod tests;
