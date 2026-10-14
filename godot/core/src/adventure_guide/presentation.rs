//! Client-only Journeys art and ordering from active Retail display metadata.
use crate::csv_util::{header_index, parse_csv_records};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JourneyPresentation {
    pub texture_kit: String,
    pub priority: i32,
}

/// Covenant IDs join UiCovenantDisplayInfo to UiTextureKit, not reputation Faction IDs.
pub fn read_journey_presentation(
    data_root: &Path,
) -> Result<BTreeMap<u32, JourneyPresentation>, String> {
    let directory = data_root.join("db2/active");
    let kits = read_texture_kits(&directory.join("UiTextureKit.csv"))?;
    let rows = read_columns(
        &directory.join("UiCovenantDisplayInfo.csv"),
        &["CovenantID", "UiTextureKitID", "UiPriority"],
    )?;
    rows.into_iter()
        .map(|row| join_display_row(&row, &kits))
        .collect()
}

fn join_display_row(
    row: &[String],
    kits: &BTreeMap<u32, String>,
) -> Result<(u32, JourneyPresentation), String> {
    let covenant = row[0]
        .parse::<u32>()
        .map_err(|error| format!("CovenantID {}: {error}", row[0]))?;
    let kit = row[1]
        .parse::<u32>()
        .map_err(|error| format!("Covenant {covenant} UiTextureKitID {}: {error}", row[1]))?;
    let priority = row[2]
        .parse::<i32>()
        .map_err(|error| format!("Covenant {covenant} UiPriority {}: {error}", row[2]))?;
    let texture_kit = kits
        .get(&kit)
        .filter(|prefix| !prefix.is_empty())
        .ok_or_else(|| format!("Covenant {covenant} has no UiTextureKit {kit} prefix"))?
        .clone();
    Ok((
        covenant,
        JourneyPresentation {
            texture_kit,
            priority,
        },
    ))
}

fn read_texture_kits(path: &Path) -> Result<BTreeMap<u32, String>, String> {
    read_columns(path, &["ID", "KitPrefix"])?
        .into_iter()
        .map(|row| {
            let id = row[0]
                .parse::<u32>()
                .map_err(|error| format!("UiTextureKit ID {}: {error}", row[0]))?;
            Ok((id, row[1].clone()))
        })
        .collect()
}

fn read_columns(path: &Path, columns: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut records = parse_csv_records(&source).into_iter();
    let header = records
        .next()
        .ok_or_else(|| format!("{} has no header", path.display()))?;
    let indexes = columns
        .iter()
        .map(|column| header_index(&header, column, path))
        .collect::<Result<Vec<_>, _>>()?;
    records
        .filter(|row| row != &[String::new()])
        .map(|row| {
            indexes
                .iter()
                .map(|index| {
                    row.get(*index)
                        .cloned()
                        .ok_or_else(|| format!("{} has a short data row", path.display()))
                })
                .collect()
        })
        .collect()
}
