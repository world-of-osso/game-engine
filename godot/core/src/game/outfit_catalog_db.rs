use crate::cache_source_mtime::{csv_mtime, source_key};
use crate::cache_sqlite::{open_read_only, rebuild_unless_fresh};
use crate::csv_util::{header_index, parse_csv_line_trimmed as parse_csv_line};
use crate::outfit_data::DisplayInfoResolved;
use crate::sqlite_util::is_missing_table_error;
use rusqlite::{Connection, Statement};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
#[path = "world_db/material_links.rs"]
mod material_links;
#[path = "world_db/outfit_cache_load.rs"]
mod outfit_cache_load;
#[path = "world_db/outfit_links.rs"]
mod outfit_links_cache;
#[path = "world_db/outfit_query.rs"]
mod outfit_query;
pub(crate) use outfit_query::{
    query_display_info, query_material_texture_fdids, query_model_fdids,
};

/// An owned-item namespace read from client CSVs; never writes the shared NPC cache.
pub(crate) fn load_owned_outfit_connection(
    items_dir: &Path,
    gear_dir: &Path,
    source_product: crate::asset_product::AssetProduct,
) -> Result<Connection, String> {
    let conn = Connection::open_in_memory()
        .map_err(|err| format!("open owned outfit namespace: {err}"))?;
    outfit_links_cache::init_schema(&conn)?;
    populate_item_modified_appearance_map(&conn, &items_dir.join("ItemModifiedAppearance.csv"))?;
    populate_item_appearance_map(&conn, &items_dir.join("ItemAppearance.csv"))?;
    populate_display_info(&conn, &gear_dir.join("ItemDisplayInfo.csv"))?;
    material_links::populate_material_textures(&conn, &gear_dir.join("TextureFileData.csv"))?;
    material_links::populate_declared_display_materials(
        &conn,
        &gear_dir.join("ItemDisplayInfoMaterialRes.csv"),
    )?;
    populate_model_to_fdid(&conn, &gear_dir.join("ModelFileData.csv"))?;
    outfit_links_cache::record_resource_products(&conn, source_product)?;
    Ok(conn)
}

pub(crate) fn query_item_display_id(conn: &Connection, item_id: u32) -> Result<u32, String> {
    conn.query_row(
        "SELECT iam.display_info_id
         FROM item_modified_appearance_map ima
         JOIN item_appearance_map iam ON iam.appearance_id = ima.appearance_id
         WHERE ima.item_id = ?1",
        [item_id],
        |row| row.get(0),
    )
    .map_err(|err| format!("resolve item {item_id} display: {err}"))
}

