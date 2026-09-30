//! Retail scene fog from LightData (WebWowViewerCpp DayNightLightHolder `FogResult` and
//! MapSceneRenderer `PSFog` packing), 12.1.0.69933 local exports.
use std::collections::HashMap;
use std::sync::OnceLock;

use game_engine_core::{
    lighting_assets::authored_to_linear_rgb,
    retail_fog::{
        FogKeyframe, FogKeyframes, blend_wmo_fog, fog_uniforms, mix_fog, parse_fog_keyframes,
        sample_fog, sample_fog_blend, sun_fog_direction, wmo_fog,
    },
};

fn keyframes() -> &'static FogKeyframes {
    static KEYFRAMES: OnceLock<FogKeyframes> = OnceLock::new();
    KEYFRAMES.get_or_init(|| {
        let text = std::fs::read_to_string("data/LightData.csv").unwrap();
        parse_fog_keyframes(&text).unwrap()
    })
}

fn close(actual: &[f32], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(a, e)| (a - e).abs() < 1e-5)
}

fn packed(color: u32) -> [f32; 3] {
    let [_, r, g, b] = color.to_be_bytes();
    authored_to_linear_rgb([r, g, b].map(|channel| f32::from(channel) / 255.0))
}

/// LightParams 17 at 06:00 (its 720 keyframe): height fog with authored coefficients
/// -0.007 + 0.447x - 0.167x² + 0.733x³, which the shader takes highest power first;
/// legacy fog (no main or height-density curves); no sun fog (angles 1).
#[test]
fn authored_height_fog_coefficients_reverse_into_shader_order() {
    let fog = sample_fog(&keyframes()[&17], 720.0, 0).unwrap();
    assert!(close(
        &fog.height_coefficients,
        &[0.732_971_2, -0.167_419_43, 0.447_448_73, -0.007_004_261]
    ));
    assert_eq!(fog.legacy_fog_scalar, 1.0);
    assert!(close(
        &[fog.fog_height, fog.fog_height_scaler, fog.fog_height_density, fog.fog_density],
        &[0.0, 0.002_857_142_8, 1.5, 4.5]
    ));
    assert_eq!((fog.sun_fog_angle, fog.sun_fog_strength, fog.sun_angle_blend), (0.0, 0.0, 1.0));
}

/// LightParams 4216 at noon (its 1440 keyframe) authors a main fog curve, so the fog is
/// artistic (LegacyFogScalar 0); its sun fog (angle 0.9848 < 1, strength 0.7, colour
/// 0xFFF5D4) is at full day strength. At 13:30 (810) the next keyframe (840) has angle 1:
/// the first keyframe's sun fog fades by 1 - t = 0.5, and the 06:30-07:00 day ramp halves it.
#[test]
fn main_fog_curves_make_artistic_fog_and_sun_fog_follows_the_day() {
    let rows = &keyframes()[&4216];
    let fog = sample_fog(rows, 1440.0, 0).unwrap();
    assert_eq!(fog.legacy_fog_scalar, 0.0);
    assert!(close(
        &fog.main_coefficients,
        &[-2.321_960_4, 3.395_996, -0.180_206_3, 0.039_840_7]
    ));
    assert!(close(&[fog.sun_fog_angle, fog.sun_fog_strength], &[0.984_807_7, 0.7]));
    assert!(close(&fog.sun_fog_color, &packed(16_774_356)));
    let flags = HashMap::new();
    let noon = sample_fog_blend(keyframes(), &flags, &[(4216, 1.0)], 1440.0).unwrap();
    assert!((fog_uniforms(&noon, [0.0; 3]).sun_percentage - 0.7).abs() < 1e-5);

    let half = sample_fog(rows, 810.0, 0).unwrap();
    assert!(close(&[half.sun_fog_angle, half.sun_angle_blend], &[0.999_600_2, 0.5]));
    let dawn = sample_fog_blend(keyframes(), &flags, &[(4216, 1.0)], 810.0).unwrap();
    assert!((dawn.sun_angle_blend - 0.25).abs() < 1e-4, "{}", dawn.sun_angle_blend);
    // LightParams flag 0x4 turns the sun fog off.
    assert_eq!(sample_fog(rows, 1440.0, 0x4).unwrap().sun_fog_angle, 1.1);
    let off = sample_fog_blend(keyframes(), &flags, &[(4216, 1.0)], 300.0).unwrap();
    assert_eq!(off.sun_angle_blend, 0.0, "night has no sun fog");
}

