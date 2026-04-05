use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use bevy::asset::RenderAssetUsages;
use bevy::color::LinearRgba;
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

    let bbox = group_bbox(root, group_index);
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

fn spawn_wmo_group_batches(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    root: &wmo::WmoRootData,
    interior_ambient: Option<[f32; 4]>,
    group_entity: Entity,
    batches: Vec<wmo::WmoGroupBatch>,
) {
    for batch in batches {
        let material_props = wmo_material_props(root, batch.material_index);
        let mat = wmo_batch_material(
            assets.materials,
            assets.images,
            batch.material_index,
            &material_props,
            interior_ambient,
            batch.has_vertex_color,
        );
        let mut child = commands.spawn((
            Mesh3d(assets.meshes.add(batch.mesh)),
            MeshMaterial3d(mat),
            Transform::default(),
            Visibility::default(),
            WmoCollisionMesh,
        ));
        if let Some(glow) = material_props.sidn_glow {
            child.insert(glow);
        }
        let child = child.id();
        commands.entity(group_entity).add_child(child);
    }
}

fn spawn_wmo_group_lights(
    commands: &mut Commands,
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    group_entity: Entity,
) {
    for (light_index, light) in collect_group_lights(root, group) {
        let Some(light_entity) = spawn_wmo_group_light(commands, light_index, light) else {
            continue;
        };
        commands.entity(group_entity).add_child(light_entity);
    }
}

fn spawn_wmo_group_fogs(
    commands: &mut Commands,
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    group_entity: Entity,
) {
    for (fog_index, fog) in collect_group_fogs(root, group) {
        let fog_entity = spawn_wmo_group_fog(commands, fog_index, fog);
        commands.entity(group_entity).add_child(fog_entity);
    }
}

