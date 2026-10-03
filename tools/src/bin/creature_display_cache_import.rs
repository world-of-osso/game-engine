use std::path::Path;

fn main() {
    match game_engine_core::creature_display_cache::import_creature_display_cache(Path::new("data"))
    {
        Ok(path) => {
            println!("wrote {}", path.display());
        }
        Err(err) => {
            eprintln!("failed to import creature display cache: {err}");
            std::process::exit(1);
        }
    }
}
