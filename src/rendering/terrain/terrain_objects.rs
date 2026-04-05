//! Doodad (M2) and WMO spawning from _obj0/_obj1/_obj2 ADT companion files.

mod terrain_objects_wmo;

use std::path::Path;

use bevy::image::Image;
use bevy::math::Mat3;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::{adt_format::adt_obj, fogs_wdt};
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_spawn;

use crate::terrain::resolve_companion_path;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_tile::TILE_SIZE;
use terrain_objects_wmo::spawn_wmos_filtered;

#[derive(Default)]
pub struct SpawnedTerrainObjects {
    pub doodads: Vec<Entity>,
    pub wmos: Vec<SpawnedWmoRoot>,
}

pub struct SpawnedWmoRoot {
    pub entity: Entity,
    pub model: String,
}

#[derive(Component, Clone)]
pub struct WmoLocalSkybox {
    pub wow_path: String,
}

#[derive(Default)]
pub struct SpawnedFogVolumes {
    pub entities: Vec<Entity>,
}

const WMO_SCALE_FLAG: u16 = 0x4;
const WMO_NAME_IS_FDID_FLAG: u16 = 0x8;
const WMO_SCALE_UNIT: f32 = 1024.0;

impl SpawnedTerrainObjects {
    pub fn all_entities(self) -> Vec<Entity> {
        let mut entities = self.doodads;
        entities.extend(self.wmos.into_iter().map(|wmo| wmo.entity));
        entities
    }
}

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

// ── obj file loading ────────────────────────────────────────────────────────

/// Try to load a companion _obj ADT file at the given LOD suffix.
fn load_obj(adt_path: &Path, suffix: &str) -> Option<adt_obj::AdtObjData> {
    let obj_path = resolve_companion_path(adt_path, suffix)?;
    let data = std::fs::read(&obj_path).ok()?;
    match adt_obj::load_adt_obj0(&data) {
        Ok(obj) => Some(obj),
        Err(e) => {
            eprintln!("Failed to parse {suffix}: {e}");
            None
        }
    }
}

/// Load the _obj0.adt companion (full detail doodads).
pub fn load_obj0(adt_path: &Path) -> Option<adt_obj::AdtObjData> {
    load_obj(adt_path, "_obj0")
}

/// Load _obj1.adt (LOD level 1), falling back to _obj0 if unavailable.
pub fn load_obj1(adt_path: &Path) -> Option<adt_obj::AdtObjData> {
    load_obj(adt_path, "_obj1").or_else(|| load_obj(adt_path, "_obj0"))
}

/// Load _obj2.adt (LOD level 2), falling back to _obj1 then _obj0.
pub fn load_obj2(adt_path: &Path) -> Option<adt_obj::AdtObjData> {
    load_obj(adt_path, "_obj2")
        .or_else(|| load_obj(adt_path, "_obj1"))
        .or_else(|| load_obj(adt_path, "_obj0"))
}

// ── doodad spawning ─────────────────────────────────────────────────────────

/// Spawn doodads and WMOs, returning the created root entities grouped by type.
pub fn spawn_obj_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: Option<&TerrainHeightmap>,
    tile_y: u32,
    tile_x: u32,
    obj_data: &adt_obj::AdtObjData,
) -> SpawnedTerrainObjects {
    let mut spawned = SpawnedTerrainObjects::default();
    let doodad_chunk_refs = build_object_chunk_refs(
        obj_data.doodads.len(),
        obj_data
            .chunk_refs
            .iter()
            .map(|chunk_refs| chunk_refs.doodad_refs.as_slice()),
    );
    spawn_doodads_filtered(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inverse_bp,
        heightmap,
        tile_y,
        tile_x,
        obj_data,
        &doodad_chunk_refs,
        |_| true,
        &mut spawned.doodads,
    );
    spawn_wmos(
        commands,
        meshes,
        materials,
        images,
        tile_y,
        tile_x,
        obj_data,
        &mut spawned.wmos,
    );
    spawned
}

