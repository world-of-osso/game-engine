use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, OpenFlags};

use crate::asset::char_texture::{TextureLayer, TextureLayout, TextureSection};
use crate::csv_util::parse_csv_line_trimmed as parse_csv_line;
use crate::sqlite_util::is_missing_table_error;

type CharTextureCacheData = (
    Vec<TextureLayer>,
    HashMap<(u32, u32), TextureSection>,
    HashMap<u32, TextureLayout>,
);

fn cache_path() -> PathBuf {
    crate::paths::shared_data_path("cache/char_texture.sqlite")
}

fn source_paths(data_dir: &Path) -> [PathBuf; 3] {
    [
        data_dir.join("ChrModelTextureLayer.csv"),
        data_dir.join("CharComponentTextureSections.csv"),
        data_dir.join("CharComponentTextureLayouts.csv"),
    ]
}

fn open_read_only(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", path.display()))
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn header_index(headers: &[String], column: &str, path: &Path) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header == column)
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

fn cache_is_fresh(conn: &Connection, csv_paths: &[PathBuf]) -> Result<bool, String> {
    let mut stmt = match conn.prepare("SELECT source, mtime_secs FROM source_files") {
        Ok(stmt) => stmt,
        Err(err) if is_missing_table_error(&err) => {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare source_files lookup: {err}")),
    };
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|err| format!("query source_files: {err}"))?;
    let mut recorded = HashMap::new();
    for row in rows {
        let (source, mtime) = row.map_err(|err| format!("read source_files row: {err}"))?;
        recorded.insert(source, mtime);
    }
    for path in csv_paths {
        let key = path.to_string_lossy().to_string();
        if recorded.get(&key).copied() != Some(csv_mtime(path)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn record_source_files(conn: &Connection, csv_paths: &[PathBuf]) -> Result<(), String> {
    let mut insert = conn
        .prepare("INSERT INTO source_files (source, mtime_secs) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare source_files insert: {err}"))?;
    for path in csv_paths {
        insert
            .execute((path.to_string_lossy().to_string(), csv_mtime(path)?))
            .map_err(|err| format!("insert source_files {}: {err}", path.display()))?;
    }
    Ok(())
}

fn parse_u32(fields: &[String], index: usize) -> u32 {
    fields
        .get(index)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0)
}

fn parse_i64(fields: &[String], index: usize) -> i64 {
    fields
        .get(index)
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0)
}

fn insert_simple_rows<T, F>(
    conn: &Connection,
    sql: &str,
    path: &Path,
    mut row_builder: F,
) -> Result<(), String>
where
    T: rusqlite::Params,
    F: FnMut(&[String], &[String], &Path) -> Result<T, String>,
{
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let mut insert = conn
        .prepare(sql)
        .map_err(|err| format!("prepare insert for {}: {err}", path.display()))?;
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
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        let params = row_builder(&headers, &fields, path)?;
        insert
            .execute(params)
            .map_err(|err| format!("insert row for {}: {err}", path.display()))?;
    }
    Ok(())
}

fn rebuild_cache(cache_path: &Path, data_dir: &Path) -> Result<(), String> {
    let csv_paths = source_paths(data_dir);
    let conn = init_cache_db(cache_path)?;

    record_source_files(&conn, &csv_paths)?;
    populate_layers(&conn, &csv_paths[0])?;
    populate_sections(&conn, &csv_paths[1])?;
    populate_layouts(&conn, &csv_paths[2])?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit char texture cache: {err}"))?;
    Ok(())
}

fn init_cache_db(cache_path: &Path) -> Result<Connection, String> {
    create_cache_parent_dir(cache_path)?;
    let conn = Connection::open(cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    init_cache_schema(&conn)?;
    Ok(conn)
}

fn create_cache_parent_dir(cache_path: &Path) -> Result<(), String> {
    let Some(parent) = cache_path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS source_files;
         DROP TABLE IF EXISTS layers;
         DROP TABLE IF EXISTS sections;
         DROP TABLE IF EXISTS layouts;
         CREATE TABLE source_files (source TEXT PRIMARY KEY, mtime_secs INTEGER NOT NULL);
         CREATE TABLE layers (
             texture_type INTEGER NOT NULL,
             layer INTEGER NOT NULL,
             blend_mode INTEGER NOT NULL,
             section_bitmask INTEGER NOT NULL,
             target_id INTEGER NOT NULL,
             layout_id INTEGER NOT NULL
         );
         CREATE TABLE sections (
             layout_id INTEGER NOT NULL,
             section_type INTEGER NOT NULL,
             x INTEGER NOT NULL,
             y INTEGER NOT NULL,
             width INTEGER NOT NULL,
             height INTEGER NOT NULL,
             PRIMARY KEY (layout_id, section_type)
         );
         CREATE TABLE layouts (
             id INTEGER PRIMARY KEY,
             width INTEGER NOT NULL,
             height INTEGER NOT NULL
         );",
    )
    .map_err(|err| format!("init char texture cache: {err}"))
}

fn populate_layers(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO layers (texture_type, layer, blend_mode, section_bitmask, target_id, layout_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        path,
        |headers, fields, path| {
            Ok((
                parse_u32(fields, header_index(headers, "TextureType", path)?),
                parse_u32(fields, header_index(headers, "Layer", path)?),
                parse_u32(fields, header_index(headers, "BlendMode", path)?),
                parse_i64(
                    fields,
                    header_index(headers, "TextureSectionTypeBitMask", path)?,
                ),
                parse_u32(
                    fields,
                    header_index(headers, "ChrModelTextureTargetID_0", path)?,
                ) as u16,
                parse_u32(
                    fields,
                    header_index(headers, "CharComponentTextureLayoutsID", path)?,
                ),
            ))
        },
    )
}

