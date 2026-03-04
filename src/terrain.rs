use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::asset::{adt, adt_obj, blp, wmo};

/// Marker component for the ADT terrain root entity.
#[derive(Component)]
pub struct AdtTerrain;

/// Result of spawning an ADT: camera and ground position for placing models.
pub struct AdtSpawnResult {
    pub camera: Transform,
    pub center: Vec3,
}

/// Composite resolution per MCNK chunk (pixels per side).
const COMPOSITE_SIZE: u32 = 256;
/// Ground textures tile this many times across one MCNK.
const TILE_REPEAT: f32 = 8.0;
/// Alpha map resolution in the ADT file.
const ALPHA_SIZE: u32 = 64;

/// A loaded ground texture: raw RGBA pixels + dimensions.
struct GroundTexture {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

/// Load an ADT file, build meshes, and spawn them into the Bevy scene.
pub fn spawn_adt(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    adt_path: &Path,
) -> Result<AdtSpawnResult, String> {
    let data = std::fs::read(adt_path)
        .map_err(|e| format!("Failed to read {}: {e}", adt_path.display()))?;
    let adt_data = adt::load_adt(&data)?;

    let tex_data = load_tex0(adt_path);
    let ground_textures = tex_data.as_ref().map(|td| load_ground_textures(td, adt_path));

    let chunk_materials = build_chunk_materials(
        materials, images, tex_data.as_ref(), ground_textures.as_ref(),
    );

    spawn_chunk_entities(commands, meshes, &chunk_materials, &adt_data);
    spawn_obj0(commands, meshes, materials, images, inverse_bp, adt_path);

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

/// Load all ground BLP textures referenced by the tex0 data.
/// MDID stores specular FDIDs; diffuse = FDID - 1.
fn load_ground_textures(tex_data: &adt::AdtTexData, adt_path: &Path) -> Vec<Option<GroundTexture>> {
    let tex_dir = adt_path.parent().unwrap_or(Path::new(".")).join("../textures");
    tex_data.texture_fdids.iter().map(|&spec_fdid| {
        let diffuse_fdid = spec_fdid - 1;
        let blp_path = tex_dir.join(format!("{diffuse_fdid}.blp"));
        match blp::load_blp_rgba(&blp_path) {
            Ok((pixels, w, h)) => {
                eprintln!("  Loaded ground texture FDID {diffuse_fdid} ({w}×{h})");
                Some(GroundTexture { pixels, width: w, height: h })
            }
            Err(e) => {
                eprintln!("  Missing ground texture FDID {diffuse_fdid}: {e}");
                None
            }
        }
    }).collect()
}

/// Build one material per MCNK chunk. Textured if data available, green fallback otherwise.
fn build_chunk_materials(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    tex_data: Option<&adt::AdtTexData>,
    ground_textures: Option<&Vec<Option<GroundTexture>>>,
) -> Vec<Handle<StandardMaterial>> {
    let fallback = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.55, 0.25),
        perceptual_roughness: 0.9,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let (Some(td), Some(gt)) = (tex_data, ground_textures) else {
        return vec![fallback; 256];
    };

    td.chunk_layers.iter().map(|chunk_tex| {
        if chunk_tex.layers.is_empty() {
            return fallback.clone();
        }
        match composite_chunk_texture(&chunk_tex.layers, gt) {
            Some(rgba) => {
                let image = rgba_to_image(rgba, COMPOSITE_SIZE, COMPOSITE_SIZE);
                let image_handle = images.add(image);
                materials.add(StandardMaterial {
                    base_color_texture: Some(image_handle),
                    perceptual_roughness: 0.9,
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                })
            }
            None => fallback.clone(),
        }
    }).collect()
}

/// Composite all texture layers for one MCNK into a single RGBA image.
fn composite_chunk_texture(
    layers: &[adt::TextureLayer],
    ground_textures: &[Option<GroundTexture>],
) -> Option<Vec<u8>> {
    let size = COMPOSITE_SIZE as usize;
    let mut rgba = vec![0u8; size * size * 4];

    for (li, layer) in layers.iter().enumerate() {
        let tex = ground_textures.get(layer.texture_index as usize)?.as_ref()?;
        blend_layer(&mut rgba, tex, layer.alpha_map.as_deref(), li == 0);
    }

    Some(rgba)
}

/// Blend one ground texture layer into the composite buffer.
fn blend_layer(
    rgba: &mut [u8],
    tex: &GroundTexture,
    alpha_map: Option<&[u8]>,
    is_base: bool,
) {
    let size = COMPOSITE_SIZE as usize;
    for y in 0..size {
        for x in 0..size {
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            let color = sample_tiled(tex, u, v);
            let alpha = if is_base { 255 } else { sample_alpha(alpha_map, u, v) };
            blend_pixel(rgba, y * size + x, color, alpha);
        }
    }
}

/// Sample a ground texture at tiled UV coordinates.
fn sample_tiled(tex: &GroundTexture, u: f32, v: f32) -> [u8; 3] {
    let tx = ((u * TILE_REPEAT).fract() * tex.width as f32) as u32 % tex.width;
    let ty = ((v * TILE_REPEAT).fract() * tex.height as f32) as u32 % tex.height;
    let idx = ((ty * tex.width + tx) * 4) as usize;
    [tex.pixels[idx], tex.pixels[idx + 1], tex.pixels[idx + 2]]
}

/// Sample the 64×64 alpha map with bilinear interpolation.
fn sample_alpha(alpha_map: Option<&[u8]>, u: f32, v: f32) -> u8 {
    let Some(map) = alpha_map else { return 255 };
    let fx = u * (ALPHA_SIZE - 1) as f32;
    let fy = v * (ALPHA_SIZE - 1) as f32;
    let x0 = (fx as u32).min(ALPHA_SIZE - 2);
    let y0 = (fy as u32).min(ALPHA_SIZE - 2);
    let dx = fx - x0 as f32;
    let dy = fy - y0 as f32;
    let i = |x: u32, y: u32| map[(y * ALPHA_SIZE + x) as usize] as f32;
    let val = i(x0, y0) * (1.0 - dx) * (1.0 - dy)
        + i(x0 + 1, y0) * dx * (1.0 - dy)
        + i(x0, y0 + 1) * (1.0 - dx) * dy
        + i(x0 + 1, y0 + 1) * dx * dy;
    val as u8
}

/// Alpha-blend a single pixel into the composite buffer.
fn blend_pixel(rgba: &mut [u8], pixel_idx: usize, color: [u8; 3], alpha: u8) {
    let i = pixel_idx * 4;
    if alpha == 255 {
        rgba[i] = color[0];
        rgba[i + 1] = color[1];
        rgba[i + 2] = color[2];
        rgba[i + 3] = 255;
    } else if alpha > 0 {
        let a = alpha as u16;
        let inv = 255 - a;
        rgba[i] = ((a * color[0] as u16 + inv * rgba[i] as u16) / 255) as u8;
        rgba[i + 1] = ((a * color[1] as u16 + inv * rgba[i + 1] as u16) / 255) as u8;
        rgba[i + 2] = ((a * color[2] as u16 + inv * rgba[i + 2] as u16) / 255) as u8;
        rgba[i + 3] = 255;
    }
}

fn rgba_to_image(pixels: Vec<u8>, width: u32, height: u32) -> Image {
    Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn spawn_chunk_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    chunk_materials: &[Handle<StandardMaterial>],
    adt_data: &adt::AdtData,
) {
    let root = commands
        .spawn((AdtTerrain, Transform::default(), Visibility::default()))
        .id();

    for (i, chunk) in adt_data.chunks.iter().enumerate() {
        let mesh_handle = meshes.add(chunk.mesh.clone());
        let mat = chunk_materials.get(i).unwrap_or(&chunk_materials[0]);
        let child = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(mat.clone()),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
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
    super::spawn_static_m2(commands, meshes, materials, images, inverse_bp, &m2_path, transform);
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
                RenderAssetUsages::default(),
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
