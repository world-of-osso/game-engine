use std::path::Path;

use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::adt::{self, ChunkHeightGrid, CHUNK_SIZE, UNIT_SIZE, vertex_index};
use crate::asset::{adt_obj, blp, wmo};
use crate::terrain_material::{self, TerrainMaterial};

/// Marker component for the ADT terrain root entity.
#[derive(Component)]
pub struct AdtTerrain;

/// Queryable heightmap for terrain collision. Built from ADT height grids.
#[derive(Resource)]
pub struct TerrainHeightmap {
    grids: Vec<Option<ChunkHeightGrid>>, // 256 slots, indexed by y*16+x
    bounds_min: Vec3,
    bounds_max: Vec3,
}

impl TerrainHeightmap {
    /// Build from parsed ADT data.
    fn from_adt(adt_data: &adt::AdtData) -> Self {
        let mut grids: Vec<Option<ChunkHeightGrid>> = vec![None; 256];
        for g in &adt_data.height_grids {
            let idx = (g.index_y * 16 + g.index_x) as usize;
            if idx < 256 {
                grids[idx] = Some(g.clone());
            }
        }
        let (bounds_min, bounds_max) = adt_data.bounds();
        Self { grids, bounds_min, bounds_max }
    }

    /// Look up terrain height at a Bevy-space (x, z) position.
    pub fn height_at(&self, bx: f32, bz: f32) -> Option<f32> {
        if bx < self.bounds_min.x || bx > self.bounds_max.x
            || bz < self.bounds_min.z || bz > self.bounds_max.z
        {
            return None;
        }
        self.grids.iter().flatten().find_map(|g| chunk_height_at(g, bx, bz))
    }
}

/// Try to get height from a single chunk. Returns None if (bx, bz) is outside this chunk.
fn chunk_height_at(g: &ChunkHeightGrid, bx: f32, bz: f32) -> Option<f32> {
    // Terrain grows in -X from origin_x, +Z from origin_z
    let local_x = g.origin_x - bx;
    let local_z = bz - g.origin_z;
    if local_x < 0.0 || local_x >= CHUNK_SIZE || local_z < 0.0 || local_z >= CHUNK_SIZE {
        return None;
    }
    let col = (local_x / UNIT_SIZE).floor() as usize;
    let row = (local_z / UNIT_SIZE).floor() as usize;
    let col = col.min(7);
    let row = row.min(7);
    let frac_x = (local_x - col as f32 * UNIT_SIZE) / UNIT_SIZE;
    let frac_z = (local_z - row as f32 * UNIT_SIZE) / UNIT_SIZE;
    Some(interpolate_quad_height(g, row, col, frac_x, frac_z))
}

/// Interpolate height within a quad using the 4-triangle fan from center vertex.
fn interpolate_quad_height(
    g: &ChunkHeightGrid,
    row: usize,
    col: usize,
    fx: f32,
    fz: f32,
) -> f32 {
    let h = |idx: usize| g.base_y + g.heights[idx];
    let tl = h(vertex_index(row * 2, col));
    let tr = h(vertex_index(row * 2, col + 1));
    let bl = h(vertex_index(row * 2 + 2, col));
    let br = h(vertex_index(row * 2 + 2, col + 1));
    let center = h(vertex_index(row * 2 + 1, col));

    // Determine which triangle: compare distance from center (0.5, 0.5)
    let dx = fx - 0.5;
    let dz = fz - 0.5;
    let (ha, hb, ax, az, bxx, bz) = if dz.abs() >= dx.abs() {
        if dz < 0.0 {
            // Top triangle: TL(0,0), TR(1,0), C(0.5,0.5)
            (tl, tr, 0.0, 0.0, 1.0, 0.0)
        } else {
            // Bottom triangle: BR(1,1), BL(0,1), C(0.5,0.5)
            (br, bl, 1.0, 1.0, 0.0, 1.0)
        }
    } else if dx > 0.0 {
        // Right triangle: TR(1,0), BR(1,1), C(0.5,0.5)
        (tr, br, 1.0, 0.0, 1.0, 1.0)
    } else {
        // Left triangle: BL(0,1), TL(0,0), C(0.5,0.5)
        (bl, tl, 0.0, 1.0, 0.0, 0.0)
    };
    barycentric_height(fx, fz, ax, az, ha, bxx, bz, hb, 0.5, 0.5, center)
}

