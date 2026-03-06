//! Doodad (M2) and WMO spawning from _obj0/_obj1/_obj2 ADT companion files.

use std::path::Path;

use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::adt::CHUNK_SIZE;
use crate::asset::{adt_obj, blp, wmo};

use crate::terrain::resolve_companion_path;

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

// ── doodad spawning ─────────────────────────────────────────────────────────

/// Spawn doodads and WMOs, returning the entities created.
pub fn spawn_obj_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    obj_data: &adt_obj::AdtObjData,
) -> Vec<Entity> {
    let mut entities = Vec::new();
    spawn_doodads(commands, meshes, materials, images, inverse_bp, obj_data, &mut entities);
    spawn_wmos(commands, meshes, materials, images, obj_data, &mut entities);
    entities
}

/// Spawn doodads (M2 models) from placement data.
fn spawn_doodads(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    obj_data: &adt_obj::AdtObjData,
    entities: &mut Vec<Entity>,
) {
    let mut spawned = 0u32;
    for doodad in &obj_data.doodads {
        if let Some(e) = try_spawn_doodad(commands, meshes, materials, images, inverse_bp, doodad) {
            entities.push(e);
            spawned += 1;
        }
    }
    eprintln!("Spawned {spawned}/{} doodads", obj_data.doodads.len());
}

/// Try to spawn a single doodad. Returns the entity if successful.
fn try_spawn_doodad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    doodad: &adt_obj::DoodadPlacement,
) -> Option<Entity> {
    let m2_path = resolve_doodad_m2(doodad)?;
    if !m2_path.exists() {
        return None;
    }
    let transform = doodad_transform(doodad);
    let entity = crate::spawn_static_m2(commands, meshes, materials, images, inverse_bp, &m2_path, transform)?;
    commands.entity(entity).insert(game_engine::culling::Doodad);
    Some(entity)
}

/// Resolve a doodad placement to a local M2 file path.
fn resolve_doodad_m2(doodad: &adt_obj::DoodadPlacement) -> Option<std::path::PathBuf> {
    if let Some(fdid) = doodad.fdid {
        return Some(std::path::PathBuf::from(format!("data/models/{fdid}.m2")));
    }
    let wow_path = doodad.path.as_ref()?;
    let fdid = game_engine::listfile::lookup_path(wow_path)?;
    Some(std::path::PathBuf::from(format!("data/models/{fdid}.m2")))
}

/// Convert WoW doodad placement to a Bevy Transform.
fn doodad_transform(d: &adt_obj::DoodadPlacement) -> Transform {
    let pos = placement_to_bevy(d.position);
    let rotation = doodad_rotation(d.rotation);
    Transform::from_translation(Vec3::from(pos))
        .with_rotation(rotation)
        .with_scale(Vec3::splat(d.scale))
}

/// Convert WoW Euler rotation (degrees around Y, X, Z) to a Bevy quaternion.
fn doodad_rotation(rot: [f32; 3]) -> Quat {
    let rx = rot[0].to_radians();
    let ry = rot[1].to_radians();
    let rz = rot[2].to_radians();
    Quat::from_euler(EulerRot::YXZ, ry, rx, rz)
}

// ── WMO spawning ────────────────────────────────────────────────────────────

/// Spawn WMOs from placement data.
fn spawn_wmos(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    obj_data: &adt_obj::AdtObjData,
    entities: &mut Vec<Entity>,
) {
    let mut spawned = 0u32;
    for placement in &obj_data.wmos {
        if let Some(e) = try_spawn_wmo(commands, meshes, materials, images, placement) {
            entities.push(e);
            spawned += 1;
        }
    }
    eprintln!("Spawned {spawned}/{} WMOs", obj_data.wmos.len());
}

/// Try to spawn a single WMO. Returns root entity if successful.
fn try_spawn_wmo(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    placement: &adt_obj::WmoPlacement,
) -> Option<Entity> {
    let root_fdid = resolve_wmo_fdid(placement)?;
    let root_path = format!("data/models/{root_fdid}.wmo");
    let root_data = std::fs::read(&root_path).ok()?;
    let root = wmo::load_wmo_root(&root_data).ok()?;

    let group_fdids = resolve_wmo_group_fdids(root_fdid, root.n_groups);
    let transform = wmo_transform(placement);
    let portal_graph = build_portal_graph(&root);
    let root_entity = commands
        .spawn((
            Name::new(format!("wmo_{root_fdid}")),
            transform,
            Visibility::default(),
            game_engine::culling::Wmo,
            portal_graph,
        ))
        .id();

    let group_count = spawn_wmo_groups(commands, meshes, materials, images, &root, &group_fdids, root_fdid, root_entity);
    log_wmo_spawn(root_fdid, group_count, &root, &transform);
    if group_count > 0 { Some(root_entity) } else { None }
}

