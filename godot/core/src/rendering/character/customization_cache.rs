use crate::cache_source_mtime::{csv_mtime, source_key};
use crate::cache_sqlite::{open_read_only, replace_atomically};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::csv_util::{header_index, parse_csv_line_trimmed as parse_csv_line};
use crate::customization_data::RaceModels;

use crate::customization_query_data::{CACHE_SCHEMA_VERSION, customization_cache_file};
use crate::sqlite_util::is_missing_table_error;

fn cache_path(data_dir: &Path) -> PathBuf {
    data_dir.join("cache").join(customization_cache_file())
}

fn required_csv_paths(data_dir: &Path) -> [PathBuf; 9] {
    [
        data_dir.join("ChrModel.csv"),
        data_dir.join("ChrCustomizationOption.csv"),
        data_dir.join("ChrCustomizationChoice.csv"),
        data_dir.join("ChrCustomizationElement.csv"),
        data_dir.join("ChrCustomizationMaterial.csv"),
        data_dir.join("ChrCustomizationGeoset.csv"),
        data_dir.join("CharHairGeosets.csv"),
        data_dir.join("ChrCustomizationCategory.csv"),
        data_dir.join("ChrCustomizationSkinnedModel.csv"),
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

fn cache_is_fresh(
    conn: &Connection,
    data_dir: &Path,
    csv_paths: &[PathBuf],
) -> Result<bool, String> {
    let version: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|err| format!("read customization cache schema version: {err}"))?;
    if version != CACHE_SCHEMA_VERSION {
        return Ok(false);
    }
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

fn rebuild_cache(cache_path: &Path, data_dir: &Path) -> Result<(), String> {
    let csv_paths = required_csv_paths(data_dir);
    let texture_file_data = texture_file_data_path(data_dir);
    let all_sources = rebuild_source_paths(&csv_paths, &texture_file_data);
    replace_atomically(cache_path, |conn| {
        init_cache_schema(conn)?;
        record_source_files(conn, data_dir, &all_sources)?;
        populate_chr_models(conn, &csv_paths[0])?;
        populate_options(conn, &csv_paths[1])?;
        populate_choices(conn, &csv_paths[2])?;
        populate_elements(conn, &csv_paths[3])?;
        populate_materials(conn, &csv_paths[4])?;
        populate_geosets(conn, &csv_paths[5])?;
        populate_hair_geosets(conn, &csv_paths[6], &RaceModels::load(data_dir)?)?;
        populate_categories(conn, &csv_paths[7])?;
        populate_skinned_models(conn, &csv_paths[8])?;
        populate_texture_fdids(conn, &texture_file_data)?;
        conn.execute_batch("COMMIT;")
            .map_err(|err| format!("commit customization cache: {err}"))
    })
}

fn rebuild_source_paths(csv_paths: &[PathBuf; 9], texture_file_data: &Path) -> Vec<PathBuf> {
    let mut all_sources = csv_paths.to_vec();
    all_sources.push(texture_file_data.to_path_buf());
    all_sources
}

fn init_cache_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(&build_customization_cache_schema_sql())
        .map_err(|err| format!("init customization cache: {err}"))?;
    conn.pragma_update(None, "user_version", CACHE_SCHEMA_VERSION)
        .map_err(|err| format!("set customization cache schema version: {err}"))
}

fn build_customization_cache_schema_sql() -> String {
    format!(
        "BEGIN;
         {drops}
         {creates}",
        drops = customization_cache_drop_tables_sql(),
        creates = customization_cache_create_tables_sql(),
    )
}

fn customization_cache_drop_tables_sql() -> &'static str {
    "DROP TABLE IF EXISTS source_files;
     DROP TABLE IF EXISTS chr_models;
     DROP TABLE IF EXISTS options;
     DROP TABLE IF EXISTS categories;
     DROP TABLE IF EXISTS choices;
     DROP TABLE IF EXISTS elements;
     DROP TABLE IF EXISTS materials;
     DROP TABLE IF EXISTS geosets;
     DROP TABLE IF EXISTS skinned_models;
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
         chr_model_id INTEGER NOT NULL,
         category_id INTEGER NOT NULL,
         order_index INTEGER NOT NULL,
         ui_type INTEGER NOT NULL,
         requirement_id INTEGER NOT NULL
     );
     CREATE TABLE categories (
         id INTEGER PRIMARY KEY,
         name TEXT NOT NULL,
         order_index INTEGER NOT NULL,
         icon INTEGER NOT NULL,
         selected_icon INTEGER NOT NULL
     );
     CREATE TABLE choices (
         id INTEGER PRIMARY KEY,
         option_id INTEGER NOT NULL,
         name TEXT NOT NULL,
         requirement_id INTEGER NOT NULL,
         order_index INTEGER NOT NULL,
         visibility_requirement_id INTEGER NOT NULL,
         swatch_color_0 INTEGER NOT NULL,
         swatch_color_1 INTEGER NOT NULL
     );"
}