/// `fixLightTimedData`: unset end/height colours take the sky fog colour, no sun colour is
/// no sun fog, a keyframe without density derives one from its FogEnd span, and without a
/// height scaler the height density is the scene's; density is at least 0.9.
#[test]
fn unset_keyframe_fog_fields_take_the_reference_fallbacks() {
    let sky = [0.2, 0.3, 0.4];
    let row = FogKeyframe {
        time: 0.0,
        fog_end: 400.0,
        fog_scaler: 0.5,
        sky_fog_color: sky,
        sun_fog_angle: 0.5,
        ..FogKeyframe::default()
    };
    let fog = sample_fog(&[row], 0.0, 0).unwrap();
    assert_eq!((fog.end_fog_color, fog.fog_height_color, fog.height_end_fog_color), (sky, sky, sky));
    assert_eq!(fog.sun_fog_angle, 0.0, "no sun colour: angle 1 in both keyframes");
    // Span 400 - 200 of min(1000, 700) - 200: (1 - 0.4) * 5.5 + 1.5.
    assert!((fog.fog_density - 4.8).abs() < 1e-5);
    assert_eq!(fog.fog_height_density, fog.fog_density);
    assert_eq!(fog.end_fog_color_distance, 1000.0);
    assert_eq!(fog.height_coefficients, [1.0, 0.0, 0.0, 0.0], "unset: x³");
    // An authored density lets FogScaler go down to -0.2; without one, 0.
    let low = FogKeyframe { fog_scaler: -0.9, ..row };
    assert_eq!(sample_fog(&[low], 0.0, 0).unwrap().fog_scaler, 0.0);
    let authored = FogKeyframe { fog_density: 0.5, ..low };
    let fog = sample_fog(&[authored], 0.0, 0).unwrap();
    assert_eq!((fog.fog_scaler, fog.fog_density), (-0.2, 0.9));
}

/// MFOG fog blended in (`blendWmoFogIntoFogResult`): colours, scaler and density mix by the
/// weight, sun fog fades out, the exterior height plane stays, legacy fog takes over.
#[test]
fn wmo_fog_blends_into_the_scene_fog() {
    let exterior = sample_fog(&keyframes()[&4216], 1440.0, 0).unwrap();
    let cave = wmo_fog(578.0, 0.129, [0.1, 0.3, 0.4]);
    assert!((cave.fog_scaler - 0.074_562).abs() < 1e-6 && cave.fog_density == 1.5);
    let half = blend_wmo_fog(&exterior, &cave, 0.5);
    assert!((half.fog_scaler - (exterior.fog_scaler + cave.fog_scaler) / 2.0).abs() < 1e-6);
    assert!((half.fog_density - (exterior.fog_density + 1.5) / 2.0).abs() < 1e-5);
    assert!((half.sun_fog_angle - exterior.sun_fog_angle / 2.0).abs() < 1e-6);
    assert!((half.sun_fog_strength - exterior.sun_fog_strength / 2.0).abs() < 1e-6);
    assert_eq!(half.fog_height, exterior.fog_height);
    assert_eq!(half.legacy_fog_scalar, 0.5);
    assert_eq!(blend_wmo_fog(&exterior, &cave, 1.0).fog_color, cave.fog_color);
    assert_eq!(mix_fog(&exterior, &cave, 0.0), exterior);
}

/// MapSceneRenderer.cpp:225-294: start min(1000, 3000) * FogScaler, end 1000, densities
/// per yard, a main curve of at least 0.001 yd, EndFogColorDistance 1000 when unset.
#[test]
fn fog_uniforms_pack_like_map_scene_renderer() {
    let mut fog = sample_fog(&keyframes()[&17], 720.0, 0).unwrap();
    fog.fog_scaler = 0.25;
    fog.main_fog_start = 50.0;
    fog.main_fog_end = 10.0;
    fog.end_fog_color_distance = 0.0;
    let uniforms = fog_uniforms(&fog, [0.0, 1.0, 0.0]);
    assert_eq!(uniforms.range, [250.0, 1000.0]);
    assert!((uniforms.density - 4.5 * 0.000_5).abs() < 1e-9);
    assert!((uniforms.height_density - 1.5 * 0.000_5).abs() < 1e-9);
    assert!(close(&uniforms.main_range, &[50.0, 50.001]));
    assert_eq!(uniforms.color_range, [0.0, 1000.0]);
}

/// The sun disc (`sunPhiTable`): 5° from the zenith at noon, 10° below the horizon at 06:00.
#[test]
fn fog_sun_direction_follows_the_sun_disc() {
    let noon = sun_fog_direction(1440.0);
    assert!((noon[1] - 5f32.to_radians().cos()).abs() < 1e-5, "{noon:?}");
    let dawn = sun_fog_direction(720.0);
    assert!((dawn[1] - 100f32.to_radians().cos()).abs() < 1e-5, "{dawn:?}");
    let length = dawn.iter().map(|c| c * c).sum::<f32>().sqrt();
    assert!((length - 1.0).abs() < 1e-5);
}
