use std::path::Path;
use std::sync::{Mutex, OnceLock};

use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::adt::{self};
use crate::asset::adt_format::adt_obj;
use crate::m2_effect_material::M2EffectMaterial;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::{self, TerrainMaterial};
use crate::terrain_objects;
use crate::water_material::{self, WaterMaterial, WaterSettings};

use super::terrain_spawn_position::choose_safe_spawn_position;
use super::{AdtTile, ParsedTile};

/// Grouped asset refs for spawn helpers (reduces per-function argument count).
pub(super) struct SpawnRefs<'a, 'w, 's> {
    pub(super) commands: &'a mut Commands<'w, 's>,
    pub(super) meshes: &'a mut Assets<Mesh>,
    pub(super) materials: &'a mut Assets<StandardMaterial>,
    pub(super) effect_materials: &'a mut Assets<M2EffectMaterial>,
    pub(super) terrain_materials: &'a mut Assets<TerrainMaterial>,
    pub(super) water_materials: &'a mut Assets<WaterMaterial>,
    pub(super) images: &'a mut Assets<Image>,
    pub(super) inverse_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
}

pub(super) fn load_and_parse_adt(adt_path: &Path) -> Result<adt::AdtData, String> {
    let (_, tile_y, tile_x) = crate::terrain_tile::parse_tile_coords_from_path(adt_path)?;
    let data = std::fs::read(adt_path)
        .map_err(|e| format!("Failed to read {}: {e}", adt_path.display()))?;
    let adt = adt::load_adt_for_tile(&data, tile_y, tile_x)?;
    if let Some(err) = &adt.water_error {
        warn_mh2o_once(adt_path, err);
    }
    Ok(adt)
}

fn warn_mh2o_once(adt_path: &Path, err: &str) {
    static WARNED_TILES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let warned = WARNED_TILES.get_or_init(|| Mutex::new(HashSet::new()));
    let key = format!("{}:{err}", adt_path.display());
    let mut warned = warned.lock().unwrap();
    if warned.insert(key) {
        warn!("Ignoring malformed MH2O in {}: {err}", adt_path.display());
    }
}

use std::collections::HashSet;

pub(super) fn spawn_terrain_chunks(
    refs: &mut SpawnRefs,
    adt_path: &Path,
    adt_data: &adt::AdtData,
    tex_data: Option<&adt::AdtTexData>,
    tile: &AdtTile,
) -> Entity {
    eprintln!(
        "spawn_terrain_chunks {} chunks={} tex={}",
        adt_path.display(),
        adt_data.chunks.len(),
        tex_data.map_or(0, |td| td.texture_fdids.len()),
    );
    let ground_images =
        tex_data.map(|td| terrain_material::load_ground_images(refs.images, td, adt_path));
    let height_images =
        tex_data.map(|td| terrain_material::load_height_images(refs.images, td, adt_path));
    eprintln!("build_terrain_materials {}", adt_path.display());
    let chunk_materials = terrain_material::build_terrain_materials(
        refs.terrain_materials,
        refs.images,
        adt_data,
        tex_data,
        ground_images.as_deref(),
        height_images.as_deref(),
    );
    let root = spawn_chunk_entities(refs.commands, refs.meshes, &chunk_materials, adt_data, tile);
    spawn_water(
        refs.commands,
        refs.meshes,
        refs.water_materials,
        refs.images,
        adt_data,
    );
    root
}

pub(super) fn spawn_parsed_tile(
    refs: &mut SpawnRefs,
    heightmap: &TerrainHeightmap,
    parsed: &ParsedTile,
) -> (Entity, Vec<Entity>) {
    let tile = parsed_adt_tile(parsed);
    log_parsed_tile(parsed);
    let root = spawn_terrain_chunks(
        refs,
        &parsed.adt_path,
        &parsed.adt_data,
        parsed.tex_data.as_ref(),
        &tile,
    );
    let doodad_entities = spawn_parsed_tile_doodads(refs, heightmap, parsed);
    (root, doodad_entities)
}

fn parsed_adt_tile(parsed: &ParsedTile) -> AdtTile {
    AdtTile {
        _tile_x: parsed.tile_x,
        _tile_y: parsed.tile_y,
    }
}

