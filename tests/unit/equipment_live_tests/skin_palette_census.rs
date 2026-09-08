//! Manual census of potential palettes from production model spawning, not render allocations.

use std::collections::HashMap;
use std::path::Path;

use bevy::asset::AssetId;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;
use serde_json::{Value, json};

use super::{collect_descendants, configure_live_test_app, spawn_live_character};

// Entity and asset identities are compared only within each model's fresh App.
type PaletteGroups = HashMap<(Vec<Entity>, AssetId<SkinnedMeshInverseBindposes>), Vec<Entity>>;

#[test]
#[ignore = "manual local-asset skin palette census; no renderer or benchmark"]
fn census_local_asset_skin_palettes() {
    for model in ["humanmale_hd", "1011653", "126487"] {
        census_model(model, true);
    }
    // Other unique model types in the saved Theron native-env-only scene.
    for model in [
        "1000764", "124507", "1112735", "126278", "119376", "190025", "3730968", "119369", "125614",
    ] {
        census_model(model, false);
    }
}

fn census_model(model: &str, required: bool) {
    let path = Path::new("data/models").join(format!("{model}.m2"));
    if !path.is_file() {
        assert!(
            !required,
            "required local model missing: {}",
            path.display()
        );
        println!(
            "SKIN_PALETTE_CENSUS {}",
            json!({"model": path, "status": "missing_local_model"})
        );
        return;
    }
    println!("SKIN_PALETTE_CENSUS_BEGIN model={}", path.display());
    let mut app = game_engine::test_harness::headless_app_with(configure_live_test_app);
    let spawned = spawn_live_character(&mut app, &path);
    let groups = group_skinned_batches(app.world(), spawned.model_root);
    assert!(
        !required || !groups.is_empty(),
        "required model spawned no SkinnedMesh batches: {}",
        path.display()
    );
    println!(
        "SKIN_PALETTE_CENSUS {}",
        build_model_report(&path, app.world(), groups)
    );
}

fn group_skinned_batches(world: &World, root: Entity) -> PaletteGroups {
    let mut descendants = vec![root];
    collect_descendants(world, root, &mut descendants);
    let mut groups = PaletteGroups::new();
    for entity in descendants {
        if let Some(skin) = world.get::<SkinnedMesh>(entity) {
            groups
                .entry((skin.joints.clone(), skin.inverse_bindposes.id()))
                .or_default()
                .push(entity);
        }
    }
    groups
}

fn build_model_report(path: &Path, world: &World, groups: PaletteGroups) -> Value {
    let batch_count = groups.values().map(Vec::len).sum::<usize>();
    let per_batch_joints = groups
        .iter()
        .map(|((joints, _), batches)| joints.len() * batches.len())
        .sum::<usize>();
    let unique_palette_joints = groups.keys().map(|(joints, _)| joints.len()).sum::<usize>();
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(_, batches)| batches[0].to_bits());
    let groups: Vec<_> = groups
        .into_iter()
        .map(|((joints, bindposes), batches)| build_group_report(world, joints, bindposes, batches))
        .collect();
    json!({
        "model": path,
        "status": "spawned",
        "fixture": "bare model through equipment_live_tests::spawn_live_character",
        "identity_scope": "one fresh headless App per model; do not compare IDs across models",
        "camera_visibility_evaluated": false,
        "renderer_allocations_measured": false,
        "potential_skinned_batches": batch_count,
        "exact_palette_groups": groups.len(),
        "potential_per_batch_joint_entries": per_batch_joints,
        "potential_shared_palette_joint_entries": unique_palette_joints,
        "groups": groups,
    })
}

fn build_group_report(
    world: &World,
    joints: Vec<Entity>,
    bindposes: AssetId<SkinnedMeshInverseBindposes>,
    batches: Vec<Entity>,
) -> Value {
    let inverse_bindpose_count = world
        .resource::<Assets<SkinnedMeshInverseBindposes>>()
        .get(bindposes)
        .map(|poses| poses.len());
    json!({
        "inverse_bindpose_asset": format!("{bindposes:?}"),
        "inverse_bindpose_count": inverse_bindpose_count,
        "joint_count": joints.len(),
        "ordered_joint_entities": joints.iter().map(|entity| entity.to_bits()).collect::<Vec<_>>(),
        "batch_count": batches.len(),
        "potential_per_batch_joint_entries": joints.len() * batches.len(),
        "batches": batches.into_iter().map(|entity| build_batch_report(world, entity)).collect::<Vec<_>>(),
    })
}

fn build_batch_report(world: &World, entity: Entity) -> Value {
    json!({
        "entity": entity.to_bits(),
        "visibility": world.get::<Visibility>(entity).map(|value| format!("{value:?}")),
        "inherited_visibility_component": world.get::<InheritedVisibility>(entity).map(|value| value.get()),
        "view_visibility_component": world.get::<ViewVisibility>(entity).map(|value| value.get()),
    })
}
