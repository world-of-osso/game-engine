use crate::cache_source_mtime::csv_mtime;
use crate::cache_sqlite::open_read_only;
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::csv_util::parse_csv_line_trimmed as parse_csv_line;
use crate::customization_data::{
    RawChoice, RawChrModel, RawData, RawElement, RawGeoset, RawMaterial, RawOption,
    chr_model_id_for_hair_row,
};
use crate::sqlite_util::is_missing_table_error;

type HairGeosetKey = (u32, u16, u16);

fn cache_path() -> PathBuf {
    crate::paths::shared_data_path("cache/customization.sqlite")
}

fn required_csv_paths(data_dir: &Path) -> [PathBuf; 7] {
    [
        data_dir.join("ChrModel.csv"),
        data_dir.join("ChrCustomizationOption.csv"),
        data_dir.join("ChrCustomizationChoice.csv"),
        data_dir.join("ChrCustomizationElement.csv"),
        data_dir.join("ChrCustomizationMaterial.csv"),
        data_dir.join("ChrCustomizationGeoset.csv"),
        data_dir.join("CharHairGeosets.csv"),
    ]
}

fn texture_file_data_path(data_dir: &Path) -> PathBuf {
    data_dir.join("TextureFileData.csv")
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

fn rebuild_cache(cache_path: &Path, data_dir: &Path) -> Result<(), String> {
    let csv_paths = required_csv_paths(data_dir);
    let texture_file_data = texture_file_data_path(data_dir);
    let all_sources = rebuild_source_paths(&csv_paths, &texture_file_data);
    let conn = init_cache_db(cache_path)?;

    record_source_files(&conn, &all_sources)?;
    populate_chr_models(&conn, &csv_paths[0])?;
    populate_options(&conn, &csv_paths[1])?;
    populate_choices(&conn, &csv_paths[2])?;
    populate_elements(&conn, &csv_paths[3])?;
    populate_materials(&conn, &csv_paths[4])?;
    populate_geosets(&conn, &csv_paths[5])?;
    populate_hair_geosets(&conn, &csv_paths[6])?;
    populate_texture_fdids(&conn, &texture_file_data)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit customization cache: {err}"))?;
    Ok(())
}

fn rebuild_source_paths(csv_paths: &[PathBuf; 7], texture_file_data: &Path) -> Vec<PathBuf> {
    let mut all_sources = csv_paths.to_vec();
    all_sources.push(texture_file_data.to_path_buf());
    all_sources
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
    conn.execute_batch(&build_customization_cache_schema_sql())
        .map_err(|err| format!("init customization cache: {err}"))
}

fn build_customization_cache_schema_sql() -> String {
    format!(
        "BEGIN;
         {drops}
         {creates}
         COMMIT;",
        drops = customization_cache_drop_tables_sql(),
        creates = customization_cache_create_tables_sql(),
    )
}

fn customization_cache_drop_tables_sql() -> &'static str {
    "DROP TABLE IF EXISTS source_files;
     DROP TABLE IF EXISTS chr_models;
     DROP TABLE IF EXISTS options;
     DROP TABLE IF EXISTS choices;
     DROP TABLE IF EXISTS elements;
     DROP TABLE IF EXISTS materials;
     DROP TABLE IF EXISTS geosets;
     DROP TABLE IF EXISTS hair_geosets;
     DROP TABLE IF EXISTS texture_fdids;"
}

fn customization_cache_core_tables_sql() -> &'static str {
    "CREATE TABLE source_files (source TEXT PRIMARY KEY, mtime_secs INTEGER NOT NULL);
     CREATE TABLE chr_models (
         id INTEGER PRIMARY KEY,
         layout_id INTEGER NOT NULL,
         customize_scale REAL NOT NULL,
         camera_distance_offset REAL NOT NULL
     );
     CREATE TABLE options (
         id INTEGER PRIMARY KEY,
         name TEXT NOT NULL,
         chr_model_id INTEGER NOT NULL
     );
     CREATE TABLE choices (
         id INTEGER PRIMARY KEY,
         option_id INTEGER NOT NULL,
         name TEXT NOT NULL,
         requirement_id INTEGER NOT NULL,
         order_index INTEGER NOT NULL
     );"
}

fn customization_cache_relation_tables_sql() -> &'static str {
    "CREATE TABLE elements (
         choice_id INTEGER NOT NULL,
         related_choice_id INTEGER NOT NULL,
         geoset_id INTEGER NOT NULL,
         material_id INTEGER NOT NULL
     );
     CREATE TABLE materials (
         id INTEGER PRIMARY KEY,
         texture_target_id INTEGER NOT NULL,
         material_resources_id INTEGER NOT NULL
     );
     CREATE TABLE geosets (
         id INTEGER PRIMARY KEY,
         geoset_type INTEGER NOT NULL,
         geoset_id INTEGER NOT NULL
     );"
}

