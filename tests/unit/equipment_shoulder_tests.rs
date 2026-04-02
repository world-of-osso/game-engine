use super::*;
use std::path::{Path, PathBuf};

fn shared_model_shoulders_appearance() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Shoulder,
            item_id: Some(1),
            display_info_id: Some(148865),
            inventory_type: 3,
            hidden: false,
        }],
    }
}

fn split_model_shoulders_appearance() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Shoulder,
            item_id: Some(1),
            display_info_id: Some(7004),
            inventory_type: 3,
            hidden: false,
        }],
    }
}

#[test]
fn shared_model_shoulders_display_spawns_left_and_right_runtime_models() {
    let data = OutfitData::load(Path::new("data"));
    let resolved = resolve_equipment_appearance(&shared_model_shoulders_appearance(), &data, 1, 0);
    let left = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::ShoulderLeft)
        .expect("expected left shoulder runtime model");
    let right = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::ShoulderRight)
        .expect("expected right shoulder runtime model");

    assert!(
        left.path
            .ends_with("lshoulder_leather_raidroguemythic_q_01.m2"),
        "unexpected left shoulder runtime path: {}",
        left.path.display()
    );
    assert!(
        right
            .path
            .ends_with("rshoulder_leather_raidroguemythic_q_01.m2"),
        "unexpected right shoulder runtime path: {}",
        right.path.display()
    );
    assert_eq!(left.skin_fdids[0], 1309094);
    assert_eq!(right.skin_fdids[0], 1309094);
}

#[test]
fn split_model_shoulders_display_preserves_side_specific_models_and_textures() {
    let data = OutfitData::load(Path::new("data"));
    let resolved = resolve_equipment_appearance(&split_model_shoulders_appearance(), &data, 1, 0);
    let left = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::ShoulderLeft)
        .expect("expected left shoulder runtime model");
    let right = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::ShoulderRight)
        .expect("expected right shoulder runtime model");

    assert!(
        left.path.ends_with("lshoulder_plate_d_04.m2"),
        "unexpected left shoulder runtime path: {}",
        left.path.display()
    );
    assert!(
        right.path.ends_with("rshoulder_mail_a_01.m2"),
        "unexpected right shoulder runtime path: {}",
        right.path.display()
    );
    assert_eq!(left.skin_fdids[0], 143651);
    assert_eq!(right.skin_fdids[0], 144157);
}

#[test]
fn explicit_shoulder_slot_clears_stale_runtime_models_when_no_runtime_model_resolves() {
    let mut equipment = Equipment::default();
    equipment
        .slots
        .insert(EquipmentSlot::ShoulderLeft, PathBuf::from("old-left"));
    equipment
        .slots
        .insert(EquipmentSlot::ShoulderRight, PathBuf::from("old-right"));
    let resolved = ResolvedEquipmentAppearance {
        explicit_slots: [EquipmentVisualSlot::Shoulder].into_iter().collect(),
        ..Default::default()
    };

    apply_runtime_equipment(&mut equipment, &resolved);

    assert!(!equipment.slots.contains_key(&EquipmentSlot::ShoulderLeft));
    assert!(!equipment.slots.contains_key(&EquipmentSlot::ShoulderRight));
}
