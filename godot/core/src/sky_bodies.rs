//! Retail sky bodies drawn over the exterior sky dome: the stars model's night alpha
//! (WebWowViewerCpp `DayNightLightHolder::updatePlanetsAndStars`, mathHelper.cpp
//! `starsBrightnessTable`).

use crate::retail_light_data::interp_day_table;

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

/// One LightSkybox model to draw this frame: its FDID, `LightSkybox.Flags` and alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyboxDraw {
    pub fdid: u32,
    /// 0x1: the model's animation follows the time of day; 0x4: an extra sky mesh carries
    /// the final fog values (not drawn).
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
