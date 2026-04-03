use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::Path;

use crate::csv_util::parse_csv_line_trimmed as parse_csv_line;
use rusqlite::{Connection, Statement};

pub(super) fn populate_material_to_texture(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = super::open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let file_data_col = super::header_index(&headers, "FileDataID", path)?;
    let usage_type_col = super::header_index(&headers, "UsageType", path)?;
    let material_col = super::header_index(&headers, "MaterialResourcesID", path)?;
    let preferred = collect_material_texture_rows(
        &mut reader,
        path,
        file_data_col,
        usage_type_col,
        material_col,
    )?;
    let mut insert = conn
        .prepare(
            "INSERT OR REPLACE INTO material_to_texture (material_resource_id, texture_fdid) VALUES (?1, ?2)",
        )
        .map_err(|err| format!("prepare material_to_texture insert: {err}"))?;
    for (material_resource_id, texture_fdid) in preferred {
        insert
            .execute((material_resource_id, texture_fdid))
            .map_err(|err| format!("insert material_to_texture row: {err}"))?;
    }
    Ok(())
}

fn collect_material_texture_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    file_data_col: usize,
    usage_type_col: usize,
    material_col: usize,
) -> Result<HashMap<u32, u32>, String> {
    let mut preferred = HashMap::new();
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
            .or_insert(file_data_id);
    }
    Ok(preferred)
}

fn record_material_texture_row(
    fields: &[String],
    file_data_col: usize,
    usage_type_col: usize,
    material_col: usize,
    preferred: &mut HashMap<u32, u32>,
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
            .or_insert(file_data_id);
    }
}

fn load_material_to_texture_map(conn: &Connection) -> Result<HashMap<u32, u32>, String> {
    let mut stmt = conn
        .prepare("SELECT material_resource_id, texture_fdid FROM material_to_texture")
        .map_err(|err| format!("prepare material_to_texture lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query material_to_texture: {err}"))?;
    let mut map = HashMap::new();
    for row in rows {
        let (material_resource_id, texture_fdid) =
            row.map_err(|err| format!("read material_to_texture row: {err}"))?;
        map.insert(material_resource_id, texture_fdid);
    }
    Ok(map)
}

pub(super) fn populate_display_material_textures(
    conn: &Connection,
    path: &Path,
) -> Result<(), String> {
    let material_to_texture = load_material_to_texture_map(conn)?;
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
            "INSERT OR IGNORE INTO display_material_textures (display_info_id, component_section, texture_fdid) VALUES (?1, ?2, ?3)",
        )
        .map_err(|err| format!("prepare display_material_textures insert: {err}"))?;
    insert_display_material_texture_rows(
        &mut reader,
        path,
        component_col,
        material_col,
        display_info_col,
        &material_to_texture,
        &mut insert,
    )?;
    Ok(())
}

fn insert_display_material_texture_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    component_col: usize,
    material_col: usize,
    display_info_col: usize,
    material_to_texture: &HashMap<u32, u32>,
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
        let Some(&texture_fdid) = material_to_texture.get(&material_resource_id) else {
            continue;
        };
        if seen.insert((display_info_id, component_section, texture_fdid)) {
            insert
                .execute((display_info_id, component_section, texture_fdid))
                .map_err(|err| format!("insert display_material_textures row: {err}"))?;
        }
    }
    Ok(())
}
