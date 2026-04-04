use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::cache_source_mtime::csv_mtime;
use crate::cache_sqlite::open_read_only;
use crate::csv_util::parse_csv_line;
use crate::sqlite_util::is_missing_table_error;
use rusqlite::Connection;

type TracksByZone = HashMap<u32, Vec<usize>>;

fn csv_path() -> PathBuf {
    crate::paths::shared_data_path("music_zone_links.csv")
}

fn cache_path() -> PathBuf {
    crate::paths::shared_data_path("cache/music_zone_links.sqlite")
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn cache_is_fresh(conn: &Connection, source_path: &Path) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT source, mtime_secs FROM source_files") {
        Ok(stmt) => stmt,
        Err(err) if is_missing_table_error(&err) => {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare source_files lookup: {err}")),
    };
    let row = stmt
        .query_row([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|err| format!("query source_files: {err}"))?;
    Ok(row.0 == source_path.to_string_lossy() && row.1 == csv_mtime(source_path)?)
}

fn rebuild_cache(cache_path: &Path) -> Result<(), String> {
    let source_path = csv_path();
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let conn = Connection::open(cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS source_files;
         DROP TABLE IF EXISTS zone_music_links;
         CREATE TABLE source_files (source TEXT PRIMARY KEY, mtime_secs INTEGER NOT NULL);
         CREATE TABLE zone_music_links (
             file_data_id INTEGER NOT NULL,
             area_id INTEGER NOT NULL,
             PRIMARY KEY (file_data_id, area_id)
         );",
    )
    .map_err(|err| format!("init music_zone_links cache: {err}"))?;

    conn.execute(
        "INSERT INTO source_files (source, mtime_secs) VALUES (?1, ?2)",
        (
            source_path.to_string_lossy().to_string(),
            csv_mtime(&source_path)?,
        ),
    )
    .map_err(|err| format!("insert source_files row: {err}"))?;
    import_zone_music_rows(&conn, &source_path)?;

    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit music_zone_links cache: {err}"))?;
    Ok(())
}

fn import_zone_music_rows(conn: &Connection, source_path: &Path) -> Result<(), String> {
    let mut reader = open_reader(source_path)?;
    let mut insert = conn
        .prepare("INSERT OR IGNORE INTO zone_music_links (file_data_id, area_id) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare zone_music_links insert: {err}"))?;
    let mut line = String::new();
    let mut line_idx = 0usize;
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", source_path.display()))?
            == 0
        {
            break;
        }
        import_zone_music_row(&mut insert, line_idx, line.trim_end_matches(['\r', '\n']))?;
        line_idx += 1;
    }
    Ok(())
}

fn import_zone_music_row(
    insert: &mut rusqlite::Statement<'_>,
    line_idx: usize,
    line: &str,
) -> Result<(), String> {
    if line_idx == 0 || line.is_empty() {
        return Ok(());
    }
    let fields = parse_csv_line(line);
    if fields.len() < 12 || fields[2] != "1" {
        return Ok(());
    }
    let Ok(file_data_id) = fields[0].parse::<u32>() else {
        return Ok(());
    };
    let Ok(area_id) = fields[9].parse::<u32>() else {
        return Ok(());
    };
    insert
        .execute((file_data_id, area_id))
        .map_err(|err| format!("insert zone_music_links row ({file_data_id}, {area_id}): {err}"))?;
    Ok(())
}

pub fn import_sound_music_zone_cache() -> Result<PathBuf, String> {
    let source_path = csv_path();
    let cache_path = cache_path();
    let needs_rebuild = if cache_path.exists() {
        let conn = open_read_only(&cache_path)?;
        !cache_is_fresh(&conn, &source_path)?
    } else {
        true
    };
    if needs_rebuild {
        rebuild_cache(&cache_path)?;
    }
    Ok(cache_path)
}

pub fn load_zone_music_catalog(
    track_index_by_fdid: &HashMap<u32, usize>,
) -> Result<TracksByZone, String> {
    let cache_path = cache_path();
    if !cache_path.exists() {
        return Err(format!(
            "{} missing; run `cargo run --bin sound_music_zone_cache_import` to build it",
            cache_path.display()
        ));
    }
    let conn = open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT file_data_id, area_id FROM zone_music_links ORDER BY area_id, file_data_id",
        )
        .map_err(|err| format!("prepare zone_music_links lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query zone_music_links: {err}"))?;

    let mut by_zone: TracksByZone = HashMap::new();
    let mut seen: HashMap<u32, HashSet<usize>> = HashMap::new();
    for row in rows {
        let (file_data_id, area_id) =
            row.map_err(|err| format!("read zone_music_links row: {err}"))?;
        let Some(&track_idx) = track_index_by_fdid.get(&file_data_id) else {
            continue;
        };
        if seen.entry(area_id).or_default().insert(track_idx) {
            by_zone.entry(area_id).or_default().push(track_idx);
        }
    }
    Ok(by_zone)
}

#[cfg(test)]
mod tests {
    use super::{import_sound_music_zone_cache, load_zone_music_catalog};
    use std::collections::HashMap;

    #[test]
    fn sound_music_zone_catalog_loads_from_imported_cache() {
        import_sound_music_zone_cache().expect("import music zone cache");
        let mut track_index_by_fdid = HashMap::new();
        track_index_by_fdid.insert(936344, 7usize);
        let by_zone = load_zone_music_catalog(&track_index_by_fdid).expect("load zone music cache");
        assert_eq!(by_zone.get(&7210), Some(&vec![7]));
    }

    #[test]
    fn sound_music_zone_cache_import_reuses_fresh_cache() {
        let cache_path = import_sound_music_zone_cache().expect("import music zone cache");
        let before = std::fs::metadata(&cache_path)
            .expect("stat music zone cache")
            .modified()
            .expect("music zone cache mtime");
        let reused_path = import_sound_music_zone_cache().expect("reuse music zone cache");
        let after = std::fs::metadata(&reused_path)
            .expect("stat reused music zone cache")
            .modified()
            .expect("reused music zone cache mtime");
        assert_eq!(cache_path, reused_path);
        assert_eq!(before, after);
    }
}
