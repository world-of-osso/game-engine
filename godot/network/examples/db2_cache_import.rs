//! Offline regeneration of the client importers against an explicit private data root.
use std::path::PathBuf;

fn main() {
    let data = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("private data root required"),
    );
    if let Err(error) = import_caches(&data) {
        eprintln!("DB2 cache import: {error}");
        std::process::exit(1);
    }
}

fn import_caches(data: &std::path::Path) -> Result<(), String> {
    use game_engine_core::{
        char_texture_cache, creature_display_cache, customization_cache, outfit_catalog_db,
        spell_catalog, spell_visual::SpellVisualCatalog,
    };
    println!(
        "{}",
        creature_display_cache::import_creature_display_cache(data)?.display()
    );
    println!(
        "{}",
        customization_cache::import_customization_cache(data)?.display()
    );
    println!(
        "{}",
        char_texture_cache::import_char_texture_cache(data)?.display()
    );
    println!(
        "{}",
        outfit_catalog_db::import_outfit_links_cache(data)?.display()
    );
    let paths = spell_catalog::SpellCatalogPaths::for_data_dir(data);
    spell_catalog::load_spell_catalog(&paths)?;
    println!("{}", paths.cache_path.display());
    let db2 = data.join("db2/active");
    let cache = data
        .join("cache")
        .join(SpellVisualCatalog::cache_file_name(&db2)?);
    SpellVisualCatalog::load(&db2, &cache)?;
    println!("{}", cache.display());
    Ok(())
}
