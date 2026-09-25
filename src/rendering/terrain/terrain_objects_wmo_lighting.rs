//! Retail WMO lighting, per WebWowViewerCpp (`commonLightFunctions.slang` `calcLight`,
//! `wmoshader_text.slang`). MOCV is light, not albedo: the doubled fixed MOCV is added
//! to the ambient, `texture * (ambient + 2 * MOCV + sun)`. Exterior light uses the scene
//! sun and ambient, interior light the WMO interior ambient without a sun; the fixed
//! MOCV alpha blends the two per vertex. StandardMaterial vertex color multiplies base
//! color, so these batches need their own fragment shader.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::asset::wmo;

pub type WmoLitMaterial = ExtendedMaterial<StandardMaterial, WmoLighting>;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct WmoLighting {
    #[uniform(100)]
    pub params: WmoLightingParams,
}

#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct WmoLightingParams {
    /// Interior ambient, sRGB-encoded like MOCV.
    pub interior_ambient: Vec4,
    /// 1 when the group always takes exterior light, overriding the MOCV alpha.
    pub exterior_lit: u32,
    /// 1 for MOMT `F_UNLIT`: the texture alone.
    pub unlit: u32,
}

impl MaterialExtension for WmoLighting {
    fn fragment_shader() -> ShaderRef {
        "shaders/wmo_lighting.wgsl".into()
    }
}

const GROUP_EXTERIOR: u32 = 0x08;
const GROUP_EXTERIOR_LIT: u32 = 0x40;
const GROUP_INTERIOR: u32 = 0x2000;

/// WebWowViewerCpp `WmoGroupObject::isInteriorLightingLit`, negated.
pub(crate) fn wmo_group_is_exterior_lit(group_flags: u32) -> bool {
    group_flags & (GROUP_EXTERIOR | GROUP_EXTERIOR_LIT) != 0 || group_flags & GROUP_INTERIOR == 0
}

/// WebWowViewerCpp `WmoObject::calculateAmbient`: the first MAVG record for an active
/// doodad set, else the first MAVD, else the MOHD ambient. Only the base color (not
/// the horizon/ground colors of flag-1 records) is used.
pub(crate) fn wmo_interior_ambient(root: &wmo::WmoRootData, active_doodad_set: u16) -> [f32; 3] {
    let global = root
        .global_ambient_volumes
        .iter()
        .find(|volume| volume.doodad_set_id == 0 || volume.doodad_set_id == active_doodad_set)
        .or(root.global_ambient_volumes.first());
    let color = global
        .or(root.ambient_volumes.first())
        .map(|volume| volume.color_1)
        .unwrap_or(root.ambient_color);
    [color[0], color[1], color[2]]
}

pub(crate) fn wmo_lit_material(
    base: StandardMaterial,
    group_flags: u32,
    material_unlit: bool,
    interior_ambient: [f32; 3],
) -> WmoLitMaterial {
    let [r, g, b] = interior_ambient;
    WmoLitMaterial {
        base,
        extension: WmoLighting {
            params: WmoLightingParams {
                interior_ambient: Vec4::new(r, g, b, 1.0),
                exterior_lit: wmo_group_is_exterior_lit(group_flags) as u32,
                unlit: material_unlit as u32,
            },
        },
    }
}

pub struct WmoLitMaterialPlugin;

impl Plugin for WmoLitMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WmoLitMaterial>::default());
    }
}
