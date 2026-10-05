use std::path::{Path, PathBuf};

use rusqlite::Connection;
use std::time::Duration;

const CACHE_BUSY_TIMEOUT: Duration = Duration::from_secs(30);

const OUTFIT_LINKS_SCHEMA_SQL: &str = "DROP TABLE IF EXISTS source_files;
DROP TABLE IF EXISTS starter_outfits;
DROP TABLE IF EXISTS item_modified_appearance_map;
DROP TABLE IF EXISTS item_appearance_map;
DROP TABLE IF EXISTS display_info;
DROP TABLE IF EXISTS material_textures;
DROP TABLE IF EXISTS display_materials;
DROP TABLE IF EXISTS model_to_fdid;
CREATE TABLE source_files (source TEXT PRIMARY KEY, mtime_secs INTEGER NOT NULL);
CREATE TABLE starter_outfits (
    race_id INTEGER NOT NULL,
    class_id INTEGER NOT NULL,
    sex_id INTEGER NOT NULL,
    item_order INTEGER NOT NULL,
    item_id INTEGER NOT NULL
);
CREATE TABLE item_modified_appearance_map (
    item_id INTEGER PRIMARY KEY,
    appearance_id INTEGER NOT NULL
);
CREATE TABLE item_appearance_map (
    appearance_id INTEGER PRIMARY KEY,
    display_info_id INTEGER NOT NULL
);
CREATE TABLE display_info (
    id INTEGER PRIMARY KEY,
    model_res_0 INTEGER NOT NULL,
    model_res_1 INTEGER NOT NULL,
    model_mat_res_0 INTEGER NOT NULL,
    model_mat_res_1 INTEGER NOT NULL,
    geoset_group_0 INTEGER NOT NULL,
    geoset_group_1 INTEGER NOT NULL,
    geoset_group_2 INTEGER NOT NULL,
    geoset_group_3 INTEGER NOT NULL,
    geoset_group_4 INTEGER NOT NULL,
    geoset_group_5 INTEGER NOT NULL,
    helmet_vis_0 INTEGER NOT NULL,
    helmet_vis_1 INTEGER NOT NULL
);
CREATE TABLE material_textures (
    material_resource_id INTEGER NOT NULL,
    file_order INTEGER NOT NULL,
    texture_fdid INTEGER NOT NULL,
    PRIMARY KEY (material_resource_id, file_order)
);
CREATE TABLE display_materials (
    display_info_id INTEGER NOT NULL,
    component_section INTEGER NOT NULL,
    material_resource_id INTEGER NOT NULL,
    PRIMARY KEY (display_info_id, component_section, material_resource_id)
);
CREATE TABLE model_to_fdid (
    model_resource_id INTEGER NOT NULL,
    model_order INTEGER NOT NULL,
    file_data_id INTEGER NOT NULL,
    PRIMARY KEY (model_resource_id, model_order)
);
CREATE INDEX idx_model_to_fdid_file_data_id ON model_to_fdid(file_data_id);";

pub(super) fn import_outfit_links_cache(data_dir: &Path) -> Result<PathBuf, String> {
    let cache_path = super::outfit_links_cache_path(data_dir);
    let csv_paths = outfit_source_paths(data_dir)?;
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    conn.busy_timeout(CACHE_BUSY_TIMEOUT)
        .map_err(|err| format!("set outfit cache busy timeout: {err}"))?;
    conn.execute_batch("BEGIN IMMEDIATE;")
        .map_err(|err| format!("lock outfit links cache: {err}"))?;
    if super::outfit_cache_is_fresh(&conn, &csv_paths)? {
        conn.execute_batch("COMMIT;")
            .map_err(|err| format!("release outfit links cache: {err}"))?;
        return Ok(cache_path);
    }
    init_schema(&conn)?;
    record_source_files(&conn, &csv_paths)?;
    import_rows(&conn, &csv_paths)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit outfit_links cache: {err}"))?;
    Ok(cache_path)
}

pub(super) fn imported_outfit_links_cache_path(data_dir: &Path) -> Result<PathBuf, String> {
    let cache_path = super::outfit_links_cache_path(data_dir);
    if !cache_path.exists() {
        return Err(format!(
            "{} missing; run `cargo run --bin outfit_links_cache_import` to build it",
            cache_path.display()
        ));
    }
    let csv_paths = outfit_source_paths(data_dir)?;
    let conn = super::open_read_only(&cache_path)?;
    if !super::outfit_cache_is_fresh(&conn, &csv_paths)? {
        return Err(format!(
            "{} is stale; run `cargo run --bin outfit_links_cache_import` to rebuild it",
            cache_path.display()
        ));
    }
    Ok(cache_path)
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(OUTFIT_LINKS_SCHEMA_SQL)
        .map_err(|err| format!("init outfit_links cache: {err}"))
}

fn record_source_files(conn: &Connection, csv_paths: &[PathBuf]) -> Result<(), String> {
    let mut source_insert = conn
        .prepare("INSERT INTO source_files (source, mtime_secs) VALUES (?1, ?2)")
        .map_err(|err| format!("prepare source_files insert: {err}"))?;
    for path in csv_paths {
        let source = super::outfit_csv_source_key(path)?;
        let mtime = super::csv_mtime(path)?;
        source_insert
            .execute((source, mtime))
            .map_err(|err| format!("insert source_files {}: {err}", path.display()))?;
    }
    Ok(())
}

