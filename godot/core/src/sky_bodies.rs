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
