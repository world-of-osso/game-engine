//! Character-select display names, independent of the UI art skin.
use std::collections::HashMap;
use std::path::Path;

use crate::csv_records::CsvTable;

const RETAIL_DIR: &str = "db2/12.1.0.69933";
const FOREVER_DIR: &str = "db2/1.60.1.70205";
const SKYBORNE_RACES: [u32; 2] = [95, 96];

pub struct CharSelectNames {
    races: HashMap<u32, String>,
    classes: HashMap<u32, String>,
}

impl CharSelectNames {
    /// Retail names, with the same explicit Skyborne overlay as the body-model catalog.
    /// Forever art selection does not change which race/class a character is.
    pub fn load(data_root: &Path) -> Result<Self, String> {
        let retail = data_root.join(RETAIL_DIR);
        let mut races = read_names(&retail.join("ChrRaces.csv"))?;
        let classes = read_names(&retail.join("ChrClasses.csv"))?;
        let forever = read_names(&data_root.join(FOREVER_DIR).join("ChrRaces.csv"))?;
        for race in SKYBORNE_RACES {
            races.insert(race, lookup(&forever, race, "Forever race")?.to_owned());
        }
        Ok(Self { races, classes })
    }

    pub fn info(&self, level: u16, race: u8, class: u8) -> Result<String, String> {
        let race = lookup(&self.races, u32::from(race), "race")?;
        let class = lookup(&self.classes, u32::from(class), "class")?;
        Ok(format!("Level {level} {race} {class}"))
    }
}

fn lookup<'a>(names: &'a HashMap<u32, String>, id: u32, kind: &str) -> Result<&'a str, String> {
    names
        .get(&id)
        .map(String::as_str)
        .ok_or_else(|| format!("Character select: missing {kind} name for ID {id}"))
}

fn read_names(path: &Path) -> Result<HashMap<u32, String>, String> {
    let table = CsvTable::read(path)?;
    let id = table.column("ID")?;
    let name = table.column("Name_lang")?;
    table
        .records()
        .map(|row| {
            let value = |column| {
                row.get(column)
                    .ok_or_else(|| format!("{}: incomplete name row", path.display()))
            };
            let id = value(id)?
                .parse::<u32>()
                .map_err(|err| format!("{}: invalid ID: {err}", path.display()))?;
            let name = value(name)?.to_string();
            if name.is_empty() {
                return Err(format!("{}: empty name for ID {id}", path.display()));
            }
            Ok((id, name))
        })
        .collect()
}
