//! Retail sky bodies drawn over the exterior sky dome: the stars model's night alpha and
//! the sun and moon discs (WebWowViewerCpp `DayNightLightHolder::updatePlanetsAndStars`,
//! mathHelper.cpp `starsBrightnessTable` and the planet tables).

use crate::{
    retail_fog::{SUN_PHI, SUN_THETA},
    retail_light_data::interp_day_table,
};

/// The stars model (`environments/stars/stars.m2`, map.cpp `m_starsModel`).
pub const STARS_FDID: u32 = 130_629;

/// LightParams flag 0x10: the Light hides the stars (`starsHideBlend`).
pub const LIGHT_PARAMS_HIDE_STARS: u32 = 0x10;

/// Star brightness over the day: 1 at night, 0 from 05:00 to 20:00, ramping 03:30-05:00
/// and 20:00-21:30.
const STARS_BRIGHTNESS: [[f32; 2]; 4] = [
    [0.145_833_33, 1.0],
    [0.208_333_33, 0.0],
    [0.833_333_3, 0.0],
    [0.895_833_3, 1.0],
];

/// The stars model's alpha at `minutes` (0..2880) under `hide_blend`, the blended
/// LightParams 0x10 flag; `None` when the stars are not drawn.
pub fn stars_alpha(minutes: f32, hide_blend: f32) -> Option<f32> {
    let day = minutes.rem_euclid(2880.0) / 2880.0;
    let byte = interp_day_table(&STARS_BRIGHTNESS, day) * 254.0 + 1.0;
    let alpha = byte / 255.0 * (1.0 - hide_blend);
    (byte >= 2.0 && alpha > 0.0).then_some(alpha)
}

/// Sun, moon and second moon disc textures (map.cpp:1236 `planetTextureFdids`).
pub const PLANET_FDIDS: [u32; 3] = [186_220, 4_629_581, 4_629_582];

/// LightParams flag 0x4 hides the sun disc (`sunPlanetHideBlend`), 0x8 both moons
/// (`moonPlanetHideBlend`), 0x100 sets a custom sun position, which hides every disc.
pub const LIGHT_PARAMS_HIDE_SUN: u32 = 0x4;
pub const LIGHT_PARAMS_HIDE_MOONS: u32 = 0x8;
pub const LIGHT_PARAMS_SUN_POSITION: u32 = 0x100;

/// `moonPhiTable`, `moonThetaTable` and `moon2ThetaTable` (mathHelper.cpp:905-929).
const MOON_PHI: [[f32; 2]; 5] = [
    [0.0, 0.610_865_24],
    [0.003_472_222_2, 0.610_865_24],
    [0.166_666_67, 1.745_329_3],
    [0.895_833_3, 1.745_329_3],
    [0.996_527_8, 0.610_865_24],
];
const MOON_THETA: [[f32; 2]; 3] = [
    [0.0, 0.785_398_2],
    [0.166_666_67, 0.785_398_2],
    [0.895_833_3, 0.785_398_2],
];
const MOON2_THETA: [[f32; 2]; 3] = [
    [0.0, 2.356_194_5],
    [0.166_666_67, 2.617_993_8],
    [0.895_833_3, 2.879_793_2],
];
/// `sunScaleTable` and `moonScaleTable` (mathHelper.cpp:948-963).
const SUN_SCALE: [[f32; 2]; 4] = [
    [0.270_833_34, 2.0],
    [0.302_083_34, 1.0],
    [0.739_583_3, 1.0],
    [0.770_833_3, 2.0],
];
const MOON_SCALE: [[f32; 2]; 4] = [
    [0.041_666_668, 1.0],
    [0.166_666_67, 1.5],
    [0.895_833_3, 1.5],
    [0.978_472_2, 1.0],
];
/// `planetDefs` load scales: sun 1.0, moon 2.2, second moon 1.2.
const PLANET_LOAD_SCALES: [f32; 3] = [1.0, 2.2, 1.2];

