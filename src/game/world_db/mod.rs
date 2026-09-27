use crate::cache_source_mtime::csv_mtime;
use crate::cache_sqlite::open_read_only;
use crate::csv_util::{header_index, parse_csv_line_trimmed as parse_csv_line};
use crate::sqlite_util::is_missing_table_error;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
#[path = "zone_names_cache.rs"]
mod zone_name_cache;
#[cfg(test)]
pub(crate) use crate::outfit_catalog_db::load_cached_display_resources;
pub use crate::outfit_catalog_db::{
    import_outfit_links_cache, load_cached_char_start_outfits, load_cached_item_appearance,
    load_cached_item_modified_appearance, resolve_cached_outfit_display_ids,
};
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

fn area_table_csv_path() -> PathBuf {
    crate::paths::resolve_data_path("AreaTable.csv")
}

fn chr_races_csv_path() -> PathBuf {
    crate::paths::resolve_data_path("ChrRaces.csv")
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

pub(crate) fn load_chr_race_prefixes() -> Result<HashMap<u8, String>, String> {
    let db_path = world_db_path();
    let conn = open_read_only(&db_path)?;
    let mut stmt = match conn.prepare(
        "SELECT id, client_prefix
         FROM chr_races
         WHERE id > 0
           AND client_prefix IS NOT NULL
           AND client_prefix != ''",
    ) {
        Ok(stmt) => stmt,
        Err(err) if is_missing_table_error(&err) => return load_chr_race_prefixes_from_csv(),
        Err(err) => return Err(format!("prepare chr_races query: {err}")),
    };
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
        return load_chr_race_prefixes_from_csv();
    }
    Ok(prefixes)
}

fn load_chr_race_prefixes_from_csv() -> Result<HashMap<u8, String>, String> {
    let path = chr_races_csv_path();
    let mut reader = open_reader(&path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let id_col = header_index(&headers, "ID", &path)?;
    let prefix_col = header_index(&headers, "ClientPrefix", &path)?;
    collect_chr_race_prefix_rows(&mut reader, &path, id_col, prefix_col)
}

fn collect_chr_race_prefix_rows<R: BufRead>(
    reader: &mut R,
    path: &Path,
    id_col: usize,
    prefix_col: usize,
) -> Result<HashMap<u8, String>, String> {
    let mut prefixes = HashMap::new();
    for line in reader.lines() {
        let line = line.map_err(|err| format!("read {} row: {err}", path.display()))?;
        let fields = parse_csv_line(&line);
        let Some(id) = fields
            .get(id_col)
            .and_then(|value| value.parse::<u8>().ok())
        else {
            continue;
        };
        let Some(prefix) = fields.get(prefix_col) else {
            continue;
        };
        let prefix = prefix.trim().to_ascii_lowercase();
        if !prefix.is_empty() {
            prefixes.insert(id, prefix);
        }
    }
    if prefixes.is_empty() {
        return Err(format!("{} returned no ClientPrefix rows", path.display()));
    }
    Ok(prefixes)
}

pub fn import_zone_name_cache() -> Result<PathBuf, String> {
    zone_name_cache::import_zone_name_cache()
}

pub fn load_zone_name(id: u32) -> Result<Option<String>, String> {
    zone_name_cache::load_zone_name(id)
}

pub fn load_area_parents() -> Result<HashMap<u32, u32>, String> {
    zone_name_cache::load_area_parents()
}

#[cfg(test)]
#[path = "../../../tests/unit/world_db_tests.rs"]
mod tests;
