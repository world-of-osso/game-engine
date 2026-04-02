//! Doodad (M2) and WMO spawning from _obj0/_obj1/_obj2 ADT companion files.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use bevy::image::Image;
use bevy::math::Mat3;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::{adt_obj, blp, fogs_wdt, wmo};
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_spawn;

use crate::terrain::resolve_companion_path;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_tile::TILE_SIZE;

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

#[derive(Clone, PartialEq, Eq, Hash)]
struct WmoTextureCacheKey {
    base_path: PathBuf,
    texture_2_fdid: u32,
    texture_3_fdid: u32,
}

static WMO_TEXTURE_CACHE: OnceLock<
    Mutex<std::collections::HashMap<WmoTextureCacheKey, Result<Handle<Image>, String>>>,
> = OnceLock::new();

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
    filter: impl Fn(&adt_obj::DoodadPlacement) -> bool,
    entities: &mut Vec<Entity>,
) {
    let mut spawned = 0u32;
    for doodad in &obj_data.doodads {
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
    commands.entity(entity).insert(game_engine::culling::Doodad);
    Some(entity)
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
    let [x, y, z] =
        crate::asset::m2::wow_to_bevy(volume.position[0], volume.position[1], volume.position[2]);
    let rotation = wow_quat_to_bevy(volume.rotation);
    let entity = commands
        .spawn((
            Name::new(format!("FogVolume_{}", volume.fog_id)),
            Transform::from_translation(Vec3::new(x, y, z))
                .with_rotation(rotation)
                .with_scale(Vec3::ONE),
            Visibility::default(),
        ))
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
    if let Some(parent) = parent {
        commands.entity(parent).add_child(entity);
    }
    Some(entity)
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

// ── WMO spawning ────────────────────────────────────────────────────────────

struct WmoAssets<'a> {
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    images: &'a mut Assets<Image>,
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
    spawn_wmos_filtered(
        commands,
        meshes,
        materials,
        images,
        tile_y,
        tile_x,
        obj_data,
        |_| true,
        entities,
    );
}

fn spawn_wmos_filtered(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    tile_y: u32,
    tile_x: u32,
    obj_data: &adt_obj::AdtObjData,
    filter: impl Fn(&adt_obj::WmoPlacement) -> bool,
    entities: &mut Vec<SpawnedWmoRoot>,
) {
    let mut spawned_count = 0u32;
    for placement in &obj_data.wmos {
        if !filter(placement) {
            continue;
        }
        let mut assets = WmoAssets {
            meshes,
            materials,
            images,
        };
        if let Some(spawned_wmo) = try_spawn_wmo(commands, &mut assets, placement, tile_y, tile_x) {
            entities.push(spawned_wmo);
            spawned_count += 1;
        }
    }
    eprintln!("Spawned {spawned_count}/{} WMOs", obj_data.wmos.len());
}

/// Try to spawn a single WMO. Returns root entity if successful.
fn try_spawn_wmo(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    placement: &adt_obj::WmoPlacement,
    tile_y: u32,
    tile_x: u32,
) -> Option<SpawnedWmoRoot> {
    let root_fdid = resolve_wmo_fdid(placement)?;
    let root_path = ensure_wmo_asset(root_fdid)?;
    let root_data = std::fs::read(&root_path).ok()?;
    let root = wmo::load_wmo_root(&root_data).ok()?;

    let group_fdids = resolve_wmo_group_fdids(root_fdid, root.n_groups);
    let transform = wmo_transform(placement, tile_y, tile_x);
    let portal_graph = build_portal_graph(&root);
    let root_entity = spawn_wmo_root_entity(
        commands,
        root_fdid,
        transform,
        portal_graph,
        root.skybox_wow_path.as_deref(),
    );

    let group_count = spawn_wmo_groups(
        commands,
        assets,
        &root,
        &group_fdids,
        root_fdid,
        root_entity,
    );
    log_wmo_spawn(root_fdid, group_count, &root, &transform);
    if group_count > 0 {
        let model = game_engine::listfile::lookup_fdid(root_fdid)
            .map(str::to_string)
            .unwrap_or_else(|| root_fdid.to_string());
        Some(SpawnedWmoRoot {
            entity: root_entity,
            model,
        })
    } else {
        None
    }
}

fn spawn_wmo_root_entity(
    commands: &mut Commands,
    root_fdid: u32,
    transform: Transform,
    portal_graph: game_engine::culling::WmoPortalGraph,
    skybox_wow_path: Option<&str>,
) -> Entity {
    let mut entity = commands.spawn((
        Name::new(format!("wmo_{root_fdid}")),
        transform,
        Visibility::default(),
        game_engine::culling::Wmo,
        portal_graph,
    ));
    if let Some(wow_path) = skybox_wow_path {
        entity.insert(WmoLocalSkybox {
            wow_path: wow_path.to_string(),
        });
    }
    entity.id()
}

/// Spawn all WMO groups as children. Returns count of successfully spawned groups.
fn spawn_wmo_groups(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    root: &wmo::WmoRootData,
    group_fdids: &[Option<u32>],
    root_fdid: u32,
    root_entity: Entity,
) -> u32 {
    let mut count = 0u32;
    for (i, group_fdid) in group_fdids.iter().enumerate() {
        let Some(fdid) = group_fdid else { continue };
        if spawn_wmo_group(commands, assets, root, *fdid, root_entity, i as u16) {
            count += 1;
        } else {
            eprintln!("  WMO {root_fdid} group {i}: missing or failed (FDID {fdid})");
        }
    }
    count
}

fn log_wmo_spawn(root_fdid: u32, group_count: u32, root: &wmo::WmoRootData, transform: &Transform) {
    let pos = transform.translation;
    eprintln!(
        "WMO {root_fdid}: {group_count}/{} groups, {} materials, pos=[{:.0}, {:.0}, {:.0}]",
        root.n_groups,
        root.materials.len(),
        pos.x,
        pos.y,
        pos.z,
    );
}

fn build_portal_graph(root: &wmo::WmoRootData) -> game_engine::culling::WmoPortalGraph {
    let mut adjacency = vec![Vec::new(); root.n_groups as usize];
    let mut refs_by_portal = vec![Vec::new(); root.portals.len()];
    for portal_ref in &root.portal_refs {
        if let Some(group_refs) = refs_by_portal.get_mut(portal_ref.portal_index as usize) {
            group_refs.push(portal_ref.group_index);
        }
    }

    for (portal_idx, groups) in refs_by_portal.iter().enumerate() {
        if groups.len() < 2 {
            continue;
        }
        for &src in groups {
            if let Some(neighbors) = adjacency.get_mut(src as usize) {
                for &dst in groups {
                    if src != dst {
                        neighbors.push((portal_idx, dst));
                    }
                }
            }
        }
    }

    let portal_verts = root
        .portals
        .iter()
        .map(|portal| {
            portal
                .vertices
                .iter()
                .map(|vertex| {
                    let [x, y, z] = *vertex;
                    Vec3::from(crate::asset::wmo::wmo_local_to_bevy(x, y, z))
                })
                .collect()
        })
        .collect();

    game_engine::culling::WmoPortalGraph {
        adjacency,
        portal_verts,
    }
}

/// Resolve a WMO placement to its root FileDataID.
fn resolve_wmo_fdid(wmo: &adt_obj::WmoPlacement) -> Option<u32> {
    if let Some(fdid) = wmo.fdid {
        return Some(fdid);
    }
    let wow_path = wmo.path.as_ref()?;
    game_engine::listfile::lookup_path(wow_path)
}

/// Resolve group file FDIDs from root FDID.
fn resolve_wmo_group_fdids(root_fdid: u32, n_groups: u32) -> Vec<Option<u32>> {
    let Some(root_path) = game_engine::listfile::lookup_fdid(root_fdid) else {
        eprintln!("  WMO {root_fdid}: not in listfile, cannot resolve group FDIDs");
        return vec![None; n_groups as usize];
    };
    let base = root_path.trim_end_matches(".wmo");
    (0..n_groups)
        .map(|i| {
            let group_path = format!("{base}_{i:03}.wmo");
            game_engine::listfile::lookup_path(&group_path)
        })
        .collect()
}

fn ensure_wmo_asset(fdid: u32) -> Option<std::path::PathBuf> {
    let out_path = std::path::PathBuf::from(format!("data/models/{fdid}.wmo"));
    crate::asset::asset_cache::file_at_path(fdid, &out_path)
}

/// Parse and spawn one WMO group file as children of the root entity.
/// Creates a group entity with `WmoGroup` for portal culling, then parents batches under it.
fn spawn_wmo_group(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    root: &wmo::WmoRootData,
    group_fdid: u32,
    root_entity: Entity,
    group_index: u16,
) -> bool {
    let Some(group_path) = ensure_wmo_asset(group_fdid) else {
        return false;
    };
    let Ok(data) = std::fs::read(&group_path) else {
        return false;
    };
    let Ok(group) = wmo::load_wmo_group(&data) else {
        return false;
    };

    let bbox = group_bbox(root, group_index);
    let group_entity = commands
        .spawn((
            Name::new(format!("wmo_group_{group_index}")),
            Transform::default(),
            Visibility::default(),
            bbox,
        ))
        .id();
    commands.entity(root_entity).add_child(group_entity);

    for batch in group.batches {
        let mat = wmo_batch_material(
            assets.materials,
            assets.images,
            root,
            batch.material_index,
            batch.has_vertex_color,
        );
        let child = commands
            .spawn((
                Mesh3d(assets.meshes.add(batch.mesh)),
                MeshMaterial3d(mat),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        commands.entity(group_entity).add_child(child);
    }
    true
}

/// Build a `WmoGroup` component from MOGI bounding box data.
fn group_bbox(root: &wmo::WmoRootData, group_index: u16) -> game_engine::culling::WmoGroup {
    let (bbox_min, bbox_max) = root
        .group_infos
        .get(group_index as usize)
        .map(|info| {
            let min = crate::asset::wmo::wmo_local_to_bevy(
                info.bbox_min[0],
                info.bbox_min[1],
                info.bbox_min[2],
            );
            let max = crate::asset::wmo::wmo_local_to_bevy(
                info.bbox_max[0],
                info.bbox_max[1],
                info.bbox_max[2],
            );
            // wow_to_bevy can flip min/max, so re-sort per axis
            (
                Vec3::new(min[0].min(max[0]), min[1].min(max[1]), min[2].min(max[2])),
                Vec3::new(min[0].max(max[0]), min[1].max(max[1]), min[2].max(max[2])),
            )
        })
        .unwrap_or((Vec3::splat(f32::MIN), Vec3::splat(f32::MAX)));
    game_engine::culling::WmoGroup {
        group_index,
        bbox_min,
        bbox_max,
    }
}

/// Build a Bevy material for a WMO batch.
fn wmo_batch_material(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: &wmo::WmoRootData,
    material_index: u16,
    has_vertex_color: bool,
) -> Handle<StandardMaterial> {
    let mat_def = root.materials.get(material_index as usize);
    let texture_fdid = mat_def.map(|m| m.texture_fdid).unwrap_or(0);
    let texture_2_fdid = mat_def.map(|m| m.texture_2_fdid).unwrap_or(0);
    let texture_3_fdid = mat_def.map(|m| m.texture_3_fdid).unwrap_or(0);
    let blend_mode = mat_def.map(|m| m.blend_mode).unwrap_or(0);
    let flags = mat_def.map(|m| m.flags).unwrap_or(0);
    let shader = mat_def.map(|m| m.shader).unwrap_or(0);

    if texture_fdid > 0 {
        match crate::asset::asset_cache::texture(texture_fdid) {
            Some(blp_path) => {
                match load_wmo_material_image(&blp_path, texture_2_fdid, texture_3_fdid, images) {
                    Ok(image) => {
                        return materials.add(wmo_standard_material(
                            Some(image),
                            blend_mode,
                            flags,
                            has_vertex_color,
                        ));
                    }
                    Err(err) => {
                        eprintln!(
                            "WMO texture decode failed for material {material_index} shader {shader} FDID {texture_fdid}: {err}"
                        );
                    }
                }
            }
            None => {
                let label = game_engine::listfile::lookup_fdid(texture_fdid).unwrap_or("unknown");
                eprintln!("WMO texture extract failed for FDID {texture_fdid}: {label}");
            }
        }
    }
    materials.add(wmo_standard_material(
        None,
        blend_mode,
        flags,
        has_vertex_color,
    ))
}

fn load_wmo_material_image(
    base_path: &std::path::Path,
    texture_2_fdid: u32,
    texture_3_fdid: u32,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let key = WmoTextureCacheKey {
        base_path: base_path.to_path_buf(),
        texture_2_fdid,
        texture_3_fdid,
    };
    let cache = WMO_TEXTURE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(cached) = cache.lock().unwrap().get(&key).cloned() {
        return cached;
    }
    let (mut pixels, w, h) = blp::load_blp_rgba(base_path)?;
    for overlay_fdid in [texture_2_fdid, texture_3_fdid] {
        if overlay_fdid == 0 {
            continue;
        }
        let Some(overlay_path) = crate::asset::asset_cache::texture(overlay_fdid) else {
            continue;
        };
        let Ok((overlay_pixels, ov_w, ov_h)) = blp::load_blp_rgba(&overlay_path) else {
            continue;
        };
        if ov_w == w && ov_h == h {
            blp::blit_region(&mut pixels, w, &overlay_pixels, ov_w, ov_h, 0, 0);
        }
    }

    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        pixels,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    let handle = images.add(image);
    cache.lock().unwrap().insert(key, Ok(handle.clone()));
    Ok(handle)
}

fn wmo_standard_material(
    texture: Option<Handle<Image>>,
    blend_mode: u32,
    flags: u32,
    has_vertex_color: bool,
) -> StandardMaterial {
    let alpha_mode = match blend_mode {
        2 | 3 => AlphaMode::Blend,
        _ if texture.is_some() => AlphaMode::Mask(0.5),
        _ => AlphaMode::Opaque,
    };
    let double_sided = (flags & 0x04) != 0;
    let prop_like_surface = double_sided || !matches!(alpha_mode, AlphaMode::Opaque);
    StandardMaterial {
        base_color: if texture.is_none() {
            Color::srgb(0.6, 0.6, 0.6)
        } else {
            Color::WHITE
        },
        base_color_texture: texture,
        perceptual_roughness: if prop_like_surface { 0.97 } else { 0.88 },
        reflectance: if prop_like_surface { 0.02 } else { 0.18 },
        unlit: has_vertex_color,
        double_sided,
        cull_mode: if double_sided {
            None
        } else {
            Some(bevy::render::render_resource::Face::Back)
        },
        alpha_mode,
        ..default()
    }
}

// ── coordinate conversion ───────────────────────────────────────────────────

/// Convert MODF/MDDF placement position to Bevy-space using the legacy
/// absolute-world ADT convention.
fn placement_to_bevy_absolute(raw: [f32; 3]) -> [f32; 3] {
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

fn wmo_position(w: &adt_obj::WmoPlacement, tile_y: u32, tile_x: u32) -> Vec3 {
    Vec3::from(placement_to_bevy_on_tile(w.position, tile_y, tile_x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_rotation_matches_current_model_rotation_formula() {
        let rot = [17.0, 123.0, -31.0];
        let actual = placement_rotation(rot);
        let expected = Quat::from_euler(
            EulerRot::YZX,
            (rot[1] - 180.0).to_radians(),
            (-rot[0]).to_radians(),
            rot[2].to_radians(),
        );

        let probe = Vec3::new(0.3, -0.4, 0.8);
        let actual_vec = actual * probe;
        let expected_vec = expected * probe;
        assert!(
            actual_vec.abs_diff_eq(expected_vec, 1e-5),
            "rotation mismatch: actual={actual_vec:?} expected={expected_vec:?}"
        );
    }

    #[test]
    fn placement_rotation_zero_matches_current_yaw_correction() {
        let rotation = placement_rotation([0.0, 0.0, 0.0]);
        let probe = Vec3::X;
        let rotated = rotation * probe;
        let expected = -Vec3::X;
        assert!(
            rotated.abs_diff_eq(expected, 1e-5),
            "zero placement rotation should match the current Y-180 mapping: rotated={rotated:?} expected={expected:?}"
        );
    }

    #[test]
    fn load_map_fogs_wdt_reads_warband_companion_file() {
        let fogs = load_map_fogs_wdt("2703").expect("expected 2703_fogs.wdt");

        assert_eq!(fogs.version, 2);
        assert_eq!(fogs.volumes.len(), 1);
        assert_eq!(fogs.volumes[0].model_fdid, 1_728_356);
        assert_eq!(fogs.volumes[0].fog_id, 1_725);
    }

    #[test]
    fn placement_to_bevy_maps_absolute_wow_world_positions_into_loaded_adt_space() {
        let raw = [17282.818, 80.921, 25931.766];
        let actual = placement_to_bevy_absolute(raw);
        let (tile_y, tile_x) = crate::terrain_tile::bevy_to_tile_coords(actual[0], actual[2]);
        assert_eq!((tile_y, tile_x), (32, 48));
        assert!(
            (actual[0] + 8865.1).abs() < 1.0,
            "expected centered Bevy X near player space, got {}",
            actual[0]
        );
        assert!(
            (actual[2] - 216.2).abs() < 1.0,
            "expected centered Bevy Z near player space, got {}",
            actual[2]
        );
    }

    #[test]
    fn placement_to_bevy_falls_back_to_local_wow_coords_when_absolute_result_misses_tile() {
        let raw = [-2982.99, 455.52, 468.06];
        let actual = placement_to_bevy_on_tile(raw, 31, 37);
        assert!(
            (actual[0] + 2982.99).abs() < 0.1,
            "expected local Bevy X near scene camera, got {}",
            actual[0]
        );
        assert!(
            (actual[1] - 455.52).abs() < 0.1,
            "expected local Bevy Y near scene camera, got {}",
            actual[1]
        );
        assert!(
            (actual[2] + 468.06).abs() < 0.1,
            "expected local Bevy Z near scene camera, got {}",
            actual[2]
        );
    }

    #[test]
    fn doodad_transform_lifts_props_to_terrain_height() {
        let data = std::fs::read("data/terrain/azeroth_32_48.adt")
            .expect("expected test ADT data/terrain/azeroth_32_48.adt");
        let adt =
            crate::asset::adt::load_adt_for_tile(&data, 32, 48).expect("expected ADT to parse");
        let mut heightmap = crate::terrain_heightmap::TerrainHeightmap::default();
        heightmap.insert_tile(32, 48, &adt);

        let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
        let terrain_y = heightmap
            .height_at(bx, bz)
            .expect("expected terrain at sample position");
        let center = 32.0 * TILE_SIZE;
        let doodad = adt_obj::DoodadPlacement {
            name_id: 0,
            unique_id: 0,
            position: [bz + center, terrain_y - 5.0, center - bx],
            rotation: [0.0, 0.0, 0.0],
            scale: 1.0,
            flags: 0,
            fdid: None,
            path: Some("test.m2".to_string()),
        };

        let transform = doodad_transform(&doodad, Some(&heightmap), 32, 48);

        assert!(
            (transform.translation.y - terrain_y).abs() < 0.001,
            "doodad should snap up to terrain, got doodad_y={} terrain_y={terrain_y}",
            transform.translation.y
        );
    }

    #[test]
    fn wmo_vertex_colored_materials_are_unlit() {
        let material = wmo_standard_material(None, 0, 0, true);
        assert!(material.unlit);
    }

    #[test]
    fn waterfall_backdrop_filter_keeps_only_waterfall_effects() {
        let waterfall = adt_obj::DoodadPlacement {
            name_id: 0,
            unique_id: 0,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: 1.0,
            flags: 0,
            fdid: None,
            path: Some("world/expansion09/doodads/exterior/10xp_waterfall04.m2".to_string()),
        };
        let campfire = adt_obj::DoodadPlacement {
            name_id: 0,
            unique_id: 0,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: 1.0,
            flags: 0,
            fdid: None,
            path: Some("world/expansion09/doodads/centaur/10ct_centaur_campfire01.m2".to_string()),
        };

        assert!(is_waterfall_backdrop_doodad(&waterfall));
        assert!(!is_waterfall_backdrop_doodad(&campfire));
    }

    #[test]
    fn charselect_filter_drops_ground_clutter_but_keeps_props() {
        let clutter = adt_obj::DoodadPlacement {
            name_id: 0,
            unique_id: 0,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: 1.0,
            flags: 0,
            fdid: None,
            path: Some("world/expansion09/doodads/highlands/10hgl_pineneedles_a02.m2".to_string()),
        };
        let prop = adt_obj::DoodadPlacement {
            name_id: 0,
            unique_id: 0,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: 1.0,
            flags: 0,
            fdid: None,
            path: Some("world/expansion09/doodads/centaur/10ct_centaur_campfire01.m2".to_string()),
        };

        assert!(is_charselect_clutter_doodad(&clutter));
        assert!(!is_charselect_clutter_doodad(&prop));
    }

    #[test]
    #[ignore]
    fn dump_charselect_nearby_doodads() {
        let adt_path = Path::new("data/terrain/2703_31_37.adt");
        let obj = load_obj0(adt_path).expect("obj0");
        let char_pos = Vec3::new(-2981.8, 452.9, -457.4);
        let camera_pos = Vec3::new(-2980.6, 455.1, -463.3);
        let view_dir = (char_pos - camera_pos).normalize();

        let mut nearest: Vec<_> = obj
            .doodads
            .iter()
            .map(|d| {
                let pos = Vec3::from(placement_to_bevy_on_tile(d.position, 31, 37));
                let to_char = pos.distance(char_pos);
                let to_camera = pos.distance(camera_pos);
                let delta = pos - camera_pos;
                let depth = delta.dot(view_dir);
                let ray_dist = (delta - view_dir * depth).length();
                let fdid = d.fdid.or_else(|| {
                    d.path
                        .as_deref()
                        .and_then(game_engine::listfile::lookup_path)
                });
                let model = fdid
                    .and_then(game_engine::listfile::lookup_fdid)
                    .map(str::to_string)
                    .or_else(|| d.path.clone())
                    .unwrap_or_else(|| "<unknown>".to_string());
                (
                    to_char,
                    to_camera,
                    depth,
                    ray_dist,
                    pos,
                    fdid,
                    d.unique_id,
                    model,
                )
            })
            .collect();

        nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
        println!("Nearest doodads to charselect character:");
        for (dist_char, dist_cam, depth, ray_dist, pos, fdid, unique_id, model) in
            nearest.iter().take(40)
        {
            println!(
                "  d_char={dist_char:6.1} d_cam={dist_cam:6.1} depth={depth:6.1} ray={ray_dist:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} {}",
                pos.x, pos.y, pos.z, unique_id, fdid, model
            );
        }

        nearest.retain(|(_, _, depth, ray_dist, ..)| *depth > 0.0 && *ray_dist < 25.0);
        nearest.sort_by(|a, b| a.3.total_cmp(&b.3).then_with(|| a.2.total_cmp(&b.2)));
        println!("\nDoodads near the camera view ray:");
        for (dist_char, dist_cam, depth, ray_dist, pos, fdid, unique_id, model) in
            nearest.into_iter().take(60)
        {
            println!(
                "  ray={ray_dist:6.1} depth={depth:6.1} d_char={dist_char:6.1} d_cam={dist_cam:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} {}",
                pos.x, pos.y, pos.z, unique_id, fdid, model
            );
        }

        let mut effects: Vec<_> = obj
            .doodads
            .iter()
            .filter_map(|d| {
                let fdid = d.fdid.or_else(|| {
                    d.path
                        .as_deref()
                        .and_then(game_engine::listfile::lookup_path)
                });
                let model = fdid
                    .and_then(game_engine::listfile::lookup_fdid)
                    .map(str::to_string)
                    .or_else(|| d.path.clone())?;
                let lower = model.to_ascii_lowercase();
                let interesting = ["water", "fall", "mist", "smoke", "fx", "ripple", "foam"]
                    .iter()
                    .any(|needle| lower.contains(needle));
                if !interesting {
                    return None;
                }
                let pos = Vec3::from(placement_to_bevy_on_tile(d.position, 31, 37));
                Some((
                    pos.distance(char_pos),
                    pos.distance(camera_pos),
                    pos,
                    fdid,
                    d.unique_id,
                    model,
                ))
            })
            .collect();
        effects.sort_by(|a, b| a.0.total_cmp(&b.0));
        println!("\nInteresting doodad effects on this tile:");
        for (dist_char, dist_cam, pos, fdid, unique_id, model) in effects.into_iter().take(80) {
            println!(
                "  d_char={dist_char:6.1} d_cam={dist_cam:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} {}",
                pos.x, pos.y, pos.z, unique_id, fdid, model
            );
        }
    }

    #[test]
    #[ignore]
    fn dump_charselect_nearby_wmos() {
        let adt_path = Path::new("data/terrain/2703_31_37.adt");
        let obj = load_obj0(adt_path).expect("obj0");
        let char_pos = Vec3::new(-2981.8, 452.9, -457.4);
        let camera_pos = Vec3::new(-2980.6, 455.1, -463.3);
        let view_dir = (char_pos - camera_pos).normalize();

        let mut nearest: Vec<_> = obj
            .wmos
            .iter()
            .map(|w| {
                let pos = Vec3::from(placement_to_bevy_on_tile(w.position, 31, 37));
                let to_char = pos.distance(char_pos);
                let to_camera = pos.distance(camera_pos);
                let delta = pos - camera_pos;
                let depth = delta.dot(view_dir);
                let ray_dist = (delta - view_dir * depth).length();
                let fdid = w.fdid.or_else(|| {
                    w.path
                        .as_deref()
                        .and_then(game_engine::listfile::lookup_path)
                });
                let model = fdid
                    .and_then(game_engine::listfile::lookup_fdid)
                    .map(str::to_string)
                    .or_else(|| w.path.clone())
                    .unwrap_or_else(|| "<unknown>".to_string());
                (
                    to_char,
                    to_camera,
                    depth,
                    ray_dist,
                    pos,
                    fdid,
                    w.unique_id,
                    w.rotation,
                    model,
                )
            })
            .collect();

        nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
        println!("Nearest WMOs to charselect character:");
        for (dist_char, dist_cam, depth, ray_dist, pos, fdid, unique_id, rotation, model) in
            nearest.iter().take(40)
        {
            println!(
                "  d_char={dist_char:6.1} d_cam={dist_cam:6.1} depth={depth:6.1} ray={ray_dist:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} rot={rotation:?} {}",
                pos.x, pos.y, pos.z, unique_id, fdid, model
            );
        }
    }

    #[test]
    #[ignore]
    fn dump_charselect_neighbor_tile_objects() {
        let char_pos = Vec3::new(-2981.8, 452.9, -457.4);
        for (tile_y, tile_x) in [(31, 36), (31, 37)] {
            let adt_path_string = format!("data/terrain/2703_{tile_y}_{tile_x}.adt");
            let adt_path = Path::new(&adt_path_string);
            let Some(obj) = load_obj0(adt_path) else {
                println!("missing obj0 for tile ({tile_y}, {tile_x})");
                continue;
            };
            println!("\nTile ({tile_y}, {tile_x}) doodads near campsite:");
            let mut doodads: Vec<_> = obj
                .doodads
                .iter()
                .filter_map(|d| {
                    let pos = Vec3::from(placement_to_bevy_on_tile(d.position, tile_y, tile_x));
                    let dist = pos.distance(char_pos);
                    if dist > 80.0 {
                        return None;
                    }
                    let fdid = d.fdid.or_else(|| {
                        d.path
                            .as_deref()
                            .and_then(game_engine::listfile::lookup_path)
                    });
                    let model = fdid
                        .and_then(game_engine::listfile::lookup_fdid)
                        .map(str::to_string)
                        .or_else(|| d.path.clone())
                        .unwrap_or_else(|| "<unknown>".to_string());
                    Some((dist, pos, fdid, d.unique_id, d.rotation, model))
                })
                .collect();
            doodads.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (dist, pos, fdid, uid, rotation, model) in doodads.into_iter().take(80) {
                println!(
                    "  d={dist:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} rot={rotation:?} {}",
                    pos.x, pos.y, pos.z, uid, fdid, model
                );
            }

            println!("\nTile ({tile_y}, {tile_x}) WMOs near campsite:");
            let mut wmos: Vec<_> = obj
                .wmos
                .iter()
                .filter_map(|w| {
                    let pos = Vec3::from(placement_to_bevy_on_tile(w.position, tile_y, tile_x));
                    let dist = pos.distance(char_pos);
                    if dist > 200.0 {
                        return None;
                    }
                    let fdid = w.fdid.or_else(|| {
                        w.path
                            .as_deref()
                            .and_then(game_engine::listfile::lookup_path)
                    });
                    let model = fdid
                        .and_then(game_engine::listfile::lookup_fdid)
                        .map(str::to_string)
                        .or_else(|| w.path.clone())
                        .unwrap_or_else(|| "<unknown>".to_string());
                    Some((dist, pos, fdid, w.unique_id, w.rotation, model))
                })
                .collect();
            wmos.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (dist, pos, fdid, uid, rotation, model) in wmos.into_iter().take(80) {
                println!(
                    "  d={dist:6.1} pos=({:.1}, {:.1}, {:.1}) uid={} fdid={:?} rot={rotation:?} {}",
                    pos.x, pos.y, pos.z, uid, fdid, model
                );
            }
        }
    }

    #[test]
    #[ignore]
    fn compare_wmo_swizzles_against_modf_extents() {
        #[derive(Clone, Copy)]
        struct RawModfEntry {
            unique_id: u32,
            fdid: Option<u32>,
            position: [f32; 3],
            rotation: [f32; 3],
            extents_min: [f32; 3],
            extents_max: [f32; 3],
            scale: f32,
        }

        fn parse_raw_modf_entries(path: &Path) -> Vec<RawModfEntry> {
            let data = std::fs::read(path).expect("obj0");
            let mut modf = None;
            for chunk in crate::asset::adt::ChunkIter::new(&data) {
                let (tag, payload) = chunk.expect("chunk");
                if tag == b"FDOM" {
                    modf = Some(payload.to_vec());
                    break;
                }
            }
            let payload = modf.expect("modf");
            let count = payload.len() / 64;
            (0..count)
                .map(|i| {
                    let base = i * 64;
                    let name_id = u32::from_le_bytes(payload[base..base + 4].try_into().unwrap());
                    let unique_id =
                        u32::from_le_bytes(payload[base + 4..base + 8].try_into().unwrap());
                    let read_f32 = |off: usize| {
                        f32::from_le_bytes(payload[base + off..base + off + 4].try_into().unwrap())
                    };
                    let position = [read_f32(8), read_f32(12), read_f32(16)];
                    let rotation = [read_f32(20), read_f32(24), read_f32(28)];
                    let extents_min = [read_f32(32), read_f32(36), read_f32(40)];
                    let extents_max = [read_f32(44), read_f32(48), read_f32(52)];
                    let flags =
                        u16::from_le_bytes(payload[base + 56..base + 58].try_into().unwrap());
                    let scale_raw =
                        u16::from_le_bytes(payload[base + 62..base + 64].try_into().unwrap());
                    let scale = if (flags & 0x4) != 0 {
                        scale_raw as f32 / 1024.0
                    } else {
                        1.0
                    };
                    let fdid = if (flags & 0x8) != 0 {
                        Some(name_id)
                    } else {
                        None
                    };
                    RawModfEntry {
                        unique_id,
                        fdid,
                        position,
                        rotation,
                        extents_min,
                        extents_max,
                        scale,
                    }
                })
                .collect()
        }

        fn sort_bbox(min: [f32; 3], max: [f32; 3]) -> (Vec3, Vec3) {
            (
                Vec3::new(min[0].min(max[0]), min[1].min(max[1]), min[2].min(max[2])),
                Vec3::new(min[0].max(max[0]), min[1].max(max[1]), min[2].max(max[2])),
            )
        }

        fn wow_bbox_to_bevy(min: [f32; 3], max: [f32; 3]) -> (Vec3, Vec3) {
            let min = placement_to_bevy_absolute(min);
            let max = placement_to_bevy_absolute(max);
            sort_bbox(min, max)
        }

        fn corners(min: [f32; 3], max: [f32; 3]) -> [[f32; 3]; 8] {
            [
                [min[0], min[1], min[2]],
                [min[0], min[1], max[2]],
                [min[0], max[1], min[2]],
                [min[0], max[1], max[2]],
                [max[0], min[1], min[2]],
                [max[0], min[1], max[2]],
                [max[0], max[1], min[2]],
                [max[0], max[1], max[2]],
            ]
        }

        fn swizzle_current(x: f32, y: f32, z: f32) -> [f32; 3] {
            crate::asset::wmo::wmo_local_to_bevy(x, y, z)
        }

        fn swizzle_like_m2(x: f32, y: f32, z: f32) -> [f32; 3] {
            crate::asset::m2::wow_to_bevy(x, y, z)
        }

        fn fitted_bbox(
            root: &crate::asset::wmo::WmoRootData,
            transform: Transform,
            swizzle: fn(f32, f32, f32) -> [f32; 3],
        ) -> (Vec3, Vec3) {
            let mut mins = Vec3::splat(f32::INFINITY);
            let mut maxs = Vec3::splat(f32::NEG_INFINITY);
            for info in &root.group_infos {
                for corner in corners(info.bbox_min, info.bbox_max) {
                    let local = Vec3::from(swizzle(corner[0], corner[1], corner[2]));
                    let world = transform.transform_point(local);
                    mins = mins.min(world);
                    maxs = maxs.max(world);
                }
            }
            (mins, maxs)
        }

        let path = Path::new("data/terrain/2703_31_37_obj0.adt");
        let raw_entries = parse_raw_modf_entries(path);
        for raw in raw_entries
            .into_iter()
            .filter(|entry| matches!(entry.fdid, Some(4214993 | 3803037)))
        {
            let root_fdid = raw.fdid.expect("fdid");
            let root_path = ensure_wmo_asset(root_fdid).expect("wmo");
            let root_data = std::fs::read(&root_path).expect("wmo root data");
            let root = crate::asset::wmo::load_wmo_root(&root_data).expect("wmo root");
            let pos = Vec3::from(placement_to_bevy_on_tile(raw.position, 31, 37));
            let rotation = placement_rotation(raw.rotation);
            let transform = Transform::from_translation(pos)
                .with_rotation(rotation)
                .with_scale(Vec3::splat(raw.scale));

            let (expected_min, expected_max) = wow_bbox_to_bevy(raw.extents_min, raw.extents_max);
            let (current_min, current_max) = fitted_bbox(&root, transform, swizzle_current);
            let (m2_min, m2_max) = fitted_bbox(&root, transform, swizzle_like_m2);

            let current_err =
                current_min.distance(expected_min) + current_max.distance(expected_max);
            let m2_err = m2_min.distance(expected_min) + m2_max.distance(expected_max);

            println!(
                "uid={} fdid={} current_err={:.3} m2_err={:.3}\n  expected min={:?} max={:?}\n  current  min={:?} max={:?}\n  m2_like  min={:?} max={:?}",
                raw.unique_id,
                root_fdid,
                current_err,
                m2_err,
                expected_min,
                expected_max,
                current_min,
                current_max,
                m2_min,
                m2_max
            );
        }
    }
}
