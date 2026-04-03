use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::sqlite_util::is_missing_table_error;
use game_engine::paths;
use rusqlite::{Connection, OpenFlags};

use crate::scenes::char_select::warband::{
    WarbandSceneEntry, WarbandScenePlacement, WarbandScenePlacementOption,
};

const WARBAND_SCENE_CACHE_PATH: &str = "cache/warband_scene.sqlite";
const CACHE_SCHEMA_VERSION: i64 = 1;

type WarbandSceneData = (
    Vec<WarbandSceneEntry>,
    Vec<WarbandScenePlacement>,
    Vec<WarbandScenePlacementOption>,
);

pub(crate) fn load_cached_warband_scenes(
    scene_csv: &Path,
    placement_csv: &Path,
    placement_option_csv: &Path,
) -> Result<WarbandSceneData, String> {
    let cache_path = ensure_warband_scene_cache(scene_csv, placement_csv, placement_option_csv)?;
    load_warband_scenes_from_sqlite(&cache_path)
}

pub(crate) fn load_warband_scenes_uncached(
    scene_csv: &Path,
    placement_csv: &Path,
    placement_option_csv: &Path,
) -> Result<WarbandSceneData, String> {
    Ok((
        load_scene_rows(scene_csv)?,
        load_placement_rows(placement_csv)?,
        load_placement_option_rows(placement_option_csv)?,
    ))
}

fn ensure_warband_scene_cache(
    scene_csv: &Path,
    placement_csv: &Path,
    placement_option_csv: &Path,
) -> Result<PathBuf, String> {
    let cache_path = paths::shared_data_path(WARBAND_SCENE_CACHE_PATH);
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }

    let source_paths = [
        scene_csv.to_path_buf(),
        placement_csv.to_path_buf(),
        placement_option_csv.to_path_buf(),
    ];
    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    if !cache_is_fresh(&conn, &source_paths)? {
        rebuild_cache(&conn, scene_csv, placement_csv, placement_option_csv)?;
        record_source_files(&conn, &source_paths)?;
    }
    Ok(cache_path)
}

fn load_warband_scenes_from_sqlite(cache_path: &Path) -> Result<WarbandSceneData, String> {
    let conn = Connection::open_with_flags(
        cache_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    Ok((
        load_scenes_from_sqlite(&conn)?,
        load_placements_from_sqlite(&conn)?,
        load_placement_options_from_sqlite(&conn)?,
    ))
}

fn load_scenes_from_sqlite(conn: &Connection) -> Result<Vec<WarbandSceneEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, pos_x, pos_y, pos_z,
                    look_x, look_y, look_z, map_id, fov, texture_kit
             FROM warband_scenes",
        )
        .map_err(|err| format!("prepare warband_scenes query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(WarbandSceneEntry {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                position: [row.get(3)?, row.get(4)?, row.get(5)?],
                look_at: [row.get(6)?, row.get(7)?, row.get(8)?],
                map_id: row.get(9)?,
                fov: row.get(10)?,
                texture_kit: row.get(11)?,
            })
        })
        .map_err(|err| format!("query warband_scenes: {err}"))?;
    collect_rows(rows, "warband_scenes")
}

fn load_placements_from_sqlite(conn: &Connection) -> Result<Vec<WarbandScenePlacement>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, scene_id, slot_type, pos_x, pos_y, pos_z, rotation, slot_id
             FROM warband_scene_placements",
        )
        .map_err(|err| format!("prepare warband_scene_placements query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(WarbandScenePlacement {
                id: row.get(0)?,
                scene_id: row.get(1)?,
                slot_type: row.get(2)?,
                position: [row.get(3)?, row.get(4)?, row.get(5)?],
                rotation: row.get(6)?,
                slot_id: row.get(7)?,
            })
        })
        .map_err(|err| format!("query warband_scene_placements: {err}"))?;
    collect_rows(rows, "warband_scene_placements")
}

fn load_placement_options_from_sqlite(
    conn: &Connection,
) -> Result<Vec<WarbandScenePlacementOption>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT placement_id, layout_key, pos_x, pos_y, pos_z, orientation, scale
             FROM warband_scene_placement_options",
        )
        .map_err(|err| format!("prepare warband_scene_placement_options query: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(WarbandScenePlacementOption {
                placement_id: row.get(0)?,
                layout_key: row.get(1)?,
                position: [row.get(2)?, row.get(3)?, row.get(4)?],
                orientation: row.get(5)?,
                scale: row.get(6)?,
            })
        })
        .map_err(|err| format!("query warband_scene_placement_options: {err}"))?;
    collect_rows(rows, "warband_scene_placement_options")
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
    table: &str,
) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|err| format!("read {table} row: {err}"))?);
    }
    Ok(values)
}

fn cache_is_fresh(conn: &Connection, source_paths: &[PathBuf]) -> Result<bool, String> {
    if !cache_schema_is_current(conn)? {
        return Ok(false);
    }
    let mut stmt = match conn.prepare("SELECT path, mtime FROM source_files") {
        Ok(stmt) => stmt,
        Err(err) if is_missing_table_error(&err) => {
            return Ok(false);
        }
        Err(err) => return Err(format!("prepare source_files query: {err}")),
    };
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|err| format!("query source_files: {err}"))?;
    let mut recorded = std::collections::HashMap::new();
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

