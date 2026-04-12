use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use bevy::asset::AssetId;
use bevy::prelude::*;

use crate::asset;
use crate::m2_effect_material::{self, M2EffectMaterial, M2EffectSettings};
use crate::skybox_m2_material::{SkyboxM2Material, SkyboxM2Settings};

use super::{BatchMaterial, PLACEHOLDER_COLORS};

static REPEAT_TEXTURE_CACHE: OnceLock<Mutex<std::collections::HashMap<u32, AssetId<Image>>>> =
    OnceLock::new();

pub(super) fn load_batch_material(
    batch: &asset::m2::M2RenderBatch,
    index: usize,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    skybox_materials: Option<&mut Assets<SkyboxM2Material>>,
    force_skybox_material: bool,
    skybox_color: Option<Color>,
    skybox_default_sequence_index: Option<usize>,
    skybox_global_sequences: Option<&[u32]>,
) -> BatchMaterial {
    let texture_dir = PathBuf::from("data/textures");
    if force_skybox_material {
        if let Some(materials) = skybox_materials {
            if let Some(mat) = try_load_skybox_material(
                batch,
                &texture_dir,
                images,
                materials,
                skybox_color,
                skybox_default_sequence_index.unwrap_or(0),
                skybox_global_sequences.unwrap_or(&[]),
            ) {
                return BatchMaterial::Skybox(mat);
            }
            return BatchMaterial::Skybox(materials.add(skybox_m2_material(
                None,
                None,
                None,
                None,
                Some(PLACEHOLDER_COLORS[index % PLACEHOLDER_COLORS.len()]),
                batch,
                skybox_default_sequence_index.unwrap_or(0),
                skybox_global_sequences.unwrap_or(&[]),
            )));
        }
    }
    if should_use_effect_material(batch)
        && let Some(mat) = try_load_effect_material(batch, &texture_dir, images, effect_materials)
    {
        return BatchMaterial::Effect(mat);
    }
    if let Some(fdid) = batch.texture_fdid {
        let blp_path = asset::asset_cache::texture(fdid)
            .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
        if let Some(mat) =
            try_load_textured_material(&blp_path, batch, &texture_dir, images, materials)
        {
            return BatchMaterial::Standard(mat);
        }
    }
    let color = PLACEHOLDER_COLORS[index % PLACEHOLDER_COLORS.len()];
    BatchMaterial::Standard(materials.add(m2_material(None, Some(color), batch)))
}

fn should_use_effect_material(batch: &asset::m2::M2RenderBatch) -> bool {
    batch.texture_2_fdid.is_some() && batch.blend_mode >= 2 && batch.overlays.is_empty()
}

