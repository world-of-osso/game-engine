use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor,
    TextureViewDimension,
};
use bevy::shader::ShaderRef;
use std::f32::consts::FRAC_PI_4;

use crate::asset::adt;
use crate::rendering::image_sampler::{clamp_linear_sampler, repeat_linear_sampler};
use crate::sky::SkyEnvMapHandle;

/// Custom terrain material: ground texture layers + alpha blending + hex tiling.
/// Replaces CPU compositing with GPU-side sampling for anti-tiling.
/// Uses height-based blending (ground texture alpha = height channel)
/// for more natural transitions between terrain layers.
#[derive(bevy::render::render_resource::ShaderType, Clone)]
pub struct TerrainMaterialSettings {
    /// x = layer_count (1-4), y = global_height_blend_strength,
    /// z = texture_repeat, w = animation_time
    pub config: Vec4,
    /// x = perceptual_roughness, y = reflectance
    pub surface: Vec4,
    /// x = height_scale, y = height_offset, z = material_id, w = overbright multiplier
    pub layer_params_0: Vec4,
    pub layer_params_1: Vec4,
    pub layer_params_2: Vec4,
    pub layer_params_3: Vec4,
    /// x/y = UV velocity, z = reflection multiplier, w = reserved
    pub animation_params_0: Vec4,
    pub animation_params_1: Vec4,
    pub animation_params_2: Vec4,
    pub animation_params_3: Vec4,
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

    #[texture(9)]
    #[sampler(10)]
    pub height_0: Handle<Image>,

    #[texture(11)]
    #[sampler(12)]
    pub height_1: Handle<Image>,

    #[texture(13)]
    #[sampler(14)]
    pub height_2: Handle<Image>,

    #[texture(15)]
    #[sampler(16)]
    pub height_3: Handle<Image>,

    /// Packed alpha: R=layer1, G=layer2, B=layer3. 64x64, ClampToEdge.
    #[texture(17)]
    #[sampler(18)]
    pub alpha_packed: Handle<Image>,

    /// Static per-chunk shadow mask expanded from MCSH. 64x64, ClampToEdge.
    #[texture(19)]
    #[sampler(20)]
    pub shadow_map: Handle<Image>,

    #[texture(21, dimension = "cube")]
    #[sampler(22)]
    pub environment_map: Handle<Image>,
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

pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(
                Update,
                (update_terrain_animation_time, sync_terrain_environment_map),
            );
    }
}

fn update_terrain_animation_time(
    time: Res<Time>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
) {
    let animation_time = time.elapsed_secs();
    for (_id, material) in terrain_materials.iter_mut() {
        material.settings.config.w = animation_time;
    }
}

fn sync_terrain_environment_map(
    env_handle: Option<Res<SkyEnvMapHandle>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
) {
    let Some(env_handle) = env_handle else { return };
    for (_id, material) in terrain_materials.iter_mut() {
        if material.environment_map != env_handle.0 {
            material.environment_map = env_handle.0.clone();
        }
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

pub fn placeholder_cubemap(images: &mut Assets<Image>) -> Handle<Image> {
    let mut img = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        vec![
            0, 56, 0, 56, 0, 56, 0, 60, 0, 56, 0, 56, 0, 56, 0, 60, 0, 56, 0, 56, 0, 56, 0, 60, 0,
            56, 0, 56, 0, 56, 0, 60, 0, 56, 0, 56, 0, 56, 0, 60, 0, 56, 0, 56, 0, 56, 0, 60,
        ],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::default(),
    );
    img.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..Default::default()
    });
    img.sampler = repeat_linear_sampler();
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

pub fn load_height_images(
    images: &mut Assets<Image>,
    tex_data: &adt::AdtTexData,
    adt_path: &std::path::Path,
) -> Vec<Option<Handle<Image>>> {
    eprintln!(
        "load_height_images {} height_texture_fdids={}",
        adt_path.display(),
        tex_data.height_texture_fdids.len(),
    );
    let tex_dir = adt_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("../textures");
    tex_data
        .height_texture_fdids
        .iter()
        .map(|&fdid| {
            let blp_path = crate::asset::asset_cache::texture(fdid)
                .unwrap_or_else(|| tex_dir.join(format!("{fdid}.blp")));
            load_blp_as_terrain_image(images, &blp_path, fdid)
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
    decode_blp_terrain_image(blp_path, fdid).map(|img| images.add(img))
}

pub fn decode_blp_terrain_image(blp_path: &std::path::Path, fdid: u32) -> Option<Image> {
    match crate::asset::blp::load_blp_gpu_image(blp_path) {
        Ok(mut img) => {
            eprintln!(
                "  Loaded ground texture FDID {fdid} ({:?})",
                img.texture_descriptor.format
            );
            img.sampler = repeat_linear_sampler();
            Some(img)
        }
        Err(e) => {
            eprintln!("  Missing ground texture FDID {fdid}: {e}");
            None
        }
    }
}

pub fn decode_ground_images(
    tex_data: &adt::AdtTexData,
    adt_path: &std::path::Path,
) -> Vec<Option<Image>> {
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
            decode_blp_terrain_image(&blp_path, diffuse_fdid)
        })
        .collect()
}

