//! ItemDisplayInfoModelMatRes: replaceable M2 texture types per display/model column.

use crate::csv_util::{header_index, parse_csv_line_trimmed};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::Path,
};

pub(super) type ModelMaterials = HashMap<(u32, usize), Vec<(u32, u32)>>;

pub(super) fn load_model_materials(data_dir: &Path) -> Result<ModelMaterials, String> {
    let path = data_dir.join("ItemDisplayInfoModelMatRes.csv");
    let file =
        std::fs::File::open(&path).map_err(|error| format!("open {}: {error}", path.display()))?;
    let mut lines = BufReader::new(file).lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{}: missing header", path.display()))?
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let columns = read_material_columns(&header, &path)?;
    let mut materials = ModelMaterials::new();
    for (index, line) in lines.enumerate() {
        let line = line.map_err(|error| format!("read {}: {error}", path.display()))?;
        let fields = parse_csv_line_trimmed(&line);
        let [display, model, kind, material] =
            parse_material_row(&fields, columns, &path, index + 2)?;
        materials
            .entry((display, model as usize))
            .or_default()
            .push((kind, material));
    }
    Ok(materials)
}

fn read_material_columns(header: &str, path: &Path) -> Result<[usize; 4], String> {
    let headers = parse_csv_line_trimmed(header);
    Ok([
        header_index(&headers, "ItemDisplayInfoID", path)?,
        header_index(&headers, "ModelIndex", path)?,
        header_index(&headers, "TextureType", path)?,
        header_index(&headers, "MaterialResourcesID", path)?,
    ])
}

fn parse_material_row(
    fields: &[String],
    columns: [usize; 4],
    path: &Path,
    line: usize,
) -> Result<[u32; 4], String> {
    let read = |column: usize| {
        fields
            .get(column)
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| {
                format!(
                    "{}:{line}: invalid material column {column}",
                    path.display()
                )
            })
    };
    Ok([
        read(columns[0])?,
        read(columns[1])?,
        read(columns[2])?,
        read(columns[3])?,
    ])
}
