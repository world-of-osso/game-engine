//! Bevy-free LightData keyframe records and authored interpolation.

use std::collections::HashMap;

/// One authored keyframe for a single LightParamID. Fog distances are raw LightData units.
#[derive(Debug, Clone)]
pub struct LightDataRow<C> {
    pub time: f32,
    pub direct_color: C,
    pub ambient_color: C,
    pub sky_top: C,
    pub sky_middle: C,
    pub sky_band1: C,
    pub sky_band2: C,
    pub sky_smog: C,
    pub fog_color: C,
    pub sun_color: C,
    pub sun_halo_color: C,
    pub cloud_emissive_color: C,
    pub cloud_layer1_ambient_color: C,
    pub cloud_layer2_ambient_color: C,
    pub ocean_close_color: C,
    pub ocean_far_color: C,
    pub river_close_color: C,
    pub river_far_color: C,
    pub horizon_ambient_color: C,
    pub ground_ambient_color: C,
    pub fog_end: f32,
    pub fog_start: f32,
    /// Raw LightData FogScaler. Bevy's loaders leave it 0: Bevy fogs linearly from fog_start/fog_end.
    pub fog_scaler: f32,
    /// Raw LightData FogDensity. Bevy's loaders leave it 0 for the same reason.
    pub fog_density: f32,
    pub glow: f32,
    pub cloud_density: f32,
    pub unk1: f32,
    pub unk2: f32,
}

/// Interpolated sky colors and auxiliary channels. Fog distances are yards.
#[derive(Debug, Clone)]
pub struct SkyColorSet<C> {
    pub sky_top: C,
    pub sky_middle: C,
    pub sky_band1: C,
    pub sky_band2: C,
    pub sky_smog: C,
    pub direct_color: C,
    pub ambient_color: C,
    pub fog_color: C,
    pub sun_color: C,
    pub sun_halo_color: C,
    pub cloud_emissive_color: C,
    pub cloud_layer1_ambient_color: C,
    pub cloud_layer2_ambient_color: C,
    pub ocean_close_color: C,
    pub ocean_far_color: C,
    pub river_close_color: C,
    pub river_far_color: C,
    pub horizon_ambient_color: C,
    pub ground_ambient_color: C,
    pub fog_end: f32,
    pub fog_start: f32,
    /// FogScaler and FogDensity after the reference's per-keyframe and per-LightParams fixes.
    pub fog_scaler: f32,
    pub fog_density: f32,
    pub glow: f32,
    pub cloud_density: f32,
    pub unk1: f32,
    pub unk2: f32,
}

/// Retail legacy exponential fog in yards (commonFogFunctions.slang calculateLegacyFog):
/// fog grows as `1 - exp(-max(0, d - start) * density)` and is total at `end`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetailFog {
    pub start: f32,
    pub end: f32,
    pub density: f32,
}

/// WebWowViewerCpp's default `farPlane` (config.h:119), the far clip its fog derives from.
const FOG_FAR_CLIP: f32 = 1000.0;

/// MapSceneRenderer.cpp:225-246: LightData FogEnd is not a fog distance for modern data;
/// fog starts at farClip * FogScaler with FogDensity * 0.0005 per yard and ends at farClip.
pub fn retail_fog<C>(colors: &SkyColorSet<C>) -> RetailFog {
    RetailFog {
        start: FOG_FAR_CLIP.min(3000.0) * colors.fog_scaler,
        end: FOG_FAR_CLIP.max(277.5).min(FOG_FAR_CLIP),
        density: colors.fog_density * 0.000_5,
    }
}

/// A WMO MFOG record's fog (`end` yards, start at `end * start_scalar`) as legacy fog:
/// DayNightLightHolder.cpp:666-691 wmoFogDataToFogResult (end clamped to [30, farClip],
/// density from the start..end span by calcFogDensityFromStartEnd :652-660, FogEnd =
/// farClip, FogScaler = max(start, 0) / min(farClip, 3000)), read like `retail_fog`.
pub fn wmo_retail_fog(end: f32, start_scalar: f32) -> RetailFog {
    let fog_end = FOG_FAR_CLIP.min(end).max(30.0);
    let fog_start = fog_end * start_scalar;
    let far = FOG_FAR_CLIP.min(700.0) - 200.0;
    let difference = fog_end - fog_start;
    let density = if difference > far || far <= 0.0 {
        1.5
    } else {
        (1.0 - difference / far) * 5.5 + 1.5
    };
    RetailFog {
        start: fog_start.max(0.0),
        end: FOG_FAR_CLIP.max(277.5).min(FOG_FAR_CLIP),
        density: density * 0.000_5,
    }
}

