//! Offline refresh of the metadata that selects model asset products.
use std::path::PathBuf;

fn main() {
    let data = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("data root argument required"),
    );
    let result = import(&data);
    if let Err(error) = result {
        eprintln!("Model metadata import: {error}");
        std::process::exit(1);
    }
}

fn import(data: &std::path::Path) -> Result<(), String> {
    let displays = game_engine_core::creature_display_cache::import_creature_display_cache(data)?;
    let outfits = game_engine_core::outfit_catalog_db::import_outfit_links_cache(data)?;
    let conn = rusqlite::Connection::open(displays).map_err(|error| error.to_string())?;
    for id in [139403, 139409] {
        let display = game_engine_core::creature_display_data::query_display(&conn, id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Display {id} missing"))?;
        println!("Display {id}: {display:?}");
    }
    println!("Outfit cache: {}", outfits.display());
    Ok(())
}
