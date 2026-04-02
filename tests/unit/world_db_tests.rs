use super::{
    import_outfit_links_cache, import_zone_name_cache, load_cached_char_start_outfits,
    load_cached_display_resources, load_cached_item_appearance,
    load_cached_item_modified_appearance, load_chr_race_prefixes, load_zone_name,
    resolve_cached_outfit_display_ids,
};
use std::path::Path;

#[test]
fn chr_race_prefixes_load_from_world_db() {
    let prefixes = load_chr_race_prefixes().expect("load chr_races prefixes from world.db");
    assert_eq!(prefixes.get(&1).map(String::as_str), Some("hu"));
}

#[test]
fn zone_name_loads_from_area_table_cache() {
    import_zone_name_cache().expect("import zone name cache");
    assert_eq!(
        load_zone_name(12).expect("load zone name"),
        Some("Elwynn Forest".to_string())
    );
}

#[test]
fn outfit_links_load_from_cache() {
    let data_dir = Path::new("data");
    import_outfit_links_cache(data_dir).expect("import outfit links cache");
    let outfits = load_cached_char_start_outfits(data_dir).expect("load starter_outfits cache");
    let item_to_appearance = load_cached_item_modified_appearance(data_dir)
        .expect("load item_modified_appearance cache");
    let appearance_to_display =
        load_cached_item_appearance(data_dir).expect("load item_appearance cache");

    assert!(
        !outfits.is_empty(),
        "starter_outfits cache should not be empty"
    );
    assert!(
        !item_to_appearance.is_empty(),
        "item_modified_appearance cache should not be empty"
    );
    assert!(
        !appearance_to_display.is_empty(),
        "item_appearance cache should not be empty"
    );
}

#[test]
fn display_resources_load_from_cache() {
    let data_dir = Path::new("data");
    import_outfit_links_cache(data_dir).expect("import outfit links cache");
    let resources = load_cached_display_resources(data_dir).expect("load display resources cache");
    assert!(!resources.display_info.is_empty());
    assert!(!resources.material_to_texture.is_empty());
    assert!(!resources.display_materials.direct.is_empty());
    assert!(!resources.model_to_fdids.is_empty());
}

#[test]
fn resolve_outfit_display_ids_loads_from_cache() {
    let data_dir = Path::new("data");
    import_outfit_links_cache(data_dir).expect("import outfit links cache");
    let display_ids =
        resolve_cached_outfit_display_ids(data_dir, 1, 1, 0).expect("resolve outfit displays");
    assert!(
        !display_ids.is_empty(),
        "starter outfit display lookup should not be empty"
    );
}
