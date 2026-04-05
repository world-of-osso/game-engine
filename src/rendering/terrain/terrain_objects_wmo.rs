use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use bevy::asset::RenderAssetUsages;
use bevy::color::LinearRgba;
use bevy::ecs::query::QueryFilter;
use bevy::image::Image;
use bevy::mesh::Indices;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

use crate::asset::{adt_format::adt_obj, blp, wmo};
use crate::collision::WmoCollisionMesh;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_spawn;
use crate::rendering::sky::GameTime;
use crate::sound_footsteps::{FootstepSurface, classify_surface_from_texture_path};
use crate::water_material::{self, WaterMaterial, WaterSettings};

use super::{
    SpawnedWmoRoot, WmoLocalSkybox, placement_to_bevy_absolute,
    terrain_objects_wmo_material::{
        composite_wmo_shader_layer, describe_wmo_shader, wmo_surface_params,
    },
    wmo_transform, wow_quat_to_bevy,
};

#[path = "terrain_objects_wmo_group.rs"]
mod terrain_objects_wmo_group;
#[path = "terrain_objects_wmo_surface.rs"]
mod terrain_objects_wmo_surface;
#[cfg(test)]
#[path = "terrain_objects_wmo_tests.rs"]
mod tests;

use self::terrain_objects_wmo_group::*;
use self::terrain_objects_wmo_surface::*;
pub(crate) use self::terrain_objects_wmo_surface::{sync_wmo_sidn_emissive, wmo_standard_material};