/// One sun or moon disc: a camera-facing quad 12 yards from the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanetDraw {
    pub fdid: u32,
    /// Godot (y-up) unit direction from the camera.
    pub direction: [f32; 3],
    /// Quad edge in yards.
    pub scale: f32,
}

/// The discs drawn at `minutes` (0..2880) under the blended LightParams flags 0x4, 0x8 and
/// 0x100 (`updatePlanetsAndStars`: a disc shows while its hide blend is under one half and
/// no custom sun position zeroes its alpha). Discs below the horizon stay listed; the
/// shader fades them out there.
pub fn planet_draws(
    minutes: f32,
    sun_hide: f32,
    moon_hide: f32,
    sun_position_override: f32,
) -> Vec<PlanetDraw> {
    if sun_position_override >= 0.5 {
        return Vec::new();
    }
    let day = minutes.rem_euclid(2880.0) / 2880.0;
    let moon_phi = interp_day_table(&MOON_PHI, day);
    let moon_scale = interp_day_table(&MOON_SCALE, day);
    let planets = [
        (
            interp_day_table(&SUN_PHI, day),
            interp_day_table(&SUN_THETA, day),
            interp_day_table(&SUN_SCALE, day),
            sun_hide,
        ),
        (
            moon_phi,
            interp_day_table(&MOON_THETA, day),
            moon_scale,
            moon_hide,
        ),
        (
            moon_phi,
            interp_day_table(&MOON2_THETA, day),
            moon_scale,
            moon_hide,
        ),
    ];
    planets
        .into_iter()
        .zip(PLANET_FDIDS.into_iter().zip(PLANET_LOAD_SCALES))
        .filter(|((_, _, _, hide), _)| *hide < 0.5)
        .map(|((phi, theta, scale, _), (fdid, load_scale))| PlanetDraw {
            fdid,
            // polarToCartesian in WoW z-up, then Godot (x, z, -y).
            direction: [phi.sin() * theta.cos(), phi.cos(), -phi.sin() * theta.sin()],
            scale: scale * load_scale,
        })
        .collect()
}

/// LightSkybox flag 0x4: the dome's lower cone draws over the skyboxes in the final fog
/// colour (`SkyBoxCollector` `m_overrideValuesWithFinalFog`).
pub const LIGHT_SKYBOX_FINAL_FOG: u32 = 0x4;

/// One LightSkybox model to draw this frame: its FDID, `LightSkybox.Flags` and alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyboxDraw {
    pub fdid: u32,
    /// 0x1: the model's animation follows the time of day; 0x4
    /// (`LIGHT_SKYBOX_FINAL_FOG`): the fog cone draws over it.
    pub flags: u32,
    pub alpha: f32,
}

/// The skyboxes of the LightParams `weights` (each overlays the result so far by its
/// weight), as WebWowViewerCpp `SkyBoxCollector::addSkyBox` collects them: a skybox takes its
/// LightParams' weight (the larger, when two LightParams share it) and every skybox
/// collected before it fades by one minus that weight. `skybox_of` maps a LightParams ID
/// to its LightSkybox `(SkyboxFileDataID, Flags)`, `None` without one.
pub fn skybox_draws(
    weights: &[(u32, f32)],
    skybox_of: impl Fn(u32) -> Option<(u32, u32)>,
) -> Vec<SkyboxDraw> {
    let mut draws: Vec<SkyboxDraw> = Vec::new();
    for &(params, weight) in weights {
        let Some((fdid, flags)) = skybox_of(params).filter(|&(fdid, _)| fdid != 0) else {
            continue;
        };
        let current = match draws.iter().position(|draw| draw.fdid == fdid) {
            Some(index) => {
                draws[index].alpha = draws[index].alpha.max(weight);
                index
            }
            None => {
                draws.push(SkyboxDraw {
                    fdid,
                    flags,
                    alpha: weight,
                });
                draws.len() - 1
            }
        };
        let alpha = draws[current].alpha;
        for (index, draw) in draws.iter_mut().enumerate() {
            if index != current {
                draw.alpha *= 1.0 - alpha;
            }
        }
    }
    draws
}
