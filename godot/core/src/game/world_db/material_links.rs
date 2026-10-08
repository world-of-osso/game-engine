use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::Path;

use crate::csv_util::parse_csv_line_trimmed as parse_csv_line;
use rusqlite::{Connection, Statement};

/// Every texture file of each material, in TextureFileData order: its UsageType 0 files,
/// or its first file when it has none. Which of them a character wears is chosen per
/// race and sex (`ComponentFileData::select_texture`).
pub(super) fn populate_material_textures(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = super::open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let file_data_col = super::header_index(&headers, "FileDataID", path)?;
    let usage_type_col = super::header_index(&headers, "UsageType", path)?;
    let material_col = super::header_index(&headers, "MaterialResourcesID", path)?;
    let candidates = collect_material_texture_rows(
        &mut reader,
        path,
        file_data_col,
        usage_type_col,
        material_col,
    )?;
    let mut insert = conn
        .prepare(
            "INSERT INTO material_textures (material_resource_id, file_order, texture_fdid) VALUES (?1, ?2, ?3)",
        )
        .map_err(|err| format!("prepare material_textures insert: {err}"))?;
    for (material_resource_id, texture_fdids) in candidates {
        for (order, texture_fdid) in texture_fdids.into_iter().enumerate() {
            insert
                .execute((material_resource_id, order as u32, texture_fdid))
                .map_err(|err| format!("insert material_textures row: {err}"))?;
        }
    }
    Ok(())
}

fn collect_material_texture_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    file_data_col: usize,
    usage_type_col: usize,
    material_col: usize,
) -> Result<HashMap<u32, Vec<u32>>, String> {
    let mut preferred: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut fallback = HashMap::new();
    let mut line = String::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", path.display()))?
            == 0
        {
            break;
        }
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        record_material_texture_row(
            &fields,
            file_data_col,
            usage_type_col,
            material_col,
            &mut preferred,
            &mut fallback,
        );
    }
    for (material_resource_id, file_data_id) in fallback {
        preferred
            .entry(material_resource_id)
            .or_insert_with(|| vec![file_data_id]);
    }
    Ok(preferred)
}

fn record_material_texture_row(
    fields: &[String],
    file_data_col: usize,
    usage_type_col: usize,
    material_col: usize,
    preferred: &mut HashMap<u32, Vec<u32>>,
    fallback: &mut HashMap<u32, u32>,
) {
    let file_data_id = fields
        .get(file_data_col)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    let usage_type = fields
        .get(usage_type_col)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    let material_resource_id = fields
        .get(material_col)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    if file_data_id == 0 || material_resource_id == 0 {
        return;
    }
    fallback.entry(material_resource_id).or_insert(file_data_id);
    if usage_type == 0 {
        preferred
            .entry(material_resource_id)
            .or_default()
            .push(file_data_id);
    }
}

fn load_textured_materials(conn: &Connection) -> Result<HashSet<u32>, String> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT material_resource_id FROM material_textures")
        .map_err(|err| format!("prepare material_textures lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, u32>(0))
        .map_err(|err| format!("query material_textures: {err}"))?;
    rows.collect::<Result<_, _>>()
        .map_err(|err| format!("read material_textures row: {err}"))
}

/// Each display's (component section, material) rows whose material has a texture.
pub(super) fn populate_display_materials(conn: &Connection, path: &Path) -> Result<(), String> {
    let textured = load_textured_materials(conn)?;
    let mut reader = super::open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let component_col = super::header_index(&headers, "ComponentSection", path)?;
    let material_col = super::header_index(&headers, "MaterialResourcesID", path)?;
    let display_info_col = super::header_index(&headers, "ItemDisplayInfoID", path)?;
    let mut insert = conn
        .prepare(
            "INSERT OR IGNORE INTO display_materials (display_info_id, component_section, material_resource_id) VALUES (?1, ?2, ?3)",
        )
        .map_err(|err| format!("prepare display_materials insert: {err}"))?;
    insert_display_material_rows(
        &mut reader,
        path,
        component_col,
        material_col,
        display_info_col,
        &textured,
        &mut insert,
    )?;
    Ok(())
}

/// Retain declared Forever materials even when their texture mapping is missing;
/// checked outfit resolution reports the missing resource instead of undressing it.
pub(super) fn populate_declared_display_materials(
    conn: &Connection,
    path: &Path,
) -> Result<(), String> {
    let mut rows = Vec::new();
    crate::csv_util::read_numeric_rows(
        path,
        [
            "ItemDisplayInfoID",
            "ComponentSection",
            "MaterialResourcesID",
        ],
        |row| rows.push(row),
    )?;
    let mut insert = conn
        .prepare("INSERT OR IGNORE INTO display_materials VALUES (?1, ?2, ?3)")
        .map_err(|err| format!("prepare declared Forever materials: {err}"))?;
    for [display, section, material] in rows {
        if material != 0 {
            insert
                .execute((display, section, material))
                .map_err(|err| format!("insert declared Forever material {material}: {err}"))?;
        }
    }
    Ok(())
}

fn insert_display_material_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    component_col: usize,
    material_col: usize,
    display_info_col: usize,
    textured: &HashSet<u32>,
    insert: &mut Statement<'_>,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut line = String::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", path.display()))?
            == 0
        {
            break;
        }
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        let display_info_id = fields
            .get(display_info_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let component_section = fields
            .get(component_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0) as u8;
        let material_resource_id = fields
            .get(material_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        if !textured.contains(&material_resource_id) {
            continue;
        }
        if seen.insert((display_info_id, component_section, material_resource_id)) {
            insert
                .execute((display_info_id, component_section, material_resource_id))
                .map_err(|err| format!("insert display_materials row: {err}"))?;
        }
    }
    Ok(())
}
