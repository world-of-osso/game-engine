//! CSV parsing for outfit DB2 tables.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::{DisplayInfoResolved, DisplayMaterialTextures};

pub fn parse_item_display_info(path: &Path) -> Result<HashMap<u32, DisplayInfoResolved>, String> {
    let (h, rows) = read_csv(path)?;
    let columns = ItemDisplayInfoColumns::from_headers(&h)?;

    let mut map = HashMap::new();
    for row in &rows {
        let id = field_u32(row, columns.id);
        if id == 0 {
            continue;
        }
        map.insert(id, parse_display_info_row(row, columns));
    }
    Ok(map)
}

fn parse_display_info_row(row: &[String], columns: ItemDisplayInfoColumns) -> DisplayInfoResolved {
    DisplayInfoResolved {
        item_textures: Vec::new(),
        geoset_overrides: Vec::new(),
        model_resource_ids: collect_unique_u32(row, &[columns.model_res_0, columns.model_res_1]),
        model_material_resource_ids: collect_unique_u32(
            row,
            &[columns.model_mat_res_0, columns.model_mat_res_1],
        ),
        model_resource_columns: [
            field_u32(row, columns.model_res_0),
            field_u32(row, columns.model_res_1),
        ],
        model_material_resource_columns: [
            field_u32(row, columns.model_mat_res_0),
            field_u32(row, columns.model_mat_res_1),
        ],
        helmet_geoset_vis_ids: collect_unique_u32(
            row,
            &[columns.helmet_vis_0, columns.helmet_vis_1],
        ),
        geoset_groups: [
            field_i32(row, columns.geoset_group_0) as i16,
            field_i32(row, columns.geoset_group_1) as i16,
            field_i32(row, columns.geoset_group_2) as i16,
            0,
            0,
            0,
        ],
    }
}

#[derive(Clone, Copy)]
struct ItemDisplayInfoColumns {
    id: usize,
    model_res_0: usize,
    model_res_1: usize,
    model_mat_res_0: usize,
    model_mat_res_1: usize,
    geoset_group_0: usize,
    geoset_group_1: usize,
    geoset_group_2: usize,
    helmet_vis_0: usize,
    helmet_vis_1: usize,
}

impl ItemDisplayInfoColumns {
    fn from_headers(headers: &[String]) -> Result<Self, String> {
        Ok(Self {
            id: col(headers, "ID")?,
            model_res_0: col(headers, "ModelResourcesID_0")?,
            model_res_1: col(headers, "ModelResourcesID_1")?,
            model_mat_res_0: col(headers, "ModelMaterialResourcesID_0")?,
            model_mat_res_1: col(headers, "ModelMaterialResourcesID_1")?,
            geoset_group_0: col(headers, "GeosetGroup_0")?,
            geoset_group_1: col(headers, "GeosetGroup_1")?,
            geoset_group_2: col(headers, "GeosetGroup_2")?,
            helmet_vis_0: col(headers, "HelmetGeosetVis_0")?,
            helmet_vis_1: col(headers, "HelmetGeosetVis_1")?,
        })
    }
}

pub fn parse_item_display_info_material_res(
    path: &Path,
    material_to_texture: &HashMap<u32, u32>,
) -> Result<DisplayMaterialTextures, String> {
    let (h, rows) = read_csv(path)?;
    let component_col = col(&h, "ComponentSection")?;
    let material_col = col(&h, "MaterialResourcesID")?;
    let display_info_col = col(&h, "ItemDisplayInfoID")?;

    let mut direct: HashMap<u32, Vec<(u8, u32)>> = HashMap::new();
    let mut seen: HashSet<(u32, u8, u32)> = HashSet::new();

    for row in &rows {
        let display_info_id = field_u32(row, display_info_col);
        let component_section = field_u32(row, component_col) as u8;
        let material_resource_id = field_u32(row, material_col);
        let Some(&texture_fdid) = material_to_texture.get(&material_resource_id) else {
            continue;
        };
        if seen.insert((display_info_id, component_section, texture_fdid)) {
            direct
                .entry(display_info_id)
                .or_default()
                .push((component_section, texture_fdid));
        }
    }

    Ok(DisplayMaterialTextures { direct })
}

pub fn parse_texture_file_data(path: &Path) -> Result<HashMap<u32, u32>, String> {
    let (h, rows) = read_csv(path)?;
    let file_data_col = col(&h, "FileDataID")?;
    let usage_type_col = col(&h, "UsageType")?;
    let material_col = col(&h, "MaterialResourcesID")?;

    let mut preferred = HashMap::new();
    let mut fallback = HashMap::new();

    for row in &rows {
        let file_data_id = field_u32(row, file_data_col);
        let usage_type = field_u32(row, usage_type_col);
        let material_resource_id = field_u32(row, material_col);
        if file_data_id == 0 || material_resource_id == 0 {
            continue;
        }

        fallback.entry(material_resource_id).or_insert(file_data_id);
        if usage_type == 0 {
            preferred
                .entry(material_resource_id)
                .or_insert(file_data_id);
        }
    }

    for (material_resource_id, file_data_id) in fallback {
        preferred
            .entry(material_resource_id)
            .or_insert(file_data_id);
    }

    Ok(preferred)
}

pub fn parse_model_file_data(path: &Path) -> Result<HashMap<u32, Vec<u32>>, String> {
    let (h, rows) = read_csv(path)?;
    let file_data_col = col(&h, "FileDataID")?;
    let model_resource_col = col(&h, "ModelResourcesID")?;

    let mut map: HashMap<u32, Vec<u32>> = HashMap::new();
    for row in &rows {
        let file_data_id = field_u32(row, file_data_col);
        let model_resource_id = field_u32(row, model_resource_col);
        if file_data_id == 0 || model_resource_id == 0 {
            continue;
        }
        let entry = map.entry(model_resource_id).or_default();
        if !entry.contains(&file_data_id) {
            entry.push(file_data_id);
        }
    }
    Ok(map)
}

pub fn parse_race_prefix(path: &Path) -> Result<HashMap<u8, String>, String> {
    let (h, rows) = read_csv(path)?;
    let id_col = col(&h, "ID")?;
    let prefix_col = col(&h, "ClientPrefix")?;

    let mut map = HashMap::new();
    for row in &rows {
        let id = field_u32(row, id_col) as u8;
        let prefix = row
            .get(prefix_col)
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if id != 0 && !prefix.is_empty() {
            map.insert(id, prefix);
        }
    }
    Ok(map)
}

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut lines = data.lines();
    let header_line = lines.next().ok_or("empty CSV")?;
    let headers = parse_csv_line(header_line);
    let rows: Vec<_> = lines.map(parse_csv_line).collect();
    Ok((headers, rows))
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == ',' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

fn col(headers: &[String], name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| format!("Column '{name}' not found"))
}

pub fn field_u32(row: &[String], col: usize) -> u32 {
    row.get(col)
        .and_then(|s| {
            s.parse::<u32>()
                .ok()
                .or_else(|| s.parse::<i32>().ok().map(|v| v as u32))
        })
        .unwrap_or(0)
}

fn field_i32(row: &[String], col: usize) -> i32 {
    row.get(col)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
}

pub fn collect_unique_u32(row: &[String], columns: &[usize]) -> Vec<u32> {
    let mut values = Vec::new();
    for &column in columns {
        let value = field_u32(row, column);
        if value != 0 && !values.contains(&value) {
            values.push(value);
        }
    }
    values
}