/// Spawn all WMO groups as children. Returns count of successfully spawned groups.
fn spawn_wmo_groups(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: &wmo::WmoRootData,
    group_fdids: &[Option<u32>],
    root_fdid: u32,
    root_entity: Entity,
) -> u32 {
    let mut count = 0u32;
    for (i, group_fdid) in group_fdids.iter().enumerate() {
        let Some(fdid) = group_fdid else { continue };
        if spawn_wmo_group(commands, meshes, materials, images, root, *fdid, root_entity) {
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
        root.n_groups, root.materials.len(), pos.x, pos.y, pos.z,
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
                    Vec3::from(crate::asset::m2::wow_to_bevy(x, y, z))
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

/// Parse and spawn one WMO group file as children of the root entity.
fn spawn_wmo_group(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: &wmo::WmoRootData,
    group_fdid: u32,
    root_entity: Entity,
) -> bool {
    let group_path = format!("data/models/{group_fdid}.wmo");
    let Ok(data) = std::fs::read(&group_path) else { return false };
    let Ok(group) = wmo::load_wmo_group(&data) else { return false };

    for batch in group.batches {
        let mat = wmo_batch_material(materials, images, root, batch.material_index);
        let child = commands
            .spawn((
                Mesh3d(meshes.add(batch.mesh)),
                MeshMaterial3d(mat),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        commands.entity(root_entity).add_child(child);
    }
    true
}

/// Build a Bevy material for a WMO batch.
fn wmo_batch_material(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: &wmo::WmoRootData,
    material_index: u16,
) -> Handle<StandardMaterial> {
    let mat_def = root.materials.get(material_index as usize);
    let texture_fdid = mat_def.map(|m| m.texture_fdid).unwrap_or(0);
    let blend_mode = mat_def.map(|m| m.blend_mode).unwrap_or(0);

    if texture_fdid > 0 {
        let blp_path = crate::asset::casc_resolver::ensure_texture(texture_fdid)
            .unwrap_or_else(|| std::path::PathBuf::from(format!("data/textures/{texture_fdid}.blp")));
        if let Ok(image) = blp::load_blp_gpu_image(&blp_path) {
            return materials.add(wmo_standard_material(Some(images.add(image)), blend_mode));
        }
    }
    materials.add(wmo_standard_material(None, blend_mode))
}

fn wmo_standard_material(texture: Option<Handle<Image>>, blend_mode: u32) -> StandardMaterial {
    let alpha_mode = match blend_mode {
        1 => AlphaMode::Mask(0.5),
        2 | 3 => AlphaMode::Blend,
        _ => AlphaMode::Opaque,
    };
    StandardMaterial {
        base_color: if texture.is_none() { Color::srgb(0.6, 0.6, 0.6) } else { Color::WHITE },
        base_color_texture: texture,
        perceptual_roughness: 0.8,
        double_sided: true,
        cull_mode: None,
        alpha_mode,
        ..default()
    }
}

// ── coordinate conversion ───────────────────────────────────────────────────

/// Convert MODF/MDDF placement position to Bevy-space.
pub fn placement_to_bevy(raw: [f32; 3]) -> [f32; 3] {
    use crate::asset::m2::wow_to_bevy;
    const MAP_OFFSET: f32 = 32.0 * CHUNK_SIZE * 16.0;
    let wx = MAP_OFFSET - raw[0];
    let wy = MAP_OFFSET - raw[2];
    let wz = raw[1];
    wow_to_bevy(wx, wy, wz)
}

/// Convert WMO placement to a Bevy Transform.
fn wmo_transform(w: &adt_obj::WmoPlacement) -> Transform {
    let pos = placement_to_bevy(w.position);
    let rotation = doodad_rotation(w.rotation);
    Transform::from_translation(Vec3::from(pos))
        .with_rotation(rotation)
        .with_scale(Vec3::splat(w.scale))
}
