use crate::csv_util::{header_index, parse_csv_line_trimmed};
use bevy::prelude::Resource;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Resource)]
pub struct CreationSceneCatalog {
    scenes: HashMap<u8, u32>,
}

impl CreationSceneCatalog {
    pub fn load(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
        Self::parse(BufReader::new(file), path)
    }

    pub fn lookup(&self, race: u8) -> Result<u32, String> {
        self.scenes
            .get(&race)
            .copied()
            .ok_or_else(|| format!("no creation scene for race {race}"))
    }

    fn parse(reader: impl BufRead, path: &Path) -> Result<Self, String> {
        let mut lines = reader.lines();
        let header = lines
            .next()
            .transpose()
            .map_err(|err| format!("read {} header: {err}", path.display()))?
            .ok_or_else(|| format!("{} missing header", path.display()))?;
        let headers = parse_csv_line_trimmed(&header);
        let race_index = header_index(&headers, "ID", path)?;
        let scene_index = header_index(&headers, "CreateScreenFileDataID", path)?;
        let mut scenes = HashMap::new();

        for (line_index, line) in lines.enumerate() {
            let line_number = line_index + 2;
            let line =
                line.map_err(|err| format!("read {}:{line_number}: {err}", path.display()))?;
            let fields = parse_csv_line_trimmed(&line);
            if fields.len() != headers.len() {
                return Err(format!(
                    "{}:{line_number}: expected {} columns, found {}",
                    path.display(),
                    headers.len(),
                    fields.len()
                ));
            }
            let location = format!("{}:{line_number}", path.display());
            let (race, scene) = parse_scene_row(&fields, race_index, scene_index, &location)?;
            if scene != 0 && scenes.insert(race, scene).is_some() {
                return Err(format!(
                    "{}:{line_number}: duplicate ID {race}",
                    path.display()
                ));
            }
        }

        Ok(Self { scenes })
    }
}

fn parse_scene_row(
    fields: &[String],
    race_index: usize,
    scene_index: usize,
    location: &str,
) -> Result<(u8, u32), String> {
    let race = fields[race_index]
        .parse::<u8>()
        .map_err(|err| format!("{location}: invalid ID {:?}: {err}", fields[race_index]))?;
    let scene = fields[scene_index].parse::<u32>().map_err(|err| {
        format!(
            "{location}: invalid CreateScreenFileDataID {:?}: {err}",
            fields[scene_index]
        )
    })?;
    Ok((race, scene))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::char_create_data::RACES;
    use std::path::Path;

    #[test]
    fn production_catalog_covers_current_roster_and_authored_scenes() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/ChrRaces.csv");
        let catalog = CreationSceneCatalog::load(&path).unwrap();
        for race in RACES {
            assert!(
                catalog.lookup(race.id).is_ok(),
                "{} ({})",
                race.name,
                race.id
            );
        }
        for (race, fdid) in [
            (1, 623712),
            (2, 623714),
            (24, 623716),
            (25, 623712),
            (26, 623714),
        ] {
            assert_eq!(catalog.lookup(race).unwrap(), fdid, "race {race}");
        }
    }

    #[test]
    fn missing_columns_fail() {
        let err = parse_catalog("ID,FactionID\n1,1\n").unwrap_err();
        assert!(err.contains("CreateScreenFileDataID"), "{err}");
        let err = parse_catalog("CreateScreenFileDataID,FactionID\n623712,1\n").unwrap_err();
        assert!(err.contains("ID"), "{err}");
    }

    #[test]
    fn invalid_ids_and_truncated_rows_fail() {
        for csv in [
            "ID,CreateScreenFileDataID\nnope,623712\n",
            "ID,CreateScreenFileDataID\n256,623712\n",
            "ID,CreateScreenFileDataID\n1,nope\n",
            "ID,CreateScreenFileDataID\n1,-1\n",
            "ID,CreateScreenFileDataID\n1\n",
        ] {
            assert!(parse_catalog(csv).is_err(), "accepted {csv}");
        }
    }

    #[test]
    fn zero_scene_rows_are_skipped_without_guessing_for_absent_races() {
        let catalog =
            parse_catalog("CreateScreenFileDataID,ID\n0,12\n623716,24\n623712,25\n").unwrap();
        assert_eq!(catalog.lookup(24).unwrap(), 623716);
        assert_eq!(catalog.lookup(25).unwrap(), 623712);
        assert!(catalog.lookup(12).is_err());
        assert!(catalog.lookup(99).is_err());
    }

    fn parse_catalog(csv: &str) -> Result<CreationSceneCatalog, String> {
        CreationSceneCatalog::parse(csv.as_bytes(), Path::new("fixture.csv"))
    }
}
