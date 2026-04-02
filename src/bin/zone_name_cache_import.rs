fn main() {
    match game_engine::world_db::import_zone_name_cache() {
        Ok(path) => {
            println!("wrote {}", path.display());
        }
        Err(err) => {
            eprintln!("failed to import zone name cache: {err}");
            std::process::exit(1);
        }
    }
}
