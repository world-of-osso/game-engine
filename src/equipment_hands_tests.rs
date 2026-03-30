use super::*;
use std::path::Path;

/// Display 510: texture-only cloth glove (GeosetGroup_0=1, no runtime model).
#[test]
fn texture_only_glove_sets_hand_geoset_override() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Hands,
            item_id: Some(1),
            display_info_id: Some(510),
            inventory_type: 10,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

    assert!(
        resolved.outfit.geoset_overrides.contains(&(4, 2)),
        "expected hand geoset override (4, 2) for group 4: {:?}",
        resolved.outfit.geoset_overrides
    );
    assert!(
        resolved
            .runtime_models
            .iter()
            .all(|m| m.slot != EquipmentSlot::Hands),
        "texture-only glove should not produce a runtime model"
    );
}

/// Display 154616: leather glove with runtime M2 + textures (GeosetGroup_0=1, ModelResourcesID=40207).
#[test]
fn m2_backed_glove_resolves_runtime_model() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Hands,
            item_id: Some(1),
            display_info_id: Some(154616),
            inventory_type: 10,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
    let runtime = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Hands);

    assert!(
        runtime.is_some(),
        "expected hands runtime model for display 154616, got: {:?}",
        resolved.runtime_models
    );
}

#[test]
fn m2_backed_glove_sets_hand_geoset_override() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Hands,
            item_id: Some(1),
            display_info_id: Some(154616),
            inventory_type: 10,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

    assert!(
        resolved.outfit.geoset_overrides.contains(&(4, 2)),
        "expected hand geoset override (4, 2) for group 4: {:?}",
        resolved.outfit.geoset_overrides
    );
}
