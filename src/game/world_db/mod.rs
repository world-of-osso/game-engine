use crate::cache_source_mtime::csv_mtime;
use crate::cache_sqlite::open_read_only;
use crate::csv_util::header_index;
use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
#[path = "zone_names_cache.rs"]
mod zone_name_cache;
pub use crate::outfit_catalog_db::{
    import_outfit_links_cache, load_cached_char_start_outfits, load_cached_item_appearance,
    load_cached_item_modified_appearance, resolve_cached_outfit_display_ids,
};
fn zone_names_cache_path() -> PathBuf {
    crate::paths::shared_data_path("cache/zone_names.sqlite")
}

fn area_table_csv_path() -> PathBuf {
    crate::paths::resolve_data_path("AreaTable.csv")
}

fn open_reader(path: &Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

pub fn import_zone_name_cache() -> Result<PathBuf, String> {
    zone_name_cache::import_zone_name_cache()
}

pub fn load_zone_name(id: u32) -> Result<Option<String>, String> {
    zone_name_cache::load_zone_name(id)
}

pub fn load_area_parents() -> Result<HashMap<u32, u32>, String> {
    zone_name_cache::load_area_parents()
}

#[cfg(test)]
#[path = "../../../tests/unit/world_db_tests.rs"]
mod tests;
