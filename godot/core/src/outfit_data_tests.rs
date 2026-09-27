use std::path::Path;

use crate::outfit_data::OutfitData;

fn catalog() -> OutfitData {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    OutfitData::load(&data_root)
}

#[test]
fn selected_human_warrior_items_resolve_original_displays_and_resources() {
    let catalog = catalog();
    for (item, display) in [
        (25, 1542),
        (38, 5729),
        (39, 6050),
        (40, 6051),
        (2362, 18730),
    ] {
        assert_eq!(catalog.resolve_item_display_id(item).unwrap(), display);
    }
    let outfit = catalog.try_resolve_outfit(1, 1, 0).unwrap();
    assert!(outfit.item_textures.contains(&(5, 157712)));
    assert!(outfit.model_fdids.contains(&(16810, 148132)));
    assert!(
        catalog
            .try_resolve_display_info(6050)
            .unwrap()
            .unwrap()
            .item_textures
            .contains(&(5, 157712))
    );
    assert_eq!(
        catalog.try_resolve_runtime_model(1542, 1, 0).unwrap(),
        Some((148132, [148134, 0, 0]))
    );
    assert_eq!(
        catalog.try_resolve_runtime_model(18730, 1, 0).unwrap(),
        Some((143001, [142735, 0, 0]))
    );
}

#[test]
fn missing_data_root_is_an_error_not_an_empty_outfit() {
    let catalog = OutfitData::load(Path::new("/nonexistent/outfit-data-root"));
    assert!(catalog.try_resolve_outfit(1, 1, 0).is_err());
}
