//! Fog volume spawning from WDT _fogs companion files.

use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::fogs_wdt;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_spawn;

use super::SpawnedFogVolumes;

pub fn load_map_fogs_wdt(map_name: &str) -> Option<fogs_wdt::FogsWdt> {
    let wow_path = format!("world/maps/{map_name}/{map_name}_fogs.wdt");
    let fdid = game_engine::listfile::lookup_path(&wow_path)?;
    let local_path = std::path::PathBuf::from(format!("data/fogs/{fdid}.wdt"));
    let path = crate::asset::asset_cache::file_at_path(fdid, &local_path)?;
    let data = std::fs::read(path).ok()?;
    match fogs_wdt::load_fogs_wdt(&data) {
        Ok(fogs) => Some(fogs),
        Err(err) => {
            eprintln!("Failed to parse {wow_path}: {err}");
            None
        }
    }
}

pub fn spawn_map_fog_volumes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    map_name: &str,
    parent: Option<Entity>,
) -> SpawnedFogVolumes {
    let Some(fogs) = load_map_fogs_wdt(map_name) else {
        return SpawnedFogVolumes::default();
    };
    let mut entities = Vec::new();
    for volume in &fogs.volumes {
        let Some(entity) = try_spawn_fog_volume(
            commands,
            meshes,
            materials,
            effect_materials,
            images,
            inverse_bp,
            volume,
            parent,
        ) else {
            continue;
        };
        entities.push(entity);
    }
    eprintln!(
        "Spawned {}/{} fog volumes for map {map_name}",
        entities.len(),
        fogs.volumes.len()
    );
    SpawnedFogVolumes { entities }
}

fn try_spawn_fog_volume(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    volume: &fogs_wdt::FogVolume,
    parent: Option<Entity>,
) -> Option<Entity> {
    let m2_path = crate::asset::asset_cache::model(volume.model_fdid)?;
    if !m2_path.exists() {
        return None;
    }
    let entity = spawn_fog_volume_entity(commands, volume);
    if !m2_spawn::spawn_m2_on_entity(
        commands,
        &mut m2_spawn::SpawnAssets {
            meshes,
            materials,
            effect_materials,
            skybox_materials: None,
            images,
            inverse_bindposes: inverse_bp,
        },
        &m2_path,
        entity,
        &[0, 0, 0],
    ) {
        commands.entity(entity).despawn();
        return None;
    }
    if let Some(parent) = parent {
        commands.entity(parent).add_child(entity);
    }
    Some(entity)
}

fn spawn_fog_volume_entity(commands: &mut Commands, volume: &fogs_wdt::FogVolume) -> Entity {
    let [x, y, z] =
        crate::asset::m2::wow_to_bevy(volume.position[0], volume.position[1], volume.position[2]);
    let rotation = super::wow_quat_to_bevy(volume.rotation);
    commands
        .spawn((
            Name::new(format!("FogVolume_{}", volume.fog_id)),
            Transform::from_translation(Vec3::new(x, y, z))
                .with_rotation(rotation)
                .with_scale(Vec3::ONE),
            Visibility::default(),
        ))
        .id()
}
