use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, OpenFlags, Statement};

use crate::outfit_data::DisplayInfoResolved;
#[cfg(test)]
use crate::outfit_data::DisplayMaterialTextures;

#[path = "outfit_cache_load.rs"]
mod outfit_cache_load;
#[path = "outfit_links.rs"]
mod outfit_links_cache;
#[path = "outfit_query.rs"]
mod outfit_query;
#[path = "outfit_resolve.rs"]
mod outfit_resolve;
#[path = "zone_names_cache.rs"]
mod zone_name_cache;

type OutfitKey = (u8, u8, u8);
type StarterOutfits = HashMap<OutfitKey, Vec<u32>>;

#[cfg(test)]
pub(crate) struct CachedDisplayResources {
    pub display_info: HashMap<u32, DisplayInfoResolved>,
    pub material_to_texture: HashMap<u32, u32>,
    pub display_materials: DisplayMaterialTextures,
    pub model_to_fdids: HashMap<u32, Vec<u32>>,
}

fn world_db_path() -> PathBuf {
    if let Some(path) = std::env::var_os("GAME_SERVER_WORLD_DB") {
        PathBuf::from(path)
    } else {
        crate::paths::shared_repo_root()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("game-server")
            .join("data")
            .join("world.db")
    }
}

fn zone_names_cache_path() -> PathBuf {
    crate::paths::shared_data_path("cache/zone_names.sqlite")
}

fn outfit_links_cache_path() -> PathBuf {
    crate::paths::shared_data_path("cache/outfit_links.sqlite")
}

fn area_table_csv_path() -> PathBuf {
    crate::paths::resolve_data_path("AreaTable.csv")
}

fn required_outfit_csv_paths(data_dir: &Path) -> [PathBuf; 7] {
    [
        data_dir.join("CharStartOutfit.csv"),
        data_dir.join("ItemModifiedAppearance.csv"),
        data_dir.join("ItemAppearance.csv"),
        data_dir.join("ItemDisplayInfo.csv"),
        data_dir.join("TextureFileData.csv"),
        data_dir.join("ItemDisplayInfoMaterialRes.csv"),
        data_dir.join("ModelFileData.csv"),
    ]
}

fn open_read_only(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", path.display()))
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current.trim().to_string());
    fields
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn header_index(headers: &[String], column: &str, path: &Path) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header == column)
        .ok_or_else(|| format!("{} missing {column} column", path.display()))
}

fn csv_mtime(path: &Path) -> Result<i64, String> {
    let modified = std::fs::metadata(path)
        .map_err(|err| format!("stat {}: {err}", path.display()))?
        .modified()
        .map_err(|err| format!("mtime {}: {err}", path.display()))?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("mtime epoch {}: {err}", path.display()))?
        .as_secs() as i64)
}

pub(crate) fn load_chr_race_prefixes() -> Result<HashMap<u8, String>, String> {
    let db_path = world_db_path();
    let conn = open_read_only(&db_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, client_prefix
             FROM chr_races
             WHERE id > 0
               AND client_prefix IS NOT NULL
               AND client_prefix != ''",
        )
        .map_err(|err| format!("prepare chr_races query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            let id: u32 = row.get(0)?;
            let prefix: String = row.get(1)?;
            Ok((id as u8, prefix.trim().to_ascii_lowercase()))
        })
        .map_err(|err| format!("query chr_races: {err}"))?;

    let mut prefixes = HashMap::new();
    for row in rows {
        let (id, prefix) = row.map_err(|err| format!("read chr_races row: {err}"))?;
        if !prefix.is_empty() {
            prefixes.insert(id, prefix);
        }
    }
    if prefixes.is_empty() {
        return Err(format!(
            "chr_races in {} returned no client_prefix rows",
            db_path.display()
        ));
    }
    Ok(prefixes)
}

pub fn import_zone_name_cache() -> Result<PathBuf, String> {
    zone_name_cache::import_zone_name_cache()
}