fn cache_schema_is_current(conn: &Connection) -> Result<bool, String> {
    let version = match conn.query_row("SELECT version FROM cache_metadata LIMIT 1", [], |row| {
        row.get::<_, i64>(0)
    }) {
        Ok(version) => version,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
        Err(err) if is_missing_table_error(&err) => {
            return Ok(false);
        }
        Err(err) => return Err(format!("query cache_metadata: {err}")),
    };
    Ok(version == CACHE_SCHEMA_VERSION)
}

fn rebuild_cache(
    conn: &Connection,
    scene_csv: &Path,
    placement_csv: &Path,
    placement_option_csv: &Path,
) -> Result<(), String> {
    init_cache_schema(conn)?;
    import_scene_rows(conn, scene_csv)?;
    import_placement_rows(conn, placement_csv)?;
    import_placement_option_rows(conn, placement_option_csv)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit warband scene cache: {err}"))?;
    Ok(())
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(cache_schema_sql())
        .map_err(|err| format!("init warband scene cache: {err}"))
}

fn record_source_files(conn: &Connection, source_paths: &[PathBuf]) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO cache_metadata (id, version) VALUES (1, ?1)",
        [CACHE_SCHEMA_VERSION],
    )
    .map_err(|err| format!("record cache schema version: {err}"))?;
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

fn import_scene_rows(conn: &Connection, source_path: &Path) -> Result<(), String> {
    let scenes = load_scene_rows(source_path)?;
    let mut insert = prepare_scene_insert(conn)?;
    for scene in &scenes {
        insert_scene_row(&mut insert, scene)?;
    }
    Ok(())
}

fn import_placement_rows(conn: &Connection, source_path: &Path) -> Result<(), String> {
    let placements = load_placement_rows(source_path)?;
    let mut insert = prepare_placement_insert(conn)?;
    for placement in &placements {
        insert_placement_row(&mut insert, placement)?;
    }
    Ok(())
}

fn import_placement_option_rows(conn: &Connection, source_path: &Path) -> Result<(), String> {
    let options = load_placement_option_rows(source_path)?;
    let mut insert = prepare_placement_option_insert(conn)?;
    for option in &options {
        insert_placement_option_row(&mut insert, option)?;
    }
    Ok(())
}

fn cache_schema_sql() -> &'static str {
    "BEGIN;
     DROP TABLE IF EXISTS source_files;
     DROP TABLE IF EXISTS cache_metadata;
     DROP TABLE IF EXISTS warband_scenes;
     DROP TABLE IF EXISTS warband_scene_placements;
     DROP TABLE IF EXISTS warband_scene_placement_options;
     CREATE TABLE cache_metadata (
         id INTEGER PRIMARY KEY CHECK (id = 1),
         version INTEGER NOT NULL
     );
     CREATE TABLE source_files (
         path TEXT PRIMARY KEY,
         mtime INTEGER NOT NULL
     );
     CREATE TABLE warband_scenes (
         id INTEGER PRIMARY KEY,
         name TEXT NOT NULL,
         description TEXT NOT NULL,
         pos_x REAL NOT NULL,
         pos_y REAL NOT NULL,
         pos_z REAL NOT NULL,
         look_x REAL NOT NULL,
         look_y REAL NOT NULL,
         look_z REAL NOT NULL,
         map_id INTEGER NOT NULL,
         fov REAL NOT NULL,
         texture_kit INTEGER NOT NULL
     );
     CREATE TABLE warband_scene_placements (
         id INTEGER PRIMARY KEY,
         scene_id INTEGER NOT NULL,
         slot_type INTEGER NOT NULL,
         pos_x REAL NOT NULL,
         pos_y REAL NOT NULL,
         pos_z REAL NOT NULL,
         rotation REAL NOT NULL,
         slot_id INTEGER NOT NULL
     );
     CREATE TABLE warband_scene_placement_options (
         placement_id INTEGER NOT NULL,
         layout_key INTEGER NOT NULL,
         pos_x REAL NOT NULL,
         pos_y REAL NOT NULL,
         pos_z REAL NOT NULL,
         orientation REAL NOT NULL,
         scale REAL NOT NULL,
         PRIMARY KEY (placement_id, layout_key)
     );"
}

fn prepare_scene_insert(conn: &Connection) -> Result<rusqlite::Statement<'_>, String> {
    conn.prepare(
        "INSERT OR REPLACE INTO warband_scenes
         (id, name, description, pos_x, pos_y, pos_z, look_x, look_y, look_z, map_id, fov, texture_kit)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .map_err(|err| format!("prepare warband_scenes insert: {err}"))
}

fn insert_scene_row(
    insert: &mut rusqlite::Statement<'_>,
    scene: &WarbandSceneEntry,
) -> Result<(), String> {
    insert
        .execute((
            scene.id,
            &scene.name,
            &scene.description,
            scene.position[0],
            scene.position[1],
            scene.position[2],
            scene.look_at[0],
            scene.look_at[1],
            scene.look_at[2],
            scene.map_id,
            scene.fov,
            scene.texture_kit,
        ))
        .map_err(|err| format!("insert warband scene row {}: {err}", scene.id))?;
    Ok(())
}

