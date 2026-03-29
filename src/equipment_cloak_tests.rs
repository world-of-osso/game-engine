use super::*;
use std::path::Path;

#[test]
fn merged_cloak_display_enables_cape_geoset_without_runtime_model() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Back,
            item_id: Some(188846),
            display_info_id: Some(192786),
            inventory_type: 16,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

    assert!(
        resolved.outfit.geoset_overrides.contains(&(15, 1)),
        "expected merged cloak to enable cape geoset 1501: {:?}",
        resolved.outfit.geoset_overrides
    );
    assert!(
        resolved.runtime_models.is_empty(),
        "merged cloak should not spawn a runtime attachment: {:?}",
        resolved.runtime_models
    );
}

#[test]
fn runtime_cloak_display_resolves_to_back_model_with_texture() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Back,
            item_id: Some(210334),
            display_info_id: Some(677577),
            inventory_type: 16,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
    let runtime = resolved
        .runtime_models
        .first()
        .expect("expected runtime back cloak model");

    assert!(
        runtime.path.ends_with("cape_special_keg_d_01.m2"),
        "unexpected runtime path: {}",
        runtime.path.display()
    );
    assert_eq!(runtime.skin_fdids[0], 5644278);
}
