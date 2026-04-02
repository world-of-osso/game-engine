use super::*;
use std::path::Path;

fn hybrid_legs_appearance() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Legs,
            item_id: Some(1),
            display_info_id: Some(159629),
            inventory_type: 7,
            hidden: false,
        }],
    }
}

fn geoset_only_legs_appearance() -> NetEquipmentAppearance {
    NetEquipmentAppearance {
        entries: vec![shared::components::EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Legs,
            item_id: Some(1),
            display_info_id: Some(73783),
            inventory_type: 7,
            hidden: false,
        }],
    }
}

fn resolved_hybrid_legs() -> ResolvedEquipmentAppearance {
    let data = OutfitData::load(Path::new("data"));
    resolve_equipment_appearance(&hybrid_legs_appearance(), &data, 1, 0)
}

#[test]
fn geoset_only_legs_display_keeps_and_extracts_leg_textures() {
    let upper = Path::new("data/textures/360325.blp");
    let lower = Path::new("data/textures/360317.blp");
    let _ = std::fs::remove_file(upper);
    let _ = std::fs::remove_file(lower);

    let data = OutfitData::load(Path::new("data"));
    let resolved = resolve_equipment_appearance(&geoset_only_legs_appearance(), &data, 1, 0);

    assert!(
        resolved.outfit.item_textures.contains(&(5, 360325))
            && resolved.outfit.item_textures.contains(&(6, 360317)),
        "expected geoset-only legs display to keep merged leg textures: {:?}",
        resolved.outfit.item_textures
    );
    assert!(
        resolved
            .runtime_models
            .iter()
            .all(|model| model.slot != EquipmentSlot::Legs),
        "expected geoset-only legs display to avoid runtime legs model: {:?}",
        resolved.runtime_models
    );
    assert_eq!(
        resolved.outfit.geoset_overrides,
        vec![(11, 4)],
        "expected geoset-only legs display to drive the pants geoset: {:?}",
        resolved.outfit.geoset_overrides
    );
    assert!(upper.exists(), "expected upper-leg texture extraction");
    assert!(lower.exists(), "expected lower-leg texture extraction");
}

#[test]
fn debug_m2_backed_legs_display_mesh_parts() {
    let data = OutfitData::load(Path::new("data"));
    let (fdid, skin_fdids) = data
        .resolve_runtime_model(159629, 1, 0)
        .expect("resolve runtime model");
    let wow_path = game_engine::listfile::lookup_fdid(fdid).expect("listfile path");
    let model_path = crate::asset::asset_cache::file_at_path(
        fdid,
        &Path::new("data/item-models").join(wow_path),
    )
    .expect("extract model");
    let _ = crate::asset::m2::ensure_primary_skin_path(&model_path);
    let model =
        crate::asset::m2::load_m2(&model_path, &skin_fdids).expect("load runtime legs model");
    let mut mesh_ids = model
        .batches
        .iter()
        .map(|batch| batch.mesh_part_id)
        .collect::<Vec<_>>();
    mesh_ids.sort_unstable();
    mesh_ids.dedup();
    println!("runtime legs mesh ids: {mesh_ids:?}");
}

#[test]
fn m2_backed_legs_display_resolves_runtime_model() {
    let resolved = resolved_hybrid_legs();
    let runtime = resolved
        .runtime_models
        .iter()
        .find(|model| model.slot == EquipmentSlot::Legs)
        .expect("expected legs runtime model");

    assert!(
        resolved.outfit.item_textures.contains(&(5, 1535330))
            && resolved.outfit.item_textures.contains(&(6, 1535318)),
        "expected legs display to keep merged leg textures: {:?}",
        resolved.outfit.item_textures
    );
    assert!(
        resolved.outfit.geoset_overrides == vec![(11, 4)],
        "expected hybrid legs display to drive the pants geoset only: {:?}",
        resolved.outfit.geoset_overrides
    );
    assert!(
        runtime
            .path
            .ends_with("collections_leather_raidmonk_r_01_hu_m.m2"),
        "unexpected runtime path: {}",
        runtime.path.display()
    );
    assert_eq!(runtime.skin_fdids[0], 1535026);
}
