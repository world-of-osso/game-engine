use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::asset::{adt, blp};

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
    adt_path: &Path,
) -> Result<AdtSpawnResult, String> {
    let data = std::fs::read(adt_path)
        .map_err(|e| format!("Failed to read {}: {e}", adt_path.display()))?;
    let adt_data = adt::load_adt(&data)?;

    let tex_data = load_tex0(adt_path);
    let ground_textures = tex_data.as_ref().map(|td| load_ground_textures(td, adt_path));

    let chunk_materials = build_chunk_materials(
        materials,
        images,
        tex_data.as_ref(),
        ground_textures.as_ref(),
    );

    spawn_chunk_entities(commands, meshes, &chunk_materials, &adt_data);

    let result = compute_spawn_result(&adt_data);
    let tex_count = tex_data.as_ref().map_or(0, |td| td.texture_fdids.len());
    eprintln!(
        "Spawned ADT terrain: {} chunks, {} ground textures from {}",
        adt_data.chunks.len(),
        tex_count,
        adt_path.display(),
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

    let eye = Vec3::new(center.x, center.y + extent * 0.5, center.z + extent * 0.3);
    let camera = Transform::from_translation(eye).looking_at(center, Vec3::Y);

    AdtSpawnResult { camera, center }
}
