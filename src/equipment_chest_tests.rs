use super::*;
use std::path::Path;

fn hybrid_chest_appearance() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Chest,
            item_id: Some(1),
            display_info_id: Some(175942),
            inventory_type: 5,
            hidden: false,
        }],
    }
}

fn geoset_only_chest_appearance() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Chest,
            item_id: Some(1),
            display_info_id: Some(692385),
            inventory_type: 5,
            hidden: false,
        }],
    }
}

#[test]
fn geoset_only_chest_display_keeps_and_extracts_chest_textures() {
    let upper = Path::new("data/textures/5213107.blp");
    let torso_upper = Path::new("data/textures/5213102.blp");
    let torso_lower = Path::new("data/textures/5213101.blp");
    let _ = std::fs::remove_file(upper);
    let _ = std::fs::remove_file(torso_upper);
    let _ = std::fs::remove_file(torso_lower);

    let data = OutfitData::load(Path::new("data"));
    let resolved = resolve_equipment_appearance(&geoset_only_chest_appearance(), &data, 1, 0);

    assert!(
        resolved.outfit.item_textures.contains(&(0, 5213107))
            && resolved.outfit.item_textures.contains(&(3, 5213102))
            && resolved.outfit.item_textures.contains(&(4, 5213101)),
        "expected geoset-only chest display to keep merged chest textures: {:?}",
        resolved.outfit.item_textures
    );
    assert!(
        resolved.runtime_models.iter().all(|model| model.slot != EquipmentSlot::Chest),
        "expected geoset-only chest display to avoid runtime chest model: {:?}",
        resolved.runtime_models
    );
    assert_eq!(
        resolved.outfit.geoset_overrides,
        vec![(22, 2)],
        "expected geoset-only chest display to drive the chest geoset: {:?}",
        resolved.outfit.geoset_overrides
    );
    assert!(upper.exists(), "expected upper-arm texture extraction");
    assert!(torso_upper.exists(), "expected upper-torso texture extraction");
    assert!(torso_lower.exists(), "expected lower-torso texture extraction");
}

#[test]
fn debug_m2_backed_chest_display_mesh_parts() {
    let data = OutfitData::load(Path::new("data"));
    let (fdid, skin_fdids) = data
        .resolve_runtime_model(175942, 1, 0)
        .expect("resolve runtime model");
    let wow_path = game_engine::listfile::lookup_fdid(fdid).expect("listfile path");
    let model_path = crate::asset::casc_resolver::ensure_file_at_path(
        fdid,
        &Path::new("data/item-models").join(wow_path),
    )
    .expect("extract model");
    let _ = crate::asset::m2::ensure_primary_skin_path(&model_path);
    let model =
        crate::asset::m2::load_m2(&model_path, &skin_fdids).expect("load runtime chest model");
    let mut mesh_ids = model
        .batches
        .iter()
        .map(|batch| batch.mesh_part_id)
        .collect::<Vec<_>>();
    mesh_ids.sort_unstable();
    mesh_ids.dedup();
    println!("runtime chest mesh ids: {mesh_ids:?}");
}

#[test]
fn debug_humanmale_hd_chest_mesh_parts() {
    let model = crate::asset::m2::load_m2(Path::new("data/models/humanmale_hd.m2"), &[0, 0, 0])
        .expect("load humanmale_hd");
    let mut mesh_ids = model
        .batches
        .iter()
        .map(|batch| batch.mesh_part_id)
        .filter(|mesh_part_id| matches!(mesh_part_id / 100, 22 | 23))
        .collect::<Vec<_>>();
    mesh_ids.sort_unstable();
    mesh_ids.dedup();
    println!("humanmale_hd chest mesh ids: {mesh_ids:?}");
}

#[test]
fn m2_backed_chest_display_resolves_runtime_model() {
    let data = OutfitData::load(Path::new("data"));
    let resolved = resolve_equipment_appearance(&hybrid_chest_appearance(), &data, 1, 0);
    let runtime = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Chest)
        .expect("expected chest runtime model");

    assert!(
        resolved.outfit.item_textures.contains(&(0, 2373869))
            && resolved.outfit.item_textures.contains(&(3, 2373863))
            && resolved.outfit.item_textures.contains(&(4, 2373861)),
        "expected chest display to keep merged chest textures: {:?}",
        resolved.outfit.item_textures
    );
    assert!(
        resolved.outfit.geoset_overrides.is_empty(),
        "expected hybrid chest display to avoid merged chest geoset override: {:?}",
        resolved.outfit.geoset_overrides
    );
    assert!(
        runtime
            .path
            .ends_with("collections_mail_warfrontsnightelfmythic_d_01_hu_m.m2"),
        "unexpected runtime path: {}",
        runtime.path.display()
    );
    assert_eq!(runtime.skin_fdids[0], 2373825);
}