fn customization_cache_relation_tables_sql() -> &'static str {
    "CREATE TABLE elements (
         choice_id INTEGER NOT NULL,
         related_choice_id INTEGER NOT NULL,
         geoset_id INTEGER NOT NULL,
         material_id INTEGER NOT NULL,
         skinned_model_id INTEGER NOT NULL,
         has_unsupported_effects INTEGER NOT NULL
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
     );
     CREATE TABLE skinned_models (
         id INTEGER PRIMARY KEY,
         collection_fdid INTEGER NOT NULL,
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

fn parse_i32(fields: &[String], index: usize, path: &Path) -> Result<i32, String> {
    let value = fields.get(index).ok_or_else(|| {
        format!(
            "missing signed integer column {index} in {}",
            path.display()
        )
    })?;
    value.parse().map_err(|err| {
        format!(
            "invalid signed integer {value:?} in {} column {index}: {err}",
            path.display()
        )
    })
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
        "INSERT INTO options (id, name, chr_model_id, category_id, order_index, ui_type, requirement_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        path,
        |headers, fields, path| {
            let id = header_index(headers, "ID", path)?;
            let name = header_index(headers, "Name_lang", path)?;
            let model = header_index(headers, "ChrModelID", path)?;
            let category = header_index(headers, "ChrCustomizationCategoryID", path)?;
            let order = header_index(headers, "OrderIndex", path)?;
            let ui_type = header_index(headers, "OptionType", path)?;
            let requirement = header_index(headers, "Requirement", path)?;
            Ok(Some((
                parse_u32(fields, id),
                parse_str(fields, name),
                parse_u32(fields, model),
                parse_u32(fields, category),
                parse_u32(fields, order),
                parse_u32(fields, ui_type),
                parse_u32(fields, requirement),
            )))
        },
    )
}

fn populate_categories(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO categories (id, name, order_index, icon, selected_icon) VALUES (?1, ?2, ?3, ?4, ?5)",
        path,
        |headers, fields, path| {
            let id = header_index(headers, "ID", path)?;
            let name = header_index(headers, "CategoryName_lang", path)?;
            let order = header_index(headers, "OrderIndex", path)?;
            let icon = header_index(headers, "CustomizeIcon", path)?;
            let selected_icon = header_index(headers, "CustomizeIconSelected", path)?;
            Ok(Some((
                parse_u32(fields, id),
                parse_str(fields, name),
                parse_u32(fields, order),
                parse_u32(fields, icon),
                parse_u32(fields, selected_icon),
            )))
        },
    )
}

fn populate_choices(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO choices (id, option_id, name, requirement_id, order_index, visibility_requirement_id, swatch_color_0, swatch_color_1) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        path,
        |headers, fields, path| {
            let id = header_index(headers, "ID", path)?;
            let option_id = header_index(headers, "ChrCustomizationOptionID", path)?;
            let name = header_index(headers, "Name_lang", path)?;
            let requirement_id = header_index(headers, "ChrCustomizationReqID", path)?;
            let order_index = header_index(headers, "OrderIndex", path)?;
            let visibility_requirement = header_index(headers, "ChrCustomizationVisReqID", path)?;
            let swatch_color_0 = header_index(headers, "SwatchColor_0", path)?;
            let swatch_color_1 = header_index(headers, "SwatchColor_1", path)?;
            Ok(Some((
                parse_u32(fields, id),
                parse_u32(fields, option_id),
                parse_str(fields, name),
                parse_u32(fields, requirement_id),
                parse_u32(fields, order_index),
                parse_u32(fields, visibility_requirement),
                parse_i32(fields, swatch_color_0, path)?,
                parse_i32(fields, swatch_color_1, path)?,
            )))
        },
    )
}

