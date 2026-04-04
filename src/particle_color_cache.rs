use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::csv_util::parse_csv_line_trimmed as parse_csv_line;
use crate::sqlite_util::is_missing_table_error;
use rusqlite::{Connection, OpenFlags};

const PARTICLE_COLOR_CACHE_PATH: &str = "cache/particle_color.sqlite";
const PARTICLE_COLOR_CSV_FILE: &str = "ParticleColor.csv";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleColorRecord {
    pub id: u32,
    pub start: [i32; 3],
    pub mid: [i32; 3],
    pub end: [i32; 3],
}

pub fn particle_color_cache_path() -> PathBuf {
    crate::paths::shared_data_path(PARTICLE_COLOR_CACHE_PATH)
}

pub fn particle_color_csv_path() -> PathBuf {
    crate::paths::resolve_data_path(PARTICLE_COLOR_CSV_FILE)
}

pub fn import_particle_color_cache() -> Result<PathBuf, String> {
    let source_path = particle_color_csv_path();
    if !source_path.exists() {
        return Err(format!(
            "{} missing; provide ParticleColor.csv locally or run `cargo run --bin particle_color_cache_import -- --fetch-wago`",
            source_path.display()
        ));
    }
    import_particle_color_cache_from_source(&source_path)
}

pub fn import_particle_color_cache_from_source(source_path: &Path) -> Result<PathBuf, String> {
    let cache_path = particle_color_cache_path();
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }

    if cache_path.exists() {
        let conn = open_read_only(&cache_path)?;
        if cache_is_fresh(&conn, source_path)? {
            return Ok(cache_path);
        }
    }

    import_particle_color_cache_into(source_path, &cache_path)
}

pub fn query_particle_color(id: u32) -> Option<ParticleColorRecord> {
    let cache_path = particle_color_cache_path();
    let conn = open_read_only(&cache_path).ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT id,
                    start_0, start_1, start_2,
                    mid_0, mid_1, mid_2,
                    end_0, end_1, end_2
             FROM particle_colors
             WHERE id = ?1",
        )
        .ok()?;
    stmt.query_row([id], decode_particle_color_row).ok()
}

fn import_particle_color_cache_into(csv_path: &Path, cache_path: &Path) -> Result<PathBuf, String> {
    let conn = Connection::open(cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    rebuild_cache(&conn, csv_path)?;
    record_source_file(&conn, csv_path)?;
    Ok(cache_path.to_path_buf())
}

fn open_read_only(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", path.display()))
}

fn cache_is_fresh(conn: &Connection, source_path: &Path) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT source_path, source_mtime FROM metadata LIMIT 1") {
        Ok(stmt) => stmt,
        Err(err) if is_missing_table_error(&err) => return Ok(false),
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
    Ok(recorded_path == source_path.to_string_lossy()
        && recorded_mtime == source_mtime(source_path)?)
}

fn rebuild_cache(conn: &Connection, source_path: &Path) -> Result<(), String> {
    init_cache_schema(conn)?;
    import_rows(conn, source_path)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit particle color cache: {err}"))?;
    Ok(())
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS metadata;
         DROP TABLE IF EXISTS particle_colors;
         CREATE TABLE metadata (
             source_path TEXT NOT NULL,
             source_mtime INTEGER NOT NULL
         );
         CREATE TABLE particle_colors (
             id INTEGER PRIMARY KEY,
             start_0 INTEGER NOT NULL,
             start_1 INTEGER NOT NULL,
             start_2 INTEGER NOT NULL,
             mid_0 INTEGER NOT NULL,
             mid_1 INTEGER NOT NULL,
             mid_2 INTEGER NOT NULL,
             end_0 INTEGER NOT NULL,
             end_1 INTEGER NOT NULL,
             end_2 INTEGER NOT NULL
         );",
    )
    .map_err(|err| format!("init particle color cache: {err}"))
}

