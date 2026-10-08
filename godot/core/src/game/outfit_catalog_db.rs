use crate::cache_source_mtime::csv_mtime;
use crate::cache_sqlite::open_read_only;
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
#[path = "world_db/outfit_resolve.rs"]
mod outfit_resolve;
pub(crate) use outfit_query::{
    query_display_info, query_material_texture_fdids, query_model_fdids,
};

/// An owned-item namespace read from client CSVs; never writes the shared NPC cache.
pub(crate) fn load_owned_outfit_connection(
    items_dir: &Path,
    gear_dir: &Path,
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

type OutfitKey = (u8, u8, u8);
type StarterOutfits = HashMap<OutfitKey, Vec<u32>>;
/// Versioned by schema: checkouts with an older schema keep reading their own file.
fn outfit_links_cache_path(data_dir: &Path) -> PathBuf {
    data_dir.join("cache/outfit_links-v3.sqlite")
}
fn required_outfit_csv_paths(data_dir: &Path) -> [PathBuf; 7] {
    [
        data_dir.join("CharStartOutfit.csv"),
        data_dir.join("ItemModifiedAppearance.csv"),
        data_dir.join("ItemAppearance.csv"),
        data_dir.join("ItemDisplayInfo.csv"),
        data_dir.join("TextureFileData.csv"),
        data_dir.join("ItemDisplayInfoMaterialRes.csv"),
        data_dir.join("ModelFileData.csv"),
    ]
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

fn outfit_csv_source_key(path: &Path) -> Result<String, String> {
    Ok(path
        .canonicalize()
        .map_err(|err| format!("canonicalize {}: {err}", path.display()))?
        .to_string_lossy()
        .to_string())
}

fn outfit_cache_is_fresh(conn: &Connection, csv_paths: &[PathBuf]) -> Result<bool, String> {
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
        let key = outfit_csv_source_key(path)?;
        if recorded.get(&key).copied() != Some(csv_mtime(path)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn import_outfit_links_cache(data_dir: &Path) -> Result<PathBuf, String> {
    outfit_links_cache::import_outfit_links_cache(data_dir)
}

fn populate_starter_outfits(conn: &Connection, path: &Path) -> Result<(), String> {
    let mut reader = open_reader(path)?;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(header.trim_end_matches(['\r', '\n']));
    let race_col = header_index(&headers, "RaceID", path)?;
    let class_col = header_index(&headers, "ClassID", path)?;
    let sex_col = header_index(&headers, "SexID", path)?;
    let item_cols = (0..12)
        .map(|i| header_index(&headers, &format!("ItemID_{i}"), path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut insert = conn
        .prepare(
            "INSERT INTO starter_outfits (race_id, class_id, sex_id, item_order, item_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(|err| format!("prepare starter_outfits insert: {err}"))?;
    insert_starter_outfit_rows(
        &mut reader,
        path,
        race_col,
        class_col,
        sex_col,
        &item_cols,
        &mut insert,
    )?;
    Ok(())
}

fn insert_starter_outfit_rows(
    reader: &mut dyn BufRead,
    path: &Path,
    race_col: usize,
    class_col: usize,
    sex_col: usize,
    item_cols: &[usize],
    insert: &mut Statement<'_>,
) -> Result<(), String> {
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
        let race_id = fields
            .get(race_col)
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(0);
        let class_id = fields
            .get(class_col)
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(0);
        let sex_id = fields
            .get(sex_col)
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(0);
        insert_starter_outfit_items(insert, &fields, item_cols, race_id, class_id, sex_id)?;
    }
    Ok(())
}

fn insert_starter_outfit_items(
    insert: &mut Statement<'_>,
    fields: &[String],
    item_cols: &[usize],
    race_id: u8,
    class_id: u8,
    sex_id: u8,
) -> Result<(), String> {
    for (item_order, &column) in item_cols.iter().enumerate() {
        let item_id = fields
            .get(column)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        if item_id == 0 || item_id == 6948 {
            continue;
        }
        insert
            .execute((race_id, class_id, sex_id, item_order as u32, item_id))
            .map_err(|err| format!("insert starter_outfits row: {err}"))?;
    }
    Ok(())
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
        let item_id = fields
            .get(item_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let appearance_id = fields
            .get(appearance_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
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
        let appearance_id = fields
            .get(id_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let display_info_id = fields
            .get(display_info_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        if appearance_id == 0 || display_info_id == 0 {
            continue;
        }
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
        let Some(row) = parse_display_info_row(&fields, columns) else {
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
) -> Option<DisplayInfoRow> {
    let get_u32 = |idx: usize| {
        fields
            .get(idx)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let get_i16 = |idx: usize| {
        fields
            .get(idx)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0) as i16
    };
    let display_id = get_u32(columns.id);
    if display_id == 0 {
        return None;
    }
    Some((
        display_id,
        get_u32(columns.model_res_0),
        get_u32(columns.model_res_1),
        get_u32(columns.model_mat_res_0),
        get_u32(columns.model_mat_res_1),
        get_i16(columns.geoset_groups[0]),
        get_i16(columns.geoset_groups[1]),
        get_i16(columns.geoset_groups[2]),
        get_i16(columns.geoset_groups[3]),
        get_i16(columns.geoset_groups[4]),
        get_i16(columns.geoset_groups[5]),
        get_u32(columns.helmet_vis_0),
        get_u32(columns.helmet_vis_1),
    ))
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
        let file_data_id = fields
            .get(file_data_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let model_resource_id = fields
            .get(model_resource_col)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
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

pub fn load_cached_char_start_outfits(data_dir: &Path) -> Result<StarterOutfits, String> {
    outfit_cache_load::load_cached_char_start_outfits(data_dir)
}

pub fn resolve_cached_outfit_display_ids(
    data_dir: &Path,
    race: u8,
    class: u8,
    sex: u8,
) -> Result<Vec<u32>, String> {
    outfit_resolve::resolve_cached_outfit_display_ids(data_dir, race, class, sex)
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