pub fn load_zone_name(id: u32) -> Result<Option<String>, String> {
    zone_name_cache::load_zone_name(id)
}

fn outfit_cache_is_fresh(conn: &Connection, csv_paths: &[PathBuf]) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT source, mtime_secs FROM source_files") {
        Ok(stmt) => stmt,
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("no such table") =>
        {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare source_files lookup: {err}")),
    };
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|err| format!("query source_files: {err}"))?;
    let mut recorded = HashMap::new();
    for row in rows {
        let (source, mtime) = row.map_err(|err| format!("read source_files row: {err}"))?;
        recorded.insert(source, mtime);
    }
    for path in csv_paths {
        let key = path.to_string_lossy().to_string();
        if recorded.get(&key).copied() != Some(csv_mtime(path)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn import_outfit_links_cache(data_dir: &Path) -> Result<PathBuf, String> {
    outfit_links_cache::import_outfit_links_cache(data_dir)
}

fn populate_starter_outfits(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let race_col = header_index(&headers, "RaceID", path)?;
    let class_col = header_index(&headers, "ClassID", path)?;
    let sex_col = header_index(&headers, "SexID", path)?;
    let item_cols = (0..12)
        .map(|i| header_index(&headers, &format!("ItemID_{i}"), path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut insert = conn
        .prepare(
            "INSERT INTO starter_outfits (race_id, class_id, sex_id, item_order, item_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(|err| format!("prepare starter_outfits insert: {err}"))?;
    insert_starter_outfit_rows(
        &mut reader,
        path,
        race_col,
        class_col,
        sex_col,
        &item_cols,
        &mut insert,
    )?;
    Ok(())
}

fn insert_starter_outfit_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    race_col: usize,
    class_col: usize,
    sex_col: usize,
    item_cols: &[usize],
    insert: &mut Statement<'_>,
) -> Result<(), String> {
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
        let race_id = fields
            .get(race_col)
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(0);
        let class_id = fields
            .get(class_col)
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(0);
        let sex_id = fields
            .get(sex_col)
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(0);
        for (item_order, &column) in item_cols.iter().enumerate() {
            let item_id = fields
                .get(column)
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            if item_id == 0 || item_id == 6948 {
                continue;
            }
            insert
                .execute((race_id, class_id, sex_id, item_order as u32, item_id))
                .map_err(|err| format!("insert starter_outfits row: {err}"))?;
        }
    }
    Ok(())
}

fn populate_item_modified_appearance_map(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let item_col = header_index(&headers, "ItemID", path)?;
    let appearance_col = header_index(&headers, "ItemAppearanceID", path)?;
    let mut insert = conn
        .prepare(
            "INSERT OR IGNORE INTO item_modified_appearance_map (item_id, appearance_id)
             VALUES (?1, ?2)",
        )
        .map_err(|err| format!("prepare item_modified_appearance_map insert: {err}"))?;
    insert_item_modified_appearance_rows(&mut reader, path, item_col, appearance_col, &mut insert)?;
    Ok(())
}

fn insert_item_modified_appearance_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    item_col: usize,
    appearance_col: usize,
    insert: &mut Statement<'_>,
) -> Result<(), String> {
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
        let item_id = fields
            .get(item_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let appearance_id = fields
            .get(appearance_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        if item_id == 0 || appearance_id == 0 {
            continue;
        }
        insert
            .execute((item_id, appearance_id))
            .map_err(|err| format!("insert item_modified_appearance_map row: {err}"))?;
    }
    Ok(())
}

fn populate_item_appearance_map(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let id_col = header_index(&headers, "ID", path)?;
    let display_info_col = header_index(&headers, "ItemDisplayInfoID", path)?;
    let mut insert = conn
        .prepare(
            "INSERT OR REPLACE INTO item_appearance_map (appearance_id, display_info_id)
             VALUES (?1, ?2)",
        )
        .map_err(|err| format!("prepare item_appearance_map insert: {err}"))?;
    insert_item_appearance_rows(&mut reader, path, id_col, display_info_col, &mut insert)?;
    Ok(())
}

fn insert_item_appearance_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    id_col: usize,
    display_info_col: usize,
    insert: &mut Statement<'_>,
) -> Result<(), String> {
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
        let appearance_id = fields
            .get(id_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let display_info_id = fields
            .get(display_info_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        if appearance_id == 0 || display_info_id == 0 {
            continue;
        }
        insert
            .execute((appearance_id, display_info_id))
            .map_err(|err| format!("insert item_appearance_map row: {err}"))?;
    }
    Ok(())
}

fn populate_display_info(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let id = header_index(&headers, "ID", path)?;
    let model_res_0 = header_index(&headers, "ModelResourcesID_0", path)?;
    let model_res_1 = header_index(&headers, "ModelResourcesID_1", path)?;
    let model_mat_res_0 = header_index(&headers, "ModelMaterialResourcesID_0", path)?;
    let model_mat_res_1 = header_index(&headers, "ModelMaterialResourcesID_1", path)?;
    let geoset_group_0 = header_index(&headers, "GeosetGroup_0", path)?;
    let geoset_group_1 = header_index(&headers, "GeosetGroup_1", path)?;
    let geoset_group_2 = header_index(&headers, "GeosetGroup_2", path)?;
    let helmet_vis_0 = header_index(&headers, "HelmetGeosetVis_0", path)?;
    let helmet_vis_1 = header_index(&headers, "HelmetGeosetVis_1", path)?;
    let mut insert = conn
        .prepare(
            "INSERT OR REPLACE INTO display_info (
            id, model_res_0, model_res_1, model_mat_res_0, model_mat_res_1,
            geoset_group_0, geoset_group_1, geoset_group_2, helmet_vis_0, helmet_vis_1
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .map_err(|err| format!("prepare display_info insert: {err}"))?;
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
        let display_id = fields
            .get(id)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        if display_id == 0 {
            continue;
        }
        let get_u32 = |idx: usize| {
            fields
                .get(idx)
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0)
        };
        let get_i16 = |idx: usize| {
            fields
                .get(idx)
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0) as i16
        };
        insert
            .execute((
                display_id,
                get_u32(model_res_0),
                get_u32(model_res_1),
                get_u32(model_mat_res_0),
                get_u32(model_mat_res_1),
                get_i16(geoset_group_0),
                get_i16(geoset_group_1),
                get_i16(geoset_group_2),
                get_u32(helmet_vis_0),
                get_u32(helmet_vis_1),
            ))
            .map_err(|err| format!("insert display_info row: {err}"))?;
    }
    Ok(())
}

fn populate_material_to_texture(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let file_data_col = header_index(&headers, "FileDataID", path)?;
    let usage_type_col = header_index(&headers, "UsageType", path)?;
    let material_col = header_index(&headers, "MaterialResourcesID", path)?;
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
    let mut insert = conn.prepare(
        "INSERT OR REPLACE INTO material_to_texture (material_resource_id, texture_fdid) VALUES (?1, ?2)"
    ).map_err(|err| format!("prepare material_to_texture insert: {err}"))?;
    for (material_resource_id, texture_fdid) in preferred {
        insert
            .execute((material_resource_id, texture_fdid))
            .map_err(|err| format!("insert material_to_texture row: {err}"))?;
    }
    Ok(())
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

fn populate_display_material_textures(conn: &Connection, path: &Path) -> Result<(), String> {
    let material_to_texture = load_material_to_texture_map(conn)?;
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let component_col = header_index(&headers, "ComponentSection", path)?;
    let material_col = header_index(&headers, "MaterialResourcesID", path)?;
    let display_info_col = header_index(&headers, "ItemDisplayInfoID", path)?;
    let mut insert = conn.prepare(
        "INSERT OR IGNORE INTO display_material_textures (display_info_id, component_section, texture_fdid) VALUES (?1, ?2, ?3)"
    ).map_err(|err| format!("prepare display_material_textures insert: {err}"))?;
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

fn populate_model_to_fdid(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let file_data_col = header_index(&headers, "FileDataID", path)?;
    let model_resource_col = header_index(&headers, "ModelResourcesID", path)?;
    let mut insert = conn.prepare(
        "INSERT INTO model_to_fdid (model_resource_id, model_order, file_data_id) VALUES (?1, ?2, ?3)"
    ).map_err(|err| format!("prepare model_to_fdid insert: {err}"))?;
    let mut seen = HashSet::new();
    let mut next_order: HashMap<u32, u32> = HashMap::new();
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
        let file_data_id = fields
            .get(file_data_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let model_resource_id = fields
            .get(model_resource_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        if file_data_id == 0
            || model_resource_id == 0
            || !seen.insert((model_resource_id, file_data_id))
        {
            continue;
        }
        let order = next_order.entry(model_resource_id).or_insert(0);
        insert
            .execute((model_resource_id, *order, file_data_id))
            .map_err(|err| format!("insert model_to_fdid row: {err}"))?;
        *order += 1;
    }
    Ok(())
}

fn imported_outfit_links_cache_path(data_dir: &Path) -> Result<PathBuf, String> {
    outfit_links_cache::imported_outfit_links_cache_path(data_dir)
}

pub fn load_cached_char_start_outfits(data_dir: &Path) -> Result<StarterOutfits, String> {
    outfit_cache_load::load_cached_char_start_outfits(data_dir)
}

pub fn resolve_cached_outfit_display_ids(
    data_dir: &Path,
    race: u8,
    class: u8,
    sex: u8,
) -> Result<Vec<u32>, String> {
    outfit_resolve::resolve_cached_outfit_display_ids(data_dir, race, class, sex)
}

pub(crate) fn load_cached_display_info(
    data_dir: &Path,
    display_info_id: u32,
) -> Result<Option<DisplayInfoResolved>, String> {
    outfit_query::load_cached_display_info(data_dir, display_info_id)
}

pub(crate) fn load_cached_material_texture_fdid(
    data_dir: &Path,
    material_resource_id: u32,
) -> Result<Option<u32>, String> {
    outfit_query::load_cached_material_texture_fdid(data_dir, material_resource_id)
}

pub(crate) fn load_cached_model_fdids(
    data_dir: &Path,
    model_resource_id: u32,
) -> Result<Vec<u32>, String> {
    outfit_query::load_cached_model_fdids(data_dir, model_resource_id)
}

pub(crate) fn resolve_cached_skin_fdids_for_model_fdid(
    data_dir: &Path,
    model_fdid: u32,
) -> Result<Option<[u32; 3]>, String> {
    outfit_query::resolve_cached_skin_fdids_for_model_fdid(data_dir, model_fdid)
}

pub(crate) fn resolve_cached_skin_fdids_for_model_name(
    data_dir: &Path,
    model_name: &str,
) -> Result<Option<[u32; 3]>, String> {
    outfit_query::resolve_cached_skin_fdids_for_model_name(data_dir, model_name)
}

pub fn load_cached_item_modified_appearance(data_dir: &Path) -> Result<HashMap<u32, u32>, String> {
    outfit_cache_load::load_cached_item_modified_appearance(data_dir)
}

pub fn load_cached_item_appearance(data_dir: &Path) -> Result<HashMap<u32, u32>, String> {
    outfit_cache_load::load_cached_item_appearance(data_dir)
}

#[cfg(test)]
pub(crate) fn load_cached_display_resources(
    data_dir: &Path,
) -> Result<CachedDisplayResources, String> {
    outfit_cache_load::load_cached_display_resources(data_dir)
}

#[cfg(test)]
#[path = "../../../tests/unit/world_db_tests.rs"]
mod tests;
