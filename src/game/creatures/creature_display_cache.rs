use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[cfg(test)]
use std::time::UNIX_EPOCH;

use crate::cache_source_mtime::csv_mtime;
use crate::sqlite_util::is_missing_table_error;
use game_engine::paths;
use rusqlite::{Connection, OpenFlags};

use crate::creature_display::CreatureDisplay;

const CREATURE_DISPLAY_CACHE_PATH: &str = "cache/creature_display.sqlite";

#[derive(Clone, Copy)]
struct CreatureModelData {
    fdid: u32,
    scale_milli: u32,
}

struct DisplayInfoColumns {
    id: usize,
    model: usize,
    scale: usize,
    tex_var: [usize; 3],
}

pub(crate) fn creature_display_cache_path() -> PathBuf {
    paths::shared_data_path(CREATURE_DISPLAY_CACHE_PATH)
}

pub(crate) fn import_creature_display_cache() -> Result<PathBuf, String> {
    let di = paths::resolve_data_path("CreatureDisplayInfo.csv");
    let md = paths::resolve_data_path("CreatureModelData.csv");
    if !di.exists() || !md.exists() {
        return Err(format!(
            "Creature display CSVs not found: {} / {}",
            di.display(),
            md.display()
        ));
    }

    let cache_path = creature_display_cache_path();
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }

    let source_paths = [di.clone(), md.clone()];
    if cache_path.exists() {
        let conn = open_read_only(&cache_path)?;
        if cache_is_fresh(&conn, &source_paths)? {
            return Ok(cache_path);
        }
    }

    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    rebuild_cache(&conn, &di, &md)?;
    build_preferred_skins(&conn)?;
    record_source_files(&conn, &source_paths)?;
    Ok(cache_path)
}

pub(crate) fn query_display(display_id: u32) -> Option<CreatureDisplay> {
    let cache_path = creature_display_cache_path();
    let conn = open_read_only(&cache_path).ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT model_fdid, skin_fdid_0, skin_fdid_1, skin_fdid_2, scale_milli
             FROM creature_displays WHERE display_id = ?1",
        )
        .ok()?;
    stmt.query_row([display_id], |row| {
        Ok(CreatureDisplay {
            model_fdid: row.get(0)?,
            skin_fdids: [row.get(1)?, row.get(2)?, row.get(3)?],
            scale_milli: row.get(4)?,
        })
    })
    .ok()
}

pub(crate) fn query_preferred_skins(model_fdid: u32) -> Option<[u32; 3]> {
    let cache_path = creature_display_cache_path();
    let conn = open_read_only(&cache_path).ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT skin_fdid_0, skin_fdid_1, skin_fdid_2
             FROM preferred_skins WHERE model_fdid = ?1",
        )
        .ok()?;
    stmt.query_row([model_fdid], |row| {
        Ok([row.get(0)?, row.get(1)?, row.get(2)?])
    })
    .ok()
}

/// Return all distinct model FDIDs in the cache.
///
/// Used by the named-model fallback path to match local filenames against
/// listfile entries. Only called once per unique model name (result is cached
/// in named-model-lookups.sqlite).
pub(crate) fn query_distinct_model_fdids() -> Vec<u32> {
    let cache_path = creature_display_cache_path();
    let conn = match open_read_only(&cache_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut stmt = match conn.prepare("SELECT DISTINCT model_fdid FROM creature_displays") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = match stmt.query_map([], |row| row.get::<_, u32>(0)) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    rows.filter_map(|r| r.ok()).collect()
}

fn open_read_only(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", path.display()))
}

fn cache_is_fresh(conn: &Connection, source_paths: &[PathBuf]) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT path, mtime FROM source_files") {
        Ok(stmt) => stmt,
        Err(err) if is_missing_table_error(&err) => {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare source_files query: {err}")),
    };
    // Also check that preferred_skins table exists (older caches lack it).
    if conn
        .prepare("SELECT 1 FROM preferred_skins LIMIT 0")
        .is_err()
    {
        return Ok(false);
    }
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|err| format!("query source_files: {err}"))?;
    let mut recorded = HashMap::new();
    for row in rows {
        let (path, mtime) = row.map_err(|err| format!("read source_files row: {err}"))?;
        recorded.insert(path, mtime);
    }
    for path in source_paths {
        let key = path.to_string_lossy().to_string();
        if recorded.get(&key).copied() != Some(csv_mtime(path)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn rebuild_cache(
    conn: &Connection,
    display_info_path: &Path,
    model_data_path: &Path,
) -> Result<(), String> {
    init_cache_schema(conn)?;
    let model_data = parse_model_data(model_data_path)?;
    import_display_rows(conn, display_info_path, &model_data)?;
    Ok(())
}

fn build_preferred_skins(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "INSERT INTO preferred_skins (model_fdid, skin_fdid_0, skin_fdid_1, skin_fdid_2)
         SELECT model_fdid,
                skin_fdid_0, skin_fdid_1, skin_fdid_2
         FROM (
             SELECT model_fdid,
                    skin_fdid_0, skin_fdid_1, skin_fdid_2,
                    (CASE WHEN skin_fdid_0 != 0 THEN 1 ELSE 0 END
                   + CASE WHEN skin_fdid_1 != 0 THEN 1 ELSE 0 END
                   + CASE WHEN skin_fdid_2 != 0 THEN 1 ELSE 0 END) AS filled,
                    display_id,
                    ROW_NUMBER() OVER (
                        PARTITION BY model_fdid
                        ORDER BY
                            (CASE WHEN skin_fdid_0 != 0 THEN 1 ELSE 0 END
                           + CASE WHEN skin_fdid_1 != 0 THEN 1 ELSE 0 END
                           + CASE WHEN skin_fdid_2 != 0 THEN 1 ELSE 0 END) DESC,
                            display_id ASC
                    ) AS rn
             FROM creature_displays
             WHERE skin_fdid_0 != 0 OR skin_fdid_1 != 0 OR skin_fdid_2 != 0
         )
         WHERE rn = 1;
         COMMIT;",
    )
    .map_err(|err| format!("build preferred_skins: {err}"))
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS source_files;
         DROP TABLE IF EXISTS creature_displays;
         DROP TABLE IF EXISTS preferred_skins;
         CREATE TABLE source_files (
             path TEXT PRIMARY KEY,
             mtime INTEGER NOT NULL
         );
         CREATE TABLE creature_displays (
             display_id INTEGER PRIMARY KEY,
             model_fdid INTEGER NOT NULL,
             skin_fdid_0 INTEGER NOT NULL,
             skin_fdid_1 INTEGER NOT NULL,
             skin_fdid_2 INTEGER NOT NULL,
             scale_milli INTEGER NOT NULL
         );
         CREATE INDEX idx_creature_displays_model_fdid
             ON creature_displays(model_fdid);
         CREATE TABLE preferred_skins (
             model_fdid INTEGER PRIMARY KEY,
             skin_fdid_0 INTEGER NOT NULL,
             skin_fdid_1 INTEGER NOT NULL,
             skin_fdid_2 INTEGER NOT NULL
         );",
    )
    .map_err(|err| format!("init creature display cache: {err}"))
}

