use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

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

pub(crate) fn load_creature_display_entries(
    display_info_path: &Path,
    model_data_path: &Path,
) -> Result<HashMap<u32, CreatureDisplay>, String> {
    let cache_path = ensure_creature_display_cache(display_info_path, model_data_path)?;
    load_entries_from_sqlite(&cache_path)
}

pub(crate) fn load_creature_display_entries_uncached(
    display_info_path: &Path,
    model_data_path: &Path,
) -> Result<HashMap<u32, CreatureDisplay>, String> {
    let model_data = parse_model_data(model_data_path)?;
    let mut reader = open_reader(display_info_path)?;
    let cols = read_display_columns(&mut reader, display_info_path)?;
    let mut entries = HashMap::new();
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
        if let Some((display_id, entry)) =
            parse_display_entry(line.trim_end_matches(['\r', '\n']), &cols, &model_data)
        {
            entries.insert(display_id, entry);
        }
    }
    Ok(entries)
}

fn load_entries_from_sqlite(cache_path: &Path) -> Result<HashMap<u32, CreatureDisplay>, String> {
    let conn = Connection::open_with_flags(
        cache_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    let mut stmt = conn
        .prepare(
            "SELECT display_id, model_fdid, skin_fdid_0, skin_fdid_1, skin_fdid_2, scale_milli
             FROM creature_displays",
        )
        .map_err(|err| format!("prepare creature_displays query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                CreatureDisplay {
                    model_fdid: row.get(1)?,
                    skin_fdids: [row.get(2)?, row.get(3)?, row.get(4)?],
                    scale_milli: row.get(5)?,
                },
            ))
        })
        .map_err(|err| format!("query creature_displays: {err}"))?;

    let mut entries = HashMap::new();
    for row in rows {
        let (display_id, entry) =
            row.map_err(|err| format!("read creature_displays row: {err}"))?;
        entries.insert(display_id, entry);
    }
    Ok(entries)
}

fn ensure_creature_display_cache(
    display_info_path: &Path,
    model_data_path: &Path,
) -> Result<PathBuf, String> {
    let cache_path = paths::shared_data_path(CREATURE_DISPLAY_CACHE_PATH);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }

    let source_paths = [
        display_info_path.to_path_buf(),
        model_data_path.to_path_buf(),
    ];
    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    if !cache_is_fresh(&conn, &source_paths)? {
        rebuild_cache(&conn, display_info_path, model_data_path)?;
        record_source_files(&conn, &source_paths)?;
    }
    Ok(cache_path)
}

fn cache_is_fresh(conn: &Connection, source_paths: &[PathBuf]) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT path, mtime FROM source_files") {
        Ok(stmt) => stmt,
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("no such table") =>
        {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare source_files query: {err}")),
    };
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
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit creature display cache: {err}"))?;
    Ok(())
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS source_files;
         DROP TABLE IF EXISTS creature_displays;
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

fn csv_mtime(path: &Path) -> Result<i64, String> {
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
    fn load_creature_display_entries_uses_cache_backed_join() {
        let dir = temp_test_dir("creature-display-cache");
        let display_path = dir.join("CreatureDisplayInfo.csv");
        let model_path = dir.join("CreatureModelData.csv");
        std::fs::write(
            &display_path,
            "ID,ModelID,CreatureModelScale,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2\n4,7,1,11,12,0\n",
        )
        .unwrap();
        std::fs::write(&model_path, "ID,FileDataID,ModelScale\n7,9001,1.25\n").unwrap();

        let entries = load_creature_display_entries(&display_path, &model_path).unwrap();

        assert_eq!(
            entries.get(&4),
            Some(&CreatureDisplay {
                model_fdid: 9001,
                skin_fdids: [11, 12, 0],
                scale_milli: 1250,
            })
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