pub fn spawn_waterfall_backdrop_doodads(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: Option<&TerrainHeightmap>,
    tile_y: u32,
    tile_x: u32,
    obj_data: &adt_obj::AdtObjData,
) -> Vec<Entity> {
    let mut entities = Vec::new();
    let doodad_chunk_refs = build_object_chunk_refs(
        obj_data.doodads.len(),
        obj_data
            .chunk_refs
            .iter()
            .map(|chunk_refs| chunk_refs.doodad_refs.as_slice()),
    );
    spawn_doodads_filtered(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inverse_bp,
        heightmap,
        tile_y,
        tile_x,
        obj_data,
        &doodad_chunk_refs,
        is_waterfall_backdrop_doodad,
        &mut entities,
    );
    entities
}

pub fn spawn_nearby_campsite_objects(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: Option<&TerrainHeightmap>,
    tile_y: u32,
    tile_x: u32,
    obj_data: &adt_obj::AdtObjData,
    focus: Vec3,
    doodad_radius: f32,
    wmo_radius: f32,
) -> SpawnedTerrainObjects {
    let mut spawned = SpawnedTerrainObjects::default();
    let doodad_chunk_refs = build_object_chunk_refs(
        obj_data.doodads.len(),
        obj_data
            .chunk_refs
            .iter()
            .map(|chunk_refs| chunk_refs.doodad_refs.as_slice()),
    );
    let wmo_chunk_refs = build_object_chunk_refs(
        obj_data.wmos.len(),
        obj_data
            .chunk_refs
            .iter()
            .map(|chunk_refs| chunk_refs.wmo_refs.as_slice()),
    );
    spawn_doodads_filtered(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inverse_bp,
        heightmap,
        tile_y,
        tile_x,
        obj_data,
        &doodad_chunk_refs,
        |doodad| {
            doodad_position(doodad, tile_y, tile_x).distance(focus) <= doodad_radius
                && !is_charselect_clutter_doodad(doodad)
        },
        &mut spawned.doodads,
    );
    spawn_wmos_filtered(
        commands,
        meshes,
        materials,
        images,
        tile_y,
        tile_x,
        obj_data,
        &wmo_chunk_refs,
        |wmo| wmo_position(wmo, tile_y, tile_x).distance(focus) <= wmo_radius,
        &mut spawned.wmos,
    );
    spawned
}

/// Spawn doodads (M2 models) from placement data.
fn spawn_doodads_filtered(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: Option<&TerrainHeightmap>,
    tile_y: u32,
    tile_x: u32,
    obj_data: &adt_obj::AdtObjData,
    chunk_refs: &[Vec<u16>],
    filter: impl Fn(&adt_obj::DoodadPlacement) -> bool,
    entities: &mut Vec<Entity>,
) {
    let mut spawned = 0u32;
    for (index, doodad) in obj_data.doodads.iter().enumerate() {
        if !filter(doodad) {
            continue;
        }
        if let Some(e) = try_spawn_doodad(
            commands,
            meshes,
            materials,
            effect_materials,
            images,
            inverse_bp,
            heightmap,
            tile_y,
            tile_x,
            doodad,
            chunk_refs.get(index).map(Vec::as_slice),
        ) {
            entities.push(e);
            spawned += 1;
        }
    }
    eprintln!("Spawned {spawned} filtered doodads");
}

/// Try to spawn a single doodad. Returns the entity if successful.
fn try_spawn_doodad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: Option<&TerrainHeightmap>,
    tile_y: u32,
    tile_x: u32,
    doodad: &adt_obj::DoodadPlacement,
    chunk_refs: Option<&[u16]>,
) -> Option<Entity> {
    let m2_path = resolve_doodad_m2(doodad)?;
    if !m2_path.exists() {
        return None;
    }
    let transform = doodad_transform(doodad, heightmap, tile_y, tile_x);
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prop");
    let entity = commands
        .spawn((Name::new(name.to_owned()), transform, Visibility::default()))
        .id();
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
    let mut entity_commands = commands.entity(entity);
    entity_commands.insert(game_engine::culling::Doodad);
    if let Some(chunk_refs) = build_chunk_refs_component(chunk_refs) {
        entity_commands.insert(chunk_refs);
    }
    Some(entity)
}

