use std::io::BufRead;
use std::path::{Path, PathBuf};

use crate::csv_util::parse_csv_line_trimmed as parse_csv_line;
use crate::sqlite_util::is_missing_table_error;
use rusqlite::{Connection, OpenFlags};

pub(super) fn import_zone_name_cache() -> Result<PathBuf, String> {
    let cache_path = super::zone_names_cache_path();
    let csv_path = super::area_table_csv_path();
    if cache_path.exists() {
        let conn = super::open_read_only(&cache_path)?;
        if cache_is_fresh(&conn, &csv_path)? {
            return Ok(cache_path);
        }
    }
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    init_schema(&conn)?;
    record_source_file(&conn, &csv_path)?;
    import_zone_rows(&conn, &csv_path)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit area_names cache: {err}"))?;
    Ok(cache_path)
}

pub(super) fn load_zone_name(id: u32) -> Result<Option<String>, String> {
    let cache_path = super::zone_names_cache_path();
    if !cache_path.exists() {
        return Err(format!(
            "{} missing; run `cargo run --bin zone_name_cache_import` to build it",
            cache_path.display()
        ));
    }
    query_zone_name(&cache_path, id).map_err(|err| format!("query area_names {id}: {err}"))
}

fn cache_is_fresh(conn: &Connection, source_path: &Path) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT source, mtime_secs FROM source_files LIMIT 1") {
        Ok(stmt) => stmt,
        Err(err) if is_missing_table_error(&err) => {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare area_names source_files lookup: {err}")),
    };
    let row = match stmt.query_row([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    }) {
        Ok(row) => row,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
        Err(err) => return Err(format!("query area_names source_files: {err}")),
    };
    Ok(row.0 == source_path.to_string_lossy() && row.1 == super::csv_mtime(source_path)?)
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS source_files;
         DROP TABLE IF EXISTS area_names;
         CREATE TABLE source_files (
             source TEXT PRIMARY KEY,
             mtime_secs INTEGER NOT NULL
         );
         CREATE TABLE area_names (
             id INTEGER PRIMARY KEY,
             name TEXT NOT NULL
         );",
    )
    .map_err(|err| format!("init area_names cache: {err}"))
}

fn record_source_file(conn: &Connection, csv_path: &Path) -> Result<(), String> {
    conn.execute(
        "INSERT INTO source_files (source, mtime_secs) VALUES (?1, ?2)",
        (
            csv_path.to_string_lossy().to_string(),
            super::csv_mtime(csv_path)?,
        ),
    )
    .map_err(|err| format!("insert area_names source row: {err}"))?;
    Ok(())
}

fn import_zone_rows(conn: &Connection, csv_path: &Path) -> Result<(), String> {
    let mut reader = super::open_reader(csv_path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", csv_path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let id_col = super::header_index(&headers, "ID", csv_path)?;
    let name_col = super::header_index(&headers, "AreaName_lang", csv_path)?;
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
        insert_zone_row(
            &mut insert,
            line.trim_end_matches(['\r', '\n']),
            id_col,
            name_col,
        )?;
    }
    Ok(())
}

fn insert_zone_row(
    insert: &mut rusqlite::Statement<'_>,
    line: &str,
    id_col: usize,
    name_col: usize,
) -> Result<(), String> {
    let fields = parse_csv_line(line);
    let Some(id) = fields
        .get(id_col)
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return Ok(());
    };
    let Some(name) = fields.get(name_col).map(String::as_str) else {
        return Ok(());
    };
    if id == 0 || name.is_empty() {
        return Ok(());
    }
    insert
        .execute((id, name))
        .map_err(|err| format!("insert area_names row {id}: {err}"))?;
    Ok(())
}

fn query_zone_name(cache_path: &Path, id: u32) -> Result<Option<String>, rusqlite::Error> {
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

#[cfg(test)]
mod tests {
    #[test]
    fn zone_name_cache_import_reuses_fresh_cache() {
        let cache_path = super::import_zone_name_cache().expect("import zone name cache");
        let before = std::fs::metadata(&cache_path)
            .expect("stat zone name cache")
            .modified()
            .expect("zone name cache mtime");
        let reused_path = super::import_zone_name_cache().expect("reuse zone name cache");
        let after = std::fs::metadata(&reused_path)
            .expect("stat reused zone name cache")
            .modified()
            .expect("reused zone name cache mtime");
        assert_eq!(cache_path, reused_path);
        assert_eq!(before, after);
    }
}