fn record_source_files(conn: &Connection, source_paths: &[PathBuf]) -> Result<(), String> {
    let mut insert = conn
        .prepare("INSERT OR REPLACE INTO source_files (path, mtime) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare source_files insert: {err}"))?;
    for path in source_paths {
        insert
            .execute((path.to_string_lossy().to_string(), csv_mtime(path)?))
            .map_err(|err| format!("insert source file {}: {err}", path.display()))?;
    }
    Ok(())
}

fn import_display_rows(
    conn: &Connection,
    display_info_path: &Path,
    model_data: &HashMap<u32, CreatureModelData>,
) -> Result<(), String> {
    let mut reader = open_reader(display_info_path)?;
    let cols = read_display_columns(&mut reader, display_info_path)?;
    let mut insert = prepare_display_insert(conn)?;
    let mut line = String::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", display_info_path.display()))?
            == 0
        {
            break;
        }
        insert_display_row(
            &mut insert,
            line.trim_end_matches(['\r', '\n']),
            &cols,
            model_data,
        )?;
    }
    Ok(())
}

fn read_display_columns(
    reader: &mut BufReader<std::fs::File>,
    display_info_path: &Path,
) -> Result<DisplayInfoColumns, String> {
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", display_info_path.display()))?;
    find_display_info_columns(header.trim_end_matches(['\r', '\n']))
        .ok_or_else(|| format!("{} missing required columns", display_info_path.display()))
}

fn prepare_display_insert(conn: &Connection) -> Result<rusqlite::Statement<'_>, String> {
    conn.prepare(
        "INSERT OR REPLACE INTO creature_displays
         (display_id, model_fdid, skin_fdid_0, skin_fdid_1, skin_fdid_2, scale_milli)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .map_err(|err| format!("prepare creature_displays insert: {err}"))
}

fn insert_display_row(
    insert: &mut rusqlite::Statement<'_>,
    line: &str,
    cols: &DisplayInfoColumns,
    model_data: &HashMap<u32, CreatureModelData>,
) -> Result<(), String> {
    let Some((display_id, entry)) = parse_display_entry(line, cols, model_data) else {
        return Ok(());
    };
    insert
        .execute((
            display_id,
            entry.model_fdid,
            entry.skin_fdids[0],
            entry.skin_fdids[1],
            entry.skin_fdids[2],
            entry.scale_milli,
        ))
        .map_err(|err| format!("insert creature display row {display_id}: {err}"))?;
    Ok(())
}

fn parse_model_data(path: &Path) -> Result<HashMap<u32, CreatureModelData>, String> {
    let mut map = HashMap::new();
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers: Vec<&str> = header.trim_end_matches(['\r', '\n']).split(',').collect();
    let id_col = header_index(&headers, "ID", path)?;
    let fdid_col = header_index(&headers, "FileDataID", path)?;
    let scale_col = headers.iter().position(|col| col.trim() == "ModelScale");

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
        insert_model_entry(
            &mut map,
            line.trim_end_matches(['\r', '\n']),
            id_col,
            fdid_col,
            scale_col,
        );
    }
    Ok(map)
}

