//! Retail scene fog from LightData: WebWowViewerCpp's `FogResult` (DayNightLightHolder.cpp
//! `fixLightTimedData` :606-649, `calcLightParamResult` :913-1030, `getLightResultsFromDB`
//! :772-856, `wmoFogDataToFogResult`/`blendWmoFogIntoFogResult` :666-750) and the
//! `PSFog` inputs of `makeFog2` (MapSceneRenderer.cpp:214-294), which
//! `shaders/retail_fog.gdshaderinc` evaluates.
//!
//! Colours are linear RGB, mixed linearly as the other LightData colours are here (the
//! reference mixes its byte colours). The far clip is WebWowViewerCpp's default 1000 yd,
//! as for the legacy fog (`sky_lightdata_data::retail_fog`). Not ported: map flag2 0x2
//! (density 1), skybox sun-direction overrides (LightParams 0x200) and
//! `getClampedFarClip`'s per-map minimum fog distances.
use std::collections::HashMap;

use crate::lighting_assets::{CsvRow, csv_rows};

const FAR_CLIP: f32 = 1000.0;
const DAY_MINUTES: f32 = 2880.0;
/// LightParams flag: no sun fog (`calcLightParamResult` :982-984).
const LIGHT_PARAMS_NO_SUN_FOG: u32 = 0x4;
/// The reference's `LengthSquared() <= FLT_EPSILON` test of a coefficient set.
const EMPTY_COEFFICIENTS: f32 = 1.192_092_9e-7;

/// One LightData keyframe's fog columns.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FogKeyframe {
    pub time: f32,
    pub fog_end: f32,
    pub fog_scaler: f32,
    pub fog_density: f32,
    pub fog_height: f32,
    pub fog_height_scaler: f32,
    pub fog_height_density: f32,
    pub fog_z_scalar: f32,
    pub main_fog_start: f32,
    pub main_fog_end: f32,
    pub sun_fog_angle: f32,
    pub sun_fog_strength: f32,
    pub end_fog_color_distance: f32,
    pub fog_start_offset: f32,
    pub sky_fog_color: [f32; 3],
    pub end_fog_color: [f32; 3],
    pub sun_fog_color: [f32; 3],
    pub fog_height_color: [f32; 3],
    pub end_fog_height_color: [f32; 3],
    /// Polynomial coefficients in LightData order: constant, x, x², x³.
    pub height_coefficients: [f32; 4],
    pub main_coefficients: [f32; 4],
    pub height_density_coefficients: [f32; 4],
}

/// Fog keyframes by LightParams ID, sorted by time.
pub type FogKeyframes = HashMap<u32, Vec<FogKeyframe>>;

/// The reference's `FogResult`: one LightParams' fog at a time of day, or a blend.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FogResult {
    pub fog_scaler: f32,
    pub fog_density: f32,
    pub fog_height: f32,
    pub fog_height_scaler: f32,
    pub fog_height_density: f32,
    pub sun_fog_angle: f32,
    pub fog_color: [f32; 3],
    pub end_fog_color: [f32; 3],
    pub end_fog_color_distance: f32,
    pub sun_fog_color: [f32; 3],
    pub sun_fog_strength: f32,
    pub fog_height_color: [f32; 3],
    pub height_end_fog_color: [f32; 3],
    /// Shader order: x³, x², x, constant.
    pub height_coefficients: [f32; 4],
    pub main_coefficients: [f32; 4],
    pub height_density_coefficients: [f32; 4],
    pub fog_z_scalar: f32,
    /// 1 legacy exponential fog, 0 artistic polynomial fog.
    pub legacy_fog_scalar: f32,
    pub main_fog_start: f32,
    pub main_fog_end: f32,
    pub fog_start_offset: f32,
    pub sun_angle_blend: f32,
}