pub fn decode_height_images(
    tex_data: &adt::AdtTexData,
    adt_path: &std::path::Path,
) -> Vec<Option<Image>> {
    let tex_dir = adt_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("../textures");
    tex_data
        .height_texture_fdids
        .iter()
        .map(|&fdid| {
            let blp_path = crate::asset::asset_cache::texture(fdid)
                .unwrap_or_else(|| tex_dir.join(format!("{fdid}.blp")));
            decode_blp_terrain_image(&blp_path, fdid)
        })
        .collect()
}

pub fn register_decoded_images(
    images: &mut Assets<Image>,
    decoded: &[Option<Image>],
) -> Vec<Option<Handle<Image>>> {
    decoded
        .iter()
        .map(|opt| opt.as_ref().map(|img| images.add(img.clone())))
        .collect()
}

/// Pack up to 3 alpha maps (64x64 each) into a single RGB image.
/// R = layer 1 alpha, G = layer 2 alpha, B = layer 3 alpha.
pub fn pack_alpha_maps(images: &mut Assets<Image>, layers: &[adt::TextureLayer]) -> Handle<Image> {
    images.add(pack_alpha_map_raw(layers))
}

pub fn pack_alpha_map_raw(layers: &[adt::TextureLayer]) -> Image {
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
    img
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
    images.add(pack_shadow_map_raw(shadow_map))
}

pub fn pack_shadow_map_raw(shadow_map: Option<&[u8; 512]>) -> Image {
    const SIZE: u32 = 64;
    let rgba = match shadow_map {
        Some(shadow_map) => pack_shadow_pixels(shadow_map, SIZE),
        None => default_shadow_pixels(SIZE),
    };
    new_shadow_image(rgba, SIZE)
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
    cubemap: Handle<Image>,
}

/// Build one TerrainMaterial per MCNK chunk.
pub fn build_terrain_materials(
    terrain_materials: &mut Assets<TerrainMaterial>,
    images: &mut Assets<Image>,
    adt_data: &adt::AdtData,
    tex_data: Option<&adt::AdtTexData>,
    ground_images: Option<&[Option<Handle<Image>>]>,
    height_images: Option<&[Option<Handle<Image>>]>,
    pre_alpha: Option<&[Handle<Image>]>,
    pre_shadow: Option<&[Handle<Image>]>,
) -> Vec<Handle<TerrainMaterial>> {
    let ph = Placeholders {
        image: placeholder_image(images),
        alpha: placeholder_alpha(images),
        cubemap: placeholder_cubemap(images),
    };

    let (Some(td), Some(gi)) = (tex_data, ground_images) else {
        return build_fallback_materials(terrain_materials, images, adt_data, pre_shadow, &ph);
    };

    td.chunk_layers
        .iter()
        .enumerate()
        .map(|(chunk_index, chunk_tex)| {
            let shadow_map = adt_data
                .chunks
                .get(chunk_index)
                .and_then(|chunk| chunk.shadow_map.as_ref());
            let pre_al = pre_alpha.and_then(|a| a.get(chunk_index));
            let pre_sh = pre_shadow.and_then(|s| s.get(chunk_index));
            build_chunk_material(
                terrain_materials,
                images,
                td,
                chunk_tex,
                gi,
                height_images,
                shadow_map,
                &ph,
                pre_al,
                pre_sh,
            )
        })
        .collect()
}

fn build_fallback_materials(
    terrain_materials: &mut Assets<TerrainMaterial>,
    images: &mut Assets<Image>,
    adt_data: &adt::AdtData,
    pre_shadow: Option<&[Handle<Image>]>,
    ph: &Placeholders,
) -> Vec<Handle<TerrainMaterial>> {
    adt_data
        .chunks
        .iter()
        .enumerate()
        .map(|(chunk_index, chunk)| {
            let pre_sh = pre_shadow.and_then(|s| s.get(chunk_index));
            terrain_materials.add(fallback_material(
                images,
                chunk.shadow_map.as_ref(),
                ph,
                pre_sh,
            ))
        })
        .collect()
}