fn customization_cache_lookup_tables_sql() -> &'static str {
    "CREATE TABLE hair_geosets (
         model_id INTEGER NOT NULL,
         geoset_type INTEGER NOT NULL,
         geoset_id INTEGER NOT NULL,
         shows_scalp INTEGER NOT NULL,
         PRIMARY KEY (model_id, geoset_type, geoset_id)
     );
     CREATE TABLE texture_fdids (
         material_resources_id INTEGER PRIMARY KEY,
         file_data_id INTEGER NOT NULL
     );"
}

fn customization_cache_create_tables_sql() -> String {
    format!(
        "{}
         {}
         {}",
        customization_cache_core_tables_sql(),
        customization_cache_relation_tables_sql(),
        customization_cache_lookup_tables_sql(),
    )
}

fn insert_simple_rows<T, F>(
    conn: &Connection,
    sql: &str,
    path: &Path,
    mut row_builder: F,
) -> Result<(), String>
where
    T: rusqlite::Params,
    F: FnMut(&[String], &[String], &Path) -> Result<Option<T>, String>,
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
        let Some(params) = row_builder(&headers, &fields, path)? else {
            continue;
        };
        insert
            .execute(params)
            .map_err(|err| format!("insert row for {}: {err}", path.display()))?;
    }
    Ok(())
}

fn parse_u32(fields: &[String], index: usize) -> u32 {
    fields
        .get(index)
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0)
}

fn parse_f32(fields: &[String], index: usize) -> f32 {
    fields
        .get(index)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0)
}

fn parse_str(fields: &[String], index: usize) -> String {
    fields.get(index).cloned().unwrap_or_default()
}

fn populate_chr_models(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO chr_models (id, layout_id, customize_scale, camera_distance_offset) VALUES (?1, ?2, ?3, ?4)",
        path,
        |headers, fields, path| {
            let id = header_index(headers, "ID", path)?;
            let layout = header_index(headers, "CharComponentTextureLayoutID", path)?;
            let customize_scale = header_index(headers, "CustomizeScale", path)?;
            let camera_distance_offset = header_index(headers, "CameraDistanceOffset", path)?;
            Ok(Some((
                parse_u32(fields, id),
                parse_u32(fields, layout),
                parse_f32(fields, customize_scale),
                parse_f32(fields, camera_distance_offset),
            )))
        },
    )
}

fn populate_options(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO options (id, name, chr_model_id) VALUES (?1, ?2, ?3)",
        path,
        |headers, fields, path| {
            let id = header_index(headers, "ID", path)?;
            let name = header_index(headers, "Name_lang", path)?;
            let model = header_index(headers, "ChrModelID", path)?;
            Ok(Some((
                parse_u32(fields, id),
                parse_str(fields, name),
                parse_u32(fields, model),
            )))
        },
    )
}

fn populate_choices(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO choices (id, option_id, name, requirement_id, order_index) VALUES (?1, ?2, ?3, ?4, ?5)",
        path,
        |headers, fields, path| {
            let id = header_index(headers, "ID", path)?;
            let option_id = header_index(headers, "ChrCustomizationOptionID", path)?;
            let name = header_index(headers, "Name_lang", path)?;
            let requirement_id = header_index(headers, "ChrCustomizationReqID", path)?;
            let order_index = header_index(headers, "OrderIndex", path)?;
            Ok(Some((
                parse_u32(fields, id),
                parse_u32(fields, option_id),
                parse_str(fields, name),
                parse_u32(fields, requirement_id),
                parse_u32(fields, order_index),
            )))
        },
    )
}

fn populate_elements(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO elements (choice_id, related_choice_id, geoset_id, material_id) VALUES (?1, ?2, ?3, ?4)",
        path,
        |headers, fields, path| {
            let choice_id = header_index(headers, "ChrCustomizationChoiceID", path)?;
            let related_choice_id = header_index(headers, "RelatedChrCustomizationChoiceID", path)?;
            let geoset_id = header_index(headers, "ChrCustomizationGeosetID", path)?;
            let material_id = header_index(headers, "ChrCustomizationMaterialID", path)?;
            Ok(Some((
                parse_u32(fields, choice_id),
                parse_u32(fields, related_choice_id),
                parse_u32(fields, geoset_id),
                parse_u32(fields, material_id),
            )))
        },
    )
}