/// DayNightLightHolder.cpp:712-722 blendWmoFogIntoFogResult: FogScaler and FogDensity
/// mix linearly by `weight`, so the derived start and density do too; the end stays farClip.
pub fn blend_wmo_fog(exterior: RetailFog, wmo: RetailFog, weight: f32) -> RetailFog {
    let mix = |a: f32, b: f32| a + (b - a) * weight;
    RetailFog {
        start: mix(exterior.start, wmo.start),
        end: mix(exterior.end, wmo.end),
        density: mix(exterior.density, wmo.density),
    }
}

/// DayNightLightHolder.cpp:620-621 and :633-645 fixLightTimedData: authored FogDensity, or
/// one derived from the FogEnd..FogEnd*FogScaler span when the keyframe has none.
fn keyframe_fog_density<C>(row: &LightDataRow<C>) -> f32 {
    if row.fog_density > 0.0 {
        return row.fog_density;
    }
    let far = FOG_FAR_CLIP.min(700.0) - 200.0;
    let fog_end = row.fog_end.max(10.0);
    let difference = fog_end - fog_end * row.fog_scaler.clamp(-1.0, 1.0);
    if difference > far || difference <= 0.0 {
        1.5
    } else {
        (1.0 - difference / far) * 5.5 + 1.5
    }
}

/// DayNightLightHolder.cpp:1020-1030: after mixing keyframes, density is at least 0.9 and
/// FogScaler at least -0.2 when a keyframe authored density, otherwise at least 0.
fn floor_fog<C>(mut colors: SkyColorSet<C>, rows: [&LightDataRow<C>; 2]) -> SkyColorSet<C> {
    let scaler_floor = if rows.iter().any(|row| row.fog_density > 0.0) {
        -0.2
    } else {
        0.0
    };
    colors.fog_density = colors.fog_density.max(0.9);
    colors.fog_scaler = colors.fog_scaler.max(scaler_floor);
    colors
}

/// LightData FogEnd is authored as fog distance multiplied by 36.
const LIGHT_DATA_FOG_UNITS_PER_YARD: f32 = 36.0;
const DAY_MINUTES: f32 = 2880.0;

fn row_colors<C: Copy>(row: &LightDataRow<C>) -> SkyColorSet<C> {
    SkyColorSet {
        sky_top: row.sky_top,
        sky_middle: row.sky_middle,
        sky_band1: row.sky_band1,
        sky_band2: row.sky_band2,
        sky_smog: row.sky_smog,
        direct_color: row.direct_color,
        ambient_color: row.ambient_color,
        fog_color: row.fog_color,
        sun_color: row.sun_color,
        sun_halo_color: row.sun_halo_color,
        cloud_emissive_color: row.cloud_emissive_color,
        cloud_layer1_ambient_color: row.cloud_layer1_ambient_color,
        cloud_layer2_ambient_color: row.cloud_layer2_ambient_color,
        ocean_close_color: row.ocean_close_color,
        ocean_far_color: row.ocean_far_color,
        river_close_color: row.river_close_color,
        river_far_color: row.river_far_color,
        horizon_ambient_color: row.horizon_ambient_color,
        ground_ambient_color: row.ground_ambient_color,
        fog_end: row.fog_end / LIGHT_DATA_FOG_UNITS_PER_YARD,
        fog_start: row.fog_start / LIGHT_DATA_FOG_UNITS_PER_YARD,
        fog_scaler: row.fog_scaler.clamp(-1.0, 1.0),
        fog_density: keyframe_fog_density(row),
        glow: row.glow,
        cloud_density: row.cloud_density,
        unk1: row.unk1,
        unk2: row.unk2,
    }
}

