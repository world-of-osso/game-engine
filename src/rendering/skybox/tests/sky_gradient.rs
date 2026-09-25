use super::*;
use crate::sky_lightdata::default_sky_colors;

/// Ring elevations of solarityclient's 12340 dome geometry, evaluated independently
/// from its `with_radius` formula (cubic cosine, dome lowered by cos 45°).
const REFERENCE_POINT_ELEVATIONS_DEGREES: [f32; SKY_DOME_POINT_COUNT] =
    [90.0, 15.842, 8.5, 2.158, 0.223, -1.634, -90.0];

fn distinct_stop_colors() -> SkyColorSet {
    SkyColorSet {
        sky_top: Color::srgb_u8(0, 31, 73),
        sky_middle: Color::srgb_u8(82, 127, 167),
        sky_band1: Color::srgb_u8(153, 220, 245),
        sky_band2: Color::srgb_u8(175, 219, 224),
        sky_smog: Color::srgb_u8(180, 180, 180),
        fog_color: Color::srgb_u8(77, 120, 143),
        ..default_sky_colors()
    }
}

#[test]
fn dome_points_sit_at_client_ring_elevations() {
    let elevations = sky_dome_profile().map(|point| point.elevation().to_degrees());
    for (actual, expected) in elevations.iter().zip(REFERENCE_POINT_ELEVATIONS_DEGREES) {
        assert!(
            (actual - expected).abs() < 0.01,
            "ring elevations {elevations:?}, expected {REFERENCE_POINT_ELEVATIONS_DEGREES:?}"
        );
    }
}

#[test]
fn band_coordinate_reaches_each_ring_at_its_elevation() {
    for (index, expected_degrees) in REFERENCE_POINT_ELEVATIONS_DEGREES.iter().enumerate() {
        let band = sky_band_at_elevation(expected_degrees.to_radians());
        assert!(
            (band - index as f32).abs() < 0.01,
            "elevation {expected_degrees}° must map to ring {index}, got band {band}"
        );
    }
}

#[test]
fn band_stops_map_zenith_to_top_and_below_horizon_to_fog() {
    let colors = distinct_stop_colors();
    let as_srgb = |band: f32| Color::from(sky_gradient_color(&colors, band)).to_srgba();
    let expected = [
        (0.0, colors.sky_top),
        (1.0, colors.sky_middle),
        (2.0, colors.sky_band1),
        (3.0, colors.sky_band2),
        (4.0, colors.sky_smog),
        (5.0, colors.fog_color),
        (6.0, colors.fog_color),
    ];
    for (band, color) in expected {
        let actual = as_srgb(band);
        let color = color.to_srgba();
        assert!(
            (actual.red - color.red).abs() < 1e-4
                && (actual.green - color.green).abs() < 1e-4
                && (actual.blue - color.blue).abs() < 1e-4,
            "band {band}: {actual:?} vs {color:?}"
        );
    }
    let nadir = sky_band_at_elevation(-std::f32::consts::FRAC_PI_2);
    assert!((nadir - SKY_BAND_MAX).abs() < 0.01, "nadir band {nadir}");
    assert!(sky_band_at_elevation(-5f32.to_radians()) > 5.0);
}

#[test]
fn upper_sky_is_top_and_middle_only() {
    // Everything above the first ring (≈15.8°) blends only SkyTop and SkyMiddle.
    for degrees in [20.0f32, 30.0, 45.0, 60.0, 89.0] {
        let band = sky_band_at_elevation(degrees.to_radians());
        assert!(
            (0.0..1.0).contains(&band),
            "{degrees}° should lie between SkyTop and SkyMiddle, got band {band}"
        );
    }
    // The straight dome edge puts 45° about halfway along the top-to-middle edge.
    let halfway = sky_band_at_elevation(45f32.to_radians());
    assert!((0.4..0.6).contains(&halfway), "45° band {halfway}");
}
