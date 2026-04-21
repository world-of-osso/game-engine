use bevy::prelude::*;

use crate::sky::SkyEnvMapHandle;

use super::TerrainMaterial;

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
    for (_id, material) in terrain_materials.iter_mut() {
        if material.environment_map != env_handle.0 {
            material.environment_map = env_handle.0.clone();
        }
    }
}