fn lerp_scalar(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Mix each color through the caller's color-space callback; distances are already yards.
pub fn lerp_color_sets<C: Copy, F: Fn(C, C, f32) -> C>(
    a: &SkyColorSet<C>,
    b: &SkyColorSet<C>,
    t: f32,
    lerp_color: F,
) -> SkyColorSet<C> {
    SkyColorSet {
        sky_top: lerp_color(a.sky_top, b.sky_top, t),
        sky_middle: lerp_color(a.sky_middle, b.sky_middle, t),
        sky_band1: lerp_color(a.sky_band1, b.sky_band1, t),
        sky_band2: lerp_color(a.sky_band2, b.sky_band2, t),
        sky_smog: lerp_color(a.sky_smog, b.sky_smog, t),
        direct_color: lerp_color(a.direct_color, b.direct_color, t),
        ambient_color: lerp_color(a.ambient_color, b.ambient_color, t),
        fog_color: lerp_color(a.fog_color, b.fog_color, t),
        sun_color: lerp_color(a.sun_color, b.sun_color, t),
        sun_halo_color: lerp_color(a.sun_halo_color, b.sun_halo_color, t),
        cloud_emissive_color: lerp_color(a.cloud_emissive_color, b.cloud_emissive_color, t),
        cloud_layer1_ambient_color: lerp_color(
            a.cloud_layer1_ambient_color,
            b.cloud_layer1_ambient_color,
            t,
        ),
        cloud_layer2_ambient_color: lerp_color(
            a.cloud_layer2_ambient_color,
            b.cloud_layer2_ambient_color,
            t,
        ),
        ocean_close_color: lerp_color(a.ocean_close_color, b.ocean_close_color, t),
        ocean_far_color: lerp_color(a.ocean_far_color, b.ocean_far_color, t),
        river_close_color: lerp_color(a.river_close_color, b.river_close_color, t),
        river_far_color: lerp_color(a.river_far_color, b.river_far_color, t),
        horizon_ambient_color: lerp_color(a.horizon_ambient_color, b.horizon_ambient_color, t),
        ground_ambient_color: lerp_color(a.ground_ambient_color, b.ground_ambient_color, t),
        fog_end: lerp_scalar(a.fog_end, b.fog_end, t),
        fog_start: lerp_scalar(a.fog_start, b.fog_start, t),
        fog_scaler: lerp_scalar(a.fog_scaler, b.fog_scaler, t),
        fog_density: lerp_scalar(a.fog_density, b.fog_density, t),
        glow: lerp_scalar(a.glow, b.glow, t),
        cloud_density: lerp_scalar(a.cloud_density, b.cloud_density, t),
        unk1: lerp_scalar(a.unk1, b.unk1, t),
        unk2: lerp_scalar(a.unk2, b.unk2, t),
    }
}

fn interpolation_factor(start: f32, end: f32, value: f32) -> f32 {
    let span = end - start;
    if span > 0.0 {
        (value - start) / span
    } else {
        0.0
    }
}

fn find_bracket<C>(rows: &[LightDataRow<C>], m: f32) -> (&LightDataRow<C>, &LightDataRow<C>, f32) {
    if let Some((a, b)) = rows.windows(2).find_map(|window| {
        let a = &window[0];
        let b = &window[1];
        (m >= a.time && m <= b.time).then_some((a, b))
    }) {
        return (a, b, interpolation_factor(a.time, b.time, m));
    }
    let last = &rows[rows.len() - 1];
    let first = &rows[0];
    let wrap_end_time = first.time + DAY_MINUTES;
    let adjusted_m = if m < last.time { m + DAY_MINUTES } else { m };
    let t = interpolation_factor(last.time, wrap_end_time, adjusted_m);
    (last, first, t)
}

/// Sample sorted keyframes at time 0–2880. Missing rows never synthesize colors.
pub fn interpolate_colors<C: Copy, F: Fn(C, C, f32) -> C>(
    rows: &[LightDataRow<C>],
    minutes: f32,
    lerp_color: F,
) -> Option<SkyColorSet<C>> {
    match rows.len() {
        0 => None,
        1 => {
            let colors = row_colors(&rows[0]);
            let colors = lerp_color_sets(&colors, &colors, 0.0, lerp_color);
            Some(floor_fog(colors, [&rows[0], &rows[0]]))
        }
        _ => {
            let m = minutes.rem_euclid(DAY_MINUTES);
            let (a, b, t) = find_bracket(rows, m);
            let colors = lerp_color_sets(&row_colors(a), &row_colors(b), t, lerp_color);
            Some(floor_fog(colors, [a, b]))
        }
    }
}

/// First available layer is the base; later LightParams overlay in input order.
/// Missing LightParams contribute nothing. The caller supplies linear-space color mixing.
pub fn sample_light_blend<C: Copy, F: Fn(C, C, f32) -> C + Copy>(
    rows_by_param: &HashMap<u32, Vec<LightDataRow<C>>>,
    blend: &[(u32, f32)],
    minutes: f32,
    lerp_color: F,
) -> Option<SkyColorSet<C>> {
    let mut layers = blend.iter().filter_map(|&(param_id, weight)| {
        rows_by_param
            .get(&param_id)
            .and_then(|rows| interpolate_colors(rows, minutes, lerp_color))
            .map(|colors| (colors, weight))
    });
    let (mut colors, _) = layers.next()?;
    for (layer, weight) in layers {
        colors = lerp_color_sets(&colors, &layer, weight, lerp_color);
    }
    Some(colors)
}
