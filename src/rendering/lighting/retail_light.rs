//! Retail world lighting: one scene light, computed from the LightParams blend,
//! that every world material shades with.
//!
//! Source: Deamon87/WebWowViewerCpp at `1a8cccbeffc46231c6497e6b3f5bfbf3507d8071`:
//! `wowViewerLib/shaders/slang/common/commonLightFunctions.slang` (`calcLight`,
//! `applyAndMixAmbients`), `.../dayNightDataHolder/DayNightLightHolder.cpp`
//! (exterior colours) and `.../algorithms/mathHelper.cpp` (directional light
//! tables). Colours stay in the space LightData authors them in (sRGB bytes / 255);
//! shaders convert textures to that space, shade, and convert the result back
//! to linear for Bevy's render target.

use bevy::asset::uuid_handle;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bevy::render::storage::ShaderBuffer;

use crate::retail_m2_material::M2Material;
use crate::sky_lightdata::SkyColorSet;

#[path = "retail_light_data.rs"]
mod retail_light_data;
use retail_light_data::{RetailLightColors, RetailLightData};

/// The one GPU copy of [`RetailSceneLight`]; every Retail-lit material binds it.
pub const RETAIL_SCENE_LIGHT_BUFFER: Handle<ShaderBuffer> =
    uuid_handle!("6a0cf3a2-5a8e-4b5e-9d0f-8e3c3f1d2b71");

/// Exterior scene light in authored (gamma) space.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct RetailSceneLight {
    pub ambient: Vec3,
    pub horizon_ambient: Vec3,
    pub ground_ambient: Vec3,
    pub direct: Vec3,
    /// Bevy world direction the sunlight travels (from the sun towards the ground).
    pub sun_direction: Vec3,
    pub fog_color: Vec3,
    /// Linear fog range in yards.
    pub fog_start: f32,
    pub fog_end: f32,
}

impl RetailSceneLight {
    /// Exterior light from sampled LightData at `minutes` (0–2880 half-minutes).
    /// A zero horizon or ground ambient takes the ambient colour, as
    /// `DayNightLightHolder::getLightResultsFromDB` does.
    pub fn from_sky_colors(colors: &SkyColorSet, minutes: f32) -> Self {
        let sampled = RetailLightColors {
            ambient: authored_rgb(colors.ambient_color).to_array(),
            horizon_ambient: authored_rgb(colors.horizon_ambient_color).to_array(),
            ground_ambient: authored_rgb(colors.ground_ambient_color).to_array(),
            direct: authored_rgb(colors.direct_color).to_array(),
            fog_color: authored_rgb(colors.fog_color).to_array(),
            fog_start: colors.fog_start,
            fog_end: colors.fog_end,
        };
        Self::from(retail_light_data::scene_light(&sampled, minutes))
    }
}

impl RetailSceneLight {
    /// Light of a standalone M2 scene (`M2Scene::updateLightAndSkyboxData` with
    /// M2 lighting): the model's ambient from every direction, no direct light.
    pub fn m2_scene(ambient: Vec3) -> Self {
        Self {
            ambient,
            horizon_ambient: ambient,
            ground_ambient: ambient,
            direct: Vec3::ZERO,
            sun_direction: Vec3::NEG_Y,
            fog_color: Vec3::ZERO,
            fog_start: 0.0,
            fog_end: f32::MAX,
        }
    }
}

impl From<RetailLightData> for RetailSceneLight {
    fn from(light: RetailLightData) -> Self {
        Self {
            ambient: Vec3::from_array(light.ambient),
            horizon_ambient: Vec3::from_array(light.horizon_ambient),
            ground_ambient: Vec3::from_array(light.ground_ambient),
            direct: Vec3::from_array(light.direct),
            sun_direction: Vec3::from_array(light.sun_direction),
            fog_color: Vec3::from_array(light.fog_color),
            fog_start: light.fog_start,
            fog_end: light.fog_end,
        }
    }
}

impl From<&RetailSceneLight> for RetailLightData {
    fn from(light: &RetailSceneLight) -> Self {
        Self {
            ambient: light.ambient.to_array(),
            horizon_ambient: light.horizon_ambient.to_array(),
            ground_ambient: light.ground_ambient.to_array(),
            direct: light.direct.to_array(),
            sun_direction: light.sun_direction.to_array(),
            fog_color: light.fog_color.to_array(),
            fog_start: light.fog_start,
            fog_end: light.fog_end,
        }
    }
}

fn authored_rgb(color: Color) -> Vec3 {
    let srgba = color.to_srgba();
    Vec3::new(srgba.red, srgba.green, srgba.blue)
}

/// Direction the exterior direct light travels at `minutes`, in Bevy world space.
pub fn retail_sun_direction(minutes: f32) -> Vec3 {
    Vec3::from_array(retail_light_data::sun_direction(minutes))
}

/// `calcLight` for an exterior surface with no point lights, specular or
/// emission: `sqrt((diffuse · (ambient + direct·N·L))²)`. `sun_visibility` is the
/// shadow-map visibility of the sun (1 = unshadowed). Mirrors
/// `assets/shaders/retail_lighting.wgsl` `retail_shade`.
pub fn retail_shade(
    light: &RetailSceneLight,
    diffuse: Vec3,
    normal: Vec3,
    sun_visibility: f32,
) -> Vec3 {
    Vec3::from_array(retail_light_data::shade(
        &RetailLightData::from(light),
        diffuse.to_array(),
        normal.to_array(),
        sun_visibility,
    ))
}

/// GPU layout of the shading part of [`RetailSceneLight`]. Shaders fog with the
/// camera's `DistanceFog`, which the sky systems set from the same resource.
#[derive(ShaderType, Clone, Copy, Debug, Default, PartialEq)]
pub struct RetailSceneLightUniform {
    pub ambient: Vec4,
    pub horizon_ambient: Vec4,
    pub ground_ambient: Vec4,
    pub direct: Vec4,
    pub sun_direction: Vec4,
}

impl From<&RetailSceneLight> for RetailSceneLightUniform {
    fn from(light: &RetailSceneLight) -> Self {
        Self {
            ambient: light.ambient.extend(1.0),
            horizon_ambient: light.horizon_ambient.extend(1.0),
            ground_ambient: light.ground_ambient.extend(1.0),
            direct: light.direct.extend(1.0),
            sun_direction: light.sun_direction.extend(0.0),
        }
    }
}

fn upload_retail_scene_light(
    light: Res<RetailSceneLight>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    if !light.is_changed() {
        return;
    }
    let uniform = RetailSceneLightUniform::from(light.as_ref());
    if let Some(mut buffer) = buffers.get_mut(&RETAIL_SCENE_LIGHT_BUFFER) {
        buffer.set_data(uniform);
        return;
    }
    buffers
        .insert(&RETAIL_SCENE_LIGHT_BUFFER, ShaderBuffer::from(uniform))
        .expect("Retail scene light buffer uses a UUID handle");
}

pub struct RetailLightingPlugin;

impl Plugin for RetailLightingPlugin {
    fn build(&self, app: &mut App) {
        let empty = crate::game::inworld_scene_stage::configured_inworld_scene_stage_for_app(app)
            == crate::game::inworld_scene_stage::InWorldSceneStage::Empty;
        if empty {
            app.init_asset::<M2Material>();
        } else {
            app.add_plugins(MaterialPlugin::<M2Material>::default());
        }
        app.init_asset::<ShaderBuffer>()
            .add_systems(PostUpdate, upload_retail_scene_light);
    }
}

#[cfg(test)]
#[path = "retail_light_tests.rs"]
mod tests;