fn populate_materials(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO materials (id, texture_target_id, material_resources_id) VALUES (?1, ?2, ?3)",
        path,
        |headers, fields, path| {
            let id = header_index(headers, "ID", path)?;
            let target = header_index(headers, "ChrModelTextureTargetID", path)?;
            let res = header_index(headers, "MaterialResourcesID", path)?;
            Ok(Some((
                parse_u32(fields, id),
                parse_u32(fields, target) as u16,
                parse_u32(fields, res),
            )))
        },
    )
}

fn populate_geosets(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO geosets (id, geoset_type, geoset_id) VALUES (?1, ?2, ?3)",
        path,
        |headers, fields, path| {
            let id = header_index(headers, "ID", path)?;
            let geoset_type = header_index(headers, "GeosetType", path)?;
            let geoset_id = header_index(headers, "GeosetID", path)?;
            Ok(Some((
                parse_u32(fields, id),
                parse_u32(fields, geoset_type) as u16,
                parse_u32(fields, geoset_id) as u16,
            )))
        },
    )
}

fn populate_hair_geosets(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT OR REPLACE INTO hair_geosets (model_id, geoset_type, geoset_id, shows_scalp) VALUES (?1, ?2, ?3, ?4)",
        path,
        |headers, fields, path| {
            let race = header_index(headers, "RaceID", path)?;
            let sex = header_index(headers, "SexID", path)?;
            let geoset_type = header_index(headers, "GeosetType", path)?;
            let geoset_id = header_index(headers, "GeosetID", path)?;
            let shows_scalp = header_index(headers, "Showscalp", path)?;
            let Some(model_id) = chr_model_id_for_hair_row(
                parse_u32(fields, race) as u8,
                parse_u32(fields, sex) as u8,
            ) else {
                return Ok(None);
            };
            Ok(Some((
                model_id,
                parse_u32(fields, geoset_type) as u16,
                parse_u32(fields, geoset_id) as u16,
                parse_u32(fields, shows_scalp) != 0,
            )))
        },
    )
}

fn populate_texture_fdids(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT OR REPLACE INTO texture_fdids (material_resources_id, file_data_id) VALUES (?1, ?2)",
        path,
        |headers, fields, path| {
            let file_data_id = header_index(headers, "FileDataID", path)?;
            let material_resources_id = header_index(headers, "MaterialResourcesID", path)?;
            Ok(Some((
                parse_u32(fields, material_resources_id),
                parse_u32(fields, file_data_id),
            )))
        },
    )
}

pub fn import_customization_cache(data_dir: &Path) -> Result<PathBuf, String> {
    let cache_path = cache_path();
    let mut csv_paths = required_csv_paths(data_dir).to_vec();
    csv_paths.push(texture_file_data_path(data_dir));
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

pub(crate) fn load_customization_raw_data(_data_dir: &Path) -> Result<RawData, String> {
    let cache_path = cache_path();
    if !cache_path.exists() {
        return Err(format!(
            "{} missing; run `cargo run --bin customization_cache_import` to build it",
            cache_path.display()
        ));
    }
    let conn = open_read_only(&cache_path)?;
    Ok(RawData {
        chr_models: load_chr_models(&conn)?,
        options: load_options(&conn)?,
        choices: load_choices(&conn)?,
        elements: load_elements(&conn)?,
        materials: load_materials(&conn)?,
        geosets: load_geosets(&conn)?,
        hair_geosets: load_hair_geosets(&conn)?,
        texture_fdids: load_texture_fdids(&conn)?,
    })
}

fn load_chr_models(conn: &Connection) -> Result<Vec<RawChrModel>, String> {
    let mut chr_models_stmt = conn
        .prepare("SELECT id, layout_id, customize_scale, camera_distance_offset FROM chr_models ORDER BY id")
        .map_err(|err| format!("prepare chr_models lookup: {err}"))?;
    chr_models_stmt
        .query_map([], |row| {
            Ok(RawChrModel {
                id: row.get(0)?,
                layout_id: row.get(1)?,
                customize_scale: row.get(2)?,
                camera_distance_offset: row.get(3)?,
            })
        })
        .map_err(|err| format!("query chr_models: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read chr_models row: {err}"))
}

fn load_options(conn: &Connection) -> Result<Vec<RawOption>, String> {
    let mut options_stmt = conn
        .prepare("SELECT id, name, chr_model_id FROM options ORDER BY id")
        .map_err(|err| format!("prepare options lookup: {err}"))?;
    options_stmt
        .query_map([], |row| {
            Ok(RawOption {
                id: row.get(0)?,
                name: row.get(1)?,
                chr_model_id: row.get(2)?,
            })
        })
        .map_err(|err| format!("query options: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read options row: {err}"))
}

fn load_choices(conn: &Connection) -> Result<Vec<RawChoice>, String> {
    let mut choices_stmt = conn
        .prepare("SELECT id, option_id, name, requirement_id, order_index FROM choices ORDER BY id")
        .map_err(|err| format!("prepare choices lookup: {err}"))?;
    choices_stmt
        .query_map([], |row| {
            Ok(RawChoice {
                id: row.get(0)?,
                option_id: row.get(1)?,
                name: row.get(2)?,
                requirement_id: row.get(3)?,
                order_index: row.get(4)?,
            })
        })
        .map_err(|err| format!("query choices: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read choices row: {err}"))
}

fn load_elements(conn: &Connection) -> Result<Vec<RawElement>, String> {
    let mut elements_stmt = conn
        .prepare("SELECT choice_id, related_choice_id, geoset_id, material_id FROM elements")
        .map_err(|err| format!("prepare elements lookup: {err}"))?;
    elements_stmt
        .query_map([], |row| {
            Ok(RawElement {
                choice_id: row.get(0)?,
                related_choice_id: row.get(1)?,
                geoset_id: row.get(2)?,
                material_id: row.get(3)?,
            })
        })
        .map_err(|err| format!("query elements: {err}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read elements row: {err}"))
}

fn load_materials(conn: &Connection) -> Result<HashMap<u32, RawMaterial>, String> {
    let mut materials_stmt = conn
        .prepare("SELECT id, texture_target_id, material_resources_id FROM materials")
        .map_err(|err| format!("prepare materials lookup: {err}"))?;
    materials_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                RawMaterial {
                    texture_target_id: row.get(1)?,
                    material_resources_id: row.get(2)?,
                },
            ))
        })
        .map_err(|err| format!("query materials: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read materials row: {err}"))
}

