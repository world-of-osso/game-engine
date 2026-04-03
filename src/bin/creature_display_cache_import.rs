fn main() {
    match game_engine::creature_display::import_creature_display_cache() {
        Ok(path) => {
            println!("wrote {}", path.display());
        }
        Err(err) => {
            eprintln!("failed to import creature display cache: {err}");
            std::process::exit(1);
        }
    }
}
