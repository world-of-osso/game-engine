use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor,
    TextureViewDimension,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use std::f32::consts::FRAC_PI_4;

use crate::asset::adt;
use crate::rendering::image_sampler::{clamp_linear_sampler, repeat_linear_sampler};
use terrain_material_systems::{sync_terrain_environment_map, terrain_material_updates_enabled};

mod terrain_material_systems;

#[cfg(test)]
#[path = "shared_material_clock_gpu_tests.rs"]
mod shared_material_clock_gpu_tests;

#[cfg(test)]
#[path = "terrain_retail_gpu_tests.rs"]
mod retail_gpu_tests;

/// Custom terrain material: ground texture layers blended by MCAL alpha maps on the GPU.
/// The map's WDT MPHD flags select the blend: layered (4-bit alpha), weighted (big alpha),
/// or Retail height-weighted with `_h` textures scaled by MTXP.
#[derive(bevy::render::render_resource::ShaderType, Clone)]
pub struct TerrainMaterialSettings {
    /// x = layer_count (1-4), y = TerrainBlendMode,
    /// z = texture_repeat, w = unused
    pub config: Vec4,
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

    #[texture(21, dimension = "cube")]
    #[sampler(22)]
    pub environment_map: Handle<Image>,

    /// Shared Retail scene light (`RETAIL_SCENE_LIGHT_BUFFER`).
    #[storage(23, read_only)]
    pub scene_light: Handle<ShaderBuffer>,
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

#[derive(Resource)]
pub(crate) struct TerrainMaterialFreezeAfter(pub std::time::Duration);

pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(
                Update,
                sync_terrain_environment_map.run_if(terrain_material_updates_enabled),
            );
    }
}

/// 1x1 placeholder for unused texture slots.
pub enum PlaceholderImageKind {
    Color,
    Alpha,
}

pub fn placeholder_image(images: &mut Assets<Image>, kind: PlaceholderImageKind) -> Handle<Image> {
    match kind {
        PlaceholderImageKind::Color => {
            add_single_pixel_placeholder(images, [128, 128, 128, 255], true)
        }
        PlaceholderImageKind::Alpha => add_single_pixel_placeholder(images, [0, 0, 0, 255], false),
    }
}

fn add_single_pixel_placeholder(
    images: &mut Assets<Image>,
    rgba: [u8; 4],
    use_repeat_sampler: bool,
) -> Handle<Image> {
    let mut img = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.to_vec(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = if use_repeat_sampler {
        repeat_linear_sampler()
    } else {
        clamp_linear_sampler()
    };
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
            if fdid == 0 {
                return None;
            }
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
            if fdid == 0 {
                return None;
            }
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
        // Blend weights are linear data; an sRGB format would gamma-decode them into steps.
        TextureFormat::Rgba8Unorm,
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
) -> Vec<Handle<TerrainMaterial>> {
    let ph = Placeholders {
        image: placeholder_image(images, PlaceholderImageKind::Color),
        alpha: placeholder_image(images, PlaceholderImageKind::Alpha),
        cubemap: placeholder_cubemap(images),
    };

    let (Some(td), Some(gi)) = (tex_data, ground_images) else {
        return build_fallback_materials(terrain_materials, adt_data, &ph);
    };

    td.chunk_layers
        .iter()
        .enumerate()
        .map(|(chunk_index, chunk_tex)| {
            let pre_al = pre_alpha.and_then(|a| a.get(chunk_index));
            build_chunk_material(
                terrain_materials,
                images,
                td,
                chunk_tex,
                gi,
                height_images,
                &ph,
                pre_al,
            )
        })
        .collect()
}

fn build_fallback_materials(
    terrain_materials: &mut Assets<TerrainMaterial>,
    adt_data: &adt::AdtData,
    ph: &Placeholders,
) -> Vec<Handle<TerrainMaterial>> {
    adt_data
        .chunks
        .iter()
        .map(|_| terrain_materials.add(fallback_material(ph)))
        .collect()
}

/// Shader blend selected by the map's WDT MPHD flags (`config.y`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainBlendMode {
    /// 4-bit alpha maps: each layer mixes over the result below it.
    Layered = 0,
    /// MPHD 0x4 big alpha: base weight is `1 - saturate(sum)`, layers add by their alpha.
    Weighted = 1,
    /// MPHD 0x80 height texturing: weighted, then re-weighted by `_h` heights (wowdev ADT/v18 MTXP).
    HeightWeighted = 2,
}

pub fn terrain_blend_mode(map_flags: adt::MphdFlags) -> TerrainBlendMode {
    if map_flags.height_texturing() {
        TerrainBlendMode::HeightWeighted
    } else if map_flags.big_alpha() {
        TerrainBlendMode::Weighted
    } else {
        TerrainBlendMode::Layered
    }
}

