//! Doodad (M2) and WMO spawning from _obj0/_obj1/_obj2 ADT companion files.

use std::path::Path;

use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::{adt_obj, blp, wmo};
use crate::m2_spawn;

use crate::terrain::resolve_companion_path;

#[derive(Default)]
pub struct SpawnedTerrainObjects {
    pub doodads: Vec<Entity>,
    pub wmos: Vec<SpawnedWmoRoot>,
}

pub struct SpawnedWmoRoot {
    pub entity: Entity,
    pub model: String,
}

impl SpawnedTerrainObjects {
    pub fn all_entities(self) -> Vec<Entity> {
        let mut entities = self.doodads;
        entities.extend(self.wmos.into_iter().map(|wmo| wmo.entity));
        entities
    }
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
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    obj_data: &adt_obj::AdtObjData,
) -> SpawnedTerrainObjects {
    let mut spawned = SpawnedTerrainObjects::default();
    spawn_doodads(
        commands,
        meshes,
        materials,
        images,
        inverse_bp,
        obj_data,
        &mut spawned.doodads,
    );
    spawn_wmos(commands, meshes, materials, images, obj_data, &mut spawned.wmos);
    spawned
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

/// Resolve a doodad placement to a local M2 file path.
fn resolve_doodad_m2(doodad: &adt_obj::DoodadPlacement) -> Option<std::path::PathBuf> {
    if let Some(fdid) = doodad.fdid {
        return crate::asset::casc_resolver::ensure_model(fdid);
    }
    let wow_path = doodad.path.as_ref()?;
    let fdid = game_engine::listfile::lookup_path(wow_path)?;
    crate::asset::casc_resolver::ensure_model(fdid)
}

/// Convert WoW doodad placement to a Bevy Transform.
fn doodad_transform(d: &adt_obj::DoodadPlacement) -> Transform {
    let pos = placement_to_bevy(d.position);
    let rotation = placement_rotation(d.rotation);
    Transform::from_translation(Vec3::from(pos))
        .with_rotation(rotation)
        .with_scale(Vec3::splat(d.scale))
}

/// Convert WoW MDDF/MODF Euler rotation (degrees) to a Bevy quaternion.
///
/// WoW stores rotation as [X, Y, Z] degrees. The reference ADT viewer
/// (`worldofwhatever`) applies these directly in the already-swizzled
/// render basis as:
///   Ry(Y - 90) * Rz(-X) * Rx(Z)
/// Our placements and mesh vertices are converted into that same basis
/// via `placement_to_bevy`/`wow_to_bevy`, so the same composition applies.
fn placement_rotation(rot: [f32; 3]) -> Quat {
    let rx = rot[2].to_radians();
    let ry = (rot[1] - 90.0).to_radians();
    let rz = (-rot[0]).to_radians();
    Quat::from_euler(EulerRot::YZX, ry, rz, rx)
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
    obj_data: &adt_obj::AdtObjData,
    entities: &mut Vec<SpawnedWmoRoot>,
) {
    let mut spawned_count = 0u32;
    for placement in &obj_data.wmos {
        let mut assets = WmoAssets {
            meshes,
            materials,
            images,
        };
        if let Some(spawned_wmo) = try_spawn_wmo(commands, &mut assets, placement) {
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
) -> Option<SpawnedWmoRoot> {
    let root_fdid = resolve_wmo_fdid(placement)?;
    let root_path = ensure_wmo_asset(root_fdid)?;
    let root_data = std::fs::read(&root_path).ok()?;
    let root = wmo::load_wmo_root(&root_data).ok()?;

    let group_fdids = resolve_wmo_group_fdids(root_fdid, root.n_groups);
    let transform = wmo_transform(placement);
    let portal_graph = build_portal_graph(&root);
    let root_entity = spawn_wmo_root_entity(commands, root_fdid, transform, portal_graph);

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
) -> Entity {
    commands
        .spawn((
            Name::new(format!("wmo_{root_fdid}")),
            transform,
            Visibility::default(),
            game_engine::culling::Wmo,
            portal_graph,
        ))
        .id()
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

fn ensure_wmo_asset(fdid: u32) -> Option<std::path::PathBuf> {
    let out_path = std::path::PathBuf::from(format!("data/models/{fdid}.wmo"));
    crate::asset::casc_resolver::ensure_file_at_path(fdid, &out_path)
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
    use crate::asset::m2::wow_to_bevy;
    let (bbox_min, bbox_max) = root
        .group_infos
        .get(group_index as usize)
        .map(|info| {
            let min = wow_to_bevy(info.bbox_min[0], info.bbox_min[1], info.bbox_min[2]);
            let max = wow_to_bevy(info.bbox_max[0], info.bbox_max[1], info.bbox_max[2]);
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
    let blend_mode = mat_def.map(|m| m.blend_mode).unwrap_or(0);
    let flags = mat_def.map(|m| m.flags).unwrap_or(0);

    if texture_fdid > 0 {
        let blp_path =
            crate::asset::casc_resolver::ensure_texture(texture_fdid).unwrap_or_else(|| {
                std::path::PathBuf::from(format!("data/textures/{texture_fdid}.blp"))
            });
        if let Ok(image) = blp::load_blp_gpu_image(&blp_path) {
            return materials.add(wmo_standard_material(
                Some(images.add(image)),
                blend_mode,
                flags,
                has_vertex_color,
            ));
        }
    }
    materials.add(wmo_standard_material(
        None,
        blend_mode,
        flags,
        has_vertex_color,
    ))
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
    StandardMaterial {
        base_color: if texture.is_none() {
            Color::srgb(0.6, 0.6, 0.6)
        } else {
            Color::WHITE
        },
        base_color_texture: texture,
        perceptual_roughness: 0.8,
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

/// Convert MODF/MDDF placement position to Bevy-space.
pub fn placement_to_bevy(raw: [f32; 3]) -> [f32; 3] {
    use crate::asset::m2::wow_to_bevy;
    wow_to_bevy(raw[0], raw[1], raw[2])
}

/// Convert WMO placement to a Bevy Transform.
fn wmo_transform(w: &adt_obj::WmoPlacement) -> Transform {
    let pos = placement_to_bevy(w.position);
    let rotation = placement_rotation(w.rotation);
    Transform::from_translation(Vec3::from(pos))
        .with_rotation(rotation)
        .with_scale(Vec3::splat(w.scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_rotation_matches_reference_viewer_formula() {
        let rot = [17.0, 123.0, -31.0];
        let actual = placement_rotation(rot);
        let expected = Quat::from_euler(
            EulerRot::YZX,
            (rot[1] - 90.0).to_radians(),
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
    fn placement_to_bevy_matches_reference_viewer_position_basis() {
        let raw = [17012.4, 83.1, 8220.7];
        let actual = placement_to_bevy(raw);
        let expected = crate::asset::m2::wow_to_bevy(raw[0], raw[1], raw[2]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn wmo_vertex_colored_materials_are_unlit() {
        let material = wmo_standard_material(None, 0, 0, true);
        assert!(material.unlit);
    }
}
