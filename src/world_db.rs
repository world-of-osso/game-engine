use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, OpenFlags};

use crate::outfit_data::{DisplayInfoResolved, DisplayMaterialTextures};

#[path = "world_db_zone_names.rs"]
mod zone_name_cache;

type OutfitKey = (u8, u8, u8);
type StarterOutfits = HashMap<OutfitKey, Vec<u32>>;

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
    let cache_path = outfit_links_cache_path();
    let csv_paths = required_outfit_csv_paths(data_dir);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let _ = std::fs::remove_file(&cache_path);
    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE source_files (source TEXT PRIMARY KEY, mtime_secs INTEGER NOT NULL);
         CREATE TABLE starter_outfits (
             race_id INTEGER NOT NULL,
             class_id INTEGER NOT NULL,
             sex_id INTEGER NOT NULL,
             item_order INTEGER NOT NULL,
             item_id INTEGER NOT NULL
         );
         CREATE TABLE item_modified_appearance_map (
             item_id INTEGER PRIMARY KEY,
             appearance_id INTEGER NOT NULL
         );
         CREATE TABLE item_appearance_map (
             appearance_id INTEGER PRIMARY KEY,
             display_info_id INTEGER NOT NULL
         );
         CREATE TABLE display_info (
             id INTEGER PRIMARY KEY,
             model_res_0 INTEGER NOT NULL,
             model_res_1 INTEGER NOT NULL,
             model_mat_res_0 INTEGER NOT NULL,
             model_mat_res_1 INTEGER NOT NULL,
             geoset_group_0 INTEGER NOT NULL,
             geoset_group_1 INTEGER NOT NULL,
             geoset_group_2 INTEGER NOT NULL,
             helmet_vis_0 INTEGER NOT NULL,
             helmet_vis_1 INTEGER NOT NULL
         );
         CREATE TABLE material_to_texture (
             material_resource_id INTEGER PRIMARY KEY,
             texture_fdid INTEGER NOT NULL
         );
         CREATE TABLE display_material_textures (
             display_info_id INTEGER NOT NULL,
             component_section INTEGER NOT NULL,
             texture_fdid INTEGER NOT NULL,
             PRIMARY KEY (display_info_id, component_section, texture_fdid)
         );
         CREATE TABLE model_to_fdid (
             model_resource_id INTEGER NOT NULL,
             model_order INTEGER NOT NULL,
             file_data_id INTEGER NOT NULL,
             PRIMARY KEY (model_resource_id, model_order)
         );",
    )
    .map_err(|err| format!("init outfit_links cache: {err}"))?;

    let mut source_insert = conn
        .prepare("INSERT INTO source_files (source, mtime_secs) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare source_files insert: {err}"))?;
    for path in &csv_paths {
        source_insert
            .execute((path.to_string_lossy().to_string(), csv_mtime(path)?))
            .map_err(|err| format!("insert source_files {}: {err}", path.display()))?;
    }

    populate_starter_outfits(&conn, &csv_paths[0])?;
    populate_item_modified_appearance_map(&conn, &csv_paths[1])?;
    populate_item_appearance_map(&conn, &csv_paths[2])?;
    populate_display_info(&conn, &csv_paths[3])?;
    populate_material_to_texture(&conn, &csv_paths[4])?;
    populate_display_material_textures(&conn, &csv_paths[5])?;
    populate_model_to_fdid(&conn, &csv_paths[6])?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit outfit_links cache: {err}"))?;
    Ok(cache_path)
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
    let cache_path = outfit_links_cache_path();
    if !cache_path.exists() {
        return Err(format!(
            "{} missing; run `cargo run --bin outfit_links_cache_import` to build it",
            cache_path.display()
        ));
    }
    let csv_paths = required_outfit_csv_paths(data_dir);
    let conn = open_read_only(&cache_path)?;
    if !outfit_cache_is_fresh(&conn, &csv_paths)? {
        return Err(format!(
            "{} is stale; run `cargo run --bin outfit_links_cache_import` to rebuild it",
            cache_path.display()
        ));
    }
    Ok(cache_path)
}