/// Barycentric interpolation of height at (px, pz) within triangle (A, B, C).
fn barycentric_height(
    px: f32, pz: f32,
    ax: f32, az: f32, ha: f32,
    bx: f32, bz: f32, hb: f32,
    cx: f32, cz: f32, hc: f32,
) -> f32 {
    let det = (bz - cz) * (ax - cx) + (cx - bx) * (az - cz);
    if det.abs() < 1e-10 {
        return (ha + hb + hc) / 3.0;
    }
    let wa = ((bz - cz) * (px - cx) + (cx - bx) * (pz - cz)) / det;
    let wb = ((cz - az) * (px - cx) + (ax - cx) * (pz - cz)) / det;
    let wc = 1.0 - wa - wb;
    wa * ha + wb * hb + wc * hc
}

/// Result of spawning an ADT: camera and ground position for placing models.
pub struct AdtSpawnResult {
    pub camera: Transform,
    pub center: Vec3,
}


/// Load an ADT file, build meshes, and spawn them into the Bevy scene.
pub fn spawn_adt(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    adt_path: &Path,
) -> Result<AdtSpawnResult, String> {
    let data = std::fs::read(adt_path)
        .map_err(|e| format!("Failed to read {}: {e}", adt_path.display()))?;
    let adt_data = adt::load_adt(&data)?;

    let tex_data = load_tex0(adt_path);
    let ground_images = tex_data
        .as_ref()
        .map(|td| terrain_material::load_ground_images(images, td, adt_path));

    let chunk_materials = terrain_material::build_terrain_materials(
        terrain_materials,
        images,
        tex_data.as_ref(),
        ground_images.as_deref(),
    );

    spawn_chunk_entities(commands, meshes, &chunk_materials, &adt_data);
    spawn_obj0(commands, meshes, materials, images, inverse_bp, adt_path);

    let heightmap = TerrainHeightmap::from_adt(&adt_data);
    commands.insert_resource(heightmap);

    let result = compute_spawn_result(&adt_data);
    let tex_count = tex_data.as_ref().map_or(0, |td| td.texture_fdids.len());
    eprintln!(
        "Spawned ADT terrain: {} chunks, {} ground textures from {}",
        adt_data.chunks.len(), tex_count, adt_path.display(),
    );
    Ok(result)
}

/// Try to load the companion _tex0.adt file.
fn load_tex0(adt_path: &Path) -> Option<adt::AdtTexData> {
    let stem = adt_path.file_stem()?.to_str()?;
    let tex0_name = format!("{stem}_tex0.adt");
    let tex0_path = adt_path.with_file_name(tex0_name);
    let data = std::fs::read(&tex0_path).ok()?;
    match adt::load_adt_tex0(&data) {
        Ok(td) => {
            eprintln!("Loaded _tex0: {} textures, {} chunks", td.texture_fdids.len(), td.chunk_layers.len());
            Some(td)
        }
        Err(e) => {
            eprintln!("Failed to parse _tex0: {e}");
            None
        }
    }
}


