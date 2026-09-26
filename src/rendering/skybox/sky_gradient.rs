//! Bevy colour adapter for the shared procedural sky-dome profile and gradient.

use bevy::prelude::*;

use crate::sky_lightdata::SkyColorSet;

pub(crate) use super::sky_cubemap_data::{
    SKY_BAND_MAX, SKY_DOME_POINT_COUNT, SkyDomePoint, sky_band_at_elevation, sky_dome_profile,
};

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

pub(crate) fn linear_sky_stops(colors: &SkyColorSet) -> [[f32; 3]; SKY_DOME_POINT_COUNT] {
    sky_dome_point_colors(colors).map(|color| {
        let linear = color.to_linear();
        [linear.red, linear.green, linear.blue]
    })
}

/// Linear colour of the dome at a band coordinate.
pub(crate) fn sky_gradient_color(colors: &SkyColorSet, band: f32) -> LinearRgba {
    let [red, green, blue] =
        super::sky_cubemap_data::sky_gradient_color(&linear_sky_stops(colors), band);
    LinearRgba::new(red, green, blue, 1.0)
}

#[cfg(test)]
#[path = "tests/sky_gradient.rs"]
mod tests;
