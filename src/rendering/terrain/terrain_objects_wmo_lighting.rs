//! WMO materials that light with MOCV. MOCV is light, not albedo: stock multiplies
//! the doubled MOCV into daylight on the ordinary MapObj path and adds it to the scene
//! light on the unified one (MOHD 0x02). Selection per solarityclient
//! `WorldModelSurfacePassPlan` (`lighting_mode`).

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
    /// MOHD ambient, sRGB-encoded like MOCV.
    pub root_ambient: Vec4,
    pub mode: u32,
}

impl MaterialExtension for WmoLighting {
    fn fragment_shader() -> ShaderRef {
        "shaders/wmo_lighting.wgsl".into()
    }
}

/// How MOCV lights a batch; mirrored as constants in `wmo_lighting.wgsl`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WmoLightingMode {
    /// Unlit material or ordinary interior batch: `2 * MOCV` alone.
    Authored = 0,
    /// Unified exterior: daylight (scene sun and ambient) plus `2 * MOCV`.
    DaylightPlusMocv = 1,
    /// Unified interior: MOHD ambient plus `2 * MOCV`, no sun.
    RootAmbientPlusMocv = 2,
    /// Ordinary exterior or transition batch: daylight times `2 * MOCV`.
    DaylightTimesMocv = 3,
}

const GROUP_EXTERIOR: u32 = 0x08;
const GROUP_EXTERIOR_LIT: u32 = 0x40;

pub(crate) fn wmo_lighting_mode(
    unified: bool,
    group_flags: u32,
    batch_type: wmo::WmoBatchType,
    material_unlit: bool,
) -> WmoLightingMode {
    if material_unlit {
        WmoLightingMode::Authored
    } else if !unified {
        if batch_type == wmo::WmoBatchType::Interior {
            WmoLightingMode::Authored
        } else {
            WmoLightingMode::DaylightTimesMocv
        }
    } else if batch_type == wmo::WmoBatchType::Transparent
        || group_flags & (GROUP_EXTERIOR | GROUP_EXTERIOR_LIT) != 0
    {
        WmoLightingMode::DaylightPlusMocv
    } else {
        WmoLightingMode::RootAmbientPlusMocv
    }
}

pub(crate) fn wmo_lit_material(
    base: StandardMaterial,
    mode: WmoLightingMode,
    root_ambient: [f32; 4],
) -> WmoLitMaterial {
    WmoLitMaterial {
        base,
        extension: WmoLighting {
            params: WmoLightingParams {
                root_ambient: Vec4::from_array(root_ambient),
                mode: mode as u32,
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
