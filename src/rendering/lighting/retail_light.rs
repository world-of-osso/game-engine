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
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bevy::render::storage::ShaderBuffer;

use crate::sky_lightdata::SkyColorSet;

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
        let ambient = authored_rgb(colors.ambient_color);
        let or_ambient = |color: Vec3| if color == Vec3::ZERO { ambient } else { color };
        Self {
            ambient,
            horizon_ambient: or_ambient(authored_rgb(colors.horizon_ambient_color)),
            ground_ambient: or_ambient(authored_rgb(colors.ground_ambient_color)),
            direct: authored_rgb(colors.direct_color),
            sun_direction: retail_sun_direction(minutes),
            fog_color: authored_rgb(colors.fog_color),
            fog_start: colors.fog_start,
            fog_end: colors.fog_end,
        }
    }
}

fn authored_rgb(color: Color) -> Vec3 {
    let srgba = color.to_srgba();
    Vec3::new(srgba.red, srgba.green, srgba.blue)
}

/// `MathHelper` `directionalLightPhiTable` / `directionalLightThetaTable`:
/// (day fraction, angle in radians).
const DIRECTIONAL_LIGHT_PHI: [[f32; 2]; 4] = [
    [0.0, 2.216_568_2],
    [0.25, 1.919_862_2],
    [0.5, 2.216_568_2],
    [0.75, 1.919_862_2],
];
const DIRECTIONAL_LIGHT_THETA: [[f32; 2]; 4] = [
    [0.0, 3.926_990_7],
    [0.25, 3.926_990_7],
    [0.5, 3.926_990_7],
    [0.75, 3.926_990_7],
];

/// `MathHelper::InterpTable`: cyclic linear interpolation over the day.
fn interp_day_table(table: &[[f32; 2]], day: f32) -> f32 {
    let day = if day >= 0.0 { day.min(1.0) } else { day };
    let first = table.iter().position(|entry| day <= entry[0]).unwrap_or(0);
    let second = if first == 0 {
        table.len() - 1
    } else {
        first - 1
    };
    let mut span = table[first][0] - table[second][0];
    if span.abs() < 0.001 {
        return table[second][1];
    }
    if span < 0.0 {
        span += 1.0;
    }
    let mut elapsed = day - table[second][0];
    if elapsed < 0.0 {
        elapsed += 1.0;
    }
    let alpha = elapsed / span;
    table[second][1] + alpha * (table[first][1] - table[second][1])
}

/// Direction the exterior direct light travels at `minutes`, in Bevy world space.
/// `polarToCartesian(phi, theta)` gives WoW z-up coordinates.
pub fn retail_sun_direction(minutes: f32) -> Vec3 {
    let day = minutes.rem_euclid(2880.0) / 2880.0;
    let phi = interp_day_table(&DIRECTIONAL_LIGHT_PHI, day);
    let theta = interp_day_table(&DIRECTIONAL_LIGHT_THETA, day);
    let wow = Vec3::new(phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos());
    Vec3::new(wow.x, wow.z, -wow.y).normalize()
}

/// `applyAndMixAmbients` with no precomputed light.
fn mix_ambients(light: &RetailSceneLight, n_dot_l: f32, n_dot_up: f32) -> Vec3 {
    let hemisphere = if n_dot_up >= 0.0 {
        light.horizon_ambient.lerp(light.ambient, n_dot_up)
    } else {
        light.horizon_ambient.lerp(light.ground_ambient, -n_dot_up)
    };
    let sky = hemisphere * 1.1;
    let ground = hemisphere * 0.7;
    ground.lerp(sky, 0.5 + 0.5 * n_dot_l)
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
    let normal = normal.normalize();
    let n_dot_up = normal.dot(Vec3::Y);
    let n_dot_l = normal.dot(-light.sun_direction.normalize()).clamp(0.0, 1.0);
    let ambient = mix_ambients(light, n_dot_l, n_dot_up);
    let direct = light.direct * n_dot_l * sun_visibility;
    diffuse * (ambient + direct)
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
        app.init_asset::<ShaderBuffer>()
            .add_systems(PostUpdate, upload_retail_scene_light);
    }
}

#[cfg(test)]
#[path = "retail_light_tests.rs"]
mod tests;