fn insert_model_entry(
    map: &mut HashMap<u32, CreatureModelData>,
    line: &str,
    id_col: usize,
    fdid_col: usize,
    scale_col: Option<usize>,
) {
    let cols: Vec<&str> = line.split(',').collect();
    let Some(id) = cols.get(id_col).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    let Some(fdid) = cols.get(fdid_col).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    if fdid == 0 {
        return;
    }
    map.insert(
        id,
        CreatureModelData {
            fdid,
            scale_milli: parse_scale_milli(scale_col.and_then(|idx| cols.get(idx).copied())),
        },
    );
}

fn find_display_info_columns(header: &str) -> Option<DisplayInfoColumns> {
    let headers: Vec<&str> = header.split(',').collect();
    let find = |name: &str| headers.iter().position(|h| h.trim() == name);
    Some(DisplayInfoColumns {
        id: find("ID")?,
        model: find("ModelID")?,
        scale: find("CreatureModelScale")?,
        tex_var: [
            find("TextureVariationFileDataID_0")?,
            find("TextureVariationFileDataID_1")?,
            find("TextureVariationFileDataID_2")?,
        ],
    })
}

fn parse_display_entry(
    line: &str,
    cols: &DisplayInfoColumns,
    model_data: &HashMap<u32, CreatureModelData>,
) -> Option<(u32, CreatureDisplay)> {
    let values: Vec<&str> = line.split(',').collect();
    let display_id = values.get(cols.id)?.parse::<u32>().ok()?;
    let model_id = values.get(cols.model)?.parse::<u32>().ok()?;
    let model = model_data.get(&model_id)?;
    let parse_fdid = |idx: usize| {
        values
            .get(idx)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let display_scale_milli = parse_scale_milli(values.get(cols.scale).copied());
    Some((
        display_id,
        CreatureDisplay {
            model_fdid: model.fdid,
            skin_fdids: [
                parse_fdid(cols.tex_var[0]),
                parse_fdid(cols.tex_var[1]),
                parse_fdid(cols.tex_var[2]),
            ],
            scale_milli: combine_scale_milli(display_scale_milli, model.scale_milli),
        },
    ))
}

fn parse_scale_milli(value: Option<&str>) -> u32 {
    let scale = value
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|scale| *scale > 0.0)
        .unwrap_or(1.0);
    (scale * 1000.0).round() as u32
}

fn combine_scale_milli(display_scale_milli: u32, model_scale_milli: u32) -> u32 {
    let display = display_scale_milli.max(1);
    let model = model_scale_milli.max(1);
    ((display as u64 * model as u64 + 500) / 1000) as u32
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn header_index(headers: &[&str], column: &str, path: &Path) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header.trim() == column)
        .ok_or_else(|| format!("{} missing {column} column", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_dir(label: &str) -> PathBuf {
        let unique = format!(
            "game-engine-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn import_and_query_creature_display() {
        let dir = temp_test_dir("creature-display-cache");
        let display_path = dir.join("CreatureDisplayInfo.csv");
        let model_path = dir.join("CreatureModelData.csv");
        std::fs::write(
            &display_path,
            "ID,ModelID,CreatureModelScale,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2\n4,7,1,11,12,0\n5,7,1,21,22,23\n",
        )
        .unwrap();
        std::fs::write(&model_path, "ID,FileDataID,ModelScale\n7,9001,1.25\n").unwrap();

        let cache_path = dir.join("creature_display.sqlite");
        let conn = Connection::open(&cache_path).unwrap();
        rebuild_cache(&conn, &display_path, &model_path).unwrap();
        build_preferred_skins(&conn).unwrap();
        drop(conn);

        let conn = open_read_only(&cache_path).unwrap();

        // Query by display_id
        let mut stmt = conn
            .prepare(
                "SELECT model_fdid, skin_fdid_0, skin_fdid_1, skin_fdid_2, scale_milli
                 FROM creature_displays WHERE display_id = ?1",
            )
            .unwrap();
        let entry = stmt
            .query_row([4u32], |row| {
                Ok(CreatureDisplay {
                    model_fdid: row.get(0)?,
                    skin_fdids: [row.get(1)?, row.get(2)?, row.get(3)?],
                    scale_milli: row.get(4)?,
                })
            })
            .unwrap();
        assert_eq!(
            entry,
            CreatureDisplay {
                model_fdid: 9001,
                skin_fdids: [11, 12, 0],
                scale_milli: 1250,
            }
        );

        // Preferred skins: display_id=5 has 3 filled slots vs display_id=4 with 2
        let mut stmt = conn
            .prepare(
                "SELECT skin_fdid_0, skin_fdid_1, skin_fdid_2
                 FROM preferred_skins WHERE model_fdid = ?1",
            )
            .unwrap();
        let skins: [u32; 3] = stmt
            .query_row([9001u32], |row| Ok([row.get(0)?, row.get(1)?, row.get(2)?]))
            .unwrap();
        assert_eq!(skins, [21, 22, 23]);

        let _ = std::fs::remove_dir_all(dir);
    }
}
