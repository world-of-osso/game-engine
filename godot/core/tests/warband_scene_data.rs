use std::path::Path;

use game_engine_core::warband_scene_data::read_authored_catalog;

#[test]
fn authored_adventurers_rest_records_preserve_camera_focus_and_primary_tile() {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = read_authored_catalog(&data_root).expect("read authored Warband CSVs");
    let scene = catalog
        .scenes
        .iter()
        .find(|scene| scene.id == 1)
        .expect("Adventurer's Rest");

    assert_eq!(scene.name, "Adventurer's Rest");
    assert_eq!(scene.map_id, 2703);
    assert_eq!(scene.position, [-2982.99, 468.057, 455.523]);
    assert_eq!(scene.look_at, [-2985.54, 456.018, 454.399]);
    assert_eq!(scene.fov, 65.0);
    assert_eq!(scene.texture_kit, 5671);
    assert_eq!(scene.tile_coords(), (31, 37));
    assert_eq!(scene.map_name(), "2703");
    assert!(!catalog.scenes.iter().any(|scene| scene.id == 29));

    let first_slot = catalog
        .placements
        .iter()
        .find(|placement| {
            placement.scene_id == 1 && placement.slot_id == 0 && placement.is_character_slot()
        })
        .expect("authored character slot zero");
    assert_eq!(first_slot.id, 1);
    assert_eq!(first_slot.position, [-2981.82, 457.35, 452.826]);
    assert_eq!(first_slot.rotation, 108.0);
    assert!(!catalog.placement_options.is_empty());
}

#[test]
fn missing_authored_catalog_is_an_error_not_an_empty_selection() {
    let absent =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/nonexistent-warband-fixture");
    let error = read_authored_catalog(&absent).expect_err("missing CSV must fail");
    assert!(error.contains("WarbandScene.csv"), "{error}");
}
