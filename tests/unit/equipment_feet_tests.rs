use super::*;
use std::path::Path;

#[test]
fn debug_m2_backed_feet_display_mesh_parts() {
    let data = OutfitData::load(Path::new("data"));
    let (fdid, skin_fdids) = data
        .resolve_runtime_model(154620, 1, 0)
        .expect("resolve runtime model");
    let wow_path = game_engine::listfile::lookup_fdid(fdid).expect("listfile path");
    let model_path = crate::asset::asset_cache::file_at_path(
        fdid,
        &Path::new("data/item-models").join(wow_path),
    )
    .expect("extract model");
    let _ = crate::asset::m2::ensure_primary_skin_path(&model_path);
    let model =
        crate::asset::m2::load_m2(&model_path, &skin_fdids).expect("load runtime feet model");
    let mut mesh_ids = model
        .batches
        .iter()
        .map(|batch| batch.mesh_part_id)
        .collect::<Vec<_>>();
    mesh_ids.sort_unstable();
    mesh_ids.dedup();
    println!("runtime feet mesh ids: {mesh_ids:?}");
}

#[test]
fn m2_backed_feet_display_resolves_runtime_model() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Feet,
            item_id: Some(1),
            display_info_id: Some(154620),
            inventory_type: 8,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
    let runtime = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Feet)
        .expect("expected feet runtime model");

    assert!(
        resolved.outfit.item_textures.contains(&(6, 1309804))
            && resolved.outfit.item_textures.contains(&(7, 1309802)),
        "expected feet display to keep merged foot textures: {:?}",
        resolved.outfit.item_textures
    );
    assert!(
        runtime
            .path
            .ends_with("collections_leather_raidroguemythic_q_01_hu_m.m2"),
        "unexpected runtime path: {}",
        runtime.path.display()
    );
    assert_eq!(runtime.skin_fdids[0], 1360784);
}

#[test]
fn m2_backed_feet_display_sets_boot_geoset_overrides() {
    let data = OutfitData::load(Path::new("data"));
    let appearance = NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Feet,
            item_id: Some(1),
            display_info_id: Some(154620),
            inventory_type: 8,
            hidden: false,
        }],
    };

    let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

    assert!(
        resolved.outfit.geoset_overrides.contains(&(5, 2)),
        "expected boot geoset override (5, 2) for group 5: {:?}",
        resolved.outfit.geoset_overrides
    );
    assert!(
        resolved.outfit.geoset_overrides.contains(&(20, 2)),
        "expected feet covering geoset override (20, 2) for group 20: {:?}",
        resolved.outfit.geoset_overrides
    );
}
