use super::*;
use game_engine::outfit_data::OutfitResult;

#[test]
fn chest_runtime_model_defaults_to_empty_without_data() {
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Chest,
            item_id: Some(1),
            display_info_id: Some(1),
            inventory_type: 5,
            hidden: false,
        }],
    };
    let data = OutfitData::default();

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

    assert!(resolved.runtime_models.is_empty());
}

#[test]
fn head_slot_maps_to_runtime_slots() {
    assert_eq!(
        visual_slot_to_runtime_slots(EquipmentVisualSlot::Head),
        vec![EquipmentSlot::Head]
    );
}

#[test]
fn chest_slot_maps_to_runtime_slots() {
    assert_eq!(
        visual_slot_to_runtime_slots(EquipmentVisualSlot::Chest),
        vec![EquipmentSlot::Chest]
    );
}

#[test]
fn waist_slot_maps_to_runtime_slots() {
    assert_eq!(
        visual_slot_to_runtime_slots(EquipmentVisualSlot::Waist),
        vec![EquipmentSlot::Waist]
    );
}

#[test]
fn feet_slot_maps_to_runtime_slots() {
    assert_eq!(
        visual_slot_to_runtime_slots(EquipmentVisualSlot::Feet),
        vec![EquipmentSlot::Feet]
    );
}

#[test]
fn legs_slot_maps_to_runtime_slots() {
    assert_eq!(
        visual_slot_to_runtime_slots(EquipmentVisualSlot::Legs),
        vec![EquipmentSlot::Legs]
    );
}

#[test]
fn apply_runtime_equipment_preserves_head_slot() {
    let mut equipment = Equipment::default();
    equipment
        .slots
        .insert(EquipmentSlot::Head, PathBuf::from("old"));
    equipment
        .slots
        .insert(EquipmentSlot::MainHand, PathBuf::from("mh"));
    equipment
        .slots
        .insert(EquipmentSlot::OffHand, PathBuf::from("oh"));
    let resolved = ResolvedEquipmentAppearance {
        runtime_models: vec![RuntimeModelAppearance {
            slot: EquipmentSlot::Head,
            path: PathBuf::from("new-head"),
            skin_fdids: [123, 0, 0],
        }],
        ..Default::default()
    };

    apply_runtime_equipment(&mut equipment, &resolved);

    assert_eq!(
        equipment.slots.get(&EquipmentSlot::Head),
        Some(&PathBuf::from("new-head"))
    );
    assert_eq!(
        equipment.slot_skin_fdids.get(&EquipmentSlot::Head),
        Some(&[123, 0, 0])
    );
}

#[test]
fn first_model_path_prefers_first_available_model() {
    let display = OutfitResult {
        model_fdids: vec![],
        ..Default::default()
    };
    assert_eq!(first_model_path(&display), None);
}

#[test]
fn human_male_helm_runtime_model_uses_human_variant_and_texture() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(1),
            display_info_id: Some(1128),
            inventory_type: 1,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
    let runtime = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Head)
        .expect("expected head runtime model");

    assert!(runtime.path.ends_with("helm_plate_d_02_hum.m2"));
    assert_eq!(runtime.skin_fdids[0], 140455);
}

#[test]
fn head_display_resolves_helmet_geoset_hide_groups() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(1),
            display_info_id: Some(173086),
            inventory_type: 1,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

    assert!(
        resolved.hidden_character_geoset_groups.contains(&1),
        "expected vis 247 to reset human head geoset group 1: {:?}",
        resolved.hidden_character_geoset_groups
    );
    assert!(
        resolved.hidden_character_geoset_groups.contains(&2),
        "expected vis 247 to reset human head geoset group 2: {:?}",
        resolved.hidden_character_geoset_groups
    );
    assert!(
        resolved.hidden_character_geoset_groups.contains(&3),
        "expected vis 247 to reset human head geoset group 3: {:?}",
        resolved.hidden_character_geoset_groups
    );
}

fn resolve_head_display(display_info_id: u32, item_id: Option<u32>) -> ResolvedEquipmentAppearance {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id,
            display_info_id: Some(display_info_id),
            inventory_type: 1,
            hidden: false,
        }],
    };
    resolve_equipment_appearance(&appearance, &data, 1, 0)
}

#[test]
fn real_mask_display_hides_scalp_and_enables_head_geosets() {
    let resolved = resolve_head_display(720086, Some(249913));

    assert!(
        resolved.hidden_character_geoset_groups.contains(&0),
        "expected vis 644/645 to hide scalp hair group 0: {:?}",
        resolved.hidden_character_geoset_groups
    );
    assert!(
        resolved.hidden_character_geoset_groups.contains(&7),
        "expected vis 644/645 to hide ear-adjacent group 7: {:?}",
        resolved.hidden_character_geoset_groups
    );
    assert!(
        resolved.outfit.geoset_overrides.contains(&(27, 2)),
        "expected equipped head slot to switch character head geoset to 2702: {:?}",
        resolved.outfit.geoset_overrides
    );
    assert!(
        !resolved.outfit.geoset_overrides.iter().any(|(g, _)| *g == 21),
        "expected GeosetGroup_1 == 0 to avoid emitting a 21xx override: {:?}",
        resolved.outfit.geoset_overrides
    );
}