fn import_rows(conn: &Connection, csv_paths: &[PathBuf]) -> Result<(), String> {
    super::populate_starter_outfits(conn, &csv_paths[0])?;
    super::populate_item_modified_appearance_map(conn, &csv_paths[1])?;
    super::populate_item_appearance_map(conn, &csv_paths[2])?;
    super::populate_display_info(conn, &csv_paths[3])?;
    super::material_links::populate_material_textures(conn, &csv_paths[4])?;
    super::material_links::populate_display_materials(conn, &csv_paths[5])?;
    super::populate_model_to_fdid(conn, &csv_paths[6])?;
    if csv_paths.len() > 7 {
        import_forever_gear_rows(conn, &csv_paths[7..])?;
    }
    Ok(())
}

fn outfit_source_paths(data_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = super::required_outfit_csv_paths(data_dir).to_vec();
    let dir = data_dir.join("db2/1.60.1.70205");
    if !dir.join("ItemDisplayInfo.csv").exists() {
        return Ok(paths);
    }
    for name in [
        "ItemDisplayInfo",
        "TextureFileData",
        "ItemDisplayInfoMaterialRes",
        "ModelFileData",
    ] {
        let path = dir.join(format!("{name}.csv"));
        if !path.is_file() {
            return Err(format!("Forever gear table missing: {}", path.display()));
        }
        paths.push(path);
    }
    Ok(paths)
}

fn import_forever_gear_rows(conn: &Connection, paths: &[PathBuf]) -> Result<(), String> {
    let overlay =
        Connection::open_in_memory().map_err(|err| format!("open Forever gear staging: {err}"))?;
    init_schema(&overlay)?;
    super::populate_display_info(&overlay, &paths[0])?;
    super::material_links::populate_material_textures(&overlay, &paths[1])?;
    super::material_links::populate_declared_display_materials(&overlay, &paths[2])?;
    super::populate_model_to_fdid(&overlay, &paths[3])?;
    // Retail resource collisions keep the entire Retail candidate group.
    copy_missing_gear_groups(
        conn,
        &overlay,
        "display_materials",
        "display_info",
        "id",
        "display_info_id",
    )?;
    copy_missing_gear_groups(conn, &overlay, "display_info", "display_info", "id", "id")?;
    copy_missing_gear_groups(
        conn,
        &overlay,
        "material_textures",
        "material_textures",
        "material_resource_id",
        "material_resource_id",
    )?;
    copy_missing_gear_groups(
        conn,
        &overlay,
        "model_to_fdid",
        "model_to_fdid",
        "model_resource_id",
        "model_resource_id",
    )?;
    Ok(())
}

fn copy_missing_gear_groups(
    retail: &Connection,
    overlay: &Connection,
    table: &str,
    retail_table: &str,
    retail_key: &str,
    overlay_key: &str,
) -> Result<(), String> {
    use std::collections::HashSet;
    let mut keys = retail
        .prepare(&format!("SELECT DISTINCT {retail_key} FROM {retail_table}"))
        .map_err(|err| format!("read Retail {retail_table} keys: {err}"))?;
    let blocked = keys
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|err| format!("query Retail gear keys: {err}"))?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|err| format!("read Retail gear key: {err}"))?;
    let mut source = overlay
        .prepare(&format!("SELECT * FROM {table}"))
        .map_err(|err| format!("read Forever {table}: {err}"))?;
    let key_index = source
        .column_index(overlay_key)
        .map_err(|err| format!("Forever {table} key: {err}"))?;
    let count = source.column_count();
    let placeholders = vec!["?"; count].join(",");
    let mut insert = retail
        .prepare(&format!("INSERT INTO {table} VALUES ({placeholders})"))
        .map_err(|err| format!("prepare Forever {table} insert: {err}"))?;
    let rows = source
        .query_map([], |row| {
            (0..count)
                .map(|i| row.get::<_, i64>(i))
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|err| format!("query Forever {table}: {err}"))?;
    for row in rows {
        let row = row.map_err(|err| format!("read Forever {table} row: {err}"))?;
        if !blocked.contains(&row[key_index]) {
            insert
                .execute(rusqlite::params_from_iter(row))
                .map_err(|err| format!("insert Forever {table}: {err}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn outfit_links_cache_import_reuses_fresh_cache() {
        let data_dir = Path::new("data");
        let cache_path = super::import_outfit_links_cache(data_dir).expect("import outfit links");
        let before = std::fs::metadata(&cache_path)
            .expect("stat outfit links cache")
            .modified()
            .expect("outfit links cache mtime");
        let reused_path =
            super::import_outfit_links_cache(data_dir).expect("reuse outfit links cache");
        let after = std::fs::metadata(&reused_path)
            .expect("stat reused outfit links cache")
            .modified()
            .expect("reused outfit links cache mtime");
        assert_eq!(cache_path, reused_path);
        assert_eq!(before, after);
    }
}
