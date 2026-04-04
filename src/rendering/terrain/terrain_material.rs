use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;

use crate::asset::adt;
use crate::rendering::image_sampler::{clamp_linear_sampler, repeat_linear_sampler};

/// Custom terrain material: ground texture layers + alpha blending + hex tiling.
/// Replaces CPU compositing with GPU-side sampling for anti-tiling.
/// Uses height-based blending (ground texture alpha = height channel)
/// for more natural transitions between terrain layers.
#[derive(bevy::render::render_resource::ShaderType, Clone)]
pub struct TerrainMaterialSettings {
    /// x = layer_count (1-4), y = global_height_blend_strength,
    /// z = perceptual_roughness, w = reflectance
    pub config: Vec4,
    /// x = height_scale, y = height_offset
    pub layer_params_0: Vec4,
    pub layer_params_1: Vec4,
    pub layer_params_2: Vec4,
    pub layer_params_3: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub settings: TerrainMaterialSettings,

    #[texture(1)]
    #[sampler(2)]
    pub ground_0: Handle<Image>,

    #[texture(3)]
    #[sampler(4)]
    pub ground_1: Handle<Image>,

    #[texture(5)]
    #[sampler(6)]
    pub ground_2: Handle<Image>,

    #[texture(7)]
    #[sampler(8)]
    pub ground_3: Handle<Image>,

    /// Packed alpha: R=layer1, G=layer2, B=layer3. 64x64, ClampToEdge.
    #[texture(9)]
    #[sampler(10)]
    pub alpha_packed: Handle<Image>,

