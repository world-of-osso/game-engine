//! Explicit retail + Forever map identities. Retail wins ID and directory conflicts.
use std::path::Path;

use crate::csv_util::{header_index, parse_csv_records};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapIdentity {
    pub id: u32,
    pub directory: String,
    pub name: String,
    pub wdt_fdid: u32,
}

pub struct MapCatalog {
    maps: Vec<MapIdentity>,
}

impl MapCatalog {
    pub fn read(data: &Path) -> Result<Self, String> {
        let read = |relative: &str| {
            let path = data.join(relative);
            std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))
        };
        Self::parse(
            &read("db2/12.1.0.69933/Map.csv")?,
            &read("db2/1.60.1.70205/Map.csv")?,
        )
    }

    pub fn parse(retail: &str, forever: &str) -> Result<Self, String> {
        let mut maps = parse_maps(retail, "retail Map.csv")?;
        for map in parse_maps(forever, "Forever Map.csv")? {
            if !maps.iter().any(|existing| {
                existing.id == map.id || existing.directory.eq_ignore_ascii_case(&map.directory)
            }) {
                maps.push(map);
            }
        }
        Ok(Self { maps })
    }

    pub fn by_directory(&self, directory: &str) -> Option<&MapIdentity> {
        self.maps
            .iter()
            .find(|map| map.directory.eq_ignore_ascii_case(directory))
    }

    pub fn by_id(&self, id: u32) -> Option<&MapIdentity> {
        self.maps.iter().find(|map| map.id == id)
    }
}

fn parse_maps(text: &str, source: &str) -> Result<Vec<MapIdentity>, String> {
    let records = parse_csv_records(text);
    let header = records
        .first()
        .ok_or_else(|| format!("{source}: empty CSV"))?;
    let columns = ["ID", "Directory", "MapName_lang", "WdtFileDataID"]
        .map(|column| header_index(header, column, Path::new(source)));
    let [id, directory, name, wdt] = columns;
    let [id, directory, name, wdt] = [id?, directory?, name?, wdt?];
    records
        .into_iter()
        .skip(1)
        .filter(|row| row.len() > 1)
        .map(|row| {
            let field = |index| {
                row.get(index)
                    .ok_or_else(|| format!("{source}: incomplete row {row:?}"))
            };
            let number = |index| {
                field(index)?
                    .parse::<u32>()
                    .map_err(|error| format!("{source}: invalid integer in {row:?}: {error}"))
            };
            Ok(MapIdentity {
                id: number(id)?,
                directory: field(directory)?.clone(),
                name: field(name)?.clone(),
                wdt_fdid: number(wdt)?,
            })
        })
        .collect()
}
