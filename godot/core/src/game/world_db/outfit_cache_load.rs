use std::collections::HashMap;
use std::path::Path;

pub(super) fn load_cached_item_modified_appearance(
    data_dir: &Path,
) -> Result<HashMap<u32, u32>, String> {
    let cache_path = super::imported_outfit_links_cache_path(data_dir)?;
    let conn = super::open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare("SELECT item_id, appearance_id FROM item_modified_appearance_map")
        .map_err(|err| format!("prepare item_modified_appearance_map lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query item_modified_appearance_map: {err}"))?;
    let mut map = HashMap::new();
    for row in rows {
        let (item_id, appearance_id) =
            row.map_err(|err| format!("read item_modified_appearance_map row: {err}"))?;
        map.insert(item_id, appearance_id);
    }
    Ok(map)
}

pub(super) fn load_cached_item_appearance(data_dir: &Path) -> Result<HashMap<u32, u32>, String> {
    let cache_path = super::imported_outfit_links_cache_path(data_dir)?;
    let conn = super::open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare("SELECT appearance_id, display_info_id FROM item_appearance_map")
        .map_err(|err| format!("prepare item_appearance_map lookup: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
        .map_err(|err| format!("query item_appearance_map: {err}"))?;
    let mut map = HashMap::new();
    for row in rows {
        let (appearance_id, display_info_id) =
            row.map_err(|err| format!("read item_appearance_map row: {err}"))?;
        map.insert(appearance_id, display_info_id);
    }
    Ok(map)
}