/// Versioned by schema: checkouts with an older schema keep reading their own file.
fn outfit_links_cache_path(data_dir: &Path) -> PathBuf {
    data_dir.join("cache/outfit_links-v5.sqlite")
}
fn required_outfit_csv_paths(data_dir: &Path) -> [PathBuf; 6] {
    let source = data_dir.join(crate::player_model_data::RETAIL_DB2_DIR);
    [
        source.join("ItemModifiedAppearance.csv"),
        source.join("ItemAppearance.csv"),
        source.join("ItemDisplayInfo.csv"),
        source.join("TextureFileData.csv"),
        source.join("ItemDisplayInfoMaterialRes.csv"),
        source.join("ModelFileData.csv"),
    ]
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn outfit_cache_is_fresh(
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
    if recorded.len() != csv_paths.len() {
        return Ok(false);
    }
    for path in csv_paths {
        let key = source_key(data_dir, path)?;
        if recorded.get(&key).copied() != Some(csv_mtime(path)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn import_outfit_links_cache(data_dir: &Path) -> Result<PathBuf, String> {
    outfit_links_cache::import_outfit_links_cache(data_dir)
}

fn populate_item_modified_appearance_map(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let item_col = header_index(&headers, "ItemID", path)?;
    let appearance_col = header_index(&headers, "ItemAppearanceID", path)?;
    let mut insert = conn
        .prepare(
            "INSERT OR IGNORE INTO item_modified_appearance_map (item_id, appearance_id)
             VALUES (?1, ?2)",
        )
        .map_err(|err| format!("prepare item_modified_appearance_map insert: {err}"))?;
    insert_item_modified_appearance_rows(&mut reader, path, item_col, appearance_col, &mut insert)?;
    Ok(())
}

fn insert_item_modified_appearance_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    item_col: usize,
    appearance_col: usize,
    insert: &mut Statement<'_>,
) -> Result<(), String> {
    let mut line = String::new();
    let mut line_number = 1;
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", path.display()))?
            == 0
        {
            break;
        }
        line_number += 1;
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        let item_id = crate::csv_util::parse_field::<u32>(&fields, item_col)
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        let appearance_id = crate::csv_util::parse_field::<u32>(&fields, appearance_col)
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        if item_id == 0 || appearance_id == 0 {
            continue;
        }
        insert
            .execute((item_id, appearance_id))
            .map_err(|err| format!("insert item_modified_appearance_map row: {err}"))?;
    }
    Ok(())
}

fn populate_item_appearance_map(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let id_col = header_index(&headers, "ID", path)?;
    let display_info_col = header_index(&headers, "ItemDisplayInfoID", path)?;
    let mut insert = conn
        .prepare(
            "INSERT OR REPLACE INTO item_appearance_map (appearance_id, display_info_id)
             VALUES (?1, ?2)",
        )
        .map_err(|err| format!("prepare item_appearance_map insert: {err}"))?;
    insert_item_appearance_rows(&mut reader, path, id_col, display_info_col, &mut insert)?;
    Ok(())
}

fn insert_item_appearance_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    id_col: usize,
    display_info_col: usize,
    insert: &mut Statement<'_>,
) -> Result<(), String> {
    let mut line = String::new();
    let mut line_number = 1;
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", path.display()))?
            == 0
        {
            break;
        }
        line_number += 1;
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        let appearance_id = crate::csv_util::parse_field::<u32>(&fields, id_col)
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        if appearance_id == 0 {
            continue;
        }
        let display_info_id = fields
            .get(display_info_col)
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| {
                format!(
                    "{} appearance {appearance_id}: invalid ItemDisplayInfoID",
                    path.display()
                )
            })?;
        // An authored display0 is no visual, not an absent appearance row.
        insert
            .execute((appearance_id, display_info_id))
            .map_err(|err| format!("insert item_appearance_map row: {err}"))?;
    }
    Ok(())
}

fn populate_display_info(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let columns = display_info_columns(&headers, path)?;
    let mut insert = conn
        .prepare(
            "INSERT OR REPLACE INTO display_info (
            id, model_res_0, model_res_1, model_mat_res_0, model_mat_res_1,
            geoset_group_0, geoset_group_1, geoset_group_2, geoset_group_3, geoset_group_4,
            geoset_group_5, helmet_vis_0, helmet_vis_1
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        )
        .map_err(|err| format!("prepare display_info insert: {err}"))?;
    insert_display_info_rows(&mut reader, path, &columns, &mut insert)?;
    Ok(())
}

struct DisplayInfoColumns {
    id: usize,
    model_res_0: usize,
    model_res_1: usize,
    model_mat_res_0: usize,
    model_mat_res_1: usize,
    geoset_groups: [usize; 6],
    helmet_vis_0: usize,
    helmet_vis_1: usize,
}

type DisplayInfoRow = (
    u32,
    u32,
    u32,
    u32,
    u32,
    i16,
    i16,
    i16,
    i16,
    i16,
    i16,
    u32,
    u32,
);