fn build_object_chunk_refs<'a>(
    object_count: usize,
    chunk_refs: impl Iterator<Item = &'a [u32]>,
) -> Vec<Vec<u16>> {
    let mut refs_by_object = vec![Vec::new(); object_count];
    for (chunk_index, object_refs) in chunk_refs.enumerate() {
        for &object_index in object_refs {
            let Some(object_chunk_refs) = refs_by_object.get_mut(object_index as usize) else {
                continue;
            };
            object_chunk_refs.push(chunk_index as u16);
        }
    }
    refs_by_object
}

fn build_chunk_refs_component(
    chunk_refs: Option<&[u16]>,
) -> Option<game_engine::culling::ChunkRefs> {
    let chunk_indices = chunk_refs?;
    (!chunk_indices.is_empty()).then(|| game_engine::culling::ChunkRefs {
        chunk_indices: chunk_indices.to_vec(),
    })
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
    attach_fog_volume_parent(commands, parent, entity);
    Some(entity)
}

fn spawn_fog_volume_entity(commands: &mut Commands, volume: &fogs_wdt::FogVolume) -> Entity {
    let [x, y, z] =
        crate::asset::m2::wow_to_bevy(volume.position[0], volume.position[1], volume.position[2]);
    let rotation = wow_quat_to_bevy(volume.rotation);
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

fn attach_fog_volume_parent(commands: &mut Commands, parent: Option<Entity>, entity: Entity) {
    if let Some(parent) = parent {
        commands.entity(parent).add_child(entity);
    }
}

/// Resolve a doodad placement to a local M2 file path.
fn resolve_doodad_m2(doodad: &adt_obj::DoodadPlacement) -> Option<std::path::PathBuf> {
    if let Some(fdid) = doodad.fdid {
        return crate::asset::asset_cache::model(fdid);
    }
    let wow_path = doodad.path.as_ref()?;
    let fdid = game_engine::listfile::lookup_path(wow_path)?;
    crate::asset::asset_cache::model(fdid)
}

fn doodad_model_name(doodad: &adt_obj::DoodadPlacement) -> Option<String> {
    doodad
        .fdid
        .and_then(game_engine::listfile::lookup_fdid)
        .map(str::to_string)
        .or_else(|| doodad.path.clone())
}

fn is_waterfall_backdrop_doodad(doodad: &adt_obj::DoodadPlacement) -> bool {
    let Some(model) = doodad_model_name(doodad) else {
        return false;
    };
    let model = model.to_ascii_lowercase();
    model.contains("waterfall") || model.contains("ripple01_misty")
}

fn is_charselect_clutter_doodad(doodad: &adt_obj::DoodadPlacement) -> bool {
    let Some(model) = doodad_model_name(doodad) else {
        return false;
    };
    let model = model.to_ascii_lowercase();
    model.contains("spells/")
        || model.contains("pineneedles")
        || model.contains("pinecone")
        || model.contains("twigs")
        || model.contains("forestflowers")
        || model.contains("spriggyplant")
        || model.contains("groundivy")
        || model.contains("grass")
        || model.contains("smoke")
}

/// Convert WoW doodad placement to a Bevy Transform.
fn doodad_transform(
    d: &adt_obj::DoodadPlacement,
    heightmap: Option<&TerrainHeightmap>,
    tile_y: u32,
    tile_x: u32,
) -> Transform {
    let mut pos = doodad_position(d, tile_y, tile_x);
    if let Some(terrain_y) = heightmap.and_then(|heightmap| heightmap.height_at(pos.x, pos.z)) {
        pos.y = pos.y.max(terrain_y);
    }
    let rotation = placement_rotation(d.rotation);
    Transform::from_translation(pos)
        .with_rotation(rotation)
        .with_scale(Vec3::splat(d.scale))
}

fn doodad_position(d: &adt_obj::DoodadPlacement, tile_y: u32, tile_x: u32) -> Vec3 {
    Vec3::from(placement_to_bevy_on_tile(d.position, tile_y, tile_x))
}

/// Convert WoW MDDF/MODF Euler rotation to Bevy.
/// stored [X, Y, Z] becomes model rotation [Z, Y - 180, -X], then YZX order.
fn placement_rotation(rot: [f32; 3]) -> Quat {
    let bank_x = rot[2].to_radians();
    let heading_y = (rot[1] - 180.0).to_radians();
    let attitude_z = (-rot[0]).to_radians();
    Quat::from_euler(EulerRot::YZX, heading_y, attitude_z, bank_x)
}

fn wow_quat_to_bevy(raw: [f32; 4]) -> Quat {
    let wow_quat = normalize_quat_or_identity(Quat::from_xyzw(raw[0], raw[1], raw[2], raw[3]));
    let basis = Mat3::from_cols(Vec3::X, -Vec3::Z, Vec3::Y);
    let bevy_rot = basis * Mat3::from_quat(wow_quat) * basis.transpose();
    normalize_quat_or_identity(Quat::from_mat3(&bevy_rot))
}

fn normalize_quat_or_identity(quat: Quat) -> Quat {
    let len = quat.length();
    if len > f32::EPSILON {
        quat / len
    } else {
        Quat::IDENTITY
    }
}

/// Spawn WMOs from placement data.
fn spawn_wmos(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    tile_y: u32,
    tile_x: u32,
    obj_data: &adt_obj::AdtObjData,
    entities: &mut Vec<SpawnedWmoRoot>,
) {
    let wmo_chunk_refs = build_object_chunk_refs(
        obj_data.wmos.len(),
        obj_data
            .chunk_refs
            .iter()
            .map(|chunk_refs| chunk_refs.wmo_refs.as_slice()),
    );
    spawn_wmos_filtered(
        commands,
        meshes,
        materials,
        images,
        tile_y,
        tile_x,
        obj_data,
        &wmo_chunk_refs,
        |_| true,
        entities,
    );
}

// ── coordinate conversion ───────────────────────────────────────────────────

/// Convert MODF/MDDF placement position to Bevy-space using the legacy
/// absolute-world ADT convention.
pub(super) fn placement_to_bevy_absolute(raw: [f32; 3]) -> [f32; 3] {
    let center = 32.0 * TILE_SIZE;
    [center - raw[2], raw[1], raw[0] - center]
}

fn placement_to_bevy_on_tile(raw: [f32; 3], tile_y: u32, tile_x: u32) -> [f32; 3] {
    let absolute = placement_to_bevy_absolute(raw);
    let (abs_ty, abs_tx) = crate::terrain_tile::bevy_to_tile_coords(absolute[0], absolute[2]);
    if abs_ty.abs_diff(tile_y) <= 1 && abs_tx.abs_diff(tile_x) <= 1 {
        return absolute;
    }
    crate::asset::m2::wow_to_bevy(raw[0], raw[2], raw[1])
}

/// Convert WMO placement to a Bevy Transform.
fn wmo_transform(w: &adt_obj::WmoPlacement, tile_y: u32, tile_x: u32) -> Transform {
    let rotation = placement_rotation(w.rotation);
    Transform::from_translation(wmo_position(w, tile_y, tile_x))
        .with_rotation(rotation)
        .with_scale(Vec3::splat(w.scale))
}

pub(super) fn wmo_position(w: &adt_obj::WmoPlacement, tile_y: u32, tile_x: u32) -> Vec3 {
    Vec3::from(placement_to_bevy_on_tile(w.position, tile_y, tile_x))
}

#[cfg(test)]
#[path = "../../../tests/unit/terrain_objects_tests.rs"]
mod tests;