/// The fog uniforms of `retail_fog.gdshaderinc` (world space, yards).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogUniforms {
    pub color: [f32; 3],
    pub end_color: [f32; 3],
    pub height_color: [f32; 3],
    pub height_end_color: [f32; 3],
    pub sun_color: [f32; 3],
    pub range: [f32; 2],
    pub density: f32,
    pub height_density: f32,
    pub height: f32,
    pub height_rate: f32,
    pub z_scalar: f32,
    pub legacy_scalar: f32,
    pub main_range: [f32; 2],
    pub color_range: [f32; 2],
    pub height_coefficients: [f32; 4],
    pub main_coefficients: [f32; 4],
    pub height_density_coefficients: [f32; 4],
    pub sun_direction: [f32; 3],
    pub sun_angle: f32,
    pub sun_percentage: f32,
}

/// The fog columns of LightData.csv, by LightParams ID.
pub fn parse_fog_keyframes(source: &str) -> Result<FogKeyframes, String> {
    let (columns, lines) = csv_rows(source)?;
    let mut keyframes = FogKeyframes::new();
    for (index, line) in lines {
        if line.is_empty() {
            continue;
        }
        let row = CsvRow::new(&columns, line, index + 2);
        let coefficients = |name: &str| -> Result<[f32; 4], String> {
            let mut values = [0.0; 4];
            for (slot, value) in values.iter_mut().enumerate() {
                *value = row.number(&format!("{name}_{slot}"))?;
            }
            Ok(values)
        };
        let keyframe = FogKeyframe {
            time: row.number("Time")?,
            fog_end: row.number("FogEnd")?,
            fog_scaler: row.number("FogScaler")?,
            fog_density: row.number("FogDensity")?,
            fog_height: row.number("FogHeight")?,
            fog_height_scaler: row.number("FogHeightScaler")?,
            fog_height_density: row.number("FogHeightDensity")?,
            fog_z_scalar: row.number("FogZScalar")?,
            main_fog_start: row.number("MainFogStartDist")?,
            main_fog_end: row.number("MainFogEndDist")?,
            sun_fog_angle: row.number("SunFogAngle")?,
            sun_fog_strength: row.number("SunFogStrength")?,
            end_fog_color_distance: row.number("EndFogColorDistance")?,
            fog_start_offset: row.number("FogStartOffset")?,
            sky_fog_color: row.color("SkyFogColor")?,
            end_fog_color: row.color("EndFogColor")?,
            sun_fog_color: row.color("SunFogColor")?,
            fog_height_color: row.color("FogHeightColor")?,
            end_fog_height_color: row.color("EndFogHeightColor")?,
            height_coefficients: coefficients("FogHeightCoefficients")?,
            main_coefficients: coefficients("MainFogCoefficients")?,
            height_density_coefficients: coefficients("HeightDensityFogCoeff")?,
        };
        keyframes
            .entry(row.parse("LightParamID")?)
            .or_default()
            .push(keyframe);
    }
    for rows in keyframes.values_mut() {
        rows.sort_by(|a, b| a.time.total_cmp(&b.time));
    }
    Ok(keyframes)
}

/// `fixLightTimedData`: unset colours fall back, ranges clamp, a keyframe without density
/// derives one from its FogEnd span.
fn fix_keyframe(mut data: FogKeyframe) -> FogKeyframe {
    if data.end_fog_color == [0.0; 3] {
        data.end_fog_color = data.sky_fog_color;
    }
    if data.fog_height_color == [0.0; 3] {
        data.fog_height_color = data.sky_fog_color;
    }
    if data.end_fog_height_color == [0.0; 3] {
        data.end_fog_height_color = data.end_fog_color;
    }
    data.fog_scaler = data.fog_scaler.clamp(-1.0, 1.0);
    data.fog_end = data.fog_end.max(10.0);
    data.fog_height = data.fog_height.max(-10_000.0);
    data.fog_height_scaler = data.fog_height_scaler.clamp(-1.0, 1.0);
    if data.sun_fog_color == [0.0; 3] {
        data.sun_fog_angle = 1.0;
    }
    if data.end_fog_color_distance <= 0.0 {
        data.end_fog_color_distance = FAR_CLIP;
    }
    if data.fog_height > 10_000.0 {
        data.fog_height = 0.0;
    }
    if data.fog_density <= 0.0 {
        let far = FAR_CLIP.min(700.0) - 200.0;
        let difference = data.fog_end - data.fog_end * data.fog_scaler;
        data.fog_density = if difference > far || difference <= 0.0 {
            1.5
        } else {
            (1.0 - difference / far) * 5.5 + 1.5
        };
    }
    if data.fog_height_scaler == 0.0 {
        data.fog_height_density = data.fog_density;
    }
    data
}

