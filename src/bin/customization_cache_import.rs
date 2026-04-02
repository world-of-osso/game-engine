use std::path::Path;

fn main() {
    match game_engine::customization_cache::import_customization_cache(Path::new("data")) {
        Ok(path) => {
            println!("wrote {}", path.display());
        }
        Err(err) => {
            eprintln!("failed to import customization cache: {err}");
            std::process::exit(1);
        }
    }
}