fn spawn_chunk_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    chunk_materials: &[Handle<TerrainMaterial>],
    adt_data: &adt::AdtData,
) {
    let root = commands
        .spawn((AdtTerrain, Transform::default(), Visibility::default()))
        .id();

    for (i, chunk) in adt_data.chunks.iter().enumerate() {
        let mesh_handle = meshes.add(chunk.mesh.clone());
        let mat = chunk_materials.get(i).unwrap_or(&chunk_materials[0]);
        let mut spawn = commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(mat.clone()),
            Transform::default(),
            Visibility::default(),
        ));
        if let Some(grid) = adt_data.height_grids.get(i) {
            spawn.insert(wow_engine::culling::TerrainChunk {
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
}

fn compute_spawn_result(adt_data: &adt::AdtData) -> AdtSpawnResult {
    let center: Vec3 = adt_data.center_surface.into();
    let (min, max) = adt_data.bounds();
    let extent = (max - min).length();

    // Northshire Abbey WMO position
    let abbey = Vec3::new(17245.0, 25964.0, -80.0);
    let eye = abbey + Vec3::new(200.0, 50.0, 200.0);
    let camera = Transform::from_translation(eye).looking_at(abbey, Vec3::Y);

    AdtSpawnResult { camera, center }
}

// ── _obj0.adt object spawning ────────────────────────────────────────────────

/// Try to load the companion _obj0.adt file.
fn load_obj0(adt_path: &Path) -> Option<adt_obj::AdtObjData> {
    let stem = adt_path.file_stem()?.to_str()?;
    let obj0_name = format!("{stem}_obj0.adt");
    let obj0_path = adt_path.with_file_name(obj0_name);
    let data = std::fs::read(&obj0_path).ok()?;
    match adt_obj::load_adt_obj0(&data) {
        Ok(obj) => Some(obj),
        Err(e) => {
            eprintln!("Failed to parse _obj0: {e}");
            None
        }
    }
}

/// Load _obj0.adt and spawn doodads + WMOs.
fn spawn_obj0(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    adt_path: &Path,
) {
    let Some(obj_data) = load_obj0(adt_path) else { return };
    spawn_obj0_doodads(commands, meshes, materials, images, inverse_bp, &obj_data);
    spawn_obj0_wmos(commands, meshes, materials, images, &obj_data);
}

/// Spawn doodads (M2 models) from _obj0 placement data.
fn spawn_obj0_doodads(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    obj_data: &adt_obj::AdtObjData,
) {
    let mut spawned = 0u32;
    for doodad in &obj_data.doodads {
        if try_spawn_doodad(commands, meshes, materials, images, inverse_bp, doodad) {
            spawned += 1;
        }
    }
    eprintln!("Spawned {spawned}/{} doodads", obj_data.doodads.len());
}

/// Try to spawn a single doodad. Returns true if the M2 was found and spawned.
fn try_spawn_doodad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    doodad: &adt_obj::DoodadPlacement,
) -> bool {
    let Some(m2_path) = resolve_doodad_m2(doodad) else { return false };
    if !m2_path.exists() {
        return false;
    }
    let transform = doodad_transform(doodad);
    let Some(entity) = super::spawn_static_m2(commands, meshes, materials, images, inverse_bp, &m2_path, transform) else {
        return false;
    };
    commands.entity(entity).insert(wow_engine::culling::Doodad);
    true
}

/// Resolve a doodad placement to a local M2 file path.
fn resolve_doodad_m2(doodad: &adt_obj::DoodadPlacement) -> Option<std::path::PathBuf> {
    // If we have a direct FDID, look it up in the listfile for the path.
    if let Some(fdid) = doodad.fdid {
        return Some(std::path::PathBuf::from(format!("data/models/{fdid}.m2")));
    }
    // Otherwise resolve the WoW path to an FDID via listfile.
    let wow_path = doodad.path.as_ref()?;
    let fdid = wow_engine::listfile::lookup_path(wow_path)?;
    Some(std::path::PathBuf::from(format!("data/models/{fdid}.m2")))
}

/// Convert WoW doodad placement (position + Euler degrees + scale) to a Bevy Transform.
fn doodad_transform(d: &adt_obj::DoodadPlacement) -> Transform {
    use super::asset::m2::wow_to_bevy;
    let pos = wow_to_bevy(d.position[0], d.position[1], d.position[2]);
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
    // WoW rotations: Y-up, applied as Y→X→Z. Remap to Bevy coordinate system.
    Quat::from_euler(EulerRot::YXZ, ry, rx, rz)
}

// ── _obj0.adt WMO spawning ──────────────────────────────────────────────────

/// Spawn WMOs from _obj0 placement data.
fn spawn_obj0_wmos(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    obj_data: &adt_obj::AdtObjData,
) {
    let mut spawned = 0u32;
    for placement in &obj_data.wmos {
        if try_spawn_wmo(commands, meshes, materials, images, placement) {
            spawned += 1;
        }
    }
    eprintln!("Spawned {spawned}/{} WMOs", obj_data.wmos.len());
}

/// Try to spawn a single WMO. Returns true if root file was found and spawned.
fn try_spawn_wmo(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    placement: &adt_obj::WmoPlacement,
) -> bool {
    let Some(root_fdid) = resolve_wmo_fdid(placement) else { return false };
    let root_path = format!("data/models/{root_fdid}.wmo");
    let Ok(root_data) = std::fs::read(&root_path) else { return false };
    let Ok(root) = wmo::load_wmo_root(&root_data) else {
        eprintln!("Failed to parse WMO root {root_fdid}");
        return false;
    };

    let group_fdids = resolve_wmo_group_fdids(root_fdid, root.n_groups);
    let transform = wmo_transform(placement);
    let root_entity = commands
        .spawn((
            Name::new(format!("wmo_{root_fdid}")),
            transform,
            Visibility::default(),
            wow_engine::culling::Wmo,
        ))
        .id();

    let mut group_count = 0u32;
    for (i, group_fdid) in group_fdids.iter().enumerate() {
        let Some(fdid) = group_fdid else { continue };
        if spawn_wmo_group(commands, meshes, materials, images, &root, *fdid, root_entity) {
            group_count += 1;
        } else {
            eprintln!("  WMO {root_fdid} group {i}: missing or failed (FDID {fdid})");
        }
    }

    let pos = transform.translation;
    eprintln!(
        "WMO {root_fdid}: {group_count}/{} groups, {} materials, pos=[{:.0}, {:.0}, {:.0}]",
        root.n_groups, root.materials.len(), pos.x, pos.y, pos.z,
    );
    group_count > 0
}

/// Resolve a WMO placement to its root FileDataID.
fn resolve_wmo_fdid(wmo: &adt_obj::WmoPlacement) -> Option<u32> {
    if let Some(fdid) = wmo.fdid {
        return Some(fdid);
    }
    let wow_path = wmo.path.as_ref()?;
    wow_engine::listfile::lookup_path(wow_path)
}

/// Resolve group file FDIDs by looking up root path in listfile and deriving group paths.
fn resolve_wmo_group_fdids(root_fdid: u32, n_groups: u32) -> Vec<Option<u32>> {
    let Some(root_path) = wow_engine::listfile::lookup_fdid(root_fdid) else {
        eprintln!("  WMO {root_fdid}: not in listfile, cannot resolve group FDIDs");
        return vec![None; n_groups as usize];
    };

    let base = root_path.trim_end_matches(".wmo");
    (0..n_groups)
        .map(|i| {
            let group_path = format!("{base}_{i:03}.wmo");
            wow_engine::listfile::lookup_path(&group_path)
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
    let Ok(group) = wmo::load_wmo_group(&data) else {
        eprintln!("Failed to parse WMO group {group_fdid}");
        return false;
    };

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

/// Build a Bevy material for a WMO batch, loading the BLP texture if available.
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
        let blp_path = format!("data/textures/{texture_fdid}.blp");
        if let Ok((pixels, w, h)) = blp::load_blp_rgba(std::path::Path::new(&blp_path)) {
            let image = Image::new(
                bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                bevy::render::render_resource::TextureDimension::D2,
                pixels,
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::default(),
            );
            return materials.add(wmo_standard_material(Some(images.add(image)), blend_mode));
        }
    }

    // Fallback: gray material
    materials.add(wmo_standard_material(None, blend_mode))
}

fn wmo_standard_material(
    texture: Option<Handle<Image>>,
    blend_mode: u32,
) -> StandardMaterial {
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

/// Convert WMO placement to a Bevy Transform (same convention as doodads).
fn wmo_transform(w: &adt_obj::WmoPlacement) -> Transform {
    use super::asset::m2::wow_to_bevy;
    let pos = wow_to_bevy(w.position[0], w.position[1], w.position[2]);
    let rotation = doodad_rotation(w.rotation);
    Transform::from_translation(Vec3::from(pos))
        .with_rotation(rotation)
        .with_scale(Vec3::splat(w.scale))
}