/// The keyframes around `minutes` and the blend between them (`getLightParamData`).
fn bracket(rows: &[FogKeyframe], minutes: f32) -> Option<(&FogKeyframe, &FogKeyframe, f32)> {
    let m = minutes.rem_euclid(DAY_MINUTES);
    let ratio = |start: f32, end: f32, value: f32| {
        let span = end - start;
        if span > 0.0 {
            ((value - start) / span).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    if let Some(pair) = rows
        .windows(2)
        .find(|pair| m >= pair[0].time && m <= pair[1].time)
    {
        return Some((&pair[0], &pair[1], ratio(pair[0].time, pair[1].time, m)));
    }
    let (first, last) = (rows.first()?, rows.last()?);
    let wrapped = if m < last.time { m + DAY_MINUTES } else { m };
    Some((last, first, ratio(last.time, first.time + DAY_MINUTES, wrapped)))
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| mix(a[i], b[i], t))
}

fn mix4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    std::array::from_fn(|i| mix(a[i], b[i], t))
}

fn is_empty(coefficients: [f32; 4]) -> bool {
    coefficients.iter().map(|c| c * c).sum::<f32>() <= EMPTY_COEFFICIENTS
}

/// LightData stores constant..x³; the shader evaluates x³..constant. The data's rows run
/// from about 0 at x = 0 to about 1 at x = 1 only in this order (e.g. 0.003 + 0.387x +
/// 0.346x² + 0.28x³), and the reference's own unset fallback "DB-ordered (0,0,0,1);
/// stored reversed" (:1009) is this reversal; its `CSqliteDB.cpp` reads them unreversed.
fn shader_order(coefficients: [f32; 4]) -> [f32; 4] {
    let [c0, c1, c2, c3] = coefficients;
    [c3, c2, c1, c0]
}

/// One LightParams' fog at `minutes` (`calcLightParamResult`), `flags` its LightParams flags.
pub fn sample_fog(rows: &[FogKeyframe], minutes: f32, flags: u32) -> Option<FogResult> {
    let (raw_a, raw_b, t) = bracket(rows, minutes)?;
    let scaler_floor = if raw_a.fog_density > 0.0 || raw_b.fog_density > 0.0 {
        -0.2
    } else {
        0.0
    };
    let (a, b) = (fix_keyframe(*raw_a), fix_keyframe(*raw_b));
    let (sun_angle, sun_strength, sun_blend) = sun_fog(&a, &b, t);
    let height_coefficients = mix4(a.height_coefficients, b.height_coefficients, t);
    let main_coefficients = mix4(a.main_coefficients, b.main_coefficients, t);
    let height_density_coefficients =
        mix4(a.height_density_coefficients, b.height_density_coefficients, t);
    let legacy = is_empty(main_coefficients) && is_empty(height_density_coefficients);
    Some(FogResult {
        fog_scaler: mix(a.fog_scaler, b.fog_scaler, t).max(scaler_floor),
        fog_density: mix(a.fog_density, b.fog_density, t).max(0.9),
        fog_height: mix(a.fog_height, b.fog_height, t),
        fog_height_scaler: mix(a.fog_height_scaler, b.fog_height_scaler, t),
        fog_height_density: mix(a.fog_height_density, b.fog_height_density, t),
        sun_fog_angle: if flags & LIGHT_PARAMS_NO_SUN_FOG != 0 {
            1.1
        } else {
            sun_angle
        },
        fog_color: mix3(a.sky_fog_color, b.sky_fog_color, t),
        end_fog_color: mix3(a.end_fog_color, b.end_fog_color, t),
        end_fog_color_distance: mix(a.end_fog_color_distance, b.end_fog_color_distance, t),
        sun_fog_color: mix3(a.sun_fog_color, b.sun_fog_color, t),
        sun_fog_strength: sun_strength,
        fog_height_color: mix3(a.fog_height_color, b.fog_height_color, t),
        height_end_fog_color: mix3(a.end_fog_height_color, b.end_fog_height_color, t),
        height_coefficients: if is_empty(height_coefficients) {
            [1.0, 0.0, 0.0, 0.0]
        } else {
            shader_order(height_coefficients)
        },
        main_coefficients: shader_order(main_coefficients),
        height_density_coefficients: shader_order(height_density_coefficients),
        fog_z_scalar: mix(a.fog_z_scalar, b.fog_z_scalar, t),
        legacy_fog_scalar: if legacy { 1.0 } else { 0.0 },
        main_fog_start: mix(a.main_fog_start, b.main_fog_start, t),
        main_fog_end: mix(a.main_fog_end, b.main_fog_end, t),
        fog_start_offset: mix(a.fog_start_offset, b.fog_start_offset, t),
        sun_angle_blend: sun_blend,
    })
}

