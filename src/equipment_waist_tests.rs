use super::*;
use std::path::Path;

fn hybrid_waist_appearance() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Waist,
            item_id: Some(1),
            display_info_id: Some(109162),
            inventory_type: 0,
            hidden: false,
        }],
    }
}

fn geoset_only_waist_appearance() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Waist,
            item_id: Some(1),
            display_info_id: Some(15040),
            inventory_type: 0,
            hidden: false,
        }],
    }
}

fn isolated_runtime_waist_appearance() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Waist,
            item_id: Some(1),
            display_info_id: Some(160997),
            inventory_type: 0,
            hidden: false,
        }],
    }
}

#[test]
fn geoset_only_waist_display_enables_belt_geoset_without_runtime_model() {
    let data = OutfitData::load(Path::new("data"));
    let resolved = resolve_equipment_appearance(&geoset_only_waist_appearance(), &data, 1, 0);

    assert_eq!(resolved.outfit.geoset_overrides, vec![(18, 2)]);
    assert!(
        resolved.outfit.item_textures.contains(&(4, 160531)),
        "expected legacy waist overlay texture, got {:?}",
        resolved.outfit.item_textures
    );
    assert!(
        resolved.runtime_models.iter().all(|model| model.slot != EquipmentSlot::Waist),
        "expected no runtime waist model: {:?}",
        resolved.runtime_models
    );
}

#[test]
fn hybrid_waist_display_resolves_runtime_model_and_belt_geoset() {
    let data = OutfitData::load(Path::new("data"));
    let resolved = resolve_equipment_appearance(&hybrid_waist_appearance(), &data, 1, 0);

    assert_eq!(resolved.outfit.geoset_overrides, vec![(18, 2)]);
    let runtime = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Waist)
        .expect("expected waist runtime model");
    assert!(
        runtime.path.ends_with("buckle_mail_challengeshaman_d_01.m2"),
        "unexpected waist runtime path: {}",
        runtime.path.display()
    );
    assert_eq!(runtime.skin_fdids[0], 604564);
}

#[test]
fn isolated_runtime_waist_display_resolves_runtime_model_without_belt_geoset() {
    let data = OutfitData::load(Path::new("data"));
    let resolved = resolve_equipment_appearance(&isolated_runtime_waist_appearance(), &data, 1, 0);

    assert!(resolved.outfit.geoset_overrides.is_empty());
    let runtime = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Waist)
        .expect("expected waist runtime model");
    assert!(
        runtime.path.ends_with("buckle_mail_challengeshaman_d_01.m2"),
        "unexpected isolated waist runtime path: {}",
        runtime.path.display()
    );
    assert_eq!(runtime.skin_fdids[0], 604564);
}

#[test]
fn debug_humanmale_hd_waist_batch_texture_types() {
    let model = crate::asset::m2::load_m2(Path::new("data/models/humanmale_hd.m2"), &[0, 0, 0])
        .expect("load humanmale_hd");
    for batch in model.batches.iter().filter(|batch| batch.mesh_part_id / 100 == 18) {
        println!(
            "waist batch mesh_part_id={} texture_type={:?} texture_fdid={:?} shader=0x{:x} blend={} tex2={:?}",
            batch.mesh_part_id,
            batch.texture_type,
            batch.texture_fdid,
            batch.shader_id,
            batch.blend_mode,
            batch.texture_2_fdid
        );
    }
}

#[test]
fn debug_runtime_waist_model_batch_mesh_parts() {
    let model = crate::asset::m2::load_m2(
        Path::new("data/item-models/item/objectcomponents/waist/buckle_mail_challengeshaman_d_01.m2"),
        &[0, 0, 0],
    )
    .expect("load runtime waist model");
    for batch in &model.batches {
        println!(
            "runtime waist batch mesh_part_id={} texture_type={:?} texture_fdid={:?} shader=0x{:x} blend={} tex2={:?}",
            batch.mesh_part_id,
            batch.texture_type,
            batch.texture_fdid,
            batch.shader_id,
            batch.blend_mode,
            batch.texture_2_fdid
        );
    }
}

#[test]
fn debug_runtime_waist_model_with_skin_fdid_resolves_textured_batch() {
    let model = crate::asset::m2::load_m2(
        Path::new("data/item-models/item/objectcomponents/waist/buckle_mail_challengeshaman_d_01.m2"),
        &[604564, 0, 0],
    )
    .expect("load runtime waist model with skin fdid");
    for batch in &model.batches {
        println!(
            "runtime waist textured batch mesh_part_id={} texture_type={:?} texture_fdid={:?} shader=0x{:x} blend={} tex2={:?}",
            batch.mesh_part_id,
            batch.texture_type,
            batch.texture_fdid,
            batch.shader_id,
            batch.blend_mode,
            batch.texture_2_fdid
        );
    }
    assert!(
        model.batches.iter().any(|batch| batch.texture_fdid == Some(604564)),
        "expected runtime waist batch to resolve skin_fdid 604564"
    );
}