/// Height blend strength: how much the texture alpha channel influences
/// layer transitions. 0 = flat alpha blending, 2-4 = natural rocky edges.
const HEIGHT_BLEND_STRENGTH: f32 = 3.0;
const BASE_TERRAIN_TEXTURE_REPEAT: f32 = 8.0;
const TERRAIN_PERCEPTUAL_ROUGHNESS: f32 = 0.95;
const TERRAIN_REFLECTANCE: f32 = 0.2;
const TERRAIN_OVERBRIGHT_MULTIPLIER: f32 = 2.0;
const DEFAULT_LAYER_PARAMS: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
const TERRAIN_ANIMATION_SPEEDS: [f32; 8] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 48.0, 64.0];
const TERRAIN_ANIMATION_BASE_SPEED: f32 = 0.176_776_69;

fn terrain_settings(
    layer_count: f32,
    texture_repeat: f32,
    layer_params: [Vec4; 4],
    animation_params: [Vec4; 4],
) -> TerrainMaterialSettings {
    TerrainMaterialSettings {
        config: Vec4::new(layer_count, HEIGHT_BLEND_STRENGTH, texture_repeat, 0.0),
        surface: Vec4::new(TERRAIN_PERCEPTUAL_ROUGHNESS, TERRAIN_REFLECTANCE, 0.0, 0.0),
        layer_params_0: layer_params[0],
        layer_params_1: layer_params[1],
        layer_params_2: layer_params[2],
        layer_params_3: layer_params[3],
        animation_params_0: animation_params[0],
        animation_params_1: animation_params[1],
        animation_params_2: animation_params[2],
        animation_params_3: animation_params[3],
    }
}

fn fallback_material(
    images: &mut Assets<Image>,
    shadow_map: Option<&[u8; 512]>,
    ph: &Placeholders,
    pre_shadow: Option<&Handle<Image>>,
) -> TerrainMaterial {
    TerrainMaterial {
        settings: terrain_settings(
            0.0,
            BASE_TERRAIN_TEXTURE_REPEAT,
            [DEFAULT_LAYER_PARAMS; 4],
            [Vec4::ZERO; 4],
        ),
        ground_0: ph.image.clone(),
        ground_1: ph.image.clone(),
        ground_2: ph.image.clone(),
        ground_3: ph.image.clone(),
        height_0: ph.image.clone(),
        height_1: ph.image.clone(),
        height_2: ph.image.clone(),
        height_3: ph.image.clone(),
        alpha_packed: ph.alpha.clone(),
        shadow_map: pre_shadow
            .cloned()
            .unwrap_or_else(|| pack_shadow_map(images, shadow_map)),
        environment_map: ph.cubemap.clone(),
    }
}

fn build_chunk_material(
    terrain_materials: &mut Assets<TerrainMaterial>,
    images: &mut Assets<Image>,
    tex_data: &adt::AdtTexData,
    chunk_tex: &adt::ChunkTexLayers,
    ground_images: &[Option<Handle<Image>>],
    height_images: Option<&[Option<Handle<Image>>]>,
    shadow_map: Option<&[u8; 512]>,
    ph: &Placeholders,
    pre_alpha: Option<&Handle<Image>>,
    pre_shadow: Option<&Handle<Image>>,
) -> Handle<TerrainMaterial> {
    if chunk_tex.layers.is_empty() {
        return terrain_materials.add(fallback_material(images, shadow_map, ph, pre_shadow));
    }

    let layer_count = chunk_tex.layers.len().min(4) as f32;
    let ground_handles = resolve_chunk_ground_images(chunk_tex, ground_images, ph);
    let height_handles = resolve_chunk_height_images(chunk_tex, &ground_handles, height_images);
    let layer_params = texture_layer_params(tex_data, &chunk_tex.layers);
    let animation_params = terrain_layer_animation_params(&chunk_tex.layers);
    let texture_repeat = terrain_texture_repeat(tex_data.texture_amplifier);

    terrain_materials.add(TerrainMaterial {
        settings: terrain_settings(layer_count, texture_repeat, layer_params, animation_params),
        ground_0: ground_handles[0].clone(),
        ground_1: ground_handles[1].clone(),
        ground_2: ground_handles[2].clone(),
        ground_3: ground_handles[3].clone(),
        height_0: height_handles[0].clone(),
        height_1: height_handles[1].clone(),
        height_2: height_handles[2].clone(),
        height_3: height_handles[3].clone(),
        alpha_packed: pre_alpha
            .cloned()
            .unwrap_or_else(|| pack_alpha_maps(images, &chunk_tex.layers)),
        shadow_map: pre_shadow
            .cloned()
            .unwrap_or_else(|| pack_shadow_map(images, shadow_map)),
        environment_map: ph.cubemap.clone(),
    })
}

