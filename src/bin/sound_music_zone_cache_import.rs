fn main() {
    match game_engine::sound_music_zone_cache::import_sound_music_zone_cache() {
        Ok(path) => {
            println!("wrote {}", path.display());
        }
        Err(err) => {
            eprintln!("failed to import sound music zone cache: {err}");
            std::process::exit(1);
        }
    }
}