fn load_geosets(conn: &Connection) -> Result<HashMap<u32, RawGeoset>, String> {
    let mut geosets_stmt = conn
        .prepare("SELECT id, geoset_type, geoset_id FROM geosets")
        .map_err(|err| format!("prepare geosets lookup: {err}"))?;
    geosets_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                RawGeoset {
                    geoset_type: row.get(1)?,
                    geoset_id: row.get(2)?,
                },
            ))
        })
        .map_err(|err| format!("query geosets: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read geosets row: {err}"))
}

fn load_hair_geosets(conn: &Connection) -> Result<HashMap<HairGeosetKey, bool>, String> {
    let mut hair_stmt = conn
        .prepare("SELECT model_id, geoset_type, geoset_id, shows_scalp FROM hair_geosets")
        .map_err(|err| format!("prepare hair_geosets lookup: {err}"))?;
    hair_stmt
        .query_map([], |row| {
            Ok((
                (
                    row.get::<_, u32>(0)?,
                    row.get::<_, u16>(1)?,
                    row.get::<_, u16>(2)?,
                ),
                row.get::<_, bool>(3)?,
            ))
        })
        .map_err(|err| format!("query hair_geosets: {err}"))?
        .collect::<Result<HashMap<HairGeosetKey, bool>, _>>()
        .map_err(|err| format!("read hair_geosets row: {err}"))
}

fn load_texture_fdids(conn: &Connection) -> Result<HashMap<u32, u32>, String> {
    let mut texture_stmt = conn
        .prepare("SELECT material_resources_id, file_data_id FROM texture_fdids")
        .map_err(|err| format!("prepare texture_fdids lookup: {err}"))?;
    texture_stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query texture_fdids: {err}"))?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|err| format!("read texture_fdids row: {err}"))
}

#[cfg(test)]
mod tests {
    use super::{import_customization_cache, load_customization_raw_data};
    use std::path::Path;

    #[test]
    fn customization_raw_data_loads_from_imported_cache() {
        import_customization_cache(Path::new("data")).expect("import customization cache");
        let raw = load_customization_raw_data(Path::new("data")).expect("load customization cache");
        assert!(!raw.chr_models.is_empty());
        assert!(!raw.options.is_empty());
        assert!(!raw.choices.is_empty());
        assert!(!raw.elements.is_empty());
        assert!(!raw.materials.is_empty());
        assert!(!raw.geosets.is_empty());
        assert!(!raw.hair_geosets.is_empty());
        assert!(!raw.texture_fdids.is_empty());
    }

    #[test]
    fn customization_cache_import_reuses_fresh_cache() {
        let cache_path =
            import_customization_cache(Path::new("data")).expect("import customization cache");
        let before = std::fs::metadata(&cache_path)
            .expect("stat customization cache")
            .modified()
            .expect("customization cache mtime");
        let reused_path =
            import_customization_cache(Path::new("data")).expect("reuse customization cache");
        let after = std::fs::metadata(&reused_path)
            .expect("stat reused customization cache")
            .modified()
            .expect("reused customization cache mtime");
        assert_eq!(cache_path, reused_path);
        assert_eq!(before, after);
    }
}