fn populate_sections(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO sections (layout_id, section_type, x, y, width, height) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        path,
        |headers, fields, path| {
            Ok((
                parse_u32(
                    fields,
                    header_index(headers, "CharComponentTextureLayoutID", path)?,
                ),
                parse_u32(fields, header_index(headers, "SectionType", path)?),
                parse_u32(fields, header_index(headers, "X", path)?),
                parse_u32(fields, header_index(headers, "Y", path)?),
                parse_u32(fields, header_index(headers, "Width", path)?),
                parse_u32(fields, header_index(headers, "Height", path)?),
            ))
        },
    )
}

fn populate_layouts(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO layouts (id, width, height) VALUES (?1, ?2, ?3)",
        path,
        |headers, fields, path| {
            Ok((
                parse_u32(fields, header_index(headers, "ID", path)?),
                parse_u32(fields, header_index(headers, "Width", path)?),
                parse_u32(fields, header_index(headers, "Height", path)?),
            ))
        },
    )
}

pub fn import_char_texture_cache(data_dir: &Path) -> Result<PathBuf, String> {
    let cache_path = cache_path();
    let csv_paths = source_paths(data_dir);
    let needs_rebuild = if cache_path.exists() {
        let conn = open_read_only(&cache_path)?;
        !cache_is_fresh(&conn, &csv_paths)?
    } else {
        true
    };
    if needs_rebuild {
        rebuild_cache(&cache_path, data_dir)?;
    }
    Ok(cache_path)
}

pub(crate) fn load_char_texture_data(_data_dir: &Path) -> Result<CharTextureCacheData, String> {
    let cache_path = cache_path();
    if !cache_path.exists() {
        return Err(format!(
            "{} missing; run `cargo run --bin char_texture_cache_import` to build it",
            cache_path.display()
        ));
    }
    let conn = open_read_only(&cache_path)?;
    let layers = load_layers(&conn)?;
    let sections = load_sections(&conn)?;
    let layouts = load_layouts(&conn)?;
    Ok((layers, sections, layouts))
}

fn load_layers(conn: &Connection) -> Result<Vec<TextureLayer>, String> {
    let mut layers_stmt = conn
        .prepare(
            "SELECT texture_type, layer, blend_mode, section_bitmask, target_id, layout_id
             FROM layers
             ORDER BY layout_id, texture_type, layer",
        )
        .map_err(|err| format!("prepare layers lookup: {err}"))?;
    layers_stmt
        .query_map([], |row| {
            Ok(TextureLayer {
                texture_type: row.get(0)?,
                layer: row.get(1)?,
                blend_mode: row.get(2)?,
                section_bitmask: row.get(3)?,
                target_id: row.get(4)?,
                layout_id: row.get(5)?,
            })
        })
        .map_err(|err| format!("query layers: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read layers row: {err}"))
}

fn load_sections(conn: &Connection) -> Result<HashMap<(u32, u32), TextureSection>, String> {
    let mut sections_stmt = conn
        .prepare("SELECT layout_id, section_type, x, y, width, height FROM sections")
        .map_err(|err| format!("prepare sections lookup: {err}"))?;
    sections_stmt
        .query_map([], |row| {
            Ok((
                (row.get::<_, u32>(0)?, row.get::<_, u32>(1)?),
                TextureSection {
                    x: row.get(2)?,
                    y: row.get(3)?,
                    width: row.get(4)?,
                    height: row.get(5)?,
                },
            ))
        })
        .map_err(|err| format!("query sections: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read sections row: {err}"))
}

fn load_layouts(conn: &Connection) -> Result<HashMap<u32, TextureLayout>, String> {
    let mut layouts_stmt = conn
        .prepare("SELECT id, width, height FROM layouts")
        .map_err(|err| format!("prepare layouts lookup: {err}"))?;
    layouts_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                TextureLayout {
                    width: row.get(1)?,
                    height: row.get(2)?,
                },
            ))
        })
        .map_err(|err| format!("query layouts: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read layouts row: {err}"))
}

#[cfg(test)]
mod tests {
    use super::{import_char_texture_cache, load_char_texture_data};
    use std::path::Path;

    #[test]
    fn char_texture_data_loads_from_imported_cache() {
        import_char_texture_cache(Path::new("data")).expect("import char texture cache");
        let (layers, sections, layouts) =
            load_char_texture_data(Path::new("data")).expect("load char texture cache");
        assert!(!layers.is_empty());
        assert!(!sections.is_empty());
        assert!(!layouts.is_empty());
    }

    #[test]
    fn char_texture_cache_import_reuses_fresh_cache() {
        let cache_path =
            import_char_texture_cache(Path::new("data")).expect("import char texture cache");
        let before = std::fs::metadata(&cache_path)
            .expect("stat char texture cache")
            .modified()
            .expect("char texture cache mtime");
        let reused_path =
            import_char_texture_cache(Path::new("data")).expect("reuse char texture cache");
        let after = std::fs::metadata(&reused_path)
            .expect("stat reused char texture cache")
            .modified()
            .expect("reused char texture cache mtime");
        assert_eq!(cache_path, reused_path);
        assert_eq!(before, after);
    }
}