    /// Static per-chunk shadow mask expanded from MCSH. 64x64, ClampToEdge.
    #[texture(11)]
    #[sampler(12)]
    pub shadow_map: Handle<Image>,
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// 1x1 placeholder for unused texture slots.
pub fn placeholder_image(images: &mut Assets<Image>) -> Handle<Image> {
    let mut img = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![128, 128, 128, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = repeat_linear_sampler();
    images.add(img)
}

/// 1x1 black alpha texture (all layers transparent).
pub fn placeholder_alpha(images: &mut Assets<Image>) -> Handle<Image> {
    let mut img = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = clamp_linear_sampler();
    images.add(img)
}

/// Load ground BLP textures as Bevy Image handles with Repeat sampler.
pub fn load_ground_images(
    images: &mut Assets<Image>,
    tex_data: &adt::AdtTexData,
    adt_path: &std::path::Path,
) -> Vec<Option<Handle<Image>>> {
    eprintln!(
        "load_ground_images {} texture_fdids={}",
        adt_path.display(),
        tex_data.texture_fdids.len(),
    );
    let tex_dir = adt_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("../textures");
    tex_data
        .texture_fdids
        .iter()
        .map(|&spec_fdid| {
            let diffuse_fdid = resolve_diffuse_fdid(spec_fdid);
            let blp_path = crate::asset::asset_cache::texture(diffuse_fdid)
                .unwrap_or_else(|| tex_dir.join(format!("{diffuse_fdid}.blp")));
            load_blp_as_terrain_image(images, &blp_path, diffuse_fdid)
        })
        .collect()
}

/// Resolve specular FDID to its diffuse counterpart via listfile path lookup.
/// MDID stores specular FDIDs (e.g. `foo_s.blp`); the diffuse is `foo.blp`
/// which can have a completely different FDID. Falls back to spec_fdid - 1.
fn resolve_diffuse_fdid(spec_fdid: u32) -> u32 {
    if let Some(spec_path) = game_engine::listfile::lookup_fdid(spec_fdid) {
        let diffuse_path = spec_path
            .strip_suffix("_s.blp")
            .or_else(|| spec_path.strip_suffix("_S.blp"))
            .map(|base| format!("{base}.blp"));
        if let Some(dp) = diffuse_path
            && let Some(fdid) = game_engine::listfile::lookup_path(&dp)
        {
            return fdid;
        }
    }
    spec_fdid - 1
}

fn load_blp_as_terrain_image(
    images: &mut Assets<Image>,
    blp_path: &std::path::Path,
    fdid: u32,
) -> Option<Handle<Image>> {
    match crate::asset::blp::load_blp_gpu_image(blp_path) {
        Ok(mut img) => {
            eprintln!(
                "  Loaded ground texture FDID {fdid} ({:?})",
                img.texture_descriptor.format
            );
            img.sampler = repeat_linear_sampler();
            Some(images.add(img))
        }
        Err(e) => {
            eprintln!("  Missing ground texture FDID {fdid}: {e}");
            None
        }
    }
}

/// Pack up to 3 alpha maps (64x64 each) into a single RGB image.
/// R = layer 1 alpha, G = layer 2 alpha, B = layer 3 alpha.
pub fn pack_alpha_maps(images: &mut Assets<Image>, layers: &[adt::TextureLayer]) -> Handle<Image> {
    const SIZE: u32 = 64;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    for (li, layer) in layers.iter().enumerate().skip(1) {
        let channel = li - 1; // 0=R, 1=G, 2=B
        if channel >= 3 {
            break;
        }
        pack_alpha_channel(&mut rgba, layer.alpha_map.as_deref(), channel, SIZE);
    }
    // Set alpha channel to 255
    for i in 0..(SIZE * SIZE) as usize {
        rgba[i * 4 + 3] = 255;
    }

    let mut img = Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = clamp_linear_sampler();
    images.add(img)
}

fn pack_alpha_channel(rgba: &mut [u8], alpha: Option<&[u8]>, channel: usize, size: u32) {
    let Some(alpha) = alpha else { return };
    for i in 0..(size * size) as usize {
        let val = if i < alpha.len() { alpha[i] } else { 0 };
        rgba[i * 4 + channel] = val;
    }
}

pub fn pack_shadow_map(
    images: &mut Assets<Image>,
    shadow_map: Option<&[u8; 512]>,
) -> Handle<Image> {
    const SIZE: u32 = 64;
    let rgba = match shadow_map {
        Some(shadow_map) => pack_shadow_pixels(shadow_map, SIZE),
        None => default_shadow_pixels(SIZE),
    };
    images.add(new_shadow_image(rgba, SIZE))
}

fn default_shadow_pixels(size: u32) -> Vec<u8> {
    let mut rgba = vec![255u8; (size * size * 4) as usize];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    rgba
}

fn write_shadow_pixel(rgba: &mut [u8], size: usize, row: usize, col: usize, shadowed: bool) {
    let value = if shadowed { 0 } else { 255 };
    let base = (row * size + col) * 4;
    rgba[base] = value;
    rgba[base + 1] = value;
    rgba[base + 2] = value;
}

fn pack_shadow_pixels(shadow_map: &[u8; 512], size: u32) -> Vec<u8> {
    let mut rgba = default_shadow_pixels(size);
    let size = size as usize;
    for row in 0..size {
        for col in 0..size {
            write_shadow_pixel(
                &mut rgba,
                size,
                row,
                col,
                shadow_bit_is_set(shadow_map, row, col),
            );
        }
    }
    rgba
}

fn new_shadow_image(rgba: Vec<u8>, size: u32) -> Image {
    let mut img = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    img.sampler = clamp_linear_sampler();
    img
}

fn shadow_bit_is_set(shadow_map: &[u8; 512], row: usize, col: usize) -> bool {
    let byte = shadow_map[row * 8 + col / 8];
    let bit = col % 8;
    ((byte >> bit) & 1) != 0
}

/// Shared placeholder handles for fallback materials.
struct Placeholders {
    image: Handle<Image>,
    alpha: Handle<Image>,
}

/// Build one TerrainMaterial per MCNK chunk.
pub fn build_terrain_materials(
    terrain_materials: &mut Assets<TerrainMaterial>,
    images: &mut Assets<Image>,
    adt_data: &adt::AdtData,
    tex_data: Option<&adt::AdtTexData>,
    ground_images: Option<&[Option<Handle<Image>>]>,
) -> Vec<Handle<TerrainMaterial>> {
    let ph = Placeholders {
        image: placeholder_image(images),
        alpha: placeholder_alpha(images),
    };

    let (Some(td), Some(gi)) = (tex_data, ground_images) else {
        return adt_data
            .chunks
            .iter()
            .map(|chunk| {
                terrain_materials.add(fallback_material(images, chunk.shadow_map.as_ref(), &ph))
            })
            .collect();
    };

    td.chunk_layers
        .iter()
        .enumerate()
        .map(|(chunk_index, chunk_tex)| {
            let shadow_map = adt_data
                .chunks
                .get(chunk_index)
                .and_then(|chunk| chunk.shadow_map.as_ref());
            build_chunk_material(
                terrain_materials,
                images,
                td,
                chunk_tex,
                gi,
                shadow_map,
                &ph,
            )
        })
        .collect()
}

/// Height blend strength: how much the texture alpha channel influences
/// layer transitions. 0 = flat alpha blending, 2-4 = natural rocky edges.
const HEIGHT_BLEND_STRENGTH: f32 = 3.0;
const TERRAIN_PERCEPTUAL_ROUGHNESS: f32 = 0.95;
const TERRAIN_REFLECTANCE: f32 = 0.2;
const DEFAULT_LAYER_PARAMS: Vec4 = Vec4::new(1.0, 0.0, 0.0, 0.0);

fn terrain_settings(layer_count: f32, layer_params: [Vec4; 4]) -> TerrainMaterialSettings {
    TerrainMaterialSettings {
        config: Vec4::new(
            layer_count,
            HEIGHT_BLEND_STRENGTH,
            TERRAIN_PERCEPTUAL_ROUGHNESS,
            TERRAIN_REFLECTANCE,
        ),
        layer_params_0: layer_params[0],
        layer_params_1: layer_params[1],
        layer_params_2: layer_params[2],
        layer_params_3: layer_params[3],
    }
}

fn fallback_material(
    images: &mut Assets<Image>,
    shadow_map: Option<&[u8; 512]>,
    ph: &Placeholders,
) -> TerrainMaterial {
    TerrainMaterial {
        settings: terrain_settings(0.0, [DEFAULT_LAYER_PARAMS; 4]),
        ground_0: ph.image.clone(),
        ground_1: ph.image.clone(),
        ground_2: ph.image.clone(),
        ground_3: ph.image.clone(),
        alpha_packed: ph.alpha.clone(),
        shadow_map: pack_shadow_map(images, shadow_map),
    }
}

fn build_chunk_material(
    terrain_materials: &mut Assets<TerrainMaterial>,
    images: &mut Assets<Image>,
    tex_data: &adt::AdtTexData,
    chunk_tex: &adt::ChunkTexLayers,
    ground_images: &[Option<Handle<Image>>],
    shadow_map: Option<&[u8; 512]>,
    ph: &Placeholders,
) -> Handle<TerrainMaterial> {
    if chunk_tex.layers.is_empty() {
        return terrain_materials.add(fallback_material(images, shadow_map, ph));
    }

    let layer_count = chunk_tex.layers.len().min(4) as f32;
    let ground = |idx: usize| -> Handle<Image> {
        chunk_tex
            .layers
            .get(idx)
            .and_then(|l| ground_images.get(l.texture_index as usize))
            .and_then(|opt| opt.clone())
            .unwrap_or_else(|| ph.image.clone())
    };
    let layer_params = texture_layer_params(tex_data, &chunk_tex.layers);

    terrain_materials.add(TerrainMaterial {
        settings: terrain_settings(layer_count, layer_params),
        ground_0: ground(0),
        ground_1: ground(1),
        ground_2: ground(2),
        ground_3: ground(3),
        alpha_packed: pack_alpha_maps(images, &chunk_tex.layers),
        shadow_map: pack_shadow_map(images, shadow_map),
    })
}

fn texture_layer_params(tex_data: &adt::AdtTexData, layers: &[adt::TextureLayer]) -> [Vec4; 4] {
    let mut params = [DEFAULT_LAYER_PARAMS; 4];
    for (slot, layer) in layers.iter().take(4).enumerate() {
        params[slot] = tex_data
            .texture_params
            .get(layer.texture_index as usize)
            .map(|param| Vec4::new(param.height_scale, param.height_offset, 0.0, 0.0))
            .unwrap_or(DEFAULT_LAYER_PARAMS);
    }
    params
}

#[cfg(test)]
mod tests {
    use super::{pack_shadow_map, shadow_bit_is_set, texture_layer_params};
    use crate::asset::adt;
    use bevy::asset::Assets;
    use bevy::image::Image;
    use bevy::math::Vec4;

    const TEST_TEXTURE_PARAM_FLAG_0: u32 = 0x10;
    const TEST_TEXTURE_PARAM_FLAG_1: u32 = 0x20;

    #[test]
    fn pack_shadow_map_expands_mcsh_bits_to_64x64_pixels() {
        let mut images = Assets::<Image>::default();
        let mut shadow_map = [0u8; 512];
        shadow_map[0] = 0b0000_0001;
        shadow_map[1] = 0b0000_0001;

        assert!(shadow_bit_is_set(&shadow_map, 0, 0));
        assert!(shadow_bit_is_set(&shadow_map, 0, 8));

        let handle = pack_shadow_map(&mut images, Some(&shadow_map));
        let image = images.get(&handle).expect("expected shadow image");
        let data = image.data.as_ref().expect("expected shadow pixels");

        assert_eq!(image.texture_descriptor.size.width, 64);
        assert_eq!(image.texture_descriptor.size.height, 64);
        assert_eq!(&data[0..4], &[0, 0, 0, 255]);
        assert_eq!(&data[4..8], &[255, 255, 255, 255]);
        assert_eq!(&data[32..36], &[0, 0, 0, 255]);
    }

    #[test]
    fn texture_layer_params_use_mtxp_per_texture_index() {
        let tex_data = adt::AdtTexData {
            texture_fdids: vec![11, 22],
            height_texture_fdids: vec![],
            texture_flags: vec![0, 0],
            texture_params: vec![
                adt::TextureParams {
                    flags: TEST_TEXTURE_PARAM_FLAG_0,
                    height_scale: 1.25,
                    height_offset: -0.5,
                },
                adt::TextureParams {
                    flags: TEST_TEXTURE_PARAM_FLAG_1,
                    height_scale: 0.75,
                    height_offset: 0.125,
                },
            ],
            chunk_layers: vec![],
        };
        let layers = vec![
            adt::TextureLayer {
                texture_index: 1,
                flags: adt::MclyFlags::default(),
                effect_id: 0,
                alpha_map: None,
            },
            adt::TextureLayer {
                texture_index: 0,
                flags: adt::MclyFlags::default(),
                effect_id: 0,
                alpha_map: None,
            },
        ];

        let params = texture_layer_params(&tex_data, &layers);

        assert_eq!(params[0], Vec4::new(0.75, 0.125, 0.0, 0.0));
        assert_eq!(params[1], Vec4::new(1.25, -0.5, 0.0, 0.0));
    }
}