/// The custom sun blend (:955-980): an angle of 1 or more is no sun fog, so a keyframe
/// without it hands the other's angle and strength over by the time blend.
/// Returns (SunFogAngle, SunFogStrength, SunAngleBlend).
fn sun_fog(a: &FogKeyframe, b: &FogKeyframe, t: f32) -> (f32, f32, f32) {
    match (a.sun_fog_angle >= 1.0, b.sun_fog_angle >= 1.0) {
        (true, true) => (0.0, 0.0, 1.0),
        (true, false) => (b.sun_fog_angle, b.sun_fog_strength, t),
        (false, false) => (
            mix(a.sun_fog_angle, b.sun_fog_angle, t),
            mix(a.sun_fog_strength, b.sun_fog_strength, t),
            1.0,
        ),
        (false, true) => (a.sun_fog_angle, a.sun_fog_strength, 1.0 - t),
    }
}

/// `mixStructure<FogResult>`: every field mixes by `t`.
pub fn mix_fog(a: &FogResult, b: &FogResult, t: f32) -> FogResult {
    FogResult {
        fog_scaler: mix(a.fog_scaler, b.fog_scaler, t),
        fog_density: mix(a.fog_density, b.fog_density, t),
        fog_height: mix(a.fog_height, b.fog_height, t),
        fog_height_scaler: mix(a.fog_height_scaler, b.fog_height_scaler, t),
        fog_height_density: mix(a.fog_height_density, b.fog_height_density, t),
        sun_fog_angle: mix(a.sun_fog_angle, b.sun_fog_angle, t),
        fog_color: mix3(a.fog_color, b.fog_color, t),
        end_fog_color: mix3(a.end_fog_color, b.end_fog_color, t),
        end_fog_color_distance: mix(a.end_fog_color_distance, b.end_fog_color_distance, t),
        sun_fog_color: mix3(a.sun_fog_color, b.sun_fog_color, t),
        sun_fog_strength: mix(a.sun_fog_strength, b.sun_fog_strength, t),
        fog_height_color: mix3(a.fog_height_color, b.fog_height_color, t),
        height_end_fog_color: mix3(a.height_end_fog_color, b.height_end_fog_color, t),
        height_coefficients: mix4(a.height_coefficients, b.height_coefficients, t),
        main_coefficients: mix4(a.main_coefficients, b.main_coefficients, t),
        height_density_coefficients: mix4(
            a.height_density_coefficients,
            b.height_density_coefficients,
            t,
        ),
        fog_z_scalar: mix(a.fog_z_scalar, b.fog_z_scalar, t),
        legacy_fog_scalar: mix(a.legacy_fog_scalar, b.legacy_fog_scalar, t),
        main_fog_start: mix(a.main_fog_start, b.main_fog_start, t),
        main_fog_end: mix(a.main_fog_end, b.main_fog_end, t),
        fog_start_offset: mix(a.fog_start_offset, b.fog_start_offset, t),
        sun_angle_blend: mix(a.sun_angle_blend, b.sun_angle_blend, t),
    }
}

