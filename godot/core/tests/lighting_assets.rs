use game_engine_core::{
    light_lookup_data::{LightParamsSlot, light_params_blend},
    lighting_assets::{parse_light_csv, parse_light_data_csv},
    sky_lightdata_data::{RetailFog, retail_fog, sample_light_blend},
};
use std::{fs, path::PathBuf};

fn fixture(name: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data")
            .join(name),
    )
    .expect("local authored lighting fixture")
}

#[test]
fn authored_light_rows_keep_world_positions_radii_and_circumstance_slots() {
    let rows = parse_light_csv(&fixture("Light.csv")).expect("authored Light.csv");
    assert_eq!(rows.len(), 5072);
    let global = rows.iter().find(|row| row.id == 1).unwrap();
    assert_eq!(global.map_id, 0);
    assert_eq!(global.position, [0.0; 3]);
    assert_eq!(global.light_params_ids, [12, 13, 10, 13, 3, 0, 0, 0]);
    let local = rows.iter().find(|row| row.id == 2).unwrap();
    assert_eq!(local.position, [-10666.672, 64.0, 0.0]);
    assert!((local.falloff_start - 399.47433).abs() < 0.001);
    assert!((local.falloff_end - 466.97827).abs() < 0.001);
}

#[test]
fn authored_lightdata_rows_are_sorted_and_keep_linear_color_and_raw_fog_units() {
    let rows = parse_light_data_csv(&fixture("LightData.csv")).expect("authored LightData.csv");
    let elwynn = rows.get(&12).expect("Elwynn LightParams 12");
    assert_eq!(elwynn.len(), 12);
    assert_eq!(
        elwynn.iter().map(|row| row.time).collect::<Vec<_>>(),
        vec![
            0.0, 540.0, 720.0, 840.0, 1440.0, 1980.0, 2040.0, 2140.0, 2280.0, 2460.0, 2520.0,
            2640.0
        ]
    );
    let noon = &elwynn[4];
    // Authored DirectColor 0x6a5642 is decoded to linear channels, not byte/255.
    let expected = [0.14412849, 0.09305898, 0.05448028];
    for (actual, expected) in noon.direct_color.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.000001,
            "{actual} != {expected}"
        );
    }
    assert_eq!(noon.horizon_ambient_color, [0.0; 3]);
    assert_eq!(noon.ground_ambient_color, [0.0; 3]);
    assert_eq!(noon.fog_end, 18000.0);
    assert_eq!(noon.fog_start, 4500.0);
    // Real signed ARGB export -3502459 retains RGB ca8e85, rather than becoming black.
    let signed = rows[&8].iter().find(|row| row.time == 1020.0).unwrap();
    for (actual, expected) in signed
        .sky_band2
        .into_iter()
        .zip([0.59061885, 0.2704978, 0.23455058])
    {
        assert!((actual - expected).abs() < 0.000001);
    }
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|index| a[index] + (b[index] - a[index]) * t)
}

// WarbandScene 7 (Freywold Spring) and 25 (Gallagio Grand Gallery) resolve to map-global
// LightParams whose every keyframe authors FogEnd 0. Retail does not read FogEnd as a linear
// range: MapSceneRenderer.cpp:225-246 fogs exponentially from farClip * FogScaler with
// FogDensity * 0.0005 per yard and fades out at farClip, so these scenes stay visible.
#[test]
fn campsite_light_params_with_zero_fog_end_use_retail_exponential_fog() {
    let lights = parse_light_csv(&fixture("Light.csv")).expect("authored Light.csv");
    let keyframes =
        parse_light_data_csv(&fixture("LightData.csv")).expect("authored LightData.csv");
    for (map_id, position, light_params_id, expected) in [
        (
            2847,
            [-2808.0034, 395.20139, 81.541237],
            5615,
            RetailFog {
                start: 20.0,
                end: 1000.0,
                density: 0.002,
            },
        ),
        (
            2851,
            [431.86978, -802.94617, 27.449316],
            6412,
            RetailFog {
                start: 0.0,
                end: 1000.0,
                density: 0.0025,
            },
        ),
    ] {
        let blend = light_params_blend(&lights, map_id, position, LightParamsSlot::Clear);
        let weights: Vec<_> = blend
            .iter()
            .map(|light| (light.light_params_id, light.weight))
            .collect();
        assert_eq!(weights, [(light_params_id, 1.0)]);
        assert!(
            keyframes[&light_params_id]
                .iter()
                .all(|row| row.fog_end == 0.0)
        );
        let sky = sample_light_blend(&keyframes, &weights, 1440.0, lerp_rgb).unwrap();
        let fog = retail_fog(&sky);
        for (actual, expected) in [
            (fog.start, expected.start),
            (fog.end, expected.end),
            (fog.density, expected.density),
        ] {
            assert!((actual - expected).abs() < 1e-5, "{fog:?} != {expected:?}");
        }
    }
}

#[test]
fn malformed_authoring_is_reported_instead_of_zero_or_empty_defaults() {
    assert!(parse_light_csv("").unwrap_err().contains("header"));
    assert!(
        parse_light_data_csv("ID,Time\n1,0\n")
            .unwrap_err()
            .contains("LightParamID")
    );
    let light = fixture("Light.csv").replacen("1,0,0,0,0,0,0,12", "1,broken,0,0,0,0,0,12", 1);
    assert!(
        parse_light_csv(&light)
            .unwrap_err()
            .contains("GameCoords_0")
    );
    let data = fixture("LightData.csv").replacen("20947,492,0,3848191", "20947,492,0,broken", 1);
    assert!(
        parse_light_data_csv(&data)
            .unwrap_err()
            .contains("DirectColor")
    );
}
