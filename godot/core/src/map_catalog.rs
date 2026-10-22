//! Explicit retail + Forever map identities. Retail wins ID and directory conflicts.
use std::path::Path;

use crate::{
    asset_product::AssetProduct,
    csv_util::{header_index, parse_csv_records},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapIdentity {
    pub source_product: AssetProduct,
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
            &read("db2/active/Map.csv")?,
            &read("db2/1.60.1.70205/Map.csv")?,
        )
    }

    pub fn parse(retail: &str, forever: &str) -> Result<Self, String> {
        let mut maps = parse_maps(retail, "retail Map.csv", AssetProduct::Retail)?;
        for map in parse_maps(forever, "Forever Map.csv", AssetProduct::Forever)? {
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

fn parse_maps(
    text: &str,
    source: &str,
    source_product: AssetProduct,
) -> Result<Vec<MapIdentity>, String> {
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
        .scan(2, |line, row| {
            let start = *line;
            *line += 1 + row
                .iter()
                .map(|field| field.matches('\n').count())
                .sum::<usize>();
            Some((start, row))
        })
        .map(|(line, row)| {
            let field = |index: usize| -> Result<&String, String> {
                row.get(index)
                    .ok_or_else(|| format!("{source}:{line}: incomplete row {row:?}"))
            };
            let number = |index| {
                field(index)?.parse::<u32>().map_err(|error| {
                    format!("{source}:{line}: invalid integer in {row:?}: {error}")
                })
            };
            Ok(MapIdentity {
                source_product,
                id: number(id)?,
                directory: field(directory)?.clone(),
                name: field(name)?.clone(),
                wdt_fdid: number(wdt)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod db2_header_tests {
    use super::*;
    #[test]
    fn db2_header_map_rejects_short_canonical_row() {
        let fixture = crate::db2_test_support::ExportFixture::copy("db2/active/Map", None);
        let csv = std::fs::read_to_string(&fixture.0).unwrap();
        let mut lines: Vec<_> = csv.lines().map(str::to_string).collect();
        lines[1] = "invalid".into();
        let csv = lines.join("\n");
        std::fs::write(&fixture.0, &csv).unwrap();
        let error = parse_maps(&csv, "Map.csv", AssetProduct::Retail).unwrap_err();
        assert!(error.contains("Map.csv:2:"), "{error}");
    }
}

#[cfg(test)]
mod db2_canonical_tests {
    use super::*;
    #[test]
    fn db2_header_maps_read_every_installed_record() {
        let fixture = crate::db2_test_support::ExportFixture::copy("db2/active/Map", None);
        let text = std::fs::read_to_string(&fixture.0).unwrap();
        let maps = parse_maps(&text, "Map.csv", AssetProduct::Retail).unwrap();
        assert_eq!(
            maps.len(),
            crate::csv_util::parse_csv_records(&text).len() - 1
        );
        assert!(maps.iter().any(|map| map.id == 2703));
    }
}