/// wowdev ADT/v18 MTXP defaults; with these the client loads no `_h` texture.
const DEFAULT_HEIGHT_SCALE: f32 = 0.0;
const DEFAULT_HEIGHT_OFFSET: f32 = 1.0;
const BASE_TERRAIN_TEXTURE_REPEAT: f32 = 8.0;
const TERRAIN_OVERBRIGHT_MULTIPLIER: f32 = 2.0;
const DEFAULT_LAYER_PARAMS: Vec4 = Vec4::new(DEFAULT_HEIGHT_SCALE, DEFAULT_HEIGHT_OFFSET, 0.0, 1.0);
const TERRAIN_ANIMATION_SPEEDS: [f32; 8] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 48.0, 64.0];
const TERRAIN_ANIMATION_BASE_SPEED: f32 = 0.176_776_69;

fn terrain_settings(
    layer_count: f32,
    blend_mode: TerrainBlendMode,
    texture_repeat: f32,
    layer_params: [Vec4; 4],
    animation_params: [Vec4; 4],
) -> TerrainMaterialSettings {
    TerrainMaterialSettings {
        config: Vec4::new(layer_count, blend_mode as u32 as f32, texture_repeat, 0.0),
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

fn fallback_material(ph: &Placeholders) -> TerrainMaterial {
    TerrainMaterial {
        settings: terrain_settings(
            0.0,
            TerrainBlendMode::Layered,
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
        environment_map: ph.cubemap.clone(),
        scene_light: crate::retail_light::RETAIL_SCENE_LIGHT_BUFFER,
    }
}

fn build_chunk_material(
    terrain_materials: &mut Assets<TerrainMaterial>,
    images: &mut Assets<Image>,
    tex_data: &adt::AdtTexData,
    chunk_tex: &adt::ChunkTexLayers,
    ground_images: &[Option<Handle<Image>>],
    height_images: Option<&[Option<Handle<Image>>]>,
    ph: &Placeholders,
    pre_alpha: Option<&Handle<Image>>,
) -> Handle<TerrainMaterial> {
    if chunk_tex.layers.is_empty() {
        return terrain_materials.add(fallback_material(ph));
    }

    let layer_count = chunk_tex.layers.len().min(4) as f32;
    let ground_handles = resolve_chunk_ground_images(chunk_tex, ground_images, ph);
    let height_images = resolve_chunk_height_images(chunk_tex, height_images);
    let has_height_texture = height_images.each_ref().map(Option::is_some);
    let height_handles = height_images.map(|image| image.unwrap_or_else(|| ph.alpha.clone()));
    let layer_params = texture_layer_params(tex_data, &chunk_tex.layers, has_height_texture);
    let animation_params = terrain_layer_animation_params(&chunk_tex.layers);
    let texture_repeat = terrain_texture_repeat(tex_data.texture_amplifier);

    terrain_materials.add(TerrainMaterial {
        settings: terrain_settings(
            layer_count,
            terrain_blend_mode(tex_data.map_flags),
            texture_repeat,
            layer_params,
            animation_params,
        ),
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
        environment_map: ph.cubemap.clone(),
        scene_light: crate::retail_light::RETAIL_SCENE_LIGHT_BUFFER,
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

/// MHID `_h` textures per layer; the diffuse alpha is a specular mask, never a height.
fn resolve_chunk_height_images(
    chunk_tex: &adt::ChunkTexLayers,
    height_images: Option<&[Option<Handle<Image>>]>,
) -> [Option<Handle<Image>>; 4] {
    std::array::from_fn(|idx| {
        let layer = chunk_tex.layers.get(idx)?;
        height_images?.get(layer.texture_index as usize)?.clone()
    })
}

fn terrain_texture_repeat(texture_amplifier: Option<u32>) -> f32 {
    let exponent = texture_amplifier.unwrap_or(0).min(8) as i32;
    BASE_TERRAIN_TEXTURE_REPEAT * 2.0f32.powi(exponent)
}

fn texture_layer_params(
    tex_data: &adt::AdtTexData,
    layers: &[adt::TextureLayer],
    has_height_texture: [bool; 4],
) -> [Vec4; 4] {
    let mut params = [DEFAULT_LAYER_PARAMS; 4];
    for (slot, layer) in layers.iter().take(4).enumerate() {
        let overbright_multiplier = if layer.flags.overbright() {
            TERRAIN_OVERBRIGHT_MULTIPLIER
        } else {
            1.0
        };
        let height = layer_height_params(tex_data, layer.texture_index, has_height_texture[slot]);
        params[slot] = Vec4::new(
            height.x,
            height.y,
            f32::from(layer.material_id),
            overbright_multiplier,
        );
    }
    params
}

/// Height term `h * scale + offset` inputs; a layer without an `_h` texture has only its offset.
fn layer_height_params(
    tex_data: &adt::AdtTexData,
    texture_index: u32,
    has_height_texture: bool,
) -> Vec2 {
    let (scale, offset) = tex_data
        .texture_params
        .get(texture_index as usize)
        .map_or((DEFAULT_HEIGHT_SCALE, DEFAULT_HEIGHT_OFFSET), |param| {
            (param.height_scale, param.height_offset)
        });
    let scale = if has_height_texture { scale } else { 0.0 };
    Vec2::new(scale, offset)
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
