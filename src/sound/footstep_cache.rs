use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::cache_source_mtime::csv_mtime;
use crate::sqlite_util::is_missing_table_error;
use game_engine::paths;
use rusqlite::{Connection, OpenFlags};

const FOOTSTEP_CACHE_PATH: &str = "cache/footstep_listfile.sqlite";

pub(crate) fn load_cached_footstep_rows(source_path: &Path) -> Result<Vec<(u32, String)>, String> {
    let cache_path = ensure_footstep_cache(source_path)?;
    load_rows_from_sqlite(&cache_path)
}

pub(crate) fn load_footstep_rows_uncached(
    source_path: &Path,
) -> Result<Vec<(u32, String)>, String> {
    let file = std::fs::File::open(source_path)
        .map_err(|err| format!("open {}: {err}", source_path.display()))?;
    let reader = BufReader::new(file);
    let mut rows = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|err| format!("read {} row: {err}", source_path.display()))?;
        for (fdid, path) in super::parse_listfile_lines(&line) {
            if super::is_supported_footstep_path(path) {
                rows.push((fdid, path.to_string()));
            }
        }
    }
    Ok(rows)
}

fn ensure_footstep_cache(source_path: &Path) -> Result<PathBuf, String> {
    let cache_path = paths::shared_data_path(FOOTSTEP_CACHE_PATH);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }

    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    if !cache_is_fresh(&conn, source_path)? {
        rebuild_cache(&conn, source_path)?;
    }
    Ok(cache_path)
}

fn load_rows_from_sqlite(cache_path: &Path) -> Result<Vec<(u32, String)>, String> {
    let conn = Connection::open_with_flags(
        cache_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    let mut stmt = conn
        .prepare("SELECT fdid, path FROM footstep_files ORDER BY fdid")
        .map_err(|err| format!("prepare footstep_files query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|err| format!("query footstep_files: {err}"))?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|err| format!("read footstep_files row: {err}"))?);
    }
    Ok(values)
}

fn cache_is_fresh(conn: &Connection, source_path: &Path) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT source_path, source_mtime FROM metadata LIMIT 1") {
        Ok(stmt) => stmt,
        Err(err) if is_missing_table_error(&err) => {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare metadata query: {err}")),
    };
    let row = stmt.query_row([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    });
    let (recorded_path, recorded_mtime) = match row {
        Ok(row) => row,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
        Err(err) => return Err(format!("query metadata: {err}")),
    };
    Ok(recorded_path == source_path.to_string_lossy() && recorded_mtime == csv_mtime(source_path)?)
}

fn rebuild_cache(conn: &Connection, source_path: &Path) -> Result<(), String> {
    init_cache_schema(conn)?;
    import_rows(conn, source_path)?;
    record_metadata(conn, source_path)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit footstep cache: {err}"))?;
    Ok(())
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS metadata;
         DROP TABLE IF EXISTS footstep_files;
         CREATE TABLE metadata (
             source_path TEXT NOT NULL,
             source_mtime INTEGER NOT NULL
         );
         CREATE TABLE footstep_files (
             fdid INTEGER PRIMARY KEY,
             path TEXT NOT NULL
         );",
    )
    .map_err(|err| format!("init footstep cache: {err}"))
}

fn import_rows(conn: &Connection, source_path: &Path) -> Result<(), String> {
    let rows = load_footstep_rows_uncached(source_path)?;
    let mut insert = conn
        .prepare("INSERT OR REPLACE INTO footstep_files (fdid, path) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare footstep_files insert: {err}"))?;
    for (fdid, path) in rows {
        insert
            .execute((fdid, path))
            .map_err(|err| format!("insert footstep_files row {fdid}: {err}"))?;
    }
    Ok(())
}

fn record_metadata(conn: &Connection, source_path: &Path) -> Result<(), String> {
    conn.execute(
        "INSERT INTO metadata (source_path, source_mtime) VALUES (?1, ?2)",
        (
            source_path.to_string_lossy().to_string(),
            csv_mtime(source_path)?,
        ),
    )
    .map_err(|err| format!("insert footstep metadata: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_cached_footstep_rows_round_trips_cache() {
        let dir = game_engine::test_harness::temp_test_dir("footstep-cache");
        let csv_path = dir.join("community-listfile.csv");
        std::fs::write(
            &csv_path,
            "1;sound/character/footsteps/mfootsmallgrassa.ogg\n2;sound/creature/horse/horse_footstepa.ogg\n3;world/maps/test/test_1_2.adt\n",
        )
        .unwrap();

        let rows = load_cached_footstep_rows(&csv_path).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, 1);
        assert!(rows[0].1.contains("footsteps"));
        assert_eq!(rows[1].0, 2);
        assert!(rows[1].1.contains("horse_footstep"));

        let _ = std::fs::remove_dir_all(dir);
    }
}
