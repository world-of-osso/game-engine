use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::OnceLock;

use rusqlite::{Connection, OpenFlags};

fn world_db_path() -> PathBuf {
    if let Some(path) = std::env::var_os("GAME_SERVER_WORLD_DB") {
        PathBuf::from(path)
    } else {
        crate::paths::shared_repo_root()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("game-server")
            .join("data")
            .join("world.db")
    }
}

fn zone_names_cache_path() -> PathBuf {
    crate::paths::shared_data_path("cache/zone_names.sqlite")
}

fn area_table_csv_path() -> PathBuf {
    crate::paths::resolve_data_path("AreaTable.csv")
}

pub(crate) fn load_chr_race_prefixes() -> Result<HashMap<u8, String>, String> {
    let db_path = world_db_path();
    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", db_path.display()))?;

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

fn rebuild_zone_name_cache(cache_path: &std::path::Path) -> Result<(), String> {
    let csv_path = area_table_csv_path();
    let file = std::fs::File::open(&csv_path)
        .map_err(|err| format!("open {}: {err}", csv_path.display()))?;
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let _ = std::fs::remove_file(cache_path);
    let conn = Connection::open(cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE IF NOT EXISTS area_names (
             id INTEGER PRIMARY KEY,
             name TEXT NOT NULL
         );
         DELETE FROM area_names;",
    )
    .map_err(|err| format!("init area_names cache: {err}"))?;

    let mut reader = BufReader::new(file);
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", csv_path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let id_col = headers
        .iter()
        .position(|column| column == "ID")
        .ok_or_else(|| format!("{} missing ID column", csv_path.display()))?;
    let name_col = headers
        .iter()
        .position(|column| column == "AreaName_lang")
        .ok_or_else(|| format!("{} missing AreaName_lang column", csv_path.display()))?;

    let mut insert = conn
        .prepare("INSERT OR REPLACE INTO area_names (id, name) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare area_names insert: {err}"))?;

    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", csv_path.display()))?;
        if bytes == 0 {
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
            if cache_path.exists() {
                return Ok(cache_path);
            }
            rebuild_zone_name_cache(&cache_path)?;
            Ok(cache_path)
        })
        .clone()
}

pub fn load_zone_name(id: u32) -> Result<Option<String>, String> {
    fn query_zone_name(
        cache_path: &std::path::Path,
        id: u32,
    ) -> Result<Option<String>, rusqlite::Error> {
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
    match query_zone_name(&cache_path, id) {
        Ok(name) => Ok(name),
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("no such table") =>
        {
            rebuild_zone_name_cache(&cache_path)?;
            query_zone_name(&cache_path, id)
                .map_err(|err| format!("query area_names {id} after rebuild: {err}"))
        }
        Err(err) => Err(format!("query area_names {id}: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{load_chr_race_prefixes, load_zone_name};

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
}
