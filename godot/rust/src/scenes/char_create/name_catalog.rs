use std::collections::HashMap;
use std::path::Path;

const MAX_CHARACTER_NAME_BYTES: usize = 12;

/// Authored NameGen names indexed by the playable race and body-type IDs.
#[derive(Default)]
pub struct NameCatalog {
    names: HashMap<(u8, u8), Vec<String>>,
}

impl NameCatalog {
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut catalog = Self::parse(&read_catalog(path)?)
            .map_err(|err| format!("Invalid {}: {err}", path.display()))?;
        let data = path
            .parent()
            .ok_or_else(|| "Name catalog has no data directory".to_string())?;
        let forever = data.join("db2/1.60.1.70205/NameGen.csv");
        let overlay = Self::parse_source(&read_catalog(&forever)?, true)
            .map_err(|err| format!("Invalid {}: {err}", forever.display()))?;
        for (key, names) in overlay.names {
            for name in names {
                catalog.insert_name(key, &name);
            }
        }
        Ok(catalog)
    }

    fn parse(csv: &str) -> Result<Self, String> {
        Self::parse_source(csv, false)
    }

    fn parse_source(csv: &str, forever: bool) -> Result<Self, String> {
        let header = if forever {
            "ID,Name,RaceID,Sex,NameType"
        } else {
            "ID,Name,RaceID,Sex"
        };
        let mut lines = csv.lines();
        if lines.next() != Some(header) {
            return Err(format!("expected {header} header"));
        }
        let mut catalog = Self::default();
        for (index, line) in lines.enumerate() {
            let fields: Vec<_> = line.split(',').collect();
            let expected = if forever { 5 } else { 4 };
            if fields.len() != expected {
                return Err(format!("line {}: expected {expected} columns", index + 2));
            }
            let (id, name, race, sex) = (fields[0], fields[1], fields[2], fields[3]);
            id.parse::<u32>()
                .map_err(|err| format!("line {}: invalid ID: {err}", index + 2))?;
            let race = race
                .parse::<u8>()
                .map_err(|err| format!("line {}: invalid RaceID: {err}", index + 2))?;
            let sex = sex
                .parse::<u8>()
                .map_err(|err| format!("line {}: invalid Sex: {err}", index + 2))?;
            if forever {
                let name_type = fields[4]
                    .parse::<u8>()
                    .map_err(|err| format!("line {}: invalid NameType: {err}", index + 2))?;
                if !matches!(race, 95 | 96) || name_type != 0 {
                    continue;
                }
            }
            // The creation server accepts only 2–12 ASCII letters. Skip incompatible
            // authored entries instead of truncating them into invented names.
            if (2..=MAX_CHARACTER_NAME_BYTES).contains(&name.len())
                && name.bytes().all(|byte| byte.is_ascii_alphabetic())
            {
                catalog.insert_name((race, sex), name);
            }
        }
        if catalog.names.is_empty() {
            return Err("no valid authored names".to_string());
        }
        Ok(catalog)
    }

    fn insert_name(&mut self, key: (u8, u8), name: &str) {
        let names = self.names.entry(key).or_default();
        if !names.iter().any(|candidate| candidate == name) {
            names.push(name.to_owned());
        }
    }

    pub fn has_names(&self, race: u8, sex: u8) -> bool {
        self.names.contains_key(&(canonical_name_race(race), sex))
    }

    pub fn pick_name(&self, race: u8, sex: u8, current: &str, seed: u64) -> Option<&str> {
        let names = self.names.get(&(canonical_name_race(race), sex))?;
        if names.len() == 1 {
            return Some(&names[0]);
        }
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

fn read_catalog(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| {
        format!(
            "Cannot read authored name catalog {}: {err}",
            path.display()
        )
    })
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

#[cfg(test)]
#[path = "name_catalog_fixture_tests.rs"]
mod fixture_tests;
