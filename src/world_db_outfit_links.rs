use std::path::{Path, PathBuf};

use rusqlite::Connection;

const OUTFIT_LINKS_SCHEMA_SQL: &str = "BEGIN;
DROP TABLE IF EXISTS source_files;
DROP TABLE IF EXISTS starter_outfits;
DROP TABLE IF EXISTS item_modified_appearance_map;
DROP TABLE IF EXISTS item_appearance_map;
DROP TABLE IF EXISTS display_info;
DROP TABLE IF EXISTS material_to_texture;
DROP TABLE IF EXISTS display_material_textures;
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
    helmet_vis_0 INTEGER NOT NULL,
    helmet_vis_1 INTEGER NOT NULL
);
CREATE TABLE material_to_texture (
    material_resource_id INTEGER PRIMARY KEY,
    texture_fdid INTEGER NOT NULL
);
CREATE TABLE display_material_textures (
    display_info_id INTEGER NOT NULL,
    component_section INTEGER NOT NULL,
    texture_fdid INTEGER NOT NULL,
    PRIMARY KEY (display_info_id, component_section, texture_fdid)
);
CREATE TABLE model_to_fdid (
    model_resource_id INTEGER NOT NULL,
    model_order INTEGER NOT NULL,
    file_data_id INTEGER NOT NULL,
    PRIMARY KEY (model_resource_id, model_order)
);";

pub(super) fn import_outfit_links_cache(data_dir: &Path) -> Result<PathBuf, String> {
    let cache_path = super::outfit_links_cache_path();
    let csv_paths = super::required_outfit_csv_paths(data_dir);
    if cache_path.exists() {
        let conn = super::open_read_only(&cache_path)?;
        if super::outfit_cache_is_fresh(&conn, &csv_paths)? {
            return Ok(cache_path);
        }
    }
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let conn = Connection::open(&cache_path)
        .map_err(|err| format!("open {}: {err}", cache_path.display()))?;
    init_schema(&conn)?;
    record_source_files(&conn, &csv_paths)?;
    import_rows(&conn, &csv_paths)?;
    conn.execute_batch("COMMIT;")
        .map_err(|err| format!("commit outfit_links cache: {err}"))?;
    Ok(cache_path)
}

pub(super) fn imported_outfit_links_cache_path(data_dir: &Path) -> Result<PathBuf, String> {
    let cache_path = super::outfit_links_cache_path();
    if !cache_path.exists() {
        return Err(format!(
            "{} missing; run `cargo run --bin outfit_links_cache_import` to build it",
            cache_path.display()
        ));
    }
    let csv_paths = super::required_outfit_csv_paths(data_dir);
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
        source_insert
            .execute((path.to_string_lossy().to_string(), super::csv_mtime(path)?))
            .map_err(|err| format!("insert source_files {}: {err}", path.display()))?;
    }
    Ok(())
}

fn import_rows(conn: &Connection, csv_paths: &[PathBuf; 7]) -> Result<(), String> {
    super::populate_starter_outfits(conn, &csv_paths[0])?;
    super::populate_item_modified_appearance_map(conn, &csv_paths[1])?;
    super::populate_item_appearance_map(conn, &csv_paths[2])?;
    super::populate_display_info(conn, &csv_paths[3])?;
    super::populate_material_to_texture(conn, &csv_paths[4])?;
    super::populate_display_material_textures(conn, &csv_paths[5])?;
    super::populate_model_to_fdid(conn, &csv_paths[6])?;
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
