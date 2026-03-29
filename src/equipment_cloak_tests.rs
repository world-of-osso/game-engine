use super::*;
use std::path::Path;
use std::path::PathBuf;

#[test]
fn debug_human_male_cape_batches() {
    let model = crate::asset::m2::load_m2(Path::new("data/models/humanmale_hd.m2"), &[0, 0, 0])
        .expect("load humanmale_hd");
    for batch in &model.batches {
        if batch.texture_type == Some(2) {
            println!(
                "type2 batch mesh_part_id={} texture_fdid={:?} texture_type={:?} shader=0x{:x} blend={} tex2={:?}",
                batch.mesh_part_id,
                batch.texture_fdid,
                batch.texture_type,
                batch.shader_id,
                batch.blend_mode,
                batch.texture_2_fdid
            );
        }
    }
}

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
        resolved.outfit.geoset_overrides.contains(&(15, 2)),
        "expected merged cloak to enable cape geoset 1502: {:?}",
        resolved.outfit.geoset_overrides
    );
    assert_eq!(
        resolved.merged_cape_texture_fdid,
        Some(4046074),
        "expected merged cloak to resolve cape texture fdid"
    );
    assert!(
        resolved.runtime_models.is_empty(),
        "merged cloak should not spawn a runtime attachment: {:?}",
        resolved.runtime_models
    );
}

#[test]
fn merged_cloak_display_extracts_display_material_texture() {
    assert_merged_cloak_texture_extracted(192738, 4046069);
}

#[test]
fn merged_cloak_alt_display_extracts_display_material_texture() {
    assert_merged_cloak_texture_extracted(192748, 4046070);
}

#[test]
fn merged_cloak_third_display_extracts_display_material_texture() {
    assert_merged_cloak_texture_extracted(192768, 4046072);
}

fn assert_merged_cloak_texture_extracted(display_info_id: u32, expected_fdid: u32) {
    let target = PathBuf::from(format!("data/textures/{expected_fdid}.blp"));
    let _ = std::fs::remove_file(&target);

    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Back,
            item_id: Some(188846),
            display_info_id: Some(display_info_id),
            inventory_type: 16,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

    assert_eq!(resolved.merged_cape_texture_fdid, Some(expected_fdid));
    assert!(
        target.exists(),
        "expected display material texture {expected_fdid} to be extracted for display {display_info_id}"
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
