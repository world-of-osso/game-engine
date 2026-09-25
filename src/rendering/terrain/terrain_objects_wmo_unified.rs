//! Unified MapObj (MOHD 0x02) WMO materials. Their MOCV is baked light added to the
//! scene light (`texture * (2 * MOCV + light)`), so it cannot ride StandardMaterial's
//! vertex color, which multiplies base color. Stock selection per solarityclient
//! `WorldModelSurfacePassPlan` (`lighting_mode`, unified branch).

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::asset::wmo;

pub type WmoUnifiedMaterial = ExtendedMaterial<StandardMaterial, WmoUnifiedLighting>;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct WmoUnifiedLighting {
    #[uniform(100)]
    pub params: WmoUnifiedLightingParams,
}

#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct WmoUnifiedLightingParams {
    /// MOHD ambient, sRGB-encoded like MOCV.
    pub root_ambient: Vec4,
    pub mode: u32,
}

impl MaterialExtension for WmoUnifiedLighting {
    fn fragment_shader() -> ShaderRef {
        "shaders/wmo_unified.wgsl".into()
    }
}

/// Which light MOCV is added to; mirrored as constants in `wmo_unified.wgsl`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WmoUnifiedLightingMode {
    /// Unlit material: MOCV alone.
    Authored = 0,
    /// Daylight (scene sun and ambient).
    Exterior = 1,
    /// MOHD ambient without the sun.
    RootAmbient = 2,
}

const GROUP_EXTERIOR: u32 = 0x08;
const GROUP_EXTERIOR_LIT: u32 = 0x40;

pub(crate) fn wmo_unified_lighting_mode(
    group_flags: u32,
    batch_type: wmo::WmoBatchType,
    material_unlit: bool,
) -> WmoUnifiedLightingMode {
    if material_unlit {
        WmoUnifiedLightingMode::Authored
    } else if batch_type == wmo::WmoBatchType::Transparent
        || group_flags & (GROUP_EXTERIOR | GROUP_EXTERIOR_LIT) != 0
    {
        WmoUnifiedLightingMode::Exterior
    } else {
        WmoUnifiedLightingMode::RootAmbient
    }
}

pub(crate) fn wmo_unified_material(
    base: StandardMaterial,
    mode: WmoUnifiedLightingMode,
    root_ambient: [f32; 4],
) -> WmoUnifiedMaterial {
    WmoUnifiedMaterial {
        base,
        extension: WmoUnifiedLighting {
            params: WmoUnifiedLightingParams {
                root_ambient: Vec4::from_array(root_ambient),
                mode: mode as u32,
            },
        },
    }
}

pub struct WmoUnifiedMaterialPlugin;

impl Plugin for WmoUnifiedMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WmoUnifiedMaterial>::default());
    }
}