fn populate_elements(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO elements (choice_id, related_choice_id, geoset_id, material_id, skinned_model_id, has_unsupported_effects) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        path,
        |headers, fields, path| {
            let choice_id = header_index(headers, "ChrCustomizationChoiceID", path)?;
            let related_choice_id = header_index(headers, "RelatedChrCustomizationChoiceID", path)?;
            let geoset_id = header_index(headers, "ChrCustomizationGeosetID", path)?;
            let material_id = header_index(headers, "ChrCustomizationMaterialID", path)?;
            let skinned_model_id = header_index(headers, "ChrCustomizationSkinnedModelID", path)?;
            Ok(Some((
                parse_u32(fields, choice_id),
                parse_u32(fields, related_choice_id),
                parse_u32(fields, geoset_id),
                parse_u32(fields, material_id),
                parse_u32(fields, skinned_model_id),
                has_unsupported_effects(headers, fields),
            )))
        },
    )
}

fn has_unsupported_effects(headers: &[String], fields: &[String]) -> bool {
    const UNSUPPORTED_COLUMNS: [&str; 8] = [
        "ChrCustomizationBoneSetID",
        "ChrCustomizationCondModelID",
        "ChrCustomizationDisplayInfoID",
        "ChrCustItemGeoModifyID",
        "ChrCustomizationVoiceID",
        "AnimKitID",
        "ParticleColorID",
        "ChrCustGeoComponentLinkID",
    ];
    headers.iter().enumerate().any(|(index, name)| {
        UNSUPPORTED_COLUMNS.contains(&name.as_str()) && parse_u32(fields, index) != 0
    })
}

