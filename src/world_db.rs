use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, OpenFlags};

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
    let secs = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("mtime epoch {}: {err}", path.display()))?
        .as_secs() as i64;
    Ok(secs)
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

fn rebuild_zone_name_cache(cache_path: &Path) -> Result<(), String> {
    let csv_path = area_table_csv_path();
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let _ = std::fs::remove_file(cache_path);
    let conn = Connection::open(cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE area_names (
             id INTEGER PRIMARY KEY,
             name TEXT NOT NULL
         );",
    )
    .map_err(|err| format!("init area_names cache: {err}"))?;

    let mut reader = open_reader(&csv_path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", csv_path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let id_col = header_index(&headers, "ID", &csv_path)?;
    let name_col = header_index(&headers, "AreaName_lang", &csv_path)?;
    let mut insert = conn
        .prepare("INSERT OR REPLACE INTO area_names (id, name) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare area_names insert: {err}"))?;

    let mut line = String::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", csv_path.display()))?
            == 0
        {
            break;
        }
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        let Some(id) = fields
            .get(id_col)
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        let Some(name) = fields.get(name_col).map(String::as_str) else {
            continue;
        };
        if id == 0 || name.is_empty() {
            continue;
        }
        insert
            .execute((id, name))
            .map_err(|err| format!("insert area_names row {id}: {err}"))?;
    }

    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit area_names cache: {err}"))?;
    Ok(())
}

fn ensure_zone_name_cache() -> Result<PathBuf, String> {
    static CACHE_INIT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    CACHE_INIT
        .get_or_init(|| {
            let cache_path = zone_names_cache_path();
            if !cache_path.exists() {
                rebuild_zone_name_cache(&cache_path)?;
            }
            Ok(cache_path)
        })
        .clone()
}

pub fn load_zone_name(id: u32) -> Result<Option<String>, String> {
    fn query(cache_path: &Path, id: u32) -> Result<Option<String>, rusqlite::Error> {
        let conn = Connection::open_with_flags(
            cache_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let mut stmt = conn.prepare("SELECT name FROM area_names WHERE id = ?1 LIMIT 1")?;
        match stmt.query_row([id], |row| row.get::<_, String>(0)) {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err),
        }
    }

    let cache_path = ensure_zone_name_cache()?;
    match query(&cache_path, id) {
        Ok(name) => Ok(name),
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("no such table") =>
        {
            rebuild_zone_name_cache(&cache_path)?;
            query(&cache_path, id)
                .map_err(|err| format!("query area_names {id} after rebuild: {err}"))
        }
        Err(err) => Err(format!("query area_names {id}: {err}")),
    }
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

fn rebuild_outfit_links_cache(cache_path: &Path, data_dir: &Path) -> Result<(), String> {
    let char_start_outfit = data_dir.join("CharStartOutfit.csv");
    let item_modified_appearance = data_dir.join("ItemModifiedAppearance.csv");
    let item_appearance = data_dir.join("ItemAppearance.csv");
    let csv_paths = [
        char_start_outfit.clone(),
        item_modified_appearance.clone(),
        item_appearance.clone(),
    ];

    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let _ = std::fs::remove_file(cache_path);
    let conn = Connection::open(cache_path)
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

    populate_starter_outfits(&conn, &char_start_outfit)?;
    populate_item_modified_appearance_map(&conn, &item_modified_appearance)?;
    populate_item_appearance_map(&conn, &item_appearance)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit outfit_links cache: {err}"))?;
    Ok(())
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

type OutfitKey = (u8, u8, u8);

type StarterOutfits = HashMap<OutfitKey, Vec<u32>>;

fn ensure_outfit_links_cache(data_dir: &Path) -> Result<PathBuf, String> {
    let cache_path = outfit_links_cache_path();
    let csv_paths = [
        data_dir.join("CharStartOutfit.csv"),
        data_dir.join("ItemModifiedAppearance.csv"),
        data_dir.join("ItemAppearance.csv"),
    ];
    let stale = if cache_path.exists() {
        let conn = open_read_only(&cache_path)?;
        !outfit_cache_is_fresh(&conn, &csv_paths)?
    } else {
        true
    };
    if stale {
        rebuild_outfit_links_cache(&cache_path, data_dir)?;
    }
    Ok(cache_path)
}

pub fn load_cached_char_start_outfits(data_dir: &Path) -> Result<StarterOutfits, String> {
    let cache_path = ensure_outfit_links_cache(data_dir)?;
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
    let cache_path = ensure_outfit_links_cache(data_dir)?;
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
    let cache_path = ensure_outfit_links_cache(data_dir)?;
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

#[cfg(test)]
mod tests {
    use super::{
        load_cached_char_start_outfits, load_cached_item_appearance,
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
        assert_eq!(
            load_zone_name(12).expect("load zone name"),
            Some("Elwynn Forest".to_string())
        );
    }

    #[test]
    fn outfit_links_load_from_cache() {
        let data_dir = Path::new("data");
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
}
