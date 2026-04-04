use std::path::Path;

use rusqlite::Connection;

use crate::cache_source_mtime::csv_mtime;
use crate::sqlite_util::is_missing_table_error;

pub fn single_source_cache_is_fresh(conn: &Connection, source_path: &Path) -> Result<bool, String> {
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