#[test]
fn live_helm_display_resolves_to_runtime_model_path() {
    let data = OutfitData::load(Path::new("data"));
    let display = data.resolve_display_info(1128);

    assert!(
        !display.model_fdids.is_empty(),
        "expected helm display 1128 to have model fdids"
    );

    let path = first_model_path(&display);
    assert!(
        path.is_some(),
        "expected helm display 1128 to resolve to a model path, model_fdids={:?}",
        display.model_fdids
    );
}

#[test]
fn hood_of_empty_eternities_resolves_to_runtime_head_model() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(190626),
            display_info_id: Some(685129),
            inventory_type: 1,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
    let runtime = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Head)
        .expect("expected head runtime model");

    assert!(
        runtime
            .path
            .ends_with("helm_leather_raidrogueprogenitor_d_01_hu_m.m2"),
        "unexpected runtime path: {}",
        runtime.path.display()
    );
    assert_eq!(runtime.skin_fdids[0], 3865285);
}

#[test]
fn cloak_display_resolves_to_runtime_back_model_with_texture() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Back,
            item_id: None,
            display_info_id: Some(181925),
            inventory_type: 16,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
    let runtime = resolved
        .runtime_models
        .first()
        .expect("expected back runtime model");

    assert!(
        runtime.path.ends_with("cape_special_bastion_d_01.m2"),
        "unexpected runtime path: {}",
        runtime.path.display()
    );
    assert_eq!(runtime.skin_fdids[0], 3150979);
}

#[test]
fn orange_hood_runtime_head_model_extracts_runtime_textures() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(190626),
            display_info_id: Some(685128),
            inventory_type: 1,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
    let runtime = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Head)
        .expect("expected head runtime model");

    assert_eq!(runtime.skin_fdids[0], 3865286);
    assert!(
        Path::new("data/textures/3865286.blp").exists(),
        "expected orange hood texture to be extracted"
    );
    assert!(
        Path::new("data/textures/3865065.blp").exists(),
        "expected gold hood texture to be extracted"
    );
}

#[test]
fn real_mask_display_does_not_fallback_to_blood_elf_collection_model() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(249913),
            display_info_id: Some(720086),
            inventory_type: 1,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
    let Some(runtime) = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Head)
    else {
        return;
    };

    let path = runtime.path.to_string_lossy().to_ascii_lowercase();
    assert!(
        !path.contains("_be_m.m2"),
        "head runtime model should not fall back to blood-elf male for human male displays: {}",
        runtime.path.display()
    );
    assert!(
        path.contains("_hu_m.m2") || path.contains("_hum.m2"),
        "head runtime model should resolve to a human-male variant when present: {}",
        runtime.path.display()
    );
}

#[test]
fn real_mask_display_skips_collection_runtime_head_model() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(249913),
            display_info_id: Some(720086),
            inventory_type: 1,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

    assert!(
        resolved
            .runtime_models
            .iter()
            .all(|model| model.slot != EquipmentSlot::Head),
        "collection-style head displays should not spawn runtime head attachments: {:?}",
        resolved.runtime_models
    );
}

// --- Helmet hair hiding ---

#[test]
fn old_helm_without_vis_data_hides_hair() {
    // Display 1128: vanilla plate helm, no HelmetGeosetVisData, has runtime M2 model
    let resolved = resolve_head_display(1128, None);

    assert!(
        resolved.hidden_character_geoset_groups.contains(&0),
        "old helm without HelmetGeosetVisData should hide hair (group 0): {:?}",
        resolved.hidden_character_geoset_groups
    );
}

#[test]
fn modern_helm_with_vis_data_hides_hair_per_rules() {
    // Display 685129: modern hood with HelmetGeosetVis 246/307
    let resolved = resolve_head_display(685129, None);

    assert!(
        resolved.hidden_character_geoset_groups.contains(&0),
        "hood vis 246 should hide hair (group 0) for human: {:?}",
        resolved.hidden_character_geoset_groups
    );
    assert!(
        resolved.hidden_character_geoset_groups.contains(&7),
        "hood vis 246 should hide ears (group 7) for human: {:?}",
        resolved.hidden_character_geoset_groups
    );
}

#[test]
fn tiara_with_vis_data_does_not_hide_hair() {
    // Display 96760: Tiara of the Oracle (item 21348), HelmetGeosetVis 245
    let resolved = resolve_head_display(96760, Some(21348));

    assert!(
        !resolved.hidden_character_geoset_groups.contains(&0),
        "tiara vis 245 should NOT hide hair for human: {:?}",
        resolved.hidden_character_geoset_groups
    );
}
