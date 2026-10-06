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
    let headers = parse_csv_line_trimmed(&header);
    let columns = [
        "ItemDisplayInfoID",
        "ModelIndex",
        "TextureType",
        "MaterialResourcesID",
    ]
    .map(|name| header_index(&headers, name, &path));
    let columns: [usize; 4] = columns
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .expect("four columns");
    let [display, model, kind, material] = columns;
    let mut materials = ModelMaterials::new();
    for (index, line) in lines.enumerate() {
        let line = line.map_err(|error| format!("read {}: {error}", path.display()))?;
        let fields = parse_csv_line_trimmed(&line);
        let read = |column: usize| -> Result<u32, String> {
            fields
                .get(column)
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| {
                    format!(
                        "{}:{}: invalid {}",
                        path.display(),
                        index + 2,
                        headers[column]
                    )
                })
        };
        let key = (read(display)?, read(model)? as usize);
        materials
            .entry(key)
            .or_default()
            .push((read(kind)?, read(material)?));
    }
    Ok(materials)
}
