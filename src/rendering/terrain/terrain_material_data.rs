//! Bevy-free terrain material inputs derived from authored ADT texture data.
use std::f32::consts::FRAC_PI_4;

use crate::asset::adt_format::adt_tex::{MclyFlags, TextureLayer, TextureParams};
use crate::asset::wdt::MphdFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainBlendMode {
    Layered = 0,
    Weighted = 1,
    HeightWeighted = 2,
}

pub fn terrain_blend_mode(map_flags: MphdFlags) -> TerrainBlendMode {
    if map_flags.height_texturing() {
        TerrainBlendMode::HeightWeighted
    } else if map_flags.big_alpha() {
        TerrainBlendMode::Weighted
    } else {
        TerrainBlendMode::Layered
    }
}

pub const DEFAULT_LAYER_PARAMS: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const BASE_TERRAIN_TEXTURE_REPEAT: f32 = 8.0;
const TERRAIN_ANIMATION_SPEEDS: [f32; 8] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 48.0, 64.0];
const TERRAIN_ANIMATION_BASE_SPEED: f32 = 0.176_776_69;

pub fn terrain_texture_repeat(texture_amplifier: Option<u32>) -> f32 {
    let exponent = texture_amplifier.unwrap_or(0).min(8) as i32;
    BASE_TERRAIN_TEXTURE_REPEAT * 2.0f32.powi(exponent)
}

pub fn texture_layer_params(
    texture_params: &[TextureParams],
    layers: &[TextureLayer],
    has_height_texture: [bool; 4],
) -> [[f32; 4]; 4] {
    let mut params = [DEFAULT_LAYER_PARAMS; 4];
    for (slot, layer) in layers.iter().take(4).enumerate() {
        let (scale, offset) = texture_params
            .get(layer.texture_index as usize)
            .map_or((0.0, 1.0), |param| {
                (param.height_scale, param.height_offset)
            });
        params[slot] = [
            if has_height_texture[slot] { scale } else { 0.0 },
            offset,
            f32::from(layer.material_id),
            if layer.flags.overbright() { 2.0 } else { 1.0 },
        ];
    }
    params
}

pub fn terrain_layer_animation_params(layers: &[TextureLayer]) -> [[f32; 4]; 4] {
    let mut params = [[0.0; 4]; 4];
    for (slot, layer) in layers.iter().take(4).enumerate() {
        params[slot] = terrain_layer_animation(layer.flags);
    }
    params
}

pub fn terrain_layer_animation(flags: MclyFlags) -> [f32; 4] {
    let velocity = if flags.animation_enabled() {
        let speed = TERRAIN_ANIMATION_SPEEDS[flags.animation_speed() as usize]
            * TERRAIN_ANIMATION_BASE_SPEED;
        let angle = FRAC_PI_4 + f32::from(flags.animation_rotation()) * FRAC_PI_4;
        let (sin, cos) = angle.sin_cos();
        [speed * cos - speed * sin, speed * sin + speed * cos]
    } else {
        [0.0; 2]
    };
    [
        velocity[0],
        velocity[1],
        if flags.use_cube_map_reflection() {
            1.0
        } else {
            0.0
        },
        0.0,
    ]
}

/// 64×64 linear RGBA: R/G/B carry MCAL layers 1/2/3 and A is opaque.
pub fn pack_alpha_map_bytes(layers: &[TextureLayer]) -> Vec<u8> {
    const PIXELS: usize = 64 * 64;
    let mut rgba = vec![0u8; PIXELS * 4];
    for (channel, layer) in layers.iter().skip(1).take(3).enumerate() {
        if let Some(alpha) = layer.alpha_map.as_deref() {
            for (pixel, &value) in alpha.iter().take(PIXELS).enumerate() {
                rgba[pixel * 4 + channel] = value;
            }
        }
    }
    for pixel in 0..PIXELS {
        rgba[pixel * 4 + 3] = 255;
    }
    rgba
}
