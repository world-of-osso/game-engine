//! Shared DB2 readers used by the recipe catalog and crafting-quality loader.
use crate::csv_records::CsvTable;
use std::path::Path;

pub fn number(raw: &str) -> Result<u32, String> {
    raw.parse()
        .map_err(|error| format!("Recipe DB2 integer {raw:?}: {error}"))
}
pub fn read_rows(
    dir: &Path,
    table: &str,
    columns: &[&str],
    mut apply: impl FnMut(&[&str]) -> Result<(), String>,
) -> Result<(), String> {
    let csv = CsvTable::read(&dir.join(format!("{table}.csv")))?;
    let indices = columns
        .iter()
        .map(|column| csv.column(column))
        .collect::<Result<Vec<_>, _>>()?;
    for record in csv.records() {
        let row = indices
            .iter()
            .map(|&index| {
                record
                    .get(index)
                    .map(|field| field.as_ref())
                    .ok_or_else(|| format!("{table}: short CSV row"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        apply(&row).map_err(|error| format!("{table}: {error}"))?;
    }
    Ok(())
}