fn resolve_chunk_ground_images(
    chunk_tex: &adt::ChunkTexLayers,
    ground_images: &[Option<Handle<Image>>],
    ph: &Placeholders,
) -> [Handle<Image>; 4] {
    std::array::from_fn(|idx| resolve_chunk_ground_image(chunk_tex, ground_images, idx, ph))
}

fn resolve_chunk_ground_image(
    chunk_tex: &adt::ChunkTexLayers,
    ground_images: &[Option<Handle<Image>>],
    idx: usize,
    ph: &Placeholders,
) -> Handle<Image> {
    chunk_tex
        .layers
        .get(idx)
        .and_then(|layer| ground_images.get(layer.texture_index as usize))
        .and_then(|image| image.clone())
        .unwrap_or_else(|| ph.image.clone())
}

fn resolve_chunk_height_images(
    chunk_tex: &adt::ChunkTexLayers,
    ground_handles: &[Handle<Image>; 4],
    height_images: Option<&[Option<Handle<Image>>]>,
) -> [Handle<Image>; 4] {
    std::array::from_fn(|idx| {
        resolve_chunk_height_image(chunk_tex, ground_handles, height_images, idx)
    })
}

fn resolve_chunk_height_image(
    chunk_tex: &adt::ChunkTexLayers,
    ground_handles: &[Handle<Image>; 4],
    height_images: Option<&[Option<Handle<Image>>]>,
    idx: usize,
) -> Handle<Image> {
    chunk_tex
        .layers
        .get(idx)
        .and_then(|layer| {
            height_images
                .and_then(|images| images.get(layer.texture_index as usize))
                .and_then(|image| image.clone())
        })
        .unwrap_or_else(|| ground_handles[idx].clone())
}

fn terrain_texture_repeat(texture_amplifier: Option<u32>) -> f32 {
    let exponent = texture_amplifier.unwrap_or(0).min(8) as i32;
    BASE_TERRAIN_TEXTURE_REPEAT * 2.0f32.powi(exponent)
}

fn texture_layer_params(tex_data: &adt::AdtTexData, layers: &[adt::TextureLayer]) -> [Vec4; 4] {
    let mut params = [DEFAULT_LAYER_PARAMS; 4];
    for (slot, layer) in layers.iter().take(4).enumerate() {
        let overbright_multiplier = if layer.flags.overbright() {
            TERRAIN_OVERBRIGHT_MULTIPLIER
        } else {
            1.0
        };
        params[slot] = tex_data
            .texture_params
            .get(layer.texture_index as usize)
            .map(|param| {
                Vec4::new(
                    param.height_scale,
                    param.height_offset,
                    f32::from(layer.material_id),
                    overbright_multiplier,
                )
            })
            .unwrap_or(Vec4::new(
                1.0,
                0.0,
                f32::from(layer.material_id),
                overbright_multiplier,
            ));
    }
    params
}

fn terrain_layer_animation_params(layers: &[adt::TextureLayer]) -> [Vec4; 4] {
    let mut params = [Vec4::ZERO; 4];
    for (slot, layer) in layers.iter().take(4).enumerate() {
        params[slot] = terrain_layer_animation(layer.flags);
    }
    params
}

fn terrain_layer_animation(flags: adt::MclyFlags) -> Vec4 {
    let velocity = if flags.animation_enabled() {
        let speed = TERRAIN_ANIMATION_SPEEDS[flags.animation_speed() as usize]
            * TERRAIN_ANIMATION_BASE_SPEED;
        let angle = FRAC_PI_4 + f32::from(flags.animation_rotation()) * FRAC_PI_4;
        let (sin, cos) = angle.sin_cos();
        let base = Vec2::splat(speed);
        Vec2::new(base.x * cos - base.y * sin, base.x * sin + base.y * cos)
    } else {
        Vec2::ZERO
    };

    Vec4::new(
        velocity.x,
        velocity.y,
        if flags.use_cube_map_reflection() {
            1.0
        } else {
            0.0
        },
        0.0,
    )
}

#[cfg(test)]
#[path = "terrain_material_tests.rs"]
mod tests;
