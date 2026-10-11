use std::path::PathBuf;

fn main() {
    // Optional data root keeps diagnostic regeneration separate from live caches.
    let data_dir = PathBuf::from(std::env::args_os().nth(1).unwrap_or_else(|| "data".into()));
    match game_engine_core::outfit_catalog_db::import_outfit_links_cache(&data_dir) {
        Ok(path) => {
            println!("wrote {}", path.display());
        }
        Err(err) => {
            eprintln!("failed to import outfit links cache: {err}");
            std::process::exit(1);
        }
    }
}
