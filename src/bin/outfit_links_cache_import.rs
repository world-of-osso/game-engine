use std::path::Path;

fn main() {
    match game_engine::world_db::import_outfit_links_cache(Path::new("data")) {
        Ok(path) => {
            println!("wrote {}", path.display());
        }
        Err(err) => {
            eprintln!("failed to import outfit links cache: {err}");
            std::process::exit(1);
        }
    }
}
