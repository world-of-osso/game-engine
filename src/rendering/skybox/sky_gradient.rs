//! Procedural sky-dome gradient: the WoW client's dome ring profile and the
//! LightData colour stop that each ring carries.
//!
//! Source: the build 12340 dome reproduced in solarityclient
//! `crates/rendering/src/weather/sky.rs` (`WorldSkyDome::with_radius` and
//! `update_packed`). Two poles and five rings; ring latitudes are fractions of
//! half a turn, evaluated with the client's cubic cosine approximation, and the
//! whole dome is lowered by cos(45°). SkyTop sits at the zenith, the four
//! remaining sky bands occupy the rings down to about 2° above the horizon,
//! and SkyFogColor covers the ring just below the horizon and the bottom pole.

use bevy::prelude::*;

use crate::sky_lightdata::SkyColorSet;

/// Dome points from the top pole to the bottom pole, in half-turn fractions.
const SKY_DOME_RING_LATITUDES: [f32; SKY_DOME_POINT_COUNT] =
    [0.0, 0.17, 0.2, 0.23, 0.24, 0.25, 1.0];

pub(crate) const SKY_DOME_POINT_COUNT: usize = 7;

/// Highest band coordinate: the bottom pole.
pub(crate) const SKY_BAND_MAX: f32 = (SKY_DOME_POINT_COUNT - 1) as f32;

/// One dome point for a unit dome: horizontal radius and height above the eye.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkyDomePoint {
    pub horizontal: f32,
    pub height: f32,
}

impl SkyDomePoint {
    pub(crate) fn elevation(self) -> f32 {
        self.height.atan2(self.horizontal)
    }
}

/// The client's periodic cubic stand-in for cos(π·phase).
fn client_periodic_cosine(phase: f64) -> f64 {
    let period = if phase <= 0.0 {
        phase as i32 - 1
    } else {
        phase as i32
    };
    let fraction = phase - f64::from(period);
    let value = 1.0 - (6.0 - 4.0 * fraction) * fraction * fraction;
    if period & 1 != 0 { -value } else { value }
}

pub(crate) fn sky_dome_profile() -> [SkyDomePoint; SKY_DOME_POINT_COUNT] {
    let lowered_by = std::f64::consts::FRAC_PI_4.cos();
    SKY_DOME_RING_LATITUDES.map(|latitude| {
        let phase = f64::from(latitude);
        SkyDomePoint {
            horizontal: client_periodic_cosine(phase - 0.5).abs() as f32,
            height: (client_periodic_cosine(phase) - lowered_by) as f32,
        }
    })
}

/// Colour stop per dome point, top pole first.
pub(crate) fn sky_dome_point_colors(colors: &SkyColorSet) -> [Color; SKY_DOME_POINT_COUNT] {
    [
        colors.sky_top,
        colors.sky_middle,
        colors.sky_band1,
        colors.sky_band2,
        colors.sky_smog,
        colors.fog_color,
        colors.fog_color,
    ]
}

/// Band coordinate seen along a view elevation: 0 at the zenith, `i` on ring `i`,
/// and `SKY_BAND_MAX` at the nadir. Between two dome points the value is the
/// position along the straight dome edge, which is how the GPU interpolates it.
pub(crate) fn sky_band_at_elevation(elevation: f32) -> f32 {
    let profile = sky_dome_profile();
    let (sin, cos) = elevation.sin_cos();
    for (index, pair) in profile.windows(2).enumerate() {
        let (upper, lower) = (pair[0], pair[1]);
        if elevation > lower.elevation() || index + 2 == SKY_DOME_POINT_COUNT {
            return index as f32 + edge_fraction(upper, lower, sin, cos);
        }
    }
    SKY_BAND_MAX
}

/// Fraction along the edge from `upper` to `lower` where the view ray crosses it.
fn edge_fraction(upper: SkyDomePoint, lower: SkyDomePoint, sin: f32, cos: f32) -> f32 {
    let dh = lower.horizontal - upper.horizontal;
    let dz = lower.height - upper.height;
    let denominator = dh * sin - dz * cos;
    if denominator.abs() <= f32::EPSILON {
        return 0.0;
    }
    ((upper.height * cos - upper.horizontal * sin) / denominator).clamp(0.0, 1.0)
}

/// Linear colour of the dome at a band coordinate.
pub(crate) fn sky_gradient_color(colors: &SkyColorSet, band: f32) -> LinearRgba {
    let stops = sky_dome_point_colors(colors);
    let band = band.clamp(0.0, SKY_BAND_MAX);
    let index = (band.floor() as usize).min(SKY_DOME_POINT_COUNT - 2);
    let t = band - index as f32;
    let a = stops[index].to_linear();
    let b = stops[index + 1].to_linear();
    LinearRgba::new(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        1.0,
    )
}

#[cfg(test)]
#[path = "tests/sky_gradient.rs"]
mod tests;