/// The scene fog of a LightParams blend at `minutes` (`getLightResultsFromDB`): the first
/// available LightParams is the base, later ones overlay by weight, as the LightData colours
/// do (`sky_lightdata_data::sample_light_blend`); the sun fog then follows the day curve.
pub fn sample_fog_blend(
    keyframes: &FogKeyframes,
    flags: &HashMap<u32, u32>,
    blend: &[(u32, f32)],
    minutes: f32,
) -> Option<FogResult> {
    let mut layers = blend.iter().filter_map(|&(id, weight)| {
        let rows = keyframes.get(&id)?;
        let flags = flags.get(&id).copied().unwrap_or(0);
        Some((sample_fog(rows, minutes, flags)?, weight))
    });
    let (mut fog, _) = layers.next()?;
    for (layer, weight) in layers {
        fog = mix_fog(&fog, &layer, weight);
    }
    fog.sun_angle_blend *= sun_fog_day_curve(minutes.rem_euclid(DAY_MINUTES) / DAY_MINUTES);
    Some(fog)
}

/// `sunFogStrengthDayCurve` (:757-770): none before 06:30 and after 18:30, full 07:00-18:00.
fn sun_fog_day_curve(day: f32) -> f32 {
    const UP_START: f32 = 0.270_833_3;
    const UP_END: f32 = 0.291_666_7;
    const DOWN_START: f32 = 0.75;
    const DOWN_END: f32 = 0.770_833_3;
    if day <= UP_START || day >= DOWN_END {
        0.0
    } else if day < UP_END {
        (day - UP_START) / (UP_END - UP_START)
    } else if day <= DOWN_START {
        1.0
    } else {
        1.0 - (day - DOWN_START) / (DOWN_END - DOWN_START)
    }
}

/// `wmoFogDataToFogResult`: an MFOG record (`end` yards, start `end * start_scalar`, linear
/// `color`) as a FogResult of legacy fog in that colour.
pub fn wmo_fog(end: f32, start_scalar: f32, color: [f32; 3]) -> FogResult {
    let fog_end = FAR_CLIP.min(end).max(30.0);
    let fog_start = fog_end * start_scalar;
    let far = FAR_CLIP.min(700.0) - 200.0;
    let difference = fog_end - fog_start;
    let density = if difference > far || far <= 0.0 {
        1.5
    } else {
        (1.0 - difference / far) * 5.5 + 1.5
    };
    FogResult {
        fog_scaler: fog_start.max(0.0) / FAR_CLIP.min(3000.0),
        fog_density: density,
        fog_height: -10_000.0,
        fog_height_scaler: 1.0,
        fog_height_density: density,
        fog_color: color,
        end_fog_color: color,
        end_fog_color_distance: 10_000.0,
        sun_fog_color: color,
        fog_height_color: color,
        height_end_fog_color: color,
        legacy_fog_scalar: 1.0,
        ..FogResult::default()
    }
}

/// `blendWmoFogIntoFogResult` (camera out of liquid): `wmo` mixes into `fog` by `weight`.
pub fn blend_wmo_fog(fog: &FogResult, wmo: &FogResult, weight: f32) -> FogResult {
    let mut result = *fog;
    result.fog_color = mix3(fog.fog_color, wmo.fog_color, weight);
    result.end_fog_color = mix3(fog.end_fog_color, wmo.end_fog_color, weight);
    result.sun_fog_color = mix3(fog.sun_fog_color, wmo.sun_fog_color, weight);
    result.fog_height_color = mix3(fog.fog_height_color, wmo.fog_height_color, weight);
    result.fog_scaler = mix(fog.fog_scaler, wmo.fog_scaler, weight);
    result.fog_density = mix(fog.fog_density, wmo.fog_density, weight);
    result.sun_fog_angle = mix(fog.sun_fog_angle, 0.0, weight);
    // A WMO fog without its own height plane (-10000) keeps the exterior's.
    result.fog_height_density = mix(fog.fog_height_density, wmo.fog_density, weight);
    result.fog_z_scalar = fog.fog_z_scalar * (1.0 - weight);
    result.end_fog_color_distance =
        mix(fog.end_fog_color_distance, wmo.end_fog_color_distance, weight);
    result.legacy_fog_scalar = mix(fog.legacy_fog_scalar, 1.0, weight);
    result.sun_fog_strength = fog.sun_fog_strength * (1.0 - weight);
    result
}

