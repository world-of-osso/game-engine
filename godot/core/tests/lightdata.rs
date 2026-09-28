use std::collections::HashMap;

use game_engine_core::sky_lightdata_data::{
    LightDataRow, interpolate_colors, lerp_color_sets, retail_fog, sample_light_blend,
};

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn assert_color(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
    }
}

fn row(time: f32, color: [f32; 3]) -> LightDataRow<[f32; 3]> {
    LightDataRow {
        time,
        direct_color: color,
        ambient_color: color,
        sky_top: color,
        sky_middle: color,
        sky_band1: color,
        sky_band2: color,
        sky_smog: color,
        fog_color: color,
        sun_color: color,
        sun_halo_color: color,
        cloud_emissive_color: color,
        cloud_layer1_ambient_color: color,
        cloud_layer2_ambient_color: color,
        ocean_close_color: color,
        ocean_far_color: color,
        river_close_color: color,
        river_far_color: color,
        horizon_ambient_color: color,
        ground_ambient_color: color,
        fog_end: 0.0,
        fog_start: 0.0,
        fog_scaler: 0.0,
        fog_density: 0.0,
        glow: 0.0,
        cloud_density: 0.0,
        unk1: 0.0,
        unk2: 0.0,
    }
}

#[test]
fn midnight_wrap_uses_last_to_first_keyframe() {
    let rows = [row(720.0, [0.0; 3]), row(2160.0, [1.0; 3])];
    assert_eq!(
        interpolate_colors(&rows, 0.0, mix).unwrap().sky_top,
        [0.5; 3]
    );
    assert_eq!(
        interpolate_colors(&rows, 2880.0, mix).unwrap().sky_top,
        [0.5; 3]
    );
}

#[test]
fn midpoint_interpolates_linear_colors_and_all_authored_channels() {
    let mut early = row(0.0, [0.0, 0.2, 0.4]);
    early.fog_end = 3600.0;
    early.fog_start = 360.0;
    early.sun_halo_color = [0.1, 0.3, 0.5];
    early.river_far_color = [0.2, 0.4, 0.6];
    early.glow = 0.2;
    early.unk1 = 2.0;
    let mut late = row(1440.0, [1.0, 0.6, 0.8]);
    late.fog_end = 7200.0;
    late.fog_start = 1440.0;
    late.sun_halo_color = [0.9, 0.7, 0.5];
    late.river_far_color = [0.6, 0.8, 1.0];
    late.glow = 0.6;
    late.unk1 = 6.0;
    late.unk2 = 8.0;
    late.cloud_density = 0.8;
    let colors = interpolate_colors(&[early, late], 720.0, mix).unwrap();
    assert_color(colors.sky_top, [0.5, 0.4, 0.6]);
    assert_eq!(colors.fog_end, 150.0);
    assert_eq!(colors.fog_start, 25.0);
    assert_color(colors.sun_halo_color, [0.5, 0.5, 0.5]);
    assert_color(colors.river_far_color, [0.4, 0.6, 0.8]);
    assert!((colors.glow - 0.4).abs() < 1e-6);
    assert!((colors.cloud_density - 0.4).abs() < 1e-6);
    assert_eq!(colors.unk1, 4.0);
    assert_eq!(colors.unk2, 4.0);
}

#[test]
fn missing_rows_return_none_instead_of_synthetic_sky() {
    assert!(interpolate_colors::<[f32; 3], _>(&[], 1440.0, mix).is_none());
    let rows = HashMap::new();
    assert!(sample_light_blend(&rows, &[(12, 1.0)], 1440.0, mix).is_none());
}

#[test]
fn overlays_are_applied_in_input_order_and_skip_missing_rows() {
    let rows = HashMap::from([
        (12, vec![row(0.0, [0.0; 3])]),
        (30, vec![row(0.0, [1.0; 3])]),
        (31, vec![row(0.0, [0.0, 0.0, 1.0])]),
    ]);
    let result = sample_light_blend(
        &rows,
        &[(99, 0.9), (12, 1.0), (30, 0.5), (31, 0.25)],
        1440.0,
        mix,
    )
    .unwrap();
    assert_eq!(result.sky_top, [0.375, 0.375, 0.625]);
    assert_eq!(result.sun_color, [0.375, 0.375, 0.625]);
}

#[test]
fn color_set_blend_does_not_convert_fog_units_twice() {
    let first = interpolate_colors(&[row(0.0, [0.0; 3])], 0.0, mix).unwrap();
    let mut second_row = row(0.0, [1.0; 3]);
    second_row.fog_end = 3600.0;
    let second = interpolate_colors(&[second_row], 0.0, mix).unwrap();
    assert_eq!(lerp_color_sets(&first, &second, 0.5, mix).fog_end, 50.0);
}

// DayNightLightHolder.cpp:606-649 fixLightTimedData, then :1020-1030 per-LightParams floors.
#[test]
fn retail_fog_follows_reference_row_fixes_and_density_floor() {
    let mut legacy = row(0.0, [0.0; 3]);
    legacy.fog_end = 200.0;
    legacy.fog_scaler = 0.5;
    // No FogDensity: derived from FogEnd - FogEnd * FogScaler = 100 within (700 - 200).
    let fog = retail_fog(&interpolate_colors(&[legacy.clone()], 0.0, mix).unwrap());
    assert!((fog.density - (((1.0 - 100.0 / 500.0) * 5.5 + 1.5) * 0.0005)).abs() < 1e-7);
    assert_eq!((fog.start, fog.end), (500.0, 1000.0));

    // FogEnd below 10 is raised to 10 before the density heuristic; FogScaler is clamped to 1.
    legacy.fog_end = 0.0;
    legacy.fog_scaler = 3.0;
    let fog = retail_fog(&interpolate_colors(&[legacy], 0.0, mix).unwrap());
    assert_eq!(fog.start, 1000.0);
    assert!((fog.density - 1.5 * 0.0005).abs() < 1e-7);

    // Authored density is floored at 0.9 and lets FogScaler reach -0.2, but no lower.
    let mut modern = row(0.0, [0.0; 3]);
    modern.fog_density = 0.5;
    modern.fog_scaler = -0.6;
    let fog = retail_fog(&interpolate_colors(&[modern], 0.0, mix).unwrap());
    assert!((fog.density - 0.9 * 0.0005).abs() < 1e-7);
    assert!((fog.start + 200.0).abs() < 1e-3);
}