fn display_info_columns(headers: &[String], path: &Path) -> Result<DisplayInfoColumns, String> {
    Ok(DisplayInfoColumns {
        id: header_index(headers, "ID", path)?,
        model_res_0: header_index(headers, "ModelResourcesID_0", path)?,
        model_res_1: header_index(headers, "ModelResourcesID_1", path)?,
        model_mat_res_0: header_index(headers, "ModelMaterialResourcesID_0", path)?,
        model_mat_res_1: header_index(headers, "ModelMaterialResourcesID_1", path)?,
        geoset_groups: [
            header_index(headers, "GeosetGroup_0", path)?,
            header_index(headers, "GeosetGroup_1", path)?,
            header_index(headers, "GeosetGroup_2", path)?,
            header_index(headers, "GeosetGroup_3", path)?,
            header_index(headers, "GeosetGroup_4", path)?,
            header_index(headers, "GeosetGroup_5", path)?,
        ],
        helmet_vis_0: header_index(headers, "HelmetGeosetVis_0", path)?,
        helmet_vis_1: header_index(headers, "HelmetGeosetVis_1", path)?,
    })
}

fn insert_display_info_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    columns: &DisplayInfoColumns,
    insert: &mut Statement<'_>,
) -> Result<(), String> {
    let mut line = String::new();
    let mut line_number = 1;
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", path.display()))?
            == 0
        {
            break;
        }
        line_number += 1;
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        let Some(row) = parse_display_info_row(&fields, columns)
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?
        else {
            continue;
        };
        insert
            .execute(row)
            .map_err(|err| format!("insert display_info row: {err}"))?;
    }
    Ok(())
}

fn parse_display_info_row(
    fields: &[String],
    columns: &DisplayInfoColumns,
) -> Result<Option<DisplayInfoRow>, String> {
    let get_u32 = |index| crate::csv_util::parse_field::<u32>(fields, index);
    let get_i16 = |index| crate::csv_util::parse_field::<i16>(fields, index);
    let display_id = get_u32(columns.id)?;
    if display_id == 0 {
        return Ok(None);
    }
    Ok(Some((
        display_id,
        get_u32(columns.model_res_0)?,
        get_u32(columns.model_res_1)?,
        get_u32(columns.model_mat_res_0)?,
        get_u32(columns.model_mat_res_1)?,
        get_i16(columns.geoset_groups[0])?,
        get_i16(columns.geoset_groups[1])?,
        get_i16(columns.geoset_groups[2])?,
        get_i16(columns.geoset_groups[3])?,
        get_i16(columns.geoset_groups[4])?,
        get_i16(columns.geoset_groups[5])?,
        get_u32(columns.helmet_vis_0)?,
        get_u32(columns.helmet_vis_1)?,
    )))
}

fn populate_model_to_fdid(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let file_data_col = header_index(&headers, "FileDataID", path)?;
    let model_resource_col = header_index(&headers, "ModelResourcesID", path)?;
    let mut insert = conn.prepare(
        "INSERT INTO model_to_fdid (model_resource_id, model_order, file_data_id) VALUES (?1, ?2, ?3)"
    ).map_err(|err| format!("prepare model_to_fdid insert: {err}"))?;
    insert_model_to_fdid_rows(
        &mut reader,
        path,
        file_data_col,
        model_resource_col,
        &mut insert,
    )?;
    Ok(())
}

fn insert_model_to_fdid_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    file_data_col: usize,
    model_resource_col: usize,
    insert: &mut Statement<'_>,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut next_order: HashMap<u32, u32> = HashMap::new();
    let mut line = String::new();
    let mut line_number = 1;
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", path.display()))?
            == 0
        {
            break;
        }
        line_number += 1;
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        let file_data_id = crate::csv_util::parse_field::<u32>(&fields, file_data_col)
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        let model_resource_id = crate::csv_util::parse_field::<u32>(&fields, model_resource_col)
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        if file_data_id == 0
            || model_resource_id == 0
            || !seen.insert((model_resource_id, file_data_id))
        {
            continue;
        }
        let order = next_order.entry(model_resource_id).or_insert(0);
        insert
            .execute((model_resource_id, *order, file_data_id))
            .map_err(|err| format!("insert model_to_fdid row: {err}"))?;
        *order += 1;
    }
    Ok(())
}