fn populate_skinned_models(conn: &Connection, path: &Path) -> Result<(), String> {
    insert_simple_rows(
        conn,
        "INSERT INTO skinned_models (id, collection_fdid, geoset_type, geoset_id) VALUES (?1, ?2, ?3, ?4)",
        path,
        |headers, fields, path| {
            let id = header_index(headers, "ID", path)?;
            let collection = header_index(headers, "CollectionsFileDataID", path)?;
            let geoset_type = header_index(headers, "GeosetType", path)?;
            let geoset_id = header_index(headers, "GeosetID", path)?;
            Ok(Some((
                parse_u32(fields, id),
                parse_u32(fields, collection),
                parse_u32(fields, geoset_type),
                parse_u32(fields, geoset_id),
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

fn populate_hair_geosets(
    conn: &Connection,
    path: &Path,
    race_models: &RaceModels,
) -> Result<(), String> {
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
            let Some(model_id) = race_models
                .chr_model_id(parse_u32(fields, race) as u8, parse_u32(fields, sex) as u8)
            else {
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
            let usage_type = header_index(headers, "UsageType", path)?;
            let material_resources_id = header_index(headers, "MaterialResourcesID", path)?;
            // A material's texture is its UsageType 0 row; others (such as the
            // Demon Hunter tattoos' opaque UsageType 2 companions) are not the
            // diffuse layer (wow.export `DBCharacterCustomization._initialize`).
            if parse_u32(fields, usage_type) != 0 {
                return Ok(None);
            }
            Ok(Some((
                parse_u32(fields, material_resources_id),
                parse_u32(fields, file_data_id),
            )))
        },
    )
}

/// Builds `<data_dir>/cache/customization-v<N>.sqlite` from the DB2 CSVs in `data_dir`,
/// reusing it when its recorded sources are unchanged.
pub fn import_customization_cache(data_dir: &Path) -> Result<PathBuf, String> {
    import_customization_cache_at(data_dir, &cache_path(data_dir))
}

fn import_customization_cache_at(data_dir: &Path, cache_path: &Path) -> Result<PathBuf, String> {
    let mut csv_paths = required_csv_paths(data_dir).to_vec();
    csv_paths.push(texture_file_data_path(data_dir));
    let needs_rebuild = if cache_path.exists() {
        let conn = open_read_only(&cache_path)?;
        !cache_is_fresh(&conn, data_dir, &csv_paths)?
    } else {
        true
    };
    if needs_rebuild {
        rebuild_cache(&cache_path, data_dir)?;
    }
    Ok(cache_path.to_path_buf())
}

#[cfg(test)]
fn load_customization_raw_data_at(
    data_dir: &Path,
    cache_path: &Path,
) -> Result<crate::customization_data::RawData, String> {
    import_customization_cache_at(data_dir, cache_path)?;
    let conn = open_read_only(cache_path)?;
    crate::customization_query_data::query_customization_raw_data(
        &conn,
        RaceModels::load(data_dir)?,
    )
}

#[cfg(test)]
#[path = "../../../tests/unit/customization_catalog_cache_tests.rs"]
mod catalog_tests;

#[cfg(test)]
mod tests {
    use super::{import_customization_cache, load_customization_raw_data_at};
    use crate::customization_data::RaceModels;
    use std::path::Path;

    #[test]
    fn query_customization_raw_data_reads_all_tables_and_keeps_order() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        super::init_cache_schema(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO chr_models VALUES (9, 90, 1.25, 2.5), (3, 30, 1.0, 0.0);
             INSERT INTO options VALUES (8, 'Hair', 9, 4, 2, 3, 7), (2, 'Face', 3, 1, 0, 1, 0);
             INSERT INTO categories VALUES (4, 'Appearance', 1, 12, 13);
             INSERT INTO choices VALUES (6, 8, 'Long', 2, 1, 3, -7, 99), (1, 2, 'Plain', 0, 0, 0, 0, 0);
             INSERT INTO elements VALUES (6, 1, 15, 16, 21, 1);
             INSERT INTO skinned_models VALUES (21, 7760205, 25, 1);
             INSERT INTO materials VALUES (16, 4, 77);
             INSERT INTO geosets VALUES (15, 3, 8);
             INSERT INTO hair_geosets VALUES (9, 3, 8, 1);
             INSERT INTO texture_fdids VALUES (77, 12345);",
        )
        .unwrap();
        let mut races = RaceModels::default();
        races.chr_model_by_race_sex.insert((1, 0), 9);
        let raw = crate::customization_query_data::query_customization_raw_data(&conn, races)
            .expect("query imported customization rows");
        assert_eq!(
            raw.chr_models.iter().map(|row| row.id).collect::<Vec<_>>(),
            [3, 9]
        );
        assert_eq!(
            raw.options.iter().map(|row| row.id).collect::<Vec<_>>(),
            [2, 8]
        );
        assert_eq!(
            raw.choices.iter().map(|row| row.id).collect::<Vec<_>>(),
            [1, 6]
        );
        assert_eq!(raw.chr_models[1].customize_scale, 1.25);
        assert_eq!(raw.options[1].requirement_id, 7);
        assert_eq!(raw.categories[&4].selected_icon, 13);
        assert_eq!(raw.choices[1].swatch_colors, [-7, 99]);
        assert!(raw.elements[0].has_unsupported_effects);
        assert_eq!(raw.elements[0].skinned_model_id, 21);
        assert_eq!(raw.skinned_models[&21].collection_fdid, 7760205);
        assert_eq!(
            (
                raw.skinned_models[&21].geoset_type,
                raw.skinned_models[&21].geoset_id
            ),
            (25, 1)
        );
        assert_eq!(raw.materials[&16].material_resources_id, 77);
        assert_eq!(raw.geosets[&15].geoset_id, 8);
        assert_eq!(raw.hair_geosets[&(9, 3, 8)], true);
        assert_eq!(raw.texture_fdids[&77], 12345);
        assert_eq!(raw.race_models.chr_model_id(1, 0), Some(9));
    }

    #[test]
    fn query_customization_raw_data_reports_missing_table_context() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let Err(error) = crate::customization_query_data::query_customization_raw_data(
            &conn,
            RaceModels::default(),
        ) else {
            panic!("missing table must fail");
        };
        assert!(error.contains("prepare chr_models lookup"), "{error}");
    }

    #[test]
    fn catalog_cache_rejects_the_previous_schema_even_with_unchanged_sources() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        super::init_cache_schema(&conn).unwrap();
        conn.pragma_update(None, "user_version", 0).unwrap();
        assert!(
            !super::cache_is_fresh(&conn, Path::new("data"), &[]).unwrap(),
            "an old cache must rebuild even when its source timestamps match"
        );
    }

    #[test]
    fn customization_raw_data_loads_from_imported_cache() {
        let cache =
            import_customization_cache(Path::new("data")).expect("import customization cache");
        let raw = load_customization_raw_data_at(Path::new("data"), &cache)
            .expect("load customization cache");
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
