//! Doodad LOD swap system for ADT terrain streaming.

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::adt_obj;
use crate::m2_effect_material::M2EffectMaterial;
use crate::terrain::{AdtManager, DoodadLod};
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_objects;
use crate::terrain_tile::{bevy_to_tile_coords, resolve_tile_path, tile_lod_for_distance};

/// Grouped asset refs for LOD spawn helpers (reduces per-function argument count).
struct LodSpawnRefs<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    effect_materials: &'a mut Assets<M2EffectMaterial>,
    images: &'a mut Assets<Image>,
    inverse_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
}

/// Swap doodad LOD levels when player crosses distance thresholds.
pub(crate) fn doodad_lod_swap_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    heightmap: Res<TerrainHeightmap>,
    mut adt_manager: ResMut<AdtManager>,
    player_q: Query<&Transform, With<crate::camera::Player>>,
) {
    if adt_manager.map_name.is_empty() {
        return;
    }
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let (cy, cx) = bevy_to_tile_coords(player_tf.translation.x, player_tf.translation.z);
    let swaps = find_lod_swaps(&adt_manager, cy, cx);
    let mut refs = LodSpawnRefs {
        commands: &mut commands,
        meshes: &mut meshes,
        materials: &mut materials,
        effect_materials: &mut effect_materials,
        images: &mut images,
        inverse_bp: &mut inverse_bp,
    };
    for (key, new_lod) in swaps {
        swap_tile_lod(&mut refs, &heightmap, &mut adt_manager, key, new_lod);
    }
}

/// Find tiles whose LOD level needs changing.
fn find_lod_swaps(adt_manager: &AdtManager, cy: u32, cx: u32) -> Vec<((u32, u32), DoodadLod)> {
    adt_manager
        .tile_lod
        .iter()
        .filter_map(|(&key, &current_lod)| {
            let desired = tile_lod_for_distance(key.0, key.1, cy, cx);
            if desired != current_lod {
                Some((key, desired))
            } else {
                None
            }
        })
        .collect()
}

/// Swap a tile's doodads to a new LOD level.
fn swap_tile_lod(
    refs: &mut LodSpawnRefs,
    heightmap: &TerrainHeightmap,
    adt_manager: &mut AdtManager,
    key: (u32, u32),
    new_lod: DoodadLod,
) {
    let Ok(adt_path) = resolve_tile_path(&adt_manager.map_name, key.0, key.1) else {
        return;
    };
    despawn_tile_doodad_entities(refs.commands, adt_manager, key);
    let new_entities = spawn_lod_doodads(refs, heightmap, &adt_path, new_lod);
    adt_manager.tile_lod.insert(key, new_lod);
    adt_manager.tile_doodad_entities.insert(key, new_entities);
    eprintln!("LOD swap tile ({}, {}): {:?}", key.0, key.1, new_lod);
}

/// Despawn doodad/WMO entities for a tile (without removing LOD tracking).
pub(crate) fn despawn_tile_doodad_entities(
    commands: &mut Commands,
    adt_manager: &mut AdtManager,
    key: (u32, u32),
) {
    if let Some(entities) = adt_manager.tile_doodad_entities.remove(&key) {
        for e in entities {
            commands.entity(e).despawn();
        }
    }
}

/// Load the appropriate obj file based on LOD level.
pub(crate) fn load_obj_for_lod(
    adt_path: &std::path::Path,
    lod: DoodadLod,
) -> Option<adt_obj::AdtObjData> {
    match lod {
        DoodadLod::Full => terrain_objects::load_obj0(adt_path),
        DoodadLod::Lod1 => terrain_objects::load_obj1(adt_path),
        DoodadLod::Lod2 => terrain_objects::load_obj2(adt_path),
    }
}

/// Load and spawn doodads/WMOs for a given LOD level.
fn spawn_lod_doodads(
    refs: &mut LodSpawnRefs,
    heightmap: &TerrainHeightmap,
    adt_path: &std::path::Path,
    lod: DoodadLod,
) -> Vec<Entity> {
    match load_obj_for_lod(adt_path, lod) {
        Some(ref obj) => terrain_objects::spawn_obj_entities(
            refs.commands,
            refs.meshes,
            refs.materials,
            refs.effect_materials,
            refs.images,
            refs.inverse_bp,
            Some(heightmap),
            crate::terrain_tile::parse_tile_coords_from_path(adt_path)
                .map(|(_, ty, _)| ty)
                .unwrap_or(0),
            crate::terrain_tile::parse_tile_coords_from_path(adt_path)
                .map(|(_, _, tx)| tx)
                .unwrap_or(0),
            obj,
        )
        .all_entities(),
        None => Vec::new(),
    }
}