fn spawn_wmo_group_liquid(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    group: &wmo::WmoGroupData,
    group_entity: Entity,
) {
    let Some(liquid) = group.liquid.as_ref() else {
        return;
    };
    let mesh = build_wmo_liquid_mesh(liquid);
    let Some(material) = build_wmo_liquid_material(assets.water_materials, assets.images) else {
        return;
    };
    let liquid_entity = commands
        .spawn((
            Name::new("wmo_liquid"),
            Mesh3d(assets.meshes.add(mesh)),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(group_entity).add_child(liquid_entity);
}

fn build_wmo_liquid_material(
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
) -> Option<Handle<WaterMaterial>> {
    let normal_map = images.add(water_material::generate_water_normal_map());
    Some(water_materials.add(WaterMaterial {
        settings: WaterSettings::default(),
        normal_map,
    }))
}

const WMO_LIQUID_TILE_SIZE: f32 = 4.166_662_5;
const WMO_LIQUID_Z_OFFSET: f32 = -1.0;

fn build_wmo_liquid_mesh(liquid: &wmo::WmoLiquid) -> Mesh {
    let (positions, normals, uvs, colors, indices) = build_wmo_liquid_geometry(liquid);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

type WmoLiquidGeometry = (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Vec<[f32; 4]>,
    Vec<u32>,
);

fn build_wmo_liquid_geometry(liquid: &wmo::WmoLiquid) -> WmoLiquidGeometry {
    let width = liquid.header.x_tiles.max(0) as usize;
    let height = liquid.header.y_tiles.max(0) as usize;
    let mut positions = Vec::with_capacity(width * height * 4);
    let mut normals = Vec::with_capacity(width * height * 4);
    let mut uvs = Vec::with_capacity(width * height * 4);
    let mut colors = Vec::with_capacity(width * height * 4);
    let mut indices = Vec::with_capacity(width * height * 6);

    for row in 0..height {
        for col in 0..width {
            if !wmo_liquid_tile_exists(liquid, row, col) {
                continue;
            }
            let base = positions.len() as u32;
            for (dr, dc) in [(0usize, 0usize), (0, 1), (1, 0), (1, 1)] {
                let local = wmo_liquid_local_pos(liquid, row + dr, col + dc);
                positions.push(crate::asset::wmo::wmo_local_to_bevy(
                    local[0], local[1], local[2],
                ));
                normals.push([0.0, 1.0, 0.0]);
                uvs.push([
                    (col + dc) as f32 / width.max(1) as f32,
                    (row + dr) as f32 / height.max(1) as f32,
                ]);
                colors.push([1.0, 1.0, 1.0, 1.0]);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
        }
    }

    (positions, normals, uvs, colors, indices)
}

fn wmo_liquid_tile_exists(liquid: &wmo::WmoLiquid, row: usize, col: usize) -> bool {
    let width = liquid.header.x_tiles.max(0) as usize;
    let tile_index = row.saturating_mul(width).saturating_add(col);
    liquid
        .tiles
        .get(tile_index)
        .is_none_or(|tile| tile.liquid_type != 0x0F)
}

fn wmo_liquid_local_pos(liquid: &wmo::WmoLiquid, row: usize, col: usize) -> [f32; 3] {
    let base = liquid.header.position;
    let x = base[0] + col as f32 * WMO_LIQUID_TILE_SIZE;
    let y = base[1] + row as f32 * WMO_LIQUID_TILE_SIZE;
    let z = wmo_liquid_height(liquid, row, col) + WMO_LIQUID_Z_OFFSET;
    [x, y, z]
}

fn wmo_liquid_height(liquid: &wmo::WmoLiquid, row: usize, col: usize) -> f32 {
    let width = liquid.header.x_verts.max(0) as usize;
    let vertex_index = row.saturating_mul(width).saturating_add(col);
    liquid
        .vertices
        .get(vertex_index)
        .map(|vertex| vertex.height)
        .unwrap_or(liquid.header.position[2])
}

fn collect_group_lights<'a>(
    root: &'a wmo::WmoRootData,
    group: &wmo::WmoGroupData,
) -> Vec<(u16, &'a wmo::WmoLight)> {
    group
        .light_refs
        .iter()
        .filter_map(|&light_index| {
            root.lights
                .get(light_index as usize)
                .map(|light| (light_index, light))
        })
        .collect()
}

fn collect_group_fogs<'a>(
    root: &'a wmo::WmoRootData,
    group: &wmo::WmoGroupData,
) -> Vec<(u8, &'a wmo::WmoFog)> {
    let mut fogs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for fog_index in group.header.fog_ids {
        if !seen.insert(fog_index) {
            continue;
        }
        let Some(fog) = root.fogs.get(fog_index as usize) else {
            continue;
        };
        fogs.push((fog_index, fog));
    }
    fogs
}

fn spawn_wmo_group_light(
    commands: &mut Commands,
    light_index: u16,
    light: &wmo::WmoLight,
) -> Option<Entity> {
    match light.light_type {
        wmo::WmoLightType::Omni => Some(spawn_wmo_point_light(commands, light_index, light)),
        wmo::WmoLightType::Spot => Some(spawn_wmo_spot_light(commands, light_index, light)),
        wmo::WmoLightType::Directional | wmo::WmoLightType::Ambient => None,
    }
}

fn spawn_wmo_group_fog(commands: &mut Commands, fog_index: u8, fog: &wmo::WmoFog) -> Entity {
    commands
        .spawn((
            Name::new(format!("WmoFog{fog_index}")),
            wmo_fog_transform(fog),
            WmoGroupFogVolume {
                fog_index,
                smaller_radius: fog.smaller_radius,
                larger_radius: fog.larger_radius,
                fog_end: fog.fog_end,
                fog_start_multiplier: fog.fog_start_multiplier,
                color_1: fog.color_1,
                underwater_fog_end: fog.underwater_fog_end,
                underwater_fog_start_multiplier: fog.underwater_fog_start_multiplier,
                color_2: fog.color_2,
            },
            Visibility::default(),
        ))
        .id()
}

fn spawn_wmo_point_light(
    commands: &mut Commands,
    light_index: u16,
    light: &wmo::WmoLight,
) -> Entity {
    commands
        .spawn((
            Name::new(format!("WmoLight{light_index}")),
            wmo_light_transform(light),
            authored_wmo_point_light(light),
            Visibility::default(),
        ))
        .id()
}

fn spawn_wmo_spot_light(
    commands: &mut Commands,
    light_index: u16,
    light: &wmo::WmoLight,
) -> Entity {
    commands
        .spawn((
            Name::new(format!("WmoSpotLight{light_index}")),
            wmo_light_transform(light),
            authored_wmo_spot_light(light),
            Visibility::default(),
        ))
        .id()
}

fn wmo_light_transform(light: &wmo::WmoLight) -> Transform {
    let [x, y, z] = crate::asset::wmo::wmo_local_to_bevy(
        light.position[0],
        light.position[1],
        light.position[2],
    );
    Transform::from_translation(Vec3::new(x, y, z)).with_rotation(wow_quat_to_bevy(light.rotation))
}

fn wmo_fog_transform(fog: &wmo::WmoFog) -> Transform {
    let [x, y, z] =
        crate::asset::wmo::wmo_local_to_bevy(fog.position[0], fog.position[1], fog.position[2]);
    Transform::from_translation(Vec3::new(x, y, z)).with_scale(Vec3::splat(
        fog.larger_radius.max(fog.smaller_radius).max(1.0),
    ))
}

fn authored_wmo_point_light(light: &wmo::WmoLight) -> PointLight {
    PointLight {
        color: Color::linear_rgb(light.color[0], light.color[1], light.color[2]),
        intensity: wmo_light_intensity(light),
        range: wmo_light_range(light),
        radius: light.attenuation_start.min(light.attenuation_end),
        shadows_enabled: false,
        ..default()
    }
}

fn authored_wmo_spot_light(light: &wmo::WmoLight) -> SpotLight {
    SpotLight {
        color: Color::linear_rgb(light.color[0], light.color[1], light.color[2]),
        intensity: wmo_light_intensity(light),
        range: wmo_light_range(light),
        radius: light.attenuation_start.min(light.attenuation_end),
        inner_angle: std::f32::consts::FRAC_PI_6,
        outer_angle: std::f32::consts::FRAC_PI_3,
        shadows_enabled: false,
        ..default()
    }
}

fn wmo_light_intensity(light: &wmo::WmoLight) -> f32 {
    light.intensity.max(0.0)
}

fn wmo_light_range(light: &wmo::WmoLight) -> f32 {
    if light.use_attenuation {
        light.attenuation_end.max(light.attenuation_start)
    } else {
        light.attenuation_end.max(1.0)
    }
}

fn spawn_wmo_group_doodads(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    group_entity: Entity,
    active_doodad_set: u16,
) {
    for doodad in collect_group_doodads(root, group, active_doodad_set) {
        let Some(entity) = spawn_wmo_group_doodad(commands, assets, &doodad) else {
            continue;
        };
        commands.entity(group_entity).add_child(entity);
    }
}

#[derive(Clone, Debug, PartialEq)]
struct WmoGroupDoodad {
    model_path: String,
    transform: Transform,
}

fn collect_group_doodads(
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    active_doodad_set: u16,
) -> Vec<WmoGroupDoodad> {
    let active_indices = active_wmo_doodad_indices(root, active_doodad_set);
    group
        .doodad_refs
        .iter()
        .filter_map(|&doodad_index| {
            if !active_indices.contains(&doodad_index) {
                return None;
            }
            let doodad_def = root.doodad_defs.get(doodad_index as usize)?;
            let model_path = resolve_wmo_doodad_name_path(root, doodad_def.name_offset)?;
            Some(WmoGroupDoodad {
                model_path,
                transform: wmo_doodad_transform(doodad_def),
            })
        })
        .collect()
}

fn active_wmo_doodad_indices(
    root: &wmo::WmoRootData,
    active_doodad_set: u16,
) -> std::collections::HashSet<u16> {
    let mut indices = std::collections::HashSet::new();
    if root.doodad_sets.is_empty() {
        indices.extend((0..root.doodad_defs.len()).filter_map(|idx| u16::try_from(idx).ok()));
        return indices;
    }

    add_wmo_doodad_set_indices(&mut indices, root.doodad_sets.first());
    if active_doodad_set != 0 {
        add_wmo_doodad_set_indices(
            &mut indices,
            root.doodad_sets.get(active_doodad_set as usize),
        );
    }
    indices
}

fn add_wmo_doodad_set_indices(
    indices: &mut std::collections::HashSet<u16>,
    doodad_set: Option<&wmo::WmoDoodadSet>,
) {
    let Some(doodad_set) = doodad_set else { return };
    let start = doodad_set.start_doodad;
    let end = start.saturating_add(doodad_set.n_doodads);
    indices.extend((start..end).filter_map(|idx| u16::try_from(idx).ok()));
}

fn resolve_wmo_doodad_name_path(root: &wmo::WmoRootData, name_offset: u32) -> Option<String> {
    root.doodad_names
        .iter()
        .find(|name| name.offset == name_offset)
        .map(|name| name.name.clone())
        .or_else(|| {
            root.doodad_file_ids
                .get(name_offset as usize)
                .and_then(|fdid| game_engine::listfile::lookup_fdid(*fdid))
                .map(str::to_string)
        })
}

fn wmo_doodad_transform(doodad_def: &wmo::WmoDoodadDef) -> Transform {
    let [x, y, z] = crate::asset::wmo::wmo_local_to_bevy(
        doodad_def.position[0],
        doodad_def.position[1],
        doodad_def.position[2],
    );
    Transform::from_translation(Vec3::new(x, y, z))
        .with_rotation(wow_quat_to_bevy(doodad_def.rotation))
        .with_scale(Vec3::splat(doodad_def.scale))
}

fn spawn_wmo_group_doodad(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    doodad: &WmoGroupDoodad,
) -> Option<Entity> {
    let fdid = game_engine::listfile::lookup_path(&doodad.model_path)?;
    let model_path = crate::asset::asset_cache::model(fdid)?;
    if !model_path.exists() {
        return None;
    }
    let name = model_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("wmo_doodad");
    let entity = commands
        .spawn((
            Name::new(name.to_owned()),
            doodad.transform,
            Visibility::default(),
        ))
        .id();
    if !m2_spawn::spawn_m2_on_entity(
        commands,
        &mut m2_spawn::SpawnAssets {
            meshes: assets.meshes,
            materials: assets.materials,
            effect_materials: assets.effect_materials,
            skybox_materials: None,
            images: assets.images,
            inverse_bindposes: assets.inverse_bindposes,
        },
        &model_path,
        entity,
        &[0, 0, 0],
    ) {
        commands.entity(entity).despawn();
        return None;
    }
    Some(entity)
}

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

fn wmo_batch_material(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    material_index: u16,
    material_props: &WmoMaterialProps,
    interior_ambient: Option<[f32; 4]>,
    has_vertex_color: bool,
) -> Handle<StandardMaterial> {
    let image = load_wmo_batch_material_image(images, material_index, &material_props);
    materials.add(wmo_standard_material(
        image,
        material_props.blend_mode,
        material_props.unculled,
        material_props.shader,
        interior_ambient,
        has_vertex_color,
        material_props.sidn_glow,
    ))
}

fn build_wmo_interior_ambient(
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
) -> Option<[f32; 4]> {
    let rgb = &root.ambient_color[..3];
    (group.header.group_flags.interior && rgb.iter().any(|channel| *channel > 0.0))
        .then_some(root.ambient_color)
}

struct WmoMaterialProps {
    texture_fdid: u32,
    texture_2_fdid: u32,
    texture_3_fdid: u32,
    blend_mode: u32,
    unculled: bool,
    shader: u32,
    sidn_glow: Option<WmoSidnGlow>,
}

fn wmo_material_props(root: &wmo::WmoRootData, material_index: u16) -> WmoMaterialProps {
    let mat_def = root.materials.get(material_index as usize);
    WmoMaterialProps {
        texture_fdid: mat_def.map(|m| m.texture_fdid).unwrap_or(0),
        texture_2_fdid: mat_def.map(|m| m.texture_2_fdid).unwrap_or(0),
        texture_3_fdid: mat_def.map(|m| m.texture_3_fdid).unwrap_or(0),
        blend_mode: mat_def.map(|m| m.blend_mode).unwrap_or(0),
        unculled: mat_def.map(|m| m.material_flags.unculled).unwrap_or(false),
        shader: mat_def.map(|m| m.shader).unwrap_or(0),
        sidn_glow: mat_def.and_then(build_wmo_sidn_glow),
    }
}

fn build_wmo_sidn_glow(mat_def: &wmo::WmoMaterialDef) -> Option<WmoSidnGlow> {
    let rgb = &mat_def.sidn_color[..3];
    (mat_def.material_flags.sidn && rgb.iter().any(|channel| *channel > 0.0)).then_some(
        WmoSidnGlow {
            base_sidn_color: mat_def.sidn_color,
        },
    )
}

fn load_wmo_batch_material_image(
    images: &mut Assets<Image>,
    material_index: u16,
    material_props: &WmoMaterialProps,
) -> Option<Handle<Image>> {
    if material_props.texture_fdid == 0 {
        return None;
    }
    let Some(blp_path) = crate::asset::asset_cache::texture(material_props.texture_fdid) else {
        log_wmo_texture_extract_failure(material_props.texture_fdid);
        return None;
    };
    match load_wmo_material_image(
        &blp_path,
        material_props.shader,
        material_props.texture_2_fdid,
        material_props.texture_3_fdid,
        images,
    ) {
        Ok(image) => Some(image),
        Err(err) => {
            log_wmo_texture_decode_failure(material_index, material_props, &err);
            None
        }
    }
}

fn log_wmo_texture_decode_failure(
    material_index: u16,
    material_props: &WmoMaterialProps,
    err: &str,
) {
    eprintln!(
        "WMO texture decode failed for material {material_index} shader {} FDID {}: {err}",
        material_props.shader, material_props.texture_fdid
    );
}

fn log_wmo_texture_extract_failure(texture_fdid: u32) {
    let label = game_engine::listfile::lookup_fdid(texture_fdid).unwrap_or("unknown");
    eprintln!("WMO texture extract failed for FDID {texture_fdid}: {label}");
}

fn load_wmo_material_image(
    base_path: &Path,
    shader: u32,
    texture_2_fdid: u32,
    texture_3_fdid: u32,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let key = WmoTextureCacheKey {
        base_path: base_path.to_path_buf(),
        shader,
        texture_2_fdid,
        texture_3_fdid,
    };
    let cache = wmo_texture_cache();
    if let Some(cached) = lookup_cached_wmo_material_image(cache, &key) {
        return cached;
    }
    let (mut pixels, w, h) = blp::load_blp_rgba(base_path)?;
    composite_wmo_overlay_layers(&mut pixels, w, h, shader, [texture_2_fdid, texture_3_fdid]);
    let handle = images.add(build_wmo_material_image(pixels, w, h));
    cache.lock().unwrap().insert(key, Ok(handle.clone()));
    Ok(handle)
}

fn wmo_texture_cache()
-> &'static Mutex<std::collections::HashMap<WmoTextureCacheKey, Result<Handle<Image>, String>>> {
    WMO_TEXTURE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn lookup_cached_wmo_material_image(
    cache: &Mutex<std::collections::HashMap<WmoTextureCacheKey, Result<Handle<Image>, String>>>,
    key: &WmoTextureCacheKey,
) -> Option<Result<Handle<Image>, String>> {
    cache.lock().unwrap().get(key).cloned()
}

fn composite_wmo_overlay_layers(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    shader: u32,
    overlay_fdids: [u32; 2],
) {
    let descriptor = describe_wmo_shader(shader);
    let combine_modes = [descriptor.second_layer, descriptor.third_layer];

    for (overlay_fdid, combine_mode) in overlay_fdids.into_iter().zip(combine_modes) {
        if overlay_fdid == 0 {
            continue;
        }
        let Some(overlay_path) = crate::asset::asset_cache::texture(overlay_fdid) else {
            continue;
        };
        let Ok((overlay_pixels, ov_w, ov_h)) = blp::load_blp_rgba(&overlay_path) else {
            continue;
        };
        if ov_w == width && ov_h == height {
            composite_wmo_shader_layer(pixels, &overlay_pixels, combine_mode);
        }
    }
}

fn build_wmo_material_image(pixels: Vec<u8>, width: u32, height: u32) -> Image {
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width,
            height,
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
    image
}

pub(super) fn wmo_standard_material(
    texture: Option<Handle<Image>>,
    blend_mode: u32,
    unculled: bool,
    shader: u32,
    interior_ambient: Option<[f32; 4]>,
    has_vertex_color: bool,
    sidn_glow: Option<WmoSidnGlow>,
) -> StandardMaterial {
    let alpha_mode = match blend_mode {
        2 | 3 => AlphaMode::Blend,
        _ if texture.is_some() => AlphaMode::Mask(0.5),
        _ => AlphaMode::Opaque,
    };
    let double_sided = unculled;
    let prop_like_surface = double_sided || !matches!(alpha_mode, AlphaMode::Opaque);
    let surface = wmo_surface_params(texture.is_some(), prop_like_surface, shader);
    let shader_emissive = if describe_wmo_shader(shader).emissive {
        LinearRgba::rgb(0.05, 0.05, 0.05)
    } else {
        LinearRgba::BLACK
    };
    StandardMaterial {
        base_color: if let Some(ambient) = interior_ambient {
            Color::linear_rgba(ambient[0], ambient[1], ambient[2], 1.0)
        } else if texture.is_none() {
            Color::srgb(0.6, 0.6, 0.6)
        } else {
            Color::WHITE
        },
        base_color_texture: texture,
        perceptual_roughness: surface.roughness,
        reflectance: surface.reflectance,
        metallic: surface.metallic,
        emissive: sidn_glow
            .map(|glow| sidn_emissive_color(glow.base_sidn_color, 0.0))
            .unwrap_or(shader_emissive),
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

pub(crate) fn sync_wmo_sidn_emissive(
    game_time: Res<GameTime>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&MeshMaterial3d<StandardMaterial>, &WmoSidnGlow)>,
    mut last_strength: Local<Option<f32>>,
) {
    let strength = sidn_glow_strength(game_time.minutes);
    if last_strength.is_some_and(|last| (last - strength).abs() < 0.001) {
        return;
    }
    *last_strength = Some(strength);

    for (material_handle, glow) in &query {
        let Some(material) = materials.get_mut(material_handle) else {
            continue;
        };
        material.emissive = sidn_emissive_color(glow.base_sidn_color, strength);
    }
}

pub(super) fn sidn_glow_strength(minutes: f32) -> f32 {
    let sun_cycle = ((minutes.rem_euclid(2880.0) / 2880.0) * std::f32::consts::TAU
        - std::f32::consts::FRAC_PI_2)
        .sin();
    (-sun_cycle).max(0.0).powf(1.25)
}

fn sidn_emissive_color(base_sidn_color: [f32; 4], strength: f32) -> LinearRgba {
    let alpha = base_sidn_color[3];
    let scale = alpha * strength;
    let linear =
        Color::srgb(base_sidn_color[0], base_sidn_color[1], base_sidn_color[2]).to_linear();
    LinearRgba::rgb(
        linear.red * scale,
        linear.green * scale,
        linear.blue * scale,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::wmo_format::parser::{WmoLiquidHeader, WmoLiquidTile, WmoLiquidVertex};
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn build_wmo_adt_metadata_preserves_modf_sets() {
        let placement = adt_obj::WmoPlacement {
            name_id: 1,
            unique_id: 77,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            extents_min: [10.0, 20.0, 30.0],
            extents_max: [40.0, 50.0, 60.0],
            flags: 0,
            doodad_set: 3,
            name_set: 9,
            scale: 1.0,
            fdid: Some(123),
            path: None,
        };

        assert_eq!(
            build_wmo_adt_metadata(&placement),
            WmoAdtMetadata {
                unique_id: 77,
                doodad_set: 3,
                name_set: 9,
            }
        );
    }

    #[test]
    fn build_wmo_root_bounds_converts_modf_extents() {
        let placement = adt_obj::WmoPlacement {
            name_id: 1,
            unique_id: 77,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            extents_min: [40.0, 50.0, 60.0],
            extents_max: [10.0, 20.0, 30.0],
            flags: 0,
            doodad_set: 3,
            name_set: 9,
            scale: 1.0,
            fdid: Some(123),
            path: None,
        };

        let expected_min = Vec3::from(placement_to_bevy_absolute(placement.extents_min)).min(
            Vec3::from(placement_to_bevy_absolute(placement.extents_max)),
        );
        let expected_max = Vec3::from(placement_to_bevy_absolute(placement.extents_min)).max(
            Vec3::from(placement_to_bevy_absolute(placement.extents_max)),
        );

        assert_eq!(
            build_wmo_root_bounds(&placement),
            game_engine::culling::WmoRootBounds {
                world_min: expected_min,
                world_max: expected_max,
            }
        );
    }

    #[test]
    fn spawn_wmo_root_entity_attaches_adt_metadata() {
        let mut app = App::new();
        let metadata = WmoAdtMetadata {
            unique_id: 88,
            doodad_set: 4,
            name_set: 6,
        };
        let bounds = game_engine::culling::WmoRootBounds {
            world_min: Vec3::new(-1.0, -2.0, -3.0),
            world_max: Vec3::new(1.0, 2.0, 3.0),
        };

        let entity = app
            .world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn_wmo_root_entity(
                    &mut commands,
                    12345,
                    Transform::IDENTITY,
                    game_engine::culling::WmoPortalGraph {
                        adjacency: Vec::new(),
                        portal_verts: Vec::new(),
                    },
                    metadata,
                    Some(game_engine::culling::ChunkRefs {
                        chunk_indices: vec![4, 8],
                    }),
                    bounds,
                    Some(WmoFootstepSurface {
                        surface: FootstepSurface::Wood,
                    }),
                    None,
                )
            });
        app.update();
        let entity = entity.expect("entity should spawn");

        let stored = app
            .world()
            .get::<WmoAdtMetadata>(entity)
            .copied()
            .expect("metadata component");
        assert_eq!(stored, metadata);
        let stored_bounds = app
            .world()
            .get::<game_engine::culling::WmoRootBounds>(entity)
            .copied()
            .expect("bounds component");
        assert_eq!(stored_bounds, bounds);
        let stored_chunk_refs = app
            .world()
            .get::<game_engine::culling::ChunkRefs>(entity)
            .cloned()
            .expect("chunk refs component");
        assert_eq!(stored_chunk_refs.chunk_indices, vec![4, 8]);
        let stored_surface = app
            .world()
            .get::<WmoFootstepSurface>(entity)
            .copied()
            .expect("footstep surface component");
        assert_eq!(
            stored_surface,
            WmoFootstepSurface {
                surface: FootstepSurface::Wood,
            }
        );
    }

    #[test]
    fn wmo_debug_label_includes_non_default_name_set() {
        assert_eq!(
            wmo_debug_label("world/wmo/test.wmo".into(), 0),
            "world/wmo/test.wmo"
        );
        assert_eq!(
            wmo_debug_label("world/wmo/test.wmo".into(), 6),
            "world/wmo/test.wmo nameSet=6"
        );
    }

    #[test]
    fn build_wmo_footstep_surface_prefers_ground_typed_materials() {
        let root = wmo::WmoRootData {
            n_groups: 0,
            flags: wmo::WmoRootFlags::default(),
            ambient_color: [0.0; 4],
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            materials: vec![
                wmo::WmoMaterialDef {
                    texture_fdid: 124134,
                    texture_2_fdid: 0,
                    texture_3_fdid: 0,
                    flags: 0,
                    material_flags: wmo::WmoMaterialFlags::default(),
                    sidn_color: [0.0; 4],
                    diff_color: [0.0; 4],
                    ground_type: 0,
                    blend_mode: 0,
                    shader: 0,
                    uv_translation_speed: None,
                },
                wmo::WmoMaterialDef {
                    texture_fdid: 123010,
                    texture_2_fdid: 0,
                    texture_3_fdid: 0,
                    flags: 0,
                    material_flags: wmo::WmoMaterialFlags::default(),
                    sidn_color: [0.0; 4],
                    diff_color: [0.0; 4],
                    ground_type: 5,
                    blend_mode: 0,
                    shader: 0,
                    uv_translation_speed: None,
                },
            ],
            lights: Vec::new(),
            doodad_sets: Vec::new(),
            group_names: Vec::new(),
            doodad_names: Vec::new(),
            doodad_file_ids: Vec::new(),
            doodad_defs: Vec::new(),
            fogs: Vec::new(),
            visible_block_vertices: Vec::new(),
            visible_blocks: Vec::new(),
            convex_volume_planes: Vec::new(),
            group_file_data_ids: Vec::new(),
            global_ambient_volumes: Vec::new(),
            ambient_volumes: Vec::new(),
            baked_ambient_box_volumes: Vec::new(),
            dynamic_lights: Vec::new(),
            portals: Vec::new(),
            portal_refs: Vec::new(),
            group_infos: Vec::new(),
            skybox_wow_path: None,
        };

        assert_eq!(
            build_wmo_footstep_surface(&root),
            Some(WmoFootstepSurface {
                surface: FootstepSurface::Wood,
            })
        );
    }

    #[test]
    fn collect_group_doodads_filters_to_default_and_selected_set_refs() {
        let group = wmo::WmoGroupData {
            header: wmo::WmoGroupHeader {
                group_name_offset: 0,
                descriptive_group_name_offset: 0,
                flags: 0,
                group_flags: Default::default(),
                bbox_min: [0.0; 3],
                bbox_max: [0.0; 3],
                portal_start: 0,
                portal_count: 0,
                trans_batch_count: 0,
                int_batch_count: 0,
                ext_batch_count: 0,
                batch_type_d: 0,
                fog_ids: [0; 4],
                group_liquid: 0,
                unique_id: 0,
                flags2: 0,
                parent_split_group_index: -1,
                next_split_child_group_index: -1,
            },
            doodad_refs: vec![0, 2, 3, 4],
            light_refs: Vec::new(),
            bsp_nodes: Vec::new(),
            bsp_face_refs: Vec::new(),
            liquid: None,
            batches: Vec::new(),
        };
        let root = wmo::WmoRootData {
            n_groups: 1,
            flags: wmo::WmoRootFlags::default(),
            ambient_color: [0.0; 4],
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            materials: Vec::new(),
            lights: Vec::new(),
            doodad_sets: vec![
                wmo::WmoDoodadSet {
                    name: "$DefaultGlobal".into(),
                    start_doodad: 0,
                    n_doodads: 2,
                },
                wmo::WmoDoodadSet {
                    name: "InnProps".into(),
                    start_doodad: 2,
                    n_doodads: 2,
                },
            ],
            group_names: Vec::new(),
            doodad_names: vec![
                wmo::WmoDoodadName {
                    offset: 0,
                    name: "world/generic/passive_doodad_0.m2".into(),
                },
                wmo::WmoDoodadName {
                    offset: 1,
                    name: "world/generic/passive_doodad_1.m2".into(),
                },
                wmo::WmoDoodadName {
                    offset: 2,
                    name: "world/generic/selected_doodad_2.m2".into(),
                },
                wmo::WmoDoodadName {
                    offset: 3,
                    name: "world/generic/selected_doodad_3.m2".into(),
                },
                wmo::WmoDoodadName {
                    offset: 4,
                    name: "world/generic/unused_doodad_4.m2".into(),
                },
            ],
            doodad_file_ids: Vec::new(),
            doodad_defs: vec![
                wmo::WmoDoodadDef {
                    name_offset: 0,
                    flags: 0,
                    position: [1.0, 2.0, 3.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: 1.0,
                    color: [1.0; 4],
                },
                wmo::WmoDoodadDef {
                    name_offset: 1,
                    flags: 0,
                    position: [0.0; 3],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: 1.0,
                    color: [1.0; 4],
                },
                wmo::WmoDoodadDef {
                    name_offset: 2,
                    flags: 0,
                    position: [4.0, 5.0, 6.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: 0.5,
                    color: [1.0; 4],
                },
                wmo::WmoDoodadDef {
                    name_offset: 3,
                    flags: 0,
                    position: [7.0, 8.0, 9.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: 2.0,
                    color: [1.0; 4],
                },
                wmo::WmoDoodadDef {
                    name_offset: 4,
                    flags: 0,
                    position: [10.0, 11.0, 12.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: 3.0,
                    color: [1.0; 4],
                },
            ],
            fogs: Vec::new(),
            visible_block_vertices: Vec::new(),
            visible_blocks: Vec::new(),
            convex_volume_planes: Vec::new(),
            group_file_data_ids: Vec::new(),
            global_ambient_volumes: Vec::new(),
            ambient_volumes: Vec::new(),
            baked_ambient_box_volumes: Vec::new(),
            dynamic_lights: Vec::new(),
            portals: Vec::new(),
            portal_refs: Vec::new(),
            group_infos: Vec::new(),
            skybox_wow_path: None,
        };

        let doodads = collect_group_doodads(&root, &group, 1);
        assert_eq!(doodads.len(), 3);
        assert_eq!(
            doodads
                .iter()
                .map(|doodad| doodad.model_path.clone())
                .collect::<Vec<_>>(),
            vec![
                "world/generic/passive_doodad_0.m2",
                "world/generic/selected_doodad_2.m2",
                "world/generic/selected_doodad_3.m2",
            ]
        );
        assert_eq!(doodads[0].transform.translation, Vec3::new(-1.0, 3.0, 2.0));
        assert_eq!(doodads[1].transform.scale, Vec3::splat(0.5));
        assert_eq!(doodads[2].transform.scale, Vec3::splat(2.0));
    }

    #[test]
    fn collect_group_lights_filters_to_group_light_refs() {
        let group = wmo::WmoGroupData {
            header: wmo::WmoGroupHeader {
                group_name_offset: 0,
                descriptive_group_name_offset: 0,
                flags: 0,
                group_flags: Default::default(),
                bbox_min: [0.0; 3],
                bbox_max: [0.0; 3],
                portal_start: 0,
                portal_count: 0,
                trans_batch_count: 0,
                int_batch_count: 0,
                ext_batch_count: 0,
                batch_type_d: 0,
                fog_ids: [0; 4],
                group_liquid: 0,
                unique_id: 0,
                flags2: 0,
                parent_split_group_index: -1,
                next_split_child_group_index: -1,
            },
            doodad_refs: Vec::new(),
            light_refs: vec![0, 2, 9],
            bsp_nodes: Vec::new(),
            bsp_face_refs: Vec::new(),
            liquid: None,
            batches: Vec::new(),
        };
        let root = wmo::WmoRootData {
            n_groups: 1,
            flags: wmo::WmoRootFlags::default(),
            ambient_color: [0.0; 4],
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            materials: Vec::new(),
            lights: vec![
                wmo::WmoLight {
                    light_type: wmo::WmoLightType::Omni,
                    use_attenuation: true,
                    color: [1.0, 0.0, 0.0, 1.0],
                    position: [1.0, 2.0, 3.0],
                    intensity: 4.0,
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    attenuation_start: 1.0,
                    attenuation_end: 5.0,
                },
                wmo::WmoLight {
                    light_type: wmo::WmoLightType::Ambient,
                    use_attenuation: false,
                    color: [0.0, 1.0, 0.0, 1.0],
                    position: [4.0, 5.0, 6.0],
                    intensity: 2.0,
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    attenuation_start: 0.0,
                    attenuation_end: 0.0,
                },
                wmo::WmoLight {
                    light_type: wmo::WmoLightType::Spot,
                    use_attenuation: true,
                    color: [0.0, 0.0, 1.0, 1.0],
                    position: [7.0, 8.0, 9.0],
                    intensity: 6.0,
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    attenuation_start: 2.0,
                    attenuation_end: 10.0,
                },
            ],
            doodad_sets: Vec::new(),
            group_names: Vec::new(),
            doodad_names: Vec::new(),
            doodad_file_ids: Vec::new(),
            doodad_defs: Vec::new(),
            fogs: Vec::new(),
            visible_block_vertices: Vec::new(),
            visible_blocks: Vec::new(),
            convex_volume_planes: Vec::new(),
            group_file_data_ids: Vec::new(),
            global_ambient_volumes: Vec::new(),
            ambient_volumes: Vec::new(),
            baked_ambient_box_volumes: Vec::new(),
            dynamic_lights: Vec::new(),
            portals: Vec::new(),
            portal_refs: Vec::new(),
            group_infos: Vec::new(),
            skybox_wow_path: None,
        };

        let lights = collect_group_lights(&root, &group);
        assert_eq!(lights.len(), 2);
        assert_eq!(lights[0].0, 0);
        assert_eq!(lights[1].0, 2);
        assert_eq!(lights[0].1.position, [1.0, 2.0, 3.0]);
        assert_eq!(lights[1].1.position, [7.0, 8.0, 9.0]);
    }

    #[test]
    fn collect_group_fogs_filters_to_valid_unique_group_fog_ids() {
        let group = wmo::WmoGroupData {
            header: wmo::WmoGroupHeader {
                group_name_offset: 0,
                descriptive_group_name_offset: 0,
                flags: 0,
                group_flags: Default::default(),
                bbox_min: [0.0; 3],
                bbox_max: [0.0; 3],
                portal_start: 0,
                portal_count: 0,
                trans_batch_count: 0,
                int_batch_count: 0,
                ext_batch_count: 0,
                batch_type_d: 0,
                fog_ids: [1, 3, 1, 9],
                group_liquid: 0,
                unique_id: 0,
                flags2: 0,
                parent_split_group_index: -1,
                next_split_child_group_index: -1,
            },
            doodad_refs: Vec::new(),
            light_refs: Vec::new(),
            bsp_nodes: Vec::new(),
            bsp_face_refs: Vec::new(),
            liquid: None,
            batches: Vec::new(),
        };
        let root = wmo::WmoRootData {
            n_groups: 1,
            flags: wmo::WmoRootFlags::default(),
            ambient_color: [0.0; 4],
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            materials: Vec::new(),
            lights: Vec::new(),
            doodad_sets: Vec::new(),
            group_names: Vec::new(),
            doodad_names: Vec::new(),
            doodad_file_ids: Vec::new(),
            doodad_defs: Vec::new(),
            fogs: vec![
                wmo::WmoFog {
                    flags: 0,
                    position: [1.0, 2.0, 3.0],
                    smaller_radius: 4.0,
                    larger_radius: 5.0,
                    fog_end: 6.0,
                    fog_start_multiplier: 0.2,
                    color_1: [0.1, 0.2, 0.3, 1.0],
                    underwater_fog_end: 7.0,
                    underwater_fog_start_multiplier: 0.3,
                    color_2: [0.4, 0.5, 0.6, 1.0],
                },
                wmo::WmoFog {
                    flags: 1,
                    position: [10.0, 20.0, 30.0],
                    smaller_radius: 40.0,
                    larger_radius: 50.0,
                    fog_end: 60.0,
                    fog_start_multiplier: 0.4,
                    color_1: [0.7, 0.2, 0.3, 1.0],
                    underwater_fog_end: 70.0,
                    underwater_fog_start_multiplier: 0.5,
                    color_2: [0.4, 0.8, 0.6, 1.0],
                },
                wmo::WmoFog {
                    flags: 2,
                    position: [100.0, 200.0, 300.0],
                    smaller_radius: 400.0,
                    larger_radius: 500.0,
                    fog_end: 600.0,
                    fog_start_multiplier: 0.6,
                    color_1: [0.1, 0.9, 0.3, 1.0],
                    underwater_fog_end: 700.0,
                    underwater_fog_start_multiplier: 0.7,
                    color_2: [0.4, 0.5, 0.9, 1.0],
                },
                wmo::WmoFog {
                    flags: 3,
                    position: [11.0, 22.0, 33.0],
                    smaller_radius: 44.0,
                    larger_radius: 55.0,
                    fog_end: 66.0,
                    fog_start_multiplier: 0.8,
                    color_1: [0.8, 0.2, 0.3, 1.0],
                    underwater_fog_end: 77.0,
                    underwater_fog_start_multiplier: 0.9,
                    color_2: [0.4, 0.8, 0.9, 1.0],
                },
            ],
            visible_block_vertices: Vec::new(),
            visible_blocks: Vec::new(),
            convex_volume_planes: Vec::new(),
            group_file_data_ids: Vec::new(),
            global_ambient_volumes: Vec::new(),
            ambient_volumes: Vec::new(),
            baked_ambient_box_volumes: Vec::new(),
            dynamic_lights: Vec::new(),
            portals: Vec::new(),
            portal_refs: Vec::new(),
            group_infos: Vec::new(),
            skybox_wow_path: None,
        };

        let fogs = collect_group_fogs(&root, &group);
        assert_eq!(fogs.len(), 2);
        assert_eq!(fogs[0].0, 1);
        assert_eq!(fogs[1].0, 3);
        assert_eq!(fogs[0].1.position, [10.0, 20.0, 30.0]);
        assert_eq!(fogs[1].1.position, [11.0, 22.0, 33.0]);
    }

    #[test]
    fn spawn_wmo_group_fog_preserves_authored_fog_fields() {
        let mut app = App::new();
        let fog = wmo::WmoFog {
            flags: 0,
            position: [10.0, 20.0, 30.0],
            smaller_radius: 4.0,
            larger_radius: 12.0,
            fog_end: 80.0,
            fog_start_multiplier: 0.25,
            color_1: [0.1, 0.2, 0.3, 1.0],
            underwater_fog_end: 90.0,
            underwater_fog_start_multiplier: 0.5,
            color_2: [0.7, 0.8, 0.9, 1.0],
        };

        let entity = app
            .world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn_wmo_group_fog(&mut commands, 2, &fog)
            })
            .expect("fog entity should spawn");
        app.update();

        let component = app
            .world()
            .get::<WmoGroupFogVolume>(entity)
            .copied()
            .expect("fog component");
        assert_eq!(
            component,
            WmoGroupFogVolume {
                fog_index: 2,
                smaller_radius: 4.0,
                larger_radius: 12.0,
                fog_end: 80.0,
                fog_start_multiplier: 0.25,
                color_1: [0.1, 0.2, 0.3, 1.0],
                underwater_fog_end: 90.0,
                underwater_fog_start_multiplier: 0.5,
                color_2: [0.7, 0.8, 0.9, 1.0],
            }
        );
        let transform = app.world().get::<Transform>(entity).expect("fog transform");
        assert_eq!(transform.translation, Vec3::new(-10.0, 30.0, 20.0));
        assert_eq!(transform.scale, Vec3::splat(12.0));
    }

    #[test]
    fn spawn_wmo_group_batches_marks_mesh_children_for_collision() {
        let mut app = App::new();
        app.world_mut().init_resource::<Assets<Mesh>>();
        app.world_mut().init_resource::<Assets<StandardMaterial>>();
        app.world_mut().init_resource::<Assets<WaterMaterial>>();
        app.world_mut().init_resource::<Assets<Image>>();
        app.world_mut().init_resource::<Assets<M2EffectMaterial>>();
        app.world_mut()
            .init_resource::<Assets<SkinnedMeshInverseBindposes>>();
        let group_entity = app.world_mut().spawn_empty().id();

        let _ = app.world_mut().run_system_once(
            move |mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<StandardMaterial>>,
                  mut water_materials: ResMut<Assets<WaterMaterial>>,
                  mut images: ResMut<Assets<Image>>,
                  mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
                  mut inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>| {
                let root = wmo::WmoRootData {
                    n_groups: 1,
                    flags: wmo::WmoRootFlags::default(),
                    ambient_color: [0.0; 4],
                    bbox_min: [0.0; 3],
                    bbox_max: [0.0; 3],
                    materials: Vec::new(),
                    lights: Vec::new(),
                    doodad_sets: Vec::new(),
                    group_names: Vec::new(),
                    doodad_names: Vec::new(),
                    doodad_file_ids: Vec::new(),
                    doodad_defs: Vec::new(),
                    fogs: Vec::new(),
                    visible_block_vertices: Vec::new(),
                    visible_blocks: Vec::new(),
                    convex_volume_planes: Vec::new(),
                    group_file_data_ids: Vec::new(),
                    global_ambient_volumes: Vec::new(),
                    ambient_volumes: Vec::new(),
                    baked_ambient_box_volumes: Vec::new(),
                    dynamic_lights: Vec::new(),
                    portals: Vec::new(),
                    portal_refs: Vec::new(),
                    group_infos: Vec::new(),
                    skybox_wow_path: None,
                };
                let mut assets = WmoAssets {
                    meshes: &mut meshes,
                    materials: &mut materials,
                    water_materials: &mut water_materials,
                    images: &mut images,
                    effect_materials: &mut effect_materials,
                    inverse_bindposes: &mut inverse_bindposes,
                };
                spawn_wmo_group_batches(
                    &mut commands,
                    &mut assets,
                    &root,
                    None,
                    group_entity,
                    vec![wmo::WmoGroupBatch {
                        mesh: Mesh::new(
                            PrimitiveTopology::TriangleList,
                            RenderAssetUsages::default(),
                        ),
                        material_index: 0,
                        batch_type: wmo::WmoBatchType::WholeGroup,
                        uses_second_color_blend_alpha: false,
                        uses_second_uv_set: false,
                        uses_third_uv_set: false,
                        uses_generated_tangents: false,
                        has_vertex_color: false,
                    }],
                );
            },
        );
        app.update();

        let children = app
            .world()
            .get::<Children>(group_entity)
            .expect("spawned batch child");
        let batch_entity = children[0];
        assert!(
            app.world().get::<WmoCollisionMesh>(batch_entity).is_some(),
            "WMO batch mesh should block player movement"
        );
    }

    #[test]
    fn build_wmo_liquid_mesh_skips_empty_tiles_and_uses_vertex_heights() {
        let liquid = wmo::WmoLiquid {
            header: WmoLiquidHeader {
                x_verts: 3,
                y_verts: 2,
                x_tiles: 2,
                y_tiles: 1,
                position: [10.0, 20.0, 30.0],
                material_id: 7,
            },
            vertices: vec![
                WmoLiquidVertex {
                    raw: [0; 4],
                    height: 30.0,
                },
                WmoLiquidVertex {
                    raw: [0; 4],
                    height: 31.0,
                },
                WmoLiquidVertex {
                    raw: [0; 4],
                    height: 32.0,
                },
                WmoLiquidVertex {
                    raw: [0; 4],
                    height: 33.0,
                },
                WmoLiquidVertex {
                    raw: [0; 4],
                    height: 34.0,
                },
                WmoLiquidVertex {
                    raw: [0; 4],
                    height: 35.0,
                },
            ],
            tiles: vec![
                WmoLiquidTile {
                    liquid_type: 3,
                    fishable: false,
                    shared: false,
                },
                WmoLiquidTile {
                    liquid_type: 0x0F,
                    fishable: false,
                    shared: false,
                },
            ],
        };

        let mesh = build_wmo_liquid_mesh(&liquid);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("expected wmo liquid positions");
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("expected wmo liquid colors");
        };
        assert_eq!(positions.len(), 4);
        assert_eq!(colors.len(), 4);
        assert_eq!(positions[0], [-10.0, 29.0, 20.0]);
        assert_eq!(positions[1], [-(10.0 + WMO_LIQUID_TILE_SIZE), 30.0, 20.0]);
        assert_eq!(positions[2], [-10.0, 32.0, 20.0 + WMO_LIQUID_TILE_SIZE]);
        assert_eq!(colors[0], [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(mesh.indices().unwrap().len(), 6);
    }
}