pub(crate) fn sidn_glow_strength(minutes: f32) -> f32 {
    terrain_objects_wmo_surface::sidn_glow_strength(minutes)
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WmoAdtMetadata {
    pub unique_id: u32,
    pub doodad_set: u16,
    pub name_set: u16,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct WmoTextureCacheKey {
    base_path: PathBuf,
    shader: u32,
    texture_2_fdid: u32,
    texture_3_fdid: u32,
}

static WMO_TEXTURE_CACHE: OnceLock<
    Mutex<std::collections::HashMap<WmoTextureCacheKey, Result<Handle<Image>, String>>>,
> = OnceLock::new();

struct WmoAssets<'a> {
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    water_materials: &'a mut Assets<WaterMaterial>,
    images: &'a mut Assets<Image>,
    effect_materials: &'a mut Assets<M2EffectMaterial>,
    inverse_bindposes: &'a mut Assets<SkinnedMeshInverseBindposes>,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct WmoSidnGlow {
    pub base_sidn_color: [f32; 4],
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WmoFootstepSurface {
    pub surface: FootstepSurface,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WmoGroupFogVolume {
    pub fog_index: u8,
    pub smaller_radius: f32,
    pub larger_radius: f32,
    pub fog_end: f32,
    pub fog_start_multiplier: f32,
    pub color_1: [f32; 4],
    pub underwater_fog_end: f32,
    pub underwater_fog_start_multiplier: f32,
    pub color_2: [f32; 4],
}

pub(super) fn spawn_wmos_filtered(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    tile_y: u32,
    tile_x: u32,
    obj_data: &adt_obj::AdtObjData,
    chunk_refs: &[Vec<u16>],
    filter: impl Fn(&adt_obj::WmoPlacement) -> bool,
    entities: &mut Vec<SpawnedWmoRoot>,
) {
    let mut spawned_count = 0u32;
    for (index, placement) in obj_data.wmos.iter().enumerate() {
        if !filter(placement) {
            continue;
        }
        let mut assets = WmoAssets {
            meshes,
            materials,
            water_materials,
            images,
            effect_materials,
            inverse_bindposes,
        };
        if let Some(spawned_wmo) = try_spawn_wmo(
            commands,
            &mut assets,
            placement,
            chunk_refs.get(index).map(Vec::as_slice),
            tile_y,
            tile_x,
        ) {
            entities.push(spawned_wmo);
            spawned_count += 1;
        }
    }
    eprintln!("Spawned {spawned_count}/{} WMOs", obj_data.wmos.len());
}

fn try_spawn_wmo(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    placement: &adt_obj::WmoPlacement,
    chunk_refs: Option<&[u16]>,
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
        build_wmo_adt_metadata(placement),
        build_chunk_refs_component(chunk_refs),
        build_wmo_root_bounds(placement),
        build_wmo_footstep_surface(&root),
        root.skybox_wow_path.as_deref(),
    );

    let group_count = spawn_wmo_groups(
        commands,
        assets,
        &root,
        &group_fdids,
        root_fdid,
        root_entity,
        placement.doodad_set,
    );
    log_wmo_spawn(root_fdid, group_count, &root, &transform);
    build_spawned_wmo_root(root_fdid, root_entity, group_count, placement)
}

fn build_spawned_wmo_root(
    root_fdid: u32,
    root_entity: Entity,
    group_count: u32,
    placement: &adt_obj::WmoPlacement,
) -> Option<SpawnedWmoRoot> {
    (group_count > 0).then(|| {
        let model = game_engine::listfile::lookup_fdid(root_fdid)
            .map(str::to_string)
            .unwrap_or_else(|| root_fdid.to_string());
        SpawnedWmoRoot {
            entity: root_entity,
            model: wmo_debug_label(model, placement.name_set),
        }
    })
}

fn wmo_debug_label(model: String, name_set: u16) -> String {
    if name_set == 0 {
        return model;
    }
    format!("{model} nameSet={name_set}")
}

fn spawn_wmo_root_entity(
    commands: &mut Commands,
    root_fdid: u32,
    transform: Transform,
    portal_graph: game_engine::culling::WmoPortalGraph,
    placement: WmoAdtMetadata,
    chunk_refs: Option<game_engine::culling::ChunkRefs>,
    root_bounds: game_engine::culling::WmoRootBounds,
    footstep_surface: Option<WmoFootstepSurface>,
    skybox_wow_path: Option<&str>,
) -> Entity {
    let mut entity = commands.spawn((
        Name::new(format!("wmo_{root_fdid}")),
        transform,
        Visibility::default(),
        game_engine::culling::Wmo,
        root_bounds,
        portal_graph,
        placement,
    ));
    if let Some(chunk_refs) = chunk_refs {
        entity.insert(chunk_refs);
    }
    if let Some(footstep_surface) = footstep_surface {
        entity.insert(footstep_surface);
    }
    if let Some(wow_path) = skybox_wow_path {
        entity.insert(WmoLocalSkybox {
            wow_path: wow_path.to_string(),
        });
    }
    entity.id()
}

fn build_chunk_refs_component(
    chunk_refs: Option<&[u16]>,
) -> Option<game_engine::culling::ChunkRefs> {
    let chunk_indices = chunk_refs?;
    (!chunk_indices.is_empty()).then(|| game_engine::culling::ChunkRefs {
        chunk_indices: chunk_indices.to_vec(),
    })
}

fn build_wmo_footstep_surface(root: &wmo::WmoRootData) -> Option<WmoFootstepSurface> {
    root.materials
        .iter()
        .filter_map(material_footstep_surface)
        .max_by_key(|candidate| candidate.priority)
        .map(|candidate| WmoFootstepSurface {
            surface: candidate.surface,
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WmoFootstepSurfaceCandidate {
    priority: (bool, bool, u32),
    surface: FootstepSurface,
}

fn material_footstep_surface(mat_def: &wmo::WmoMaterialDef) -> Option<WmoFootstepSurfaceCandidate> {
    let path = game_engine::listfile::lookup_fdid(mat_def.texture_fdid)?;
    Some(WmoFootstepSurfaceCandidate {
        priority: (
            mat_def.ground_type != 0,
            mat_def.diff_color[3] > 0.0,
            mat_def.texture_fdid,
        ),
        surface: classify_surface_from_texture_path(path),
    })
}

fn build_wmo_adt_metadata(placement: &adt_obj::WmoPlacement) -> WmoAdtMetadata {
    WmoAdtMetadata {
        unique_id: placement.unique_id,
        doodad_set: placement.doodad_set,
        name_set: placement.name_set,
    }
}

fn build_wmo_root_bounds(placement: &adt_obj::WmoPlacement) -> game_engine::culling::WmoRootBounds {
    let min = Vec3::from(placement_to_bevy_absolute(placement.extents_min));
    let max = Vec3::from(placement_to_bevy_absolute(placement.extents_max));
    game_engine::culling::WmoRootBounds {
        world_min: min.min(max),
        world_max: min.max(max),
    }
}

fn spawn_wmo_groups(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    root: &wmo::WmoRootData,
    group_fdids: &[Option<u32>],
    root_fdid: u32,
    root_entity: Entity,
    active_doodad_set: u16,
) -> u32 {
    let mut count = 0u32;
    for (i, group_fdid) in group_fdids.iter().enumerate() {
        let Some(fdid) = group_fdid else { continue };
        if spawn_wmo_group(
            commands,
            assets,
            root,
            *fdid,
            root_entity,
            i as u16,
            active_doodad_set,
        ) {
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
    let refs_by_portal = collect_portal_group_refs(root);
    let adjacency = build_portal_adjacency(root.n_groups, &refs_by_portal);
    let portal_verts = collect_portal_vertices(root);

    game_engine::culling::WmoPortalGraph {
        adjacency,
        portal_verts,
    }
}

fn collect_portal_group_refs(root: &wmo::WmoRootData) -> Vec<Vec<u16>> {
    let mut refs_by_portal = vec![Vec::new(); root.portals.len()];
    for portal_ref in &root.portal_refs {
        if let Some(group_refs) = refs_by_portal.get_mut(portal_ref.portal_index as usize) {
            group_refs.push(portal_ref.group_index);
        }
    }
    refs_by_portal
}

fn build_portal_adjacency(n_groups: u32, refs_by_portal: &[Vec<u16>]) -> Vec<Vec<(usize, u16)>> {
    let mut adjacency = vec![Vec::new(); n_groups as usize];
    for (portal_idx, groups) in refs_by_portal.iter().enumerate() {
        if groups.len() < 2 {
            continue;
        }
        for &src in groups {
            if let Some(neighbors) = adjacency.get_mut(src as usize) {
                add_portal_neighbors(neighbors, portal_idx, groups, src);
            }
        }
    }
    adjacency
}

fn add_portal_neighbors(
    neighbors: &mut Vec<(usize, u16)>,
    portal_idx: usize,
    groups: &[u16],
    src: u16,
) {
    for &dst in groups {
        if src != dst {
            neighbors.push((portal_idx, dst));
        }
    }
}

fn collect_portal_vertices(root: &wmo::WmoRootData) -> Vec<Vec<Vec3>> {
    root.portals.iter().map(portal_vertices).collect()
}

fn portal_vertices(portal: &wmo::WmoPortal) -> Vec<Vec3> {
    portal
        .vertices
        .iter()
        .map(|vertex| {
            let [x, y, z] = *vertex;
            Vec3::from(crate::asset::wmo::wmo_local_to_bevy(x, y, z))
        })
        .collect()
}

fn resolve_wmo_fdid(wmo: &adt_obj::WmoPlacement) -> Option<u32> {
    if let Some(fdid) = wmo.fdid {
        return Some(fdid);
    }
    let wow_path = wmo.path.as_ref()?;
    game_engine::listfile::lookup_path(wow_path)
}

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

pub(super) fn ensure_wmo_asset(fdid: u32) -> Option<PathBuf> {
    let out_path = PathBuf::from(format!("data/models/{fdid}.wmo"));
    crate::asset::asset_cache::file_at_path(fdid, &out_path)
}

fn spawn_wmo_group(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    root: &wmo::WmoRootData,
    group_fdid: u32,
    root_entity: Entity,
    group_index: u16,
    active_doodad_set: u16,
) -> bool {
    let Some(group_path) = ensure_wmo_asset(group_fdid) else {
        return false;
    };
    let Ok(data) = std::fs::read(&group_path) else {
        return false;
    };
    let Ok(group) = wmo::load_wmo_group_with_root(&data, Some(root)) else {
        return false;
    };

    let bbox = group_bbox(root, group_index, &group.header);
    let group_entity = spawn_wmo_group_entity(commands, group_index, bbox);
    commands.entity(root_entity).add_child(group_entity);
    spawn_wmo_group_lights(commands, root, &group, group_entity);
    spawn_wmo_group_fogs(commands, root, &group, group_entity);
    spawn_wmo_group_doodads(
        commands,
        assets,
        root,
        &group,
        group_entity,
        active_doodad_set,
    );
    let interior_ambient = build_wmo_interior_ambient(root, &group);
    spawn_wmo_group_liquid(commands, assets, &group, group_entity);
    spawn_wmo_group_batches(
        commands,
        assets,
        root,
        interior_ambient,
        group_entity,
        group.batches,
    );
    true
}

fn spawn_wmo_group_entity(
    commands: &mut Commands,
    group_index: u16,
    bbox: game_engine::culling::WmoGroup,
) -> Entity {
    commands
        .spawn((
            Name::new(format!("wmo_group_{group_index}")),
            Transform::default(),
            Visibility::default(),
            bbox,
        ))
        .id()
}