fn import_rows(conn: &Connection, source_path: &Path) -> Result<(), String> {
    let mut reader = open_reader(source_path)?;
    let columns = read_import_columns(&mut reader, source_path)?;
    let mut insert = prepare_particle_color_insert(conn)?;
    import_particle_color_rows(&mut reader, &mut insert, &columns, source_path)?;
    Ok(())
}

fn read_import_columns<R: BufRead>(
    reader: &mut R,
    source_path: &Path,
) -> Result<[usize; 10], String> {
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", source_path.display()))?;
    resolve_column_indices(
        parse_csv_line(header.trim_end_matches(['\r', '\n'])).as_slice(),
        source_path,
    )
}

fn prepare_particle_color_insert(conn: &Connection) -> Result<rusqlite::Statement<'_>, String> {
    conn.prepare(
        "INSERT OR REPLACE INTO particle_colors
         (id, start_0, start_1, start_2, mid_0, mid_1, mid_2, end_0, end_1, end_2)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .map_err(|err| format!("prepare particle_colors insert: {err}"))
}

fn import_particle_color_rows<R: BufRead>(
    reader: &mut R,
    insert: &mut rusqlite::Statement<'_>,
    columns: &[usize; 10],
    source_path: &Path,
) -> Result<(), String> {
    let mut line = String::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", source_path.display()))?
            == 0
        {
            break;
        }
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        if let Some(record) = parse_particle_color_row(&fields, columns, source_path)? {
            insert
                .execute((
                    record.id,
                    record.start[0],
                    record.start[1],
                    record.start[2],
                    record.mid[0],
                    record.mid[1],
                    record.mid[2],
                    record.end[0],
                    record.end[1],
                    record.end[2],
                ))
                .map_err(|err| format!("insert particle color {}: {err}", record.id))?;
        }
    }
    Ok(())
}

fn record_source_file(conn: &Connection, source_path: &Path) -> Result<(), String> {
    conn.execute(
        "INSERT INTO metadata (source_path, source_mtime) VALUES (?1, ?2)",
        (
            source_path.to_string_lossy().to_string(),
            source_mtime(source_path)?,
        ),
    )
    .map_err(|err| format!("insert particle color metadata: {err}"))?;
    Ok(())
}

fn resolve_column_indices(headers: &[String], path: &Path) -> Result<[usize; 10], String> {
    Ok([
        header_index(headers, "ID", path)?,
        header_index(headers, "Start_0", path)?,
        header_index(headers, "Start_1", path)?,
        header_index(headers, "Start_2", path)?,
        header_index(headers, "MID_0", path)?,
        header_index(headers, "MID_1", path)?,
        header_index(headers, "MID_2", path)?,
        header_index(headers, "End_0", path)?,
        header_index(headers, "End_1", path)?,
        header_index(headers, "End_2", path)?,
    ])
}

fn parse_particle_color_row(
    fields: &[String],
    columns: &[usize; 10],
    path: &Path,
) -> Result<Option<ParticleColorRecord>, String> {
    let max_index = columns.iter().copied().max().unwrap_or(0);
    if fields.len() <= max_index {
        return Ok(None);
    }
    Ok(Some(ParticleColorRecord {
        id: parse_u32_field(fields, columns[0], path, "ID")?,
        start: [
            parse_i32_field(fields, columns[1], path, "Start_0")?,
            parse_i32_field(fields, columns[2], path, "Start_1")?,
            parse_i32_field(fields, columns[3], path, "Start_2")?,
        ],
        mid: [
            parse_i32_field(fields, columns[4], path, "MID_0")?,
            parse_i32_field(fields, columns[5], path, "MID_1")?,
            parse_i32_field(fields, columns[6], path, "MID_2")?,
        ],
        end: [
            parse_i32_field(fields, columns[7], path, "End_0")?,
            parse_i32_field(fields, columns[8], path, "End_1")?,
            parse_i32_field(fields, columns[9], path, "End_2")?,
        ],
    }))
}

