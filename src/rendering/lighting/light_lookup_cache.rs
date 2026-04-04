use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::cache_metadata::single_source_cache_is_fresh;
use crate::cache_source_mtime::csv_mtime;
use crate::csv_util::skip_csv_header;
use game_engine::paths;
use rusqlite::{Connection, OpenFlags};

use crate::light_lookup::LightEntry;

const LIGHT_CACHE_PATH: &str = "cache/light_lookup.sqlite";

pub(crate) fn load_light_entries(path: &Path) -> Result<Vec<LightEntry>, String> {
    let cache_path = ensure_light_cache(path)?;
    load_light_entries_from_sqlite(&cache_path)
}

pub(crate) fn load_light_entries_uncached(path: &Path) -> Result<Vec<LightEntry>, String> {
    let mut reader = open_reader(path)?;
    skip_csv_header(&mut reader, path)?;
    let mut entries = Vec::new();
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
        if let Some(entry) = parse_light_line(line.trim_end_matches(['\r', '\n'])) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn ensure_light_cache(source_path: &Path) -> Result<PathBuf, String> {
    let cache_path = paths::shared_data_path(LIGHT_CACHE_PATH);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    if !single_source_cache_is_fresh(&conn, source_path)? {
        rebuild_cache(&conn, source_path)?;
    }
    Ok(cache_path)
}

fn load_light_entries_from_sqlite(cache_path: &Path) -> Result<Vec<LightEntry>, String> {
    let conn = Connection::open_with_flags(
        cache_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    let mut stmt = prepare_light_query(&conn)?;
    collect_light_rows(&mut stmt)
}

fn prepare_light_query(conn: &Connection) -> Result<rusqlite::Statement<'_>, String> {
    conn.prepare(
        "SELECT id, map_id, pos_x, pos_y, pos_z, falloff_end,
                light_params_0, light_params_1, light_params_2, light_params_3,
                light_params_4, light_params_5, light_params_6, light_params_7
         FROM lights",
    )
    .map_err(|err| format!("prepare lights query: {err}"))
}

fn collect_light_rows(stmt: &mut rusqlite::Statement<'_>) -> Result<Vec<LightEntry>, String> {
    let rows = stmt
        .query_map([], |row| {
            Ok(LightEntry {
                id: row.get(0)?,
                map_id: row.get(1)?,
                position: [row.get(2)?, row.get(3)?, row.get(4)?],
                falloff_end: row.get(5)?,
                light_params_ids: [
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                ],
            })
        })
        .map_err(|err| format!("query lights: {err}"))?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(|err| format!("read lights row: {err}"))?);
    }
    Ok(entries)
}

fn rebuild_cache(conn: &Connection, source_path: &Path) -> Result<(), String> {
    init_cache_schema(conn)?;
    import_light_rows(conn, source_path)?;
    record_metadata(conn, source_path)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit light cache: {err}"))?;
    Ok(())
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS metadata;
         DROP TABLE IF EXISTS lights;
         CREATE TABLE metadata (
             source_path TEXT NOT NULL,
             source_mtime INTEGER NOT NULL
         );
         CREATE TABLE lights (
             id INTEGER PRIMARY KEY,
             map_id INTEGER NOT NULL,
             pos_x REAL NOT NULL,
             pos_y REAL NOT NULL,
             pos_z REAL NOT NULL,
             falloff_end REAL NOT NULL,
             light_params_0 INTEGER NOT NULL,
             light_params_1 INTEGER NOT NULL,
             light_params_2 INTEGER NOT NULL,
             light_params_3 INTEGER NOT NULL,
             light_params_4 INTEGER NOT NULL,
             light_params_5 INTEGER NOT NULL,
             light_params_6 INTEGER NOT NULL,
             light_params_7 INTEGER NOT NULL
         );",
    )
    .map_err(|err| format!("init light cache: {err}"))
}

fn import_light_rows(conn: &Connection, source_path: &Path) -> Result<(), String> {
    let mut reader = open_reader(source_path)?;
    skip_csv_header(&mut reader, source_path)?;
    let mut insert = conn
        .prepare(
            "INSERT OR REPLACE INTO lights
             (id, map_id, pos_x, pos_y, pos_z, falloff_end,
              light_params_0, light_params_1, light_params_2, light_params_3,
              light_params_4, light_params_5, light_params_6, light_params_7)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .map_err(|err| format!("prepare lights insert: {err}"))?;
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
        insert_light_row(&mut insert, line.trim_end_matches(['\r', '\n']))?;
    }
    Ok(())
}

fn insert_light_row(insert: &mut rusqlite::Statement<'_>, line: &str) -> Result<(), String> {
    let Some(entry) = parse_light_line(line) else {
        return Ok(());
    };
    insert
        .execute((
            entry.id,
            entry.map_id,
            entry.position[0],
            entry.position[1],
            entry.position[2],
            entry.falloff_end,
            entry.light_params_ids[0],
            entry.light_params_ids[1],
            entry.light_params_ids[2],
            entry.light_params_ids[3],
            entry.light_params_ids[4],
            entry.light_params_ids[5],
            entry.light_params_ids[6],
            entry.light_params_ids[7],
        ))
        .map_err(|err| format!("insert light row {}: {err}", entry.id))?;
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
    .map_err(|err| format!("insert light metadata: {err}"))?;
    Ok(())
}

fn parse_light_line(line: &str) -> Option<LightEntry> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() < 15 {
        return None;
    }
    Some(LightEntry {
        id: fields[0].parse().ok()?,
        position: [
            fields[1].parse().ok()?,
            fields[2].parse().ok()?,
            fields[3].parse().ok()?,
        ],
        falloff_end: fields[5].parse().ok()?,
        map_id: fields[6].parse().ok()?,
        light_params_ids: [
            fields[7].parse().ok()?,
            fields[8].parse().ok()?,
            fields[9].parse().ok()?,
            fields[10].parse().ok()?,
            fields[11].parse().ok()?,
            fields[12].parse().ok()?,
            fields[13].parse().ok()?,
            fields[14].parse().ok()?,
        ],
    })
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_light_entries_round_trips_cache() {
        let dir = game_engine::test_harness::temp_test_dir("light-cache");
        let csv_path = dir.join("Light.csv");
        std::fs::write(
            &csv_path,
            "ID,GameCoords_0,GameCoords_1,GameCoords_2,GameFalloffStart,GameFalloffEnd,ContinentID,LightParamsID_0,LightParamsID_1,LightParamsID_2,LightParamsID_3,LightParamsID_4,LightParamsID_5,LightParamsID_6,LightParamsID_7\n1,10,20,30,0,40,1643,11,12,13,14,15,16,17,18\n",
        )
        .unwrap();

        let rows = load_light_entries(&csv_path).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            LightEntry {
                id: 1,
                map_id: 1643,
                position: [10.0, 20.0, 30.0],
                falloff_end: 40.0,
                light_params_ids: [11, 12, 13, 14, 15, 16, 17, 18],
            }
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
