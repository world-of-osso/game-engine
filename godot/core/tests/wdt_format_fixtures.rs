use game_engine_core::wdt::*;

#[test]
fn stockade_wdt_places_its_prison_wmo_at_the_world_origin() {
    // world/maps/stormwindjail/stormwindjail.wdt (FDID 791060), extracted from local CASC.
    let data = std::fs::read("data/terrain/791060.wdt").expect("Stockade WDT in data/terrain");
    let wmo = parse_wdt_global_wmo(&data)
        .expect("parses")
        .expect("global WMO");
    // world/wmo/dungeon/az_stormwindprisons/stormwindjail.wmo.
    assert_eq!(wmo.fdid, Some(108_631));
    assert_eq!(wmo.position, [0.0, 0.0, 0.0]);
    assert_eq!(wmo.scale, 1.0);
}

#[test]
fn legacy_blacksmith_wdt_retains_authored_wmo_path() {
    let data = std::fs::read("data/terrain/952203.wdt").unwrap();
    let wmo = parse_wdt_global_wmo(&data).unwrap().unwrap();
    assert_eq!(wmo.fdid, None);
    assert_eq!(
        wmo.path.as_deref(),
        Some("world/wmo/draenor/human/6hu_garrison_blacksmithing_hub.wmo")
    );
    assert_eq!(wmo.position, [0.0; 3]);
    assert_eq!(wmo.scale, 1.0);
}

#[test]
fn eastern_kingdoms_wdt_has_no_global_wmo() {
    let data = std::fs::read("data/terrain/775971.wdt").expect("azeroth WDT in data/terrain");
    assert!(parse_wdt_global_wmo(&data).expect("parses").is_none());
}

#[test]
fn zephras_maid_identifies_all_active_tiles_and_sample_companions() {
    let data = std::fs::read("data/terrain/7198644.wdt").unwrap();
    let tiles = parse_wdt_tiles(&data).unwrap();
    assert_eq!(tiles.active.len(), 72);
    let sample = tiles.file_ids(29, 26).unwrap();
    assert_eq!(sample.root, 7199999);
    assert_eq!(sample.obj0, 7200000);
    assert_eq!(sample.obj1, 7200001);
    assert_eq!(sample.tex0, 7200002);
    assert_eq!(sample.lod, 7200003);
    assert_eq!(sample.map_texture, 7199321);
    assert_eq!(sample.map_normal, 7199345);
    assert_eq!(sample.minimap, 7199297);
    assert!(tiles.file_ids(0, 0).is_none());
}

#[test]
fn zephras_reader_coordinates_also_address_retail_maid_ids() {
    let data = std::fs::read("data/terrain/775971.wdt").unwrap();
    let tiles = parse_wdt_tiles(&data).unwrap();
    assert!(tiles.active.contains(&(32, 48)));
    assert_eq!(tiles.file_ids(32, 48).unwrap().root, 778027);
}