fn decode_particle_color_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ParticleColorRecord> {
    Ok(ParticleColorRecord {
        id: row.get(0)?,
        start: [row.get(1)?, row.get(2)?, row.get(3)?],
        mid: [row.get(4)?, row.get(5)?, row.get(6)?],
        end: [row.get(7)?, row.get(8)?, row.get(9)?],
    })
}

fn header_index(headers: &[String], column: &str, path: &Path) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header == column)
        .ok_or_else(|| format!("{} missing {column} column", path.display()))
}

fn parse_u32_field(
    fields: &[String],
    index: usize,
    path: &Path,
    name: &str,
) -> Result<u32, String> {
    fields
        .get(index)
        .ok_or_else(|| format!("{} missing {name} field", path.display()))?
        .parse()
        .map_err(|err| format!("parse {} {name}: {err}", path.display()))
}

fn parse_i32_field(
    fields: &[String],
    index: usize,
    path: &Path,
    name: &str,
) -> Result<i32, String> {
    fields
        .get(index)
        .ok_or_else(|| format!("{} missing {name} field", path.display()))?
        .parse()
        .map_err(|err| format!("parse {} {name}: {err}", path.display()))
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn source_mtime(path: &Path) -> Result<i64, String> {
    let modified = std::fs::metadata(path)
        .map_err(|err| format!("stat {}: {err}", path.display()))?
        .modified()
        .map_err(|err| format!("mtime {}: {err}", path.display()))?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("mtime epoch {}: {err}", path.display()))?
        .as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_color_cache_round_trips_local_csv() {
        let dir = crate::test_harness::temp_test_dir("particle-color-cache");
        let csv_path = dir.join("ParticleColor.csv");
        let cache_path = dir.join("particle_color.sqlite");
        std::fs::write(
            &csv_path,
            "ID,Start_0,Start_1,Start_2,MID_0,MID_1,MID_2,End_0,End_1,End_2\n281,-16777216,-65281,-65281,-16777216,-65281,-65281,-16777216,-65281,-65281\n",
        )
        .unwrap();

        import_particle_color_cache_into(&csv_path, &cache_path)
            .expect("import particle color cache");

        let conn = open_read_only(&cache_path).expect("open cache");
        let row = conn
            .query_row(
                "SELECT id, start_0, start_1, start_2, mid_0, mid_1, mid_2, end_0, end_1, end_2 FROM particle_colors WHERE id = 281",
                [],
                decode_particle_color_row,
            )
            .expect("query particle color");
        assert_eq!(
            row,
            ParticleColorRecord {
                id: 281,
                start: [-16777216, -65281, -65281],
                mid: [-16777216, -65281, -65281],
                end: [-16777216, -65281, -65281],
            }
        );
    }

    #[test]
    fn particle_color_cache_import_reuses_fresh_cache() {
        let dir = crate::test_harness::temp_test_dir("particle-color-reuse");
        let csv_path = dir.join("ParticleColor.csv");
        let cache_path = dir.join("particle_color.sqlite");
        std::fs::write(
            &csv_path,
            "ID,Start_0,Start_1,Start_2,MID_0,MID_1,MID_2,End_0,End_1,End_2\n281,-1,-2,-3,-4,-5,-6,-7,-8,-9\n",
        )
        .unwrap();

        import_particle_color_cache_into(&csv_path, &cache_path)
            .expect("import particle color cache");
        let before = std::fs::metadata(&cache_path)
            .expect("stat cache")
            .modified()
            .expect("cache mtime");

        let conn = open_read_only(&cache_path).expect("open cache");
        assert!(cache_is_fresh(&conn, &csv_path).expect("cache freshness"));

        let after = std::fs::metadata(&cache_path)
            .expect("stat cache")
            .modified()
            .expect("cache mtime");
        assert_eq!(before, after);
    }
}
