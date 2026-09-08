use bevy::prelude::*;

use crate::sky::SkyEnvMapHandle;

use super::{TerrainMaterial, TerrainMaterialFreezeAfter};

pub(super) fn terrain_material_updates_enabled(
    time: Res<Time<Real>>,
    freeze_after: Option<Res<TerrainMaterialFreezeAfter>>,
    materials: Res<Assets<TerrainMaterial>>,
    mut reported: Local<bool>,
) -> bool {
    let Some(freeze_after) = freeze_after else {
        return true;
    };
    if time.elapsed() < freeze_after.0 {
        return true;
    }
    if !*reported {
        eprintln!(
            "terrain_material_freeze elapsed_s={:.3} deadline_s={} materials={}",
            time.elapsed_secs_f64(),
            freeze_after.0.as_secs(),
            materials.len(),
        );
        *reported = true;
    }
    false
}

#[cfg(test)]
mod tests;

pub(super) fn update_terrain_animation_time(
    time: Res<Time>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
) {
    let animation_time = time.elapsed_secs();
    for (_id, material) in terrain_materials.iter_mut() {
        material.settings.config.w = animation_time;
    }
}

pub(super) fn sync_terrain_environment_map(
    env_handle: Option<Res<SkyEnvMapHandle>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
) {
    let Some(env_handle) = env_handle else { return };
    let changed: Vec<_> = terrain_materials
        .iter()
        .filter_map(|(id, material)| (material.environment_map != env_handle.0).then_some(id))
        .collect();
    for id in changed {
        let mut material = terrain_materials
            .get_mut(id)
            .expect("terrain material collected from the same asset storage");
        material.environment_map = env_handle.0.clone();
    }
}
