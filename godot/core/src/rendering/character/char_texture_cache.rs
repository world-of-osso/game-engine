use crate::cache_source_mtime::{csv_mtime, source_key};
use crate::cache_sqlite::{open_read_only, replace_atomically};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::char_texture_query_data;
use crate::csv_util::{header_index, parse_csv_line_trimmed as parse_csv_line};
use crate::sqlite_util::is_missing_table_error;

fn cache_path(data_dir: &Path) -> PathBuf {
    data_dir
        .join("cache")
        .join(char_texture_query_data::char_texture_cache_file())
}

fn source_paths(data_dir: &Path) -> [PathBuf; 4] {
    [
        data_dir.join("ChrModelTextureLayer.csv"),
        data_dir.join("CharComponentTextureSections.csv"),
        data_dir.join("CharComponentTextureLayouts.csv"),
        data_dir.join("ChrModelMaterial.csv"),
    ]
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn cache_is_fresh(
    conn: &Connection,
    data_dir: &Path,
    csv_paths: &[PathBuf],
) -> Result<bool, String> {
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
        let key = source_key(data_dir, path)?;
        if recorded.get(&key).copied() != Some(csv_mtime(path)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn record_source_files(
    conn: &Connection,
    data_dir: &Path,
    csv_paths: &[PathBuf],
) -> Result<(), String> {
    let mut insert = conn
        .prepare("INSERT INTO source_files (source, mtime_secs) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare source_files insert: {err}"))?;
    for path in csv_paths {
        insert
            .execute((source_key(data_dir, path)?, csv_mtime(path)?))
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
    replace_atomically(cache_path, |conn| {
        init_cache_schema(conn)?;
        record_source_files(conn, data_dir, &csv_paths)?;
        populate_layers(conn, &csv_paths[0])?;
        populate_sections(conn, &csv_paths[1])?;
        populate_layouts(conn, &csv_paths[2])?;
        populate_model_materials(conn, &csv_paths[3])?;
        conn.execute_batch("COMMIT;")
            .map_err(|err| format!("commit char texture cache: {err}"))
    })
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DROP TABLE IF EXISTS source_files;
         DROP TABLE IF EXISTS layers;
         DROP TABLE IF EXISTS sections;
         DROP TABLE IF EXISTS layouts;
         DROP TABLE IF EXISTS model_materials;
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
         );
         CREATE TABLE model_materials (
             layout_id INTEGER NOT NULL,
             texture_type INTEGER NOT NULL,
             width INTEGER NOT NULL,
             height INTEGER NOT NULL,
             PRIMARY KEY (layout_id, texture_type)
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

/// ChrModelMaterial: the canvas size of each (layout, M2 texture type).
fn populate_model_materials(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT OR REPLACE INTO model_materials (layout_id, texture_type, width, height) VALUES (?1, ?2, ?3, ?4)",
        path,
        |headers, fields, path| {
            Ok((
                parse_u32(
                    fields,
                    header_index(headers, "CharComponentTextureLayoutsID", path)?,
                ),
                parse_u32(fields, header_index(headers, "TextureType", path)?),
                parse_u32(fields, header_index(headers, "Width", path)?),
                parse_u32(fields, header_index(headers, "Height", path)?),
            ))
        },
    )
}

/// Builds `<data_dir>/cache/char_texture-v<N>.sqlite` from the DB2 CSVs in `data_dir`,
/// reusing it when its recorded sources are unchanged.
pub fn import_char_texture_cache(data_dir: &Path) -> Result<PathBuf, String> {
    let cache_path = cache_path(data_dir);
    let csv_paths = source_paths(data_dir);
    let needs_rebuild = if cache_path.exists() {
        let conn = open_read_only(&cache_path)?;
        !cache_is_fresh(&conn, data_dir, &csv_paths)?
    } else {
        true
    };
    if needs_rebuild {
        rebuild_cache(&cache_path, data_dir)?;
    }
    Ok(cache_path)
}

#[cfg(test)]
mod tests {
    use super::import_char_texture_cache;
    use crate::cache_sqlite::open_read_only;
    use std::path::{Path, PathBuf};

    /// A data root whose texture CSVs link to the real ones, with its own `cache/`.
    fn linked_source_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("char-texture-{name}-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        std::fs::create_dir_all(&root).unwrap();
        for source in super::source_paths(Path::new("data")) {
            std::os::unix::fs::symlink(
                source.canonicalize().unwrap(),
                root.join(source.file_name().unwrap()),
            )
            .unwrap();
        }
        root
    }

    #[test]
    fn concurrent_imports_of_a_missing_cache_all_succeed() {
        const IMPORTERS: usize = 8;
        let root = linked_source_root("concurrent");
        let barrier = std::sync::Barrier::new(IMPORTERS);
        let results = std::thread::scope(|scope| {
            let workers = (0..IMPORTERS)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        import_char_texture_cache(&root)
                    })
                })
                .collect::<Vec<_>>();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .collect::<Vec<_>>()
        });
        for result in results {
            result.expect("every concurrent importer must succeed");
        }
        let conn = open_read_only(&import_char_texture_cache(&root).unwrap()).unwrap();
        let (layers, _, layouts) =
            crate::char_texture_query_data::query_char_texture_data(&conn).unwrap();
        assert!(!layers.is_empty());
        assert!(!layouts.is_empty());
        drop(conn);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn equivalent_data_root_spelling_reuses_the_cache() {
        let root = linked_source_root("alias");
        let cache = import_char_texture_cache(&root).unwrap();
        let conn = rusqlite::Connection::open(&cache).unwrap();
        conn.execute("DELETE FROM layouts", []).unwrap();
        drop(conn);
        let alias = root.join("..").join(root.file_name().unwrap());
        import_char_texture_cache(&alias).unwrap();
        let conn = open_read_only(&cache).unwrap();
        let layouts: i64 = conn
            .query_row("SELECT COUNT(*) FROM layouts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            layouts, 0,
            "an equivalent data root must not rebuild the cache"
        );
        drop(conn);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn query_char_texture_data_reads_sorted_layers_and_keyed_regions() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory cache");
        conn.execute_batch(
            "CREATE TABLE layers (texture_type INTEGER, layer INTEGER, blend_mode INTEGER, section_bitmask INTEGER, target_id INTEGER, layout_id INTEGER);
             CREATE TABLE sections (layout_id INTEGER, section_type INTEGER, x INTEGER, y INTEGER, width INTEGER, height INTEGER);
             CREATE TABLE layouts (id INTEGER, width INTEGER, height INTEGER);
             INSERT INTO layers VALUES (6, 2, 1, 8, 42, 3), (1, 5, 0, 2, 7, 2), (1, 1, 0, 4, 9, 2);
             INSERT INTO sections VALUES (3, 6, 5, 10, 20, 25), (2, 4, 11, 12, 13, 14);
             INSERT INTO layouts VALUES (3, 512, 256), (2, 1024, 512);",
        )
        .expect("seed texture cache");

        let (layers, sections, layouts) =
            crate::char_texture_query_data::query_char_texture_data(&conn).expect("query cache");
        let layer_keys: Vec<_> = layers
            .iter()
            .map(|layer| (layer.layout_id, layer.texture_type, layer.layer))
            .collect();
        assert_eq!(layer_keys, [(2, 1, 1), (2, 1, 5), (3, 6, 2)]);
        assert_eq!(layers[0].section_bitmask, 4);
        assert_eq!(layers[2].target_id, 42);
        assert_eq!(sections[&(3, 6)].x, 5);
        assert_eq!(sections[&(2, 4)].height, 14);
        assert_eq!(layouts[&2].width, 1024);
        assert_eq!(layouts[&3].height, 256);
    }

    #[test]
    fn char_texture_data_loads_from_imported_cache() {
        let cache =
            import_char_texture_cache(Path::new("data")).expect("import char texture cache");
        let conn = open_read_only(&cache).expect("open char texture cache");
        let (layers, sections, layouts) =
            crate::char_texture_query_data::query_char_texture_data(&conn)
                .expect("load char texture cache");
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