fn prepare_placement_insert(conn: &Connection) -> Result<rusqlite::Statement<'_>, String> {
    conn.prepare(
        "INSERT OR REPLACE INTO warband_scene_placements
         (id, scene_id, slot_type, pos_x, pos_y, pos_z, rotation, slot_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .map_err(|err| format!("prepare warband_scene_placements insert: {err}"))
}

fn insert_placement_row(
    insert: &mut rusqlite::Statement<'_>,
    placement: &WarbandScenePlacement,
) -> Result<(), String> {
    insert
        .execute((
            placement.id,
            placement.scene_id,
            placement.slot_type,
            placement.position[0],
            placement.position[1],
            placement.position[2],
            placement.rotation,
            placement.slot_id,
        ))
        .map_err(|err| format!("insert warband placement row {}: {err}", placement.id))?;
    Ok(())
}

fn prepare_placement_option_insert(conn: &Connection) -> Result<rusqlite::Statement<'_>, String> {
    conn.prepare(
        "INSERT OR REPLACE INTO warband_scene_placement_options
         (placement_id, layout_key, pos_x, pos_y, pos_z, orientation, scale)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .map_err(|err| format!("prepare warband_scene_placement_options insert: {err}"))
}

fn insert_placement_option_row(
    insert: &mut rusqlite::Statement<'_>,
    option: &WarbandScenePlacementOption,
) -> Result<(), String> {
    insert
        .execute((
            option.placement_id,
            option.layout_key,
            option.position[0],
            option.position[1],
            option.position[2],
            option.orientation,
            option.scale,
        ))
        .map_err(|err| {
            format!(
                "insert warband placement option row {}:{}: {err}",
                option.placement_id, option.layout_key
            )
        })?;
    Ok(())
}

fn load_scene_rows(source_path: &Path) -> Result<Vec<WarbandSceneEntry>, String> {
    load_rows(source_path, super::parse_scene_line)
}

fn load_placement_rows(source_path: &Path) -> Result<Vec<WarbandScenePlacement>, String> {
    load_rows(source_path, super::parse_placement_line)
}

fn load_placement_option_rows(
    source_path: &Path,
) -> Result<Vec<WarbandScenePlacementOption>, String> {
    load_rows(source_path, super::parse_placement_option_line)
}

fn load_rows<T>(source_path: &Path, parse_row: fn(&str) -> Option<T>) -> Result<Vec<T>, String> {
    let mut reader = open_reader(source_path)?;
    skip_header(&mut reader, source_path)?;
    let mut rows = Vec::new();
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
        if let Some(row) = parse_row(line.trim_end_matches(['\r', '\n'])) {
            rows.push(row);
        }
    }
    Ok(rows)
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn skip_header(reader: &mut BufReader<std::fs::File>, path: &Path) -> Result<(), String> {
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    Ok(())
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
    fn load_cached_warband_scenes_round_trips_cache() {
        let dir = temp_test_dir("warband-scene-cache");
        let scene_csv = dir.join("WarbandScene.csv");
        let placement_csv = dir.join("WarbandScenePlacement.csv");
        let placement_option_csv = dir.join("WarbandScenePlacementOption.csv");

        std::fs::write(
            &scene_csv,
            "Name_lang,Description_lang,Position_0,Position_1,Position_2,LookAt_0,LookAt_1,LookAt_2,ID,MapID,Fov,UiModelSceneID,Flags,VerifiedBuild,SortIndex,TextureKitID\n\"Test Camp\",\"Desc\",1,2,3,4,5,6,7,8,65,0,8,0,0,9\n",
        )
        .unwrap();
        std::fs::write(
            &placement_csv,
            "Position_0,Position_1,Position_2,ID,WarbandSceneID,SlotType,Rotation,Scale,VerifiedBuild,MountCreatureDisplayID,Flags,SlotID\n10,11,12,13,7,0,90,1,0,0,0,2\n",
        )
        .unwrap();
        std::fs::write(
            &placement_option_csv,
            "ID,WarbandScenePlacementID,WarbandScenePlacementOptionSetID,Position_0,Position_1,Position_2,Orientation,Scale\n1,13,21,14,15,16,180,1.5\n",
        )
        .unwrap();

        let (scenes, placements, options) =
            load_cached_warband_scenes(&scene_csv, &placement_csv, &placement_option_csv).unwrap();

        assert_eq!(scenes.len(), 1);
        assert_eq!(placements.len(), 1);
        assert_eq!(options.len(), 1);
        assert_eq!(scenes[0].id, 7);
        assert_eq!(scenes[0].texture_kit, 9);
        assert_eq!(placements[0].scene_id, 7);
        assert_eq!(placements[0].slot_id, 2);
        assert_eq!(options[0].placement_id, 13);
        assert_eq!(options[0].layout_key, 21);

        let _ = std::fs::remove_dir_all(dir);
    }
}