pub fn load_cached_char_start_outfits(data_dir: &Path) -> Result<StarterOutfits, String> {
    let cache_path = imported_outfit_links_cache_path(data_dir)?;
    let conn = open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT race_id, class_id, sex_id, item_id
             FROM starter_outfits
             ORDER BY race_id, class_id, sex_id, item_order",
        )
        .map_err(|err| format!("prepare starter_outfits lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u8>(0)?,
                row.get::<_, u8>(1)?,
                row.get::<_, u8>(2)?,
                row.get::<_, u32>(3)?,
            ))
        })
        .map_err(|err| format!("query starter_outfits: {err}"))?;
    let mut outfits = HashMap::new();
    for row in rows {
        let (race, class, sex, item_id) =
            row.map_err(|err| format!("read starter_outfits row: {err}"))?;
        outfits
            .entry((race, class, sex))
            .or_insert_with(Vec::new)
            .push(item_id);
    }
    Ok(outfits)
}

pub fn load_cached_item_modified_appearance(data_dir: &Path) -> Result<HashMap<u32, u32>, String> {
    let cache_path = imported_outfit_links_cache_path(data_dir)?;
    let conn = open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare("SELECT item_id, appearance_id FROM item_modified_appearance_map")
        .map_err(|err| format!("prepare item_modified_appearance_map lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query item_modified_appearance_map: {err}"))?;
    let mut map = HashMap::new();
    for row in rows {
        let (item_id, appearance_id) =
            row.map_err(|err| format!("read item_modified_appearance_map row: {err}"))?;
        map.insert(item_id, appearance_id);
    }
    Ok(map)
}

pub fn load_cached_item_appearance(data_dir: &Path) -> Result<HashMap<u32, u32>, String> {
    let cache_path = imported_outfit_links_cache_path(data_dir)?;
    let conn = open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare("SELECT appearance_id, display_info_id FROM item_appearance_map")
        .map_err(|err| format!("prepare item_appearance_map lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query item_appearance_map: {err}"))?;
    let mut map = HashMap::new();
    for row in rows {
        let (appearance_id, display_info_id) =
            row.map_err(|err| format!("read item_appearance_map row: {err}"))?;
        map.insert(appearance_id, display_info_id);
    }
    Ok(map)
}

