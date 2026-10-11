use std::path::PathBuf;

fn main() {
    // Optional data root keeps diagnostic regeneration separate from live caches.
    let data_dir = PathBuf::from(std::env::args_os().nth(1).unwrap_or_else(|| "data".into()));
    match game_engine_core::creature_display_cache::import_creature_display_cache(&data_dir) {
        Ok(path) => {
            println!("wrote {}", path.display());
        }
        Err(err) => {
            eprintln!("failed to import creature display cache: {err}");
            std::process::exit(1);
        }
    }
}