/// MapSceneRenderer.cpp:214-294 packing: start = min(farClip, 3000) * FogScaler, end the far
/// clip, densities per yard (x 0.0005), a main-fog curve at least 0.001 yd long.
/// `sun_direction` is the world direction toward the sun.
pub fn fog_uniforms(fog: &FogResult, sun_direction: [f32; 3]) -> FogUniforms {
    const DENSITY_PER_YARD: f32 = 0.000_5;
    let main_start = fog.main_fog_start.max(0.0);
    let main_end = if fog.main_fog_start + 0.001 <= fog.main_fog_end {
        fog.main_fog_end
    } else {
        fog.main_fog_start + 0.001
    };
    let end_color_distance = if fog.end_fog_color_distance > 0.0 {
        fog.end_fog_color_distance
    } else {
        1000.0
    };
    FogUniforms {
        color: fog.fog_color,
        end_color: fog.end_fog_color,
        height_color: fog.fog_height_color,
        height_end_color: fog.height_end_fog_color,
        sun_color: fog.sun_fog_color,
        range: [
            FAR_CLIP.min(3000.0) * fog.fog_scaler,
            FAR_CLIP.max(277.5).min(FAR_CLIP),
        ],
        density: fog.fog_density * DENSITY_PER_YARD,
        height_density: fog.fog_height_density * DENSITY_PER_YARD,
        height: fog.fog_height,
        height_rate: fog.fog_height_scaler,
        z_scalar: fog.fog_z_scalar,
        legacy_scalar: fog.legacy_fog_scalar,
        main_range: [main_start, main_end],
        color_range: [fog.fog_start_offset, end_color_distance],
        height_coefficients: fog.height_coefficients,
        main_coefficients: fog.main_coefficients,
        height_density_coefficients: fog.height_density_coefficients,
        sun_direction,
        sun_angle: fog.sun_fog_angle,
        sun_percentage: fog.sun_angle_blend * fog.sun_fog_strength,
    }
}

/// `sunPhiTable` and `sunThetaTable` (mathHelper.cpp:887-903): the sun disc's path.
const SUN_PHI: [[f32; 2]; 5] = [
    [0.25, 1.745_329_3],
    [0.496_527_8, 0.087_266_46],
    [0.5, 0.087_266_46],
    [0.503_472_2, 0.087_266_46],
    [0.791_666_7, 1.745_329_3],
];
const SUN_THETA: [[f32; 2]; 3] = [[0.25, 0.785_398_2], [0.5, 0.785_398_2], [0.791_666_7, 0.785_398_2]];

/// World (y-up) direction toward the sun disc at `minutes` (`calcSunDirForFog`: the sun
/// planet's `polarToCartesian(phi, theta)`), not the direct light's direction.
pub fn sun_fog_direction(minutes: f32) -> [f32; 3] {
    let day = minutes.rem_euclid(DAY_MINUTES) / DAY_MINUTES;
    let phi = crate::retail_light_data::interp_day_table(&SUN_PHI, day);
    let theta = crate::retail_light_data::interp_day_table(&SUN_THETA, day);
    // WoW z-up (sin φ cos θ, sin φ sin θ, cos φ) → Godot (x, z, -y).
    [phi.sin() * theta.cos(), phi.cos(), -phi.sin() * theta.sin()]
}
