use std::path::Path;

fn main() {
    match game_engine::char_texture_cache::import_char_texture_cache(Path::new("data")) {
        Ok(path) => {
            println!("wrote {}", path.display());
        }
        Err(err) => {
            eprintln!("failed to import char texture cache: {err}");
            std::process::exit(1);
        }
    }
}