fn try_load_textured_material(
    blp_path: &Path,
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Option<Handle<StandardMaterial>> {
    if !blp_path.exists() {
        return None;
    }
    let image =
        crate::m2_texture_composite::load_composited_texture(blp_path, batch, texture_dir, images)
            .map_err(|e| eprintln!("{e}"))
            .ok()?;
    Some(materials.add(m2_material(Some(image), None, batch)))
}

fn try_load_effect_material(
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    materials: &mut Assets<M2EffectMaterial>,
) -> Option<Handle<M2EffectMaterial>> {
    let base_fdid = batch.texture_fdid?;
    let second_fdid = batch.texture_2_fdid?;
    let base_texture = load_repeat_texture(base_fdid, texture_dir, images)?;
    let second_texture = load_repeat_texture(second_fdid, texture_dir, images)?;
    let alpha_test =
        m2_effect_material::alpha_test_threshold_for_blend(batch.blend_mode, batch.transparency);
    Some(materials.add(M2EffectMaterial {
        settings: M2EffectSettings {
            transparency: batch.transparency,
            alpha_test,
            shader_id: batch.shader_id as u32,
            blend_mode: batch.blend_mode as u32,
            uv_mode_1: u32::from(batch.use_uv_2_1),
            uv_mode_2: u32::from(batch.use_uv_2_2),
            render_flags: batch.render_flags as u32,
            uv_offset_1: Vec2::ZERO,
            uv_offset_2: Vec2::ZERO,
        },
        base_texture,
        second_texture,
        blend_mode: batch.blend_mode,
        two_sided: batch.render_flags & 0x04 != 0,
        texture_anim_1: batch.texture_anim.clone(),
        texture_anim_2: batch.texture_anim_2.clone(),
    }))
}

fn try_load_skybox_material(
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    materials: &mut Assets<SkyboxM2Material>,
    color: Option<Color>,
    default_sequence_index: usize,
    global_sequences: &[u32],
) -> Option<Handle<SkyboxM2Material>> {
    let advanced_batch = skybox_batch_needs_effect_combine(batch);
    let base_texture = load_skybox_base_texture(batch, texture_dir, images, advanced_batch)?;
    let [second_texture, third_texture, fourth_texture] =
        load_advanced_skybox_stage_textures(batch, texture_dir, images, advanced_batch);
    Some(materials.add(skybox_m2_material(
        Some(base_texture),
        second_texture,
        third_texture,
        fourth_texture,
        color,
        batch,
        default_sequence_index,
        global_sequences,
    )))
}

fn load_skybox_base_texture(
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    advanced_batch: bool,
) -> Option<Handle<Image>> {
    let fdid = batch.texture_fdid?;
    if advanced_batch {
        return load_repeat_texture(fdid, texture_dir, images);
    }
    let blp_path = asset::asset_cache::texture(fdid)
        .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
    if !blp_path.exists() {
        return None;
    }
    crate::m2_texture_composite::load_composited_texture(&blp_path, batch, texture_dir, images)
        .map_err(|e| eprintln!("{e}"))
        .ok()
}

fn load_advanced_skybox_stage_textures(
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    advanced_batch: bool,
) -> [Option<Handle<Image>>; 3] {
    if !advanced_batch {
        return [None, None, None];
    }
    let second_texture = batch
        .texture_2_fdid
        .and_then(|fdid| load_repeat_texture(fdid, texture_dir, images));
    let third_texture = load_extra_skybox_stage_texture(batch, 0, texture_dir, images);
    let fourth_texture = load_extra_skybox_stage_texture(batch, 1, texture_dir, images);
    [second_texture, third_texture, fourth_texture]
}

fn load_extra_skybox_stage_texture(
    batch: &asset::m2::M2RenderBatch,
    index: usize,
    texture_dir: &Path,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    batch
        .extra_texture_fdids
        .get(index)
        .and_then(|fdid| load_repeat_texture(*fdid, texture_dir, images))
}

fn skybox_shader_supports_runtime_combine(shader_id: u16) -> bool {
    matches!(
        shader_id,
        0x0010 | 0x0011 | 0x4014 | 0x4016 | 0x8001 | 0x8002 | 0x8003 | 0x8012 | 0x8015 | 0x8016
    )
}

const SKYBOX_SINGLE_TEXTURE_FRAGMENT_MODES: [u16; 6] = [0x1, 0x2, 0x3, 0x4, 0x5, 0x6];
const SKYBOX_TWO_TEXTURE_FRAGMENT_MODES: [[u16; 6]; 6] = [
    [0x7, 0x8, 0x9, 0xA, 0xB, 0xC],
    [0xD, 0xE, 0xF, 0x10, 0x11, 0x12],
    [0x13, 0x14, 0x15, 0x16, 0x17, 0x18],
    [0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E],
    [0x1F, 0x20, 0x21, 0x22, 0x23, 0x24],
    [0x1F, 0x20, 0x21, 0x22, 0x23, 0x24],
];
const SKYBOX_STATIC_FRAGMENT_MODES: [usize; 7] = [0, 1, 1, 1, 1, 5, 5];

fn static_fragment_mode_for_blend(blend_mode: u16) -> usize {
    SKYBOX_STATIC_FRAGMENT_MODES
        .get(blend_mode as usize)
        .copied()
        .unwrap_or(1)
}

fn resolve_skybox_combine_mode(batch: &asset::m2::M2RenderBatch) -> u32 {
    if batch.uses_texture_combiner_combos || (batch.shader_id & 0x8000) != 0 {
        return batch.shader_id as u32;
    }
    let static_mode = static_fragment_mode_for_blend(batch.blend_mode);
    if batch.texture_count > 1 {
        SKYBOX_TWO_TEXTURE_FRAGMENT_MODES[static_mode][static_mode] as u32
    } else {
        SKYBOX_SINGLE_TEXTURE_FRAGMENT_MODES[static_mode] as u32
    }
}

fn resolve_skybox_uv_modes(batch: &asset::m2::M2RenderBatch) -> [u32; 4] {
    match batch.shader_id {
        0x8012 => [0, 0, 0, 0],
        0x8016 => [0, 0, 0, 1],
        _ => [
            u32::from(batch.use_uv_2_1),
            u32::from(batch.use_uv_2_2),
            0,
            0,
        ],
    }
}

fn skybox_batch_needs_effect_combine(batch: &asset::m2::M2RenderBatch) -> bool {
    if batch.texture_count > 1 {
        return true;
    }
    if !skybox_shader_supports_runtime_combine(batch.shader_id) {
        return false;
    }
    let uses_multitexture_shader = batch.texture_count > 1 && batch.shader_id != 0;
    batch.texture_2_fdid.is_some()
        || batch.texture_count > 1
        || batch.texture_anim.is_some()
        || batch.texture_anim_2.is_some()
        || batch.use_uv_2_1
        || batch.use_uv_2_2
        || batch.use_env_map_2
        || uses_multitexture_shader
}

fn load_repeat_texture(
    fdid: u32,
    texture_dir: &Path,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let cache = REPEAT_TEXTURE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(handle) = crate::asset_lifetime::lookup_cached_asset_handle(cache, &fdid, images) {
        return Some(handle);
    }
    let blp_path = asset::asset_cache::texture(fdid)
        .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
    if !blp_path.exists() {
        return None;
    }
    let (pixels, width, height) = asset::blp::load_blp_rgba(&blp_path).ok()?;
    let mut image = crate::rgba_image(pixels, width, height);
    image.sampler = m2_effect_material::repeat_sampler();
    let handle = images.add(image);
    crate::asset_lifetime::prune_unused_asset_handles(cache, images);
    cache.lock().unwrap().insert(fdid, handle.id());
    Some(handle)
}

/// Build a StandardMaterial from M2 render flags (two-sided, unlit, blend mode).
pub(super) fn m2_material(
    texture: Option<Handle<Image>>,
    color: Option<Color>,
    batch: &asset::m2::M2RenderBatch,
) -> StandardMaterial {
    let two_sided = batch.render_flags & 0x04 != 0;
    let unlit = batch.render_flags & 0x01 != 0;
    let cull_mode = if two_sided {
        None
    } else {
        Some(bevy::render::render_resource::Face::Back)
    };
    let alpha_mode = m2_effect_material::alpha_mode_for_blend(batch.blend_mode);
    StandardMaterial {
        base_color_texture: texture,
        base_color: color.unwrap_or(Color::srgba(1.0, 1.0, 1.0, batch.transparency)),
        unlit,
        cull_mode,
        double_sided: two_sided,
        alpha_mode,
        ..default()
    }
}

pub(crate) fn skybox_m2_material(
    texture: Option<Handle<Image>>,
    second_texture: Option<Handle<Image>>,
    third_texture: Option<Handle<Image>>,
    fourth_texture: Option<Handle<Image>>,
    color: Option<Color>,
    batch: &asset::m2::M2RenderBatch,
    default_sequence_index: usize,
    global_sequences: &[u32],
) -> SkyboxM2Material {
    let stage_textures =
        resolve_skybox_stage_textures(texture, second_texture, third_texture, fourth_texture);
    SkyboxM2Material {
        settings: build_skybox_material_settings(color, batch, &stage_textures),
        base_texture: stage_textures.base_texture,
        second_texture: stage_textures.second_texture,
        third_texture: stage_textures.third_texture,
        fourth_texture: stage_textures.fourth_texture,
        blend_mode: batch.blend_mode,
        two_sided: batch.render_flags & 0x04 != 0,
        default_sequence_index: default_sequence_index as u32,
        global_sequences: global_sequences.to_vec(),
        texture_anim_1: batch.texture_anim.clone(),
        texture_anim_2: batch.texture_anim_2.clone(),
    }
}

struct ResolvedSkyboxStageTextures {
    base_texture: Handle<Image>,
    second_texture: Handle<Image>,
    third_texture: Handle<Image>,
    fourth_texture: Handle<Image>,
    has_second_texture: bool,
    has_third_texture: bool,
    has_fourth_texture: bool,
}

fn resolve_skybox_stage_textures(
    texture: Option<Handle<Image>>,
    second_texture: Option<Handle<Image>>,
    third_texture: Option<Handle<Image>>,
    fourth_texture: Option<Handle<Image>>,
) -> ResolvedSkyboxStageTextures {
    let has_second_texture = second_texture.is_some();
    let has_third_texture = third_texture.is_some();
    let has_fourth_texture = fourth_texture.is_some();
    let base_texture = texture.unwrap_or_default();
    let second_texture = second_texture.unwrap_or_else(|| base_texture.clone());
    let third_texture = third_texture.unwrap_or_else(|| base_texture.clone());
    let fourth_texture = fourth_texture.unwrap_or_else(|| base_texture.clone());
    ResolvedSkyboxStageTextures {
        base_texture,
        second_texture,
        third_texture,
        fourth_texture,
        has_second_texture,
        has_third_texture,
        has_fourth_texture,
    }
}

fn build_skybox_material_settings(
    color: Option<Color>,
    batch: &asset::m2::M2RenderBatch,
    stage_textures: &ResolvedSkyboxStageTextures,
) -> SkyboxM2Settings {
    let uv_modes = resolve_skybox_uv_modes(batch);
    SkyboxM2Settings {
        color: color
            .unwrap_or(Color::WHITE)
            .to_linear()
            .to_f32_array()
            .into(),
        transparency: 1.0,
        // Authored skybox textures use low alpha values for soft cloud edges.
        // Applying the normal M2 alpha-test thresholds discards the entire dome.
        alpha_test: 0.0,
        combine_mode: resolve_skybox_combine_mode(batch),
        blend_mode: batch.blend_mode as u32,
        uv_mode_1: uv_modes[0],
        uv_mode_2: uv_modes[1],
        uv_mode_3: uv_modes[2],
        uv_mode_4: uv_modes[3],
        render_flags: batch.render_flags as u32,
        has_second_texture: u32::from(stage_textures.has_second_texture),
        has_third_texture: u32::from(stage_textures.has_third_texture),
        has_fourth_texture: u32::from(stage_textures.has_fourth_texture),
        uv_offset_1: Vec2::ZERO,
        uv_offset_2: Vec2::ZERO,
    }
}

#[cfg(test)]
#[path = "m2_spawn_material_tests.rs"]
mod tests;