fn log_parsed_tile(parsed: &ParsedTile) {
    eprintln!(
        "Spawning parsed tile ({}, {}) {} tex={} doodads={} wmos={}",
        parsed.tile_y,
        parsed.tile_x,
        parsed.adt_path.display(),
        parsed
            .tex_data
            .as_ref()
            .map_or(0, |td| td.texture_fdids.len()),
        parsed.obj_data.as_ref().map_or(0, |obj| obj.doodads.len()),
        parsed.obj_data.as_ref().map_or(0, |obj| obj.wmos.len()),
    );
}

fn spawn_parsed_tile_doodads(
    refs: &mut SpawnRefs,
    heightmap: &TerrainHeightmap,
    parsed: &ParsedTile,
) -> Vec<Entity> {
    let Some(ref obj_data) = parsed.obj_data else {
        return Vec::new();
    };

    terrain_objects::spawn_obj_entities(
        refs.commands,
        refs.meshes,
        refs.materials,
        refs.effect_materials,
        refs.images,
        refs.inverse_bp,
        Some(heightmap),
        parsed.tile_y,
        parsed.tile_x,
        obj_data,
    )
    .all_entities()
}

pub(super) fn log_adt_spawn(adt_data: &adt::AdtData, adt_path: &Path) {
    let water_count = adt_data.water.as_ref().map_or(0, |w| {
        w.chunks.iter().filter(|c| !c.layers.is_empty()).count()
    });
    eprintln!(
        "Spawned ADT terrain: {} chunks, {} water chunks from {}",
        adt_data.chunks.len(),
        water_count,
        adt_path.display(),
    );
}

pub(super) fn spawn_chunk_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    chunk_materials: &[Handle<TerrainMaterial>],
    adt_data: &adt::AdtData,
    tile: &AdtTile,
) -> Entity {
    let root = commands
        .spawn((
            super::AdtTerrain,
            tile.clone(),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    for (i, chunk) in adt_data.chunks.iter().enumerate() {
        let mesh_handle = meshes.add(chunk.mesh.clone());
        let mat = chunk_materials.get(i).unwrap_or(&chunk_materials[0]);
        let mut spawn = commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(mat.clone()),
            tile.clone(),
            Transform::default(),
            Visibility::default(),
        ));
        if let Some(grid) = adt_data.height_grids.get(i) {
            use crate::asset::adt::CHUNK_SIZE;
            spawn.insert(game_engine::culling::TerrainChunk {
                chunk_index: i as u16,
                world_center: Vec3::new(
                    grid.origin_x + CHUNK_SIZE / 2.0,
                    grid.base_y,
                    grid.origin_z - CHUNK_SIZE / 2.0,
                ),
            });
        }
        let child = spawn.id();
        commands.entity(root).add_child(child);
    }
    root
}

pub(super) fn compute_spawn_result(
    adt_data: &adt::AdtData,
    obj_data: Option<&adt_obj::AdtObjData>,
) -> (Transform, Vec3) {
    let spawn = choose_safe_spawn_position(adt_data, obj_data)
        .unwrap_or_else(|| adt_data.center_surface.into());
    let focus = spawn + Vec3::Y * 1.8;
    let eye = focus + Vec3::new(0.0, 28.0, 18.0);
    let camera = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
    (camera, spawn)
}

pub(super) fn spawn_water(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    adt_data: &adt::AdtData,
) {
    let Some(ref water_data) = adt_data.water else {
        return;
    };
    let normal_map = images.add(water_material::generate_water_normal_map());
    let mat = water_materials.add(WaterMaterial {
        settings: WaterSettings::default(),
        normal_map,
    });

    for (i, chunk_water) in water_data.chunks.iter().enumerate() {
        for layer in &chunk_water.layers {
            if layer.width == 0 || layer.height == 0 {
                continue;
            }
            let chunk_pos = adt_data.chunk_positions[i];
            let mesh = adt::build_water_mesh(chunk_pos, layer);
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(mat.clone()),
                Transform::default(),
                Visibility::default(),
            ));
        }
    }
}
