use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use bevy::image::Image;
use bevy::prelude::*;

use crate::asset::{adt_format::adt_obj, blp, wmo};

use super::{SpawnedWmoRoot, WmoLocalSkybox, wmo_transform};

const WMO_DOUBLE_SIDED_FLAG: u32 = 0x04;

#[derive(Clone, PartialEq, Eq, Hash)]
struct WmoTextureCacheKey {
    base_path: PathBuf,
    texture_2_fdid: u32,
    texture_3_fdid: u32,
}

static WMO_TEXTURE_CACHE: OnceLock<
    Mutex<std::collections::HashMap<WmoTextureCacheKey, Result<Handle<Image>, String>>>,
> = OnceLock::new();

struct WmoAssets<'a> {
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    images: &'a mut Assets<Image>,
}

pub(super) fn spawn_wmos_filtered(
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
    root: &wmo::WmoRootData,
    material_index: u16,
    has_vertex_color: bool,
) -> Handle<StandardMaterial> {
    let material_props = wmo_material_props(root, material_index);
    let image = load_wmo_batch_material_image(images, material_index, &material_props);
    materials.add(wmo_standard_material(
        image,
        material_props.blend_mode,
        material_props.flags,
        has_vertex_color,
    ))
}

struct WmoMaterialProps {
    texture_fdid: u32,
    texture_2_fdid: u32,
    texture_3_fdid: u32,
    blend_mode: u32,
    flags: u32,
    shader: u32,
}

fn wmo_material_props(root: &wmo::WmoRootData, material_index: u16) -> WmoMaterialProps {
    let mat_def = root.materials.get(material_index as usize);
    WmoMaterialProps {
        texture_fdid: mat_def.map(|m| m.texture_fdid).unwrap_or(0),
        texture_2_fdid: mat_def.map(|m| m.texture_2_fdid).unwrap_or(0),
        texture_3_fdid: mat_def.map(|m| m.texture_3_fdid).unwrap_or(0),
        blend_mode: mat_def.map(|m| m.blend_mode).unwrap_or(0),
        flags: mat_def.map(|m| m.flags).unwrap_or(0),
        shader: mat_def.map(|m| m.shader).unwrap_or(0),
    }
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

pub(super) fn wmo_standard_material(
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
    let double_sided = (flags & WMO_DOUBLE_SIDED_FLAG) != 0;
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