fn imported_outfit_links_cache_path(data_dir: &Path) -> Result<PathBuf, String> {
    outfit_links_cache::imported_outfit_links_cache_path(data_dir)
}

pub(crate) fn load_cached_display_info(
    data_dir: &Path,
    display_info_id: u32,
) -> Result<Option<DisplayInfoResolved>, String> {
    outfit_query::load_cached_display_info(data_dir, display_info_id)
}

pub(crate) fn load_cached_material_texture_fdids(
    data_dir: &Path,
    material_resource_id: u32,
) -> Result<Vec<u32>, String> {
    outfit_query::load_cached_material_texture_fdids(data_dir, material_resource_id)
}

pub(crate) fn load_cached_model_fdids(
    data_dir: &Path,
    model_resource_id: u32,
) -> Result<Vec<u32>, String> {
    outfit_query::load_cached_model_fdids(data_dir, model_resource_id)
}

pub(crate) fn resolve_cached_skin_fdids_for_model_fdid(
    data_dir: &Path,
    model_fdid: u32,
) -> Result<Option<[u32; 3]>, String> {
    outfit_query::resolve_cached_skin_fdids_for_model_fdid(data_dir, model_fdid)
}

pub(crate) fn resolve_cached_skin_fdids_for_model_name(
    data_dir: &Path,
    model_name: &str,
) -> Result<Option<[u32; 3]>, String> {
    outfit_query::resolve_cached_skin_fdids_for_model_name(data_dir, model_name)
}

pub fn load_cached_item_modified_appearance(data_dir: &Path) -> Result<HashMap<u32, u32>, String> {
    outfit_cache_load::load_cached_item_modified_appearance(data_dir)
}

pub fn load_cached_item_appearance(data_dir: &Path) -> Result<HashMap<u32, u32>, String> {
    outfit_cache_load::load_cached_item_appearance(data_dir)
}

#[cfg(test)]
mod db2_header_tests {
    use super::*;
    #[test]
    fn db2_header_outfit_rejects_invalid_canonical_row() {
        let fixture =
            crate::db2_test_support::ExportFixture::copy("ItemModifiedAppearance", Some("ItemID"));
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE item_modified_appearance_map(item_id INTEGER, appearance_id INTEGER)",
        )
        .unwrap();
        let error = populate_item_modified_appearance_map(&conn, &fixture.0).unwrap_err();
        assert!(error.contains("ItemModifiedAppearance.csv:2:"), "{error}");
    }
}

#[cfg(test)]
mod db2_canonical_tests {
    use super::*;
    #[test]
    fn db2_header_outfits_read_all_installed_tables() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let conn = Connection::open_in_memory().unwrap();
        outfit_links_cache::init_schema(&conn).unwrap();
        for (file, table, populate) in [
            (
                "ItemModifiedAppearance",
                "item_modified_appearance_map",
                populate_item_modified_appearance_map
                    as fn(&Connection, &Path) -> Result<(), String>,
            ),
            (
                "ItemAppearance",
                "item_appearance_map",
                populate_item_appearance_map,
            ),
            ("ItemDisplayInfo", "display_info", populate_display_info),
            ("ModelFileData", "model_to_fdid", populate_model_to_fdid),
            (
                "TextureFileData",
                "material_textures",
                material_links::populate_material_textures,
            ),
            (
                "ItemDisplayInfoMaterialRes",
                "display_materials",
                material_links::populate_display_materials,
            ),
        ] {
            populate(&conn, &source.join(format!("{file}.csv"))).unwrap();
            let count: usize = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert!(count > 0, "{file} imported no rows");
        }
        assert!(query_item_display_id(&conn, 5580).unwrap() > 0);
    }
}