pub(crate) fn load_cached_display_resources(
    data_dir: &Path,
) -> Result<CachedDisplayResources, String> {
    let cache_path = imported_outfit_links_cache_path(data_dir)?;
    let conn = open_read_only(&cache_path)?;

    let mut display_info_stmt = conn
        .prepare(
            "SELECT id, model_res_0, model_res_1, model_mat_res_0, model_mat_res_1,
                    geoset_group_0, geoset_group_1, geoset_group_2, helmet_vis_0, helmet_vis_1
             FROM display_info",
        )
        .map_err(|err| format!("prepare display_info lookup: {err}"))?;
    let display_rows = display_info_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, u32>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, i16>(5)?,
                row.get::<_, i16>(6)?,
                row.get::<_, i16>(7)?,
                row.get::<_, u32>(8)?,
                row.get::<_, u32>(9)?,
            ))
        })
        .map_err(|err| format!("query display_info: {err}"))?;
    let mut display_info = HashMap::new();
    for row in display_rows {
        let (id, mr0, mr1, mm0, mm1, gg0, gg1, gg2, hv0, hv1) =
            row.map_err(|err| format!("read display_info row: {err}"))?;
        let collect = |values: [u32; 2]| values.into_iter().filter(|v| *v != 0).collect::<Vec<_>>();
        display_info.insert(
            id,
            DisplayInfoResolved {
                item_textures: Vec::new(),
                geoset_overrides: Vec::new(),
                model_resource_ids: collect([mr0, mr1]),
                model_material_resource_ids: collect([mm0, mm1]),
                model_resource_columns: [mr0, mr1],
                model_material_resource_columns: [mm0, mm1],
                helmet_geoset_vis_ids: collect([hv0, hv1]),
                geoset_groups: [gg0, gg1, gg2, 0, 0, 0],
            },
        );
    }

    let material_to_texture = load_material_to_texture_map(&conn)?;

    let mut display_materials_stmt = conn
        .prepare(
            "SELECT display_info_id, component_section, texture_fdid
             FROM display_material_textures
             ORDER BY display_info_id, component_section, texture_fdid",
        )
        .map_err(|err| format!("prepare display_material_textures lookup: {err}"))?;
    let display_material_rows = display_materials_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u8>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })
        .map_err(|err| format!("query display_material_textures: {err}"))?;
    let mut direct = HashMap::new();
    for row in display_material_rows {
        let (display_info_id, component_section, texture_fdid) =
            row.map_err(|err| format!("read display_material_textures row: {err}"))?;
        direct
            .entry(display_info_id)
            .or_insert_with(Vec::new)
            .push((component_section, texture_fdid));
    }

    let mut model_stmt = conn
        .prepare(
            "SELECT model_resource_id, file_data_id
             FROM model_to_fdid
             ORDER BY model_resource_id, model_order",
        )
        .map_err(|err| format!("prepare model_to_fdid lookup: {err}"))?;
    let model_rows = model_stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query model_to_fdid: {err}"))?;
    let mut model_to_fdids = HashMap::new();
    for row in model_rows {
        let (model_resource_id, file_data_id) =
            row.map_err(|err| format!("read model_to_fdid row: {err}"))?;
        model_to_fdids
            .entry(model_resource_id)
            .or_insert_with(Vec::new)
            .push(file_data_id);
    }

    Ok(CachedDisplayResources {
        display_info,
        material_to_texture,
        display_materials: DisplayMaterialTextures { direct },
        model_to_fdids,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        import_outfit_links_cache, import_zone_name_cache, load_cached_char_start_outfits,
        load_cached_display_resources, load_cached_item_appearance,
        load_cached_item_modified_appearance, load_chr_race_prefixes, load_zone_name,
    };
    use std::path::Path;

    #[test]
    fn chr_race_prefixes_load_from_world_db() {
        let prefixes = load_chr_race_prefixes().expect("load chr_races prefixes from world.db");
        assert_eq!(prefixes.get(&1).map(String::as_str), Some("hu"));
    }

    #[test]
    fn zone_name_loads_from_area_table_cache() {
        import_zone_name_cache().expect("import zone name cache");
        assert_eq!(
            load_zone_name(12).expect("load zone name"),
            Some("Elwynn Forest".to_string())
        );
    }

    #[test]
    fn outfit_links_load_from_cache() {
        let data_dir = Path::new("data");
        import_outfit_links_cache(data_dir).expect("import outfit links cache");
        let outfits = load_cached_char_start_outfits(data_dir).expect("load starter_outfits cache");
        let item_to_appearance = load_cached_item_modified_appearance(data_dir)
            .expect("load item_modified_appearance cache");
        let appearance_to_display =
            load_cached_item_appearance(data_dir).expect("load item_appearance cache");

        assert!(
            !outfits.is_empty(),
            "starter_outfits cache should not be empty"
        );
        assert!(
            !item_to_appearance.is_empty(),
            "item_modified_appearance cache should not be empty"
        );
        assert!(
            !appearance_to_display.is_empty(),
            "item_appearance cache should not be empty"
        );
    }

    #[test]
    fn display_resources_load_from_cache() {
        let data_dir = Path::new("data");
        import_outfit_links_cache(data_dir).expect("import outfit links cache");
        let resources =
            load_cached_display_resources(data_dir).expect("load display resources cache");
        assert!(!resources.display_info.is_empty());
        assert!(!resources.material_to_texture.is_empty());
        assert!(!resources.model_to_fdids.is_empty());
    }
}
