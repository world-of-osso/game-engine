use super::*;

#[test]
fn starter_item_entries_resolve_like_explicit_displays() {
    let data = OutfitData::load(Path::new("data"));
    let items =
        game_engine::world_db::load_cached_item_modified_appearance(Path::new("data")).unwrap();
    let appearances =
        game_engine::world_db::load_cached_item_appearance(Path::new("data")).unwrap();
    for (item_id, slot) in [
        (25, EquipmentVisualSlot::MainHand),
        (38, EquipmentVisualSlot::Shirt),
        (39, EquipmentVisualSlot::Legs),
        (40, EquipmentVisualSlot::Feet),
        (2362, EquipmentVisualSlot::OffHand),
    ] {
        let entry = shared::components::EquippedAppearanceEntry {
            slot,
            item_id: Some(item_id),
            display_info_id: None,
            inventory_type: 0,
            hidden: false,
        };
        let resolve = |entry| {
            resolve_equipment_appearance_with_texture_cache(
                &NetEquipmentAppearance {
                    entries: vec![entry],
                },
                &data,
                1,
                0,
                &mut |_| {},
            )
        };
        let actual = resolve(entry.clone());
        let mut explicit = entry.clone();
        explicit.display_info_id = Some(appearances[&items[&item_id]]);
        let expected = resolve(explicit.clone());
        assert!(
            !expected.outfit.item_textures.is_empty() || !expected.runtime_models.is_empty(),
            "starter {item_id} has no render assets"
        );
        assert_eq!(
            actual.outfit.item_textures, expected.outfit.item_textures,
            "item {item_id} textures"
        );
        assert_eq!(
            actual.runtime_models, expected.runtime_models,
            "item {item_id} models"
        );
        assert_eq!(
            actual.outfit.geoset_overrides,
            expected.outfit.geoset_overrides
        );
        explicit.item_id = Some(u32::MAX);
        assert_eq!(
            resolve(explicit).runtime_models,
            expected.runtime_models,
            "explicit display takes precedence"
        );
        let mut hidden = entry;
        hidden.hidden = true;
        let hidden = resolve(hidden);
        assert!(hidden.runtime_models.is_empty() && hidden.outfit.item_textures.is_empty());
        assert!(hidden.explicit_slots.contains(&slot));
    }
}
