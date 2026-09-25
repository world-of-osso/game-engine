//! World M2 material: StandardMaterial texture, vertex colour and alpha handling,
//! shaded with Retail lighting (`assets/shaders/retail_m2.wgsl`).
//!
//! Source: Deamon87/WebWowViewerCpp `1a8cccbeffc46231c6497e6b3f5bfbf3507d8071`
//! `wowViewerLib/shaders/slang/bindless/m2/m2shader_text.slang`: M2 render flag
//! 0x1 clears IsAffectedByLight, 0x2 is UnFogged, and the batch blend mode picks
//! the fog colour through `M2BlendingModeToEGxBlend`.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;

use crate::retail_light::RETAIL_SCENE_LIGHT_BUFFER;

/// M2 blend mode → `EGxBlend` (`M2BlendingModeToEGxBlend`): 0 Opaque, 1 AlphaKey,
/// 2 Alpha, 3 NoAlphaAdd, 4 Add, 5 Mod, 6 Mod2x, 7 BlendAdd.
pub fn m2_blend_to_gx_blend(m2_blend_mode: u16) -> u32 {
    match m2_blend_mode {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 10,
        4 => 3,
        5 => 4,
        6 => 5,
        7 => 13,
        _ => 0,
    }
}

pub const RETAIL_UNLIT: u32 = 1 << 0;
pub const RETAIL_UNFOGGED: u32 = 1 << 1;

#[derive(ShaderType, Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub struct RetailLitParams {
    /// `RETAIL_UNLIT` | `RETAIL_UNFOGGED`.
    pub flags: u32,
    /// `EGxBlend` of the batch, for the blend-mode fog colour.
    pub gx_blend: u32,
}

/// Retail lighting on top of a `StandardMaterial` texture/alpha setup.
#[derive(Asset, AsBindGroup, Reflect, Clone, Debug)]
pub struct RetailLit {
    #[storage(100, read_only)]
    pub scene_light: Handle<ShaderBuffer>,
    #[uniform(101)]
    pub params: RetailLitParams,
}

impl Default for RetailLit {
    fn default() -> Self {
        Self {
            scene_light: RETAIL_SCENE_LIGHT_BUFFER,
            params: RetailLitParams::default(),
        }
    }
}

impl MaterialExtension for RetailLit {
    fn fragment_shader() -> ShaderRef {
        "shaders/retail_m2.wgsl".into()
    }
}

/// World M2 material: StandardMaterial textures and alpha, Retail lighting and fog.
pub type M2Material = ExtendedMaterial<StandardMaterial, RetailLit>;

/// Wrap `base` for an M2 batch. M2 render flag 0x1 is unlit, 0x2 unfogged.
pub fn retail_m2_material(
    mut base: StandardMaterial,
    render_flags: u16,
    blend_mode: u16,
) -> M2Material {
    let mut flags = 0;
    if render_flags & 0x01 != 0 {
        flags |= RETAIL_UNLIT;
    }
    if render_flags & 0x02 != 0 {
        flags |= RETAIL_UNFOGGED;
    }
    // The Retail shader fogs in authored space; Bevy's linear fog must not repeat it.
    base.fog_enabled = false;
    ExtendedMaterial {
        base,
        extension: RetailLit {
            params: RetailLitParams {
                flags,
                gx_blend: m2_blend_to_gx_blend(blend_mode),
            },
            ..default()
        },
    }
}

#[cfg(test)]
#[path = "retail_m2_material_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "retail_m2_material_gpu_tests.rs"]
mod gpu_tests;
