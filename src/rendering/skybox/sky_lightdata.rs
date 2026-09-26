//! LightData keyframe loading and sky color interpolation.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use bevy::prelude::Color;
use serde::Deserialize;

#[path = "sky_lightdata_data.rs"]
mod data;

pub type LightDataRow = data::LightDataRow<Color>;
pub type SkyColorSet = data::SkyColorSet<Color>;

#[path = "sky_lightdata_cache.rs"]
mod cache;

#[derive(Debug, Deserialize)]
struct LightDataFile {
    by_param: BTreeMap<u32, Vec<LightDataSerializedRow>>,
}

#[derive(Debug, Deserialize)]
struct LightDataSerializedRow {
    time: f32,
    direct_color: u32,
    ambient_color: u32,
    sky_top: u32,
    sky_middle: u32,
    sky_band1: u32,
    sky_band2: u32,
    sky_smog: u32,
    #[serde(default)]
    fog_color: u32,
    #[serde(default)]
    sun_color: u32,
    #[serde(default)]
    sun_halo_color: u32,
    #[serde(default)]
    cloud_emissive_color: u32,
    #[serde(default)]
    cloud_layer1_ambient_color: u32,
    #[serde(default)]
    cloud_layer2_ambient_color: u32,
    #[serde(default)]
    ocean_close_color: u32,
    #[serde(default)]
    ocean_far_color: u32,
    #[serde(default)]
    river_close_color: u32,
    #[serde(default)]
    river_far_color: u32,
    #[serde(default)]
    horizon_ambient_color: u32,
    #[serde(default)]
    ground_ambient_color: u32,
    #[serde(default)]
    fog_end: f32,
    #[serde(default)]
    fog_start: f32,
    #[serde(default)]
    glow: f32,
    #[serde(default)]
    cloud_density: f32,
    #[serde(default)]
    unk1: f32,
    #[serde(default)]
    unk2: f32,
}

/// Decode a LightData colour: the integer value is 0x00RRGGBB, and the bytes are
/// sRGB-encoded authored colours.
pub fn decode_light_color(val: u32) -> Color {
    let [_, r, g, b] = val.to_be_bytes();
    Color::srgb_u8(r, g, b)
}

fn deserialize_light_row(row: LightDataSerializedRow) -> LightDataRow {
    LightDataRow {
        time: row.time,
        direct_color: decode_light_color(row.direct_color),
        ambient_color: decode_light_color(row.ambient_color),
        sky_top: decode_light_color(row.sky_top),
        sky_middle: decode_light_color(row.sky_middle),
        sky_band1: decode_light_color(row.sky_band1),
        sky_band2: decode_light_color(row.sky_band2),
        sky_smog: decode_light_color(row.sky_smog),
        fog_color: decode_light_color(row.fog_color),
        sun_color: decode_light_color(row.sun_color),
        sun_halo_color: decode_light_color(row.sun_halo_color),
        cloud_emissive_color: decode_light_color(row.cloud_emissive_color),
        cloud_layer1_ambient_color: decode_light_color(row.cloud_layer1_ambient_color),
        cloud_layer2_ambient_color: decode_light_color(row.cloud_layer2_ambient_color),
        ocean_close_color: decode_light_color(row.ocean_close_color),
        ocean_far_color: decode_light_color(row.ocean_far_color),
        river_close_color: decode_light_color(row.river_close_color),
        river_far_color: decode_light_color(row.river_far_color),
        horizon_ambient_color: decode_light_color(row.horizon_ambient_color),
        ground_ambient_color: decode_light_color(row.ground_ambient_color),
        fog_end: row.fog_end,
        fog_start: row.fog_start,
        glow: row.glow,
        cloud_density: row.cloud_density,
        unk1: row.unk1,
        unk2: row.unk2,
    }
}

fn load_light_data_ron(path: &str, param_id: u32) -> Result<Vec<LightDataRow>, String> {
    let contents = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let mut file: LightDataFile =
        ron::from_str(&contents).map_err(|e| format!("parse {path}: {e}"))?;
    let mut rows: Vec<LightDataRow> = file
        .by_param
        .remove(&param_id)
        .unwrap_or_default()
        .into_iter()
        .map(deserialize_light_row)
        .collect();
    rows.sort_by(|a, b| a.time.total_cmp(&b.time));
    Ok(rows)
}

/// Resolve CSV column indices for legacy LightData.csv fallback.
fn resolve_csv_fallback_column_indices(header: &str) -> [usize; 27] {
    let cols: Vec<&str> = header.split(',').collect();
    let idx =
        |name: &str, fallback: usize| cols.iter().position(|c| *c == name).unwrap_or(fallback);
    let mut indices = [0usize; 27];
    indices[..20].copy_from_slice(&resolve_csv_fallback_color_column_indices(&idx));
    indices[20..26].copy_from_slice(&resolve_csv_fallback_fog_and_aux_column_indices(&idx));
    indices[26] = idx("GroundAmbientColor", 35);
    indices
}

fn resolve_csv_fallback_color_column_indices(idx: &impl Fn(&str, usize) -> usize) -> [usize; 20] {
    [
        idx("LightParamID", 1),
        idx("Time", 2),
        idx("DirectColor", 3),
        idx("AmbientColor", 4),
        idx("SkyTopColor", 5),
        idx("SkyMiddleColor", 6),
        idx("SkyBand1Color", 7),
        idx("SkyBand2Color", 8),
        idx("SkySmogColor", 9),
        idx("SkyFogColor", 10),
        idx("SunColor", 11),
        idx("CloudSunColor", 12),
        idx("CloudEmissiveColor", 13),
        idx("CloudLayer1AmbientColor", 14),
        idx("CloudLayer2AmbientColor", 15),
        idx("OceanCloseColor", 16),
        idx("OceanFarColor", 17),
        idx("RiverCloseColor", 18),
        idx("RiverFarColor", 19),
        idx("HorizonAmbientColor", 34),
    ]
}

fn resolve_csv_fallback_fog_and_aux_column_indices(
    idx: &impl Fn(&str, usize) -> usize,
) -> [usize; 6] {
    [
        idx("FogEnd", 21),
        idx("FogScaler", 22),
        idx("SunFogStrength", 40),
        idx("CloudDensity", 31),
        idx("Field_10_0_0_44649_042", 43),
        idx("Field_12_0_0_63854_043", 44),
    ]
}

fn parse_csv_fallback_light_row(
    line: &str,
    ci: &[usize; 27],
    param_id: u32,
) -> Option<LightDataRow> {
    let fields: Vec<&str> = line.split(',').collect();
    let pid: u32 = fields.get(ci[0])?.parse().ok()?;
    if pid != param_id {
        return None;
    }
    let p = |i: usize| -> u32 { fields.get(ci[i]).and_then(|s| s.parse().ok()).unwrap_or(0) };
    let pf = |i: usize| -> f32 {
        fields
            .get(ci[i])
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0)
    };
    let mut row = parse_csv_fallback_color_row(&p);
    fill_csv_fallback_fog_and_aux_row(&mut row, &pf);
    Some(row)
}

fn parse_csv_fallback_color_row(p: &impl Fn(usize) -> u32) -> LightDataRow {
    LightDataRow {
        time: p(1) as f32,
        direct_color: decode_light_color(p(2)),
        ambient_color: decode_light_color(p(3)),
        sky_top: decode_light_color(p(4)),
        sky_middle: decode_light_color(p(5)),
        sky_band1: decode_light_color(p(6)),
        sky_band2: decode_light_color(p(7)),
        sky_smog: decode_light_color(p(8)),
        fog_color: decode_light_color(p(9)),
        sun_color: decode_light_color(p(10)),
        sun_halo_color: decode_light_color(p(11)),
        cloud_emissive_color: decode_light_color(p(12)),
        cloud_layer1_ambient_color: decode_light_color(p(13)),
        cloud_layer2_ambient_color: decode_light_color(p(14)),
        ocean_close_color: decode_light_color(p(15)),
        ocean_far_color: decode_light_color(p(16)),
        river_close_color: decode_light_color(p(17)),
        river_far_color: decode_light_color(p(18)),
        horizon_ambient_color: decode_light_color(p(19)),
        ground_ambient_color: decode_light_color(p(26)),
        fog_end: 0.0,
        fog_start: 0.0,
        glow: 0.0,
        cloud_density: 0.0,
        unk1: 0.0,
        unk2: 0.0,
    }
}

fn fill_csv_fallback_fog_and_aux_row(row: &mut LightDataRow, pf: &impl Fn(usize) -> f32) {
    row.fog_end = pf(20);
    row.fog_start = pf(20) * pf(21);
    row.glow = pf(22);
    row.cloud_density = pf(23);
    row.unk1 = pf(24);
    row.unk2 = pf(25);
}

fn load_light_data_csv_fallback(path: &Path, param_id: u32) -> Vec<LightDataRow> {
    match cache::load_light_data_csv_fallback(path, param_id) {
        Ok(rows) => rows,
        Err(err) => {
            eprintln!("Failed to load fallback {} cache: {err}", path.display());
            cache::load_light_data_csv_fallback_uncached(path, param_id).unwrap_or_default()
        }
    }
}

fn rows_have_extended_color_data(rows: &[LightDataRow]) -> bool {
    rows.iter().any(|row| {
        row.sun_color.to_srgba() != Color::BLACK.to_srgba()
            || row.sun_halo_color.to_srgba() != Color::BLACK.to_srgba()
            || row.cloud_emissive_color.to_srgba() != Color::BLACK.to_srgba()
            || row.cloud_layer1_ambient_color.to_srgba() != Color::BLACK.to_srgba()
            || row.cloud_layer2_ambient_color.to_srgba() != Color::BLACK.to_srgba()
            || row.ocean_close_color.to_srgba() != Color::BLACK.to_srgba()
            || row.ocean_far_color.to_srgba() != Color::BLACK.to_srgba()
            || row.river_close_color.to_srgba() != Color::BLACK.to_srgba()
            || row.river_far_color.to_srgba() != Color::BLACK.to_srgba()
            || row.horizon_ambient_color.to_srgba() != Color::BLACK.to_srgba()
            || row.ground_ambient_color.to_srgba() != Color::BLACK.to_srgba()
    })
}

fn rows_have_dynamic_fog_data(rows: &[LightDataRow]) -> bool {
    rows.iter().any(|row| row.fog_end > 0.0)
}

/// Load LightData.ron rows for a specific LightParamID, with CSV fallback.
pub fn load_light_data(path: &str, param_id: u32) -> Vec<LightDataRow> {
    match load_light_data_ron(path, param_id) {
        Ok(rows) if rows_have_dynamic_fog_data(&rows) && rows_have_extended_color_data(&rows) => {
            rows
        }
        Ok(rows) => {
            if let Some(base) = path.strip_suffix(".ron") {
                let csv_path = format!("{base}.csv");
                let csv_rows = load_light_data_csv_fallback(Path::new(&csv_path), param_id);
                if rows_have_dynamic_fog_data(&csv_rows) || rows_have_extended_color_data(&csv_rows)
                {
                    return csv_rows;
                }
            }
            rows
        }
        Err(err) => {
            eprintln!("{err}");
            if let Some(base) = path.strip_suffix(".ron") {
                let csv_path = format!("{base}.csv");
                eprintln!("Falling back to legacy CSV: {csv_path}");
                return load_light_data_csv_fallback(Path::new(&csv_path), param_id);
            }
            Vec::new()
        }
    }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let a = a.to_linear();
    let b = b.to_linear();
    Color::linear_rgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        1.0,
    )
}

/// Default sky colors when no LightData is available.
pub fn default_sky_colors() -> SkyColorSet {
    SkyColorSet {
        sky_top: Color::linear_rgb(0.2, 0.4, 0.8),
        sky_middle: Color::linear_rgb(0.4, 0.6, 0.9),
        sky_band1: Color::linear_rgb(0.5, 0.7, 0.9),
        sky_band2: Color::linear_rgb(0.6, 0.75, 0.9),
        sky_smog: Color::linear_rgb(0.7, 0.8, 0.85),
        direct_color: Color::WHITE,
        ambient_color: Color::linear_rgb(0.3, 0.3, 0.4),
        fog_color: Color::linear_rgb(0.7, 0.8, 0.9),
        sun_color: Color::WHITE,
        sun_halo_color: Color::linear_rgb(1.0, 0.95, 0.85),
        cloud_emissive_color: Color::BLACK,
        cloud_layer1_ambient_color: Color::BLACK,
        cloud_layer2_ambient_color: Color::BLACK,
        ocean_close_color: Color::linear_rgb(0.08, 0.16, 0.22),
        ocean_far_color: Color::linear_rgb(0.04, 0.08, 0.14),
        river_close_color: Color::linear_rgb(0.08, 0.16, 0.22),
        river_far_color: Color::linear_rgb(0.04, 0.08, 0.14),
        horizon_ambient_color: Color::linear_rgb(0.2, 0.25, 0.3),
        ground_ambient_color: Color::linear_rgb(0.2, 0.25, 0.3),
        fog_end: 500.0,
        fog_start: 125.0,
        glow: 1.0,
        cloud_density: 0.0,
        unk1: 0.0,
        unk2: 0.0,
    }
}

/// Interpolate every channel from `a` towards `b`.
pub fn lerp_color_sets(a: &SkyColorSet, b: &SkyColorSet, t: f32) -> SkyColorSet {
    data::lerp_color_sets(a, b, t, lerp_color)
}

/// Sample a light blend at the given time: the first entry (the map's global
/// light) is the base and every later LightParams overlays it by its weight.
/// LightParams without keyframes contribute nothing.
pub fn sample_light_blend(
    rows_by_param: &HashMap<u32, Vec<LightDataRow>>,
    blend: &[crate::light_lookup::WeightedLightParams],
    minutes: f32,
) -> SkyColorSet {
    let weights: Vec<_> = blend
        .iter()
        .map(|light| (light.light_params_id, light.weight))
        .collect();
    data::sample_light_blend(rows_by_param, &weights, minutes, lerp_color)
        .unwrap_or_else(default_sky_colors)
}

/// Interpolate between LightData keyframes at the given time (0–2880).
pub fn interpolate_colors(rows: &[LightDataRow], minutes: f32) -> SkyColorSet {
    data::interpolate_colors(rows, minutes, lerp_color).unwrap_or_else(default_sky_colors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srgb_bytes(color: Color) -> [u8; 3] {
        let srgba = color.to_srgba();
        [srgba.red, srgba.green, srgba.blue].map(|channel| (channel * 255.0).round() as u8)
    }

    #[test]
    fn light_color_decodes_packed_rgb_as_srgb_bytes() {
        // LightParams 12 noon SkyTopColor: 8009 = 0x001F49.
        let color = decode_light_color(8009);
        assert_eq!(srgb_bytes(color), [0, 31, 73]);
        assert_eq!(color.to_srgba(), Color::srgb_u8(0, 31, 73).to_srgba());
    }

    #[test]
    fn light_params_12_noon_row_decodes_authored_colors() {
        let rows = load_light_data("data/LightData.ron", 12);
        let noon = rows
            .iter()
            .find(|row| row.time == 1440.0)
            .expect("LightParams 12 has a 1440 (noon) keyframe");
        assert_eq!(srgb_bytes(noon.sky_top), [0, 31, 73]);
        assert_eq!(srgb_bytes(noon.sky_middle), [82, 127, 167]);
        assert_eq!(srgb_bytes(noon.sky_band1), [153, 220, 245]);
        assert_eq!(srgb_bytes(noon.sun_color), [255, 247, 222]);
        assert_eq!(srgb_bytes(noon.ambient_color), [127, 149, 170]);
        assert_eq!(srgb_bytes(noon.fog_color), [77, 120, 143]);
    }

    #[test]
    fn interpolation_midpoint() {
        let rows = vec![
            LightDataRow {
                time: 0.0,
                direct_color: Color::WHITE,
                ambient_color: Color::WHITE,
                sky_top: Color::linear_rgb(0.0, 0.0, 0.0),
                sky_middle: Color::BLACK,
                sky_band1: Color::BLACK,
                sky_band2: Color::BLACK,
                sky_smog: Color::BLACK,
                fog_color: Color::BLACK,
                sun_color: Color::BLACK,
                sun_halo_color: Color::BLACK,
                cloud_emissive_color: Color::BLACK,
                cloud_layer1_ambient_color: Color::BLACK,
                cloud_layer2_ambient_color: Color::BLACK,
                ocean_close_color: Color::BLACK,
                ocean_far_color: Color::BLACK,
                river_close_color: Color::BLACK,
                river_far_color: Color::BLACK,
                horizon_ambient_color: Color::BLACK,
                ground_ambient_color: Color::BLACK,
                fog_end: 1000.0,
                fog_start: 100.0,
                glow: 0.0,
                cloud_density: 0.0,
                unk1: 0.0,
                unk2: 0.0,
            },
            LightDataRow {
                time: 1440.0,
                direct_color: Color::WHITE,
                ambient_color: Color::WHITE,
                sky_top: Color::linear_rgb(1.0, 1.0, 1.0),
                sky_middle: Color::WHITE,
                sky_band1: Color::WHITE,
                sky_band2: Color::WHITE,
                sky_smog: Color::WHITE,
                fog_color: Color::WHITE,
                sun_color: Color::WHITE,
                sun_halo_color: Color::WHITE,
                cloud_emissive_color: Color::WHITE,
                cloud_layer1_ambient_color: Color::WHITE,
                cloud_layer2_ambient_color: Color::WHITE,
                ocean_close_color: Color::WHITE,
                ocean_far_color: Color::WHITE,
                river_close_color: Color::WHITE,
                river_far_color: Color::WHITE,
                horizon_ambient_color: Color::WHITE,
                ground_ambient_color: Color::WHITE,
                fog_end: 2000.0,
                fog_start: 200.0,
                glow: 1.0,
                cloud_density: 1.0,
                unk1: 1.0,
                unk2: 1.0,
            },
        ];
        let result = interpolate_colors(&rows, 720.0);
        let top = result.sky_top.to_linear();
        assert!((top.red - 0.5).abs() < 0.05);
        assert_eq!(
            result.sun_color.to_srgba(),
            Color::linear_rgb(0.5, 0.5, 0.5).to_srgba()
        );
        assert!((result.fog_end - 1500.0 / 36.0).abs() < 0.01);
        assert!((result.fog_start - 150.0 / 36.0).abs() < 0.01);
    }

    #[test]
    fn interpolation_wraparound_uses_last_to_first_segment() {
        let rows = vec![
            LightDataRow {
                time: 0.0,
                direct_color: Color::BLACK,
                ambient_color: Color::BLACK,
                sky_top: Color::linear_rgb(0.0, 0.0, 0.0),
                sky_middle: Color::BLACK,
                sky_band1: Color::BLACK,
                sky_band2: Color::BLACK,
                sky_smog: Color::BLACK,
                fog_color: Color::BLACK,
                sun_color: Color::BLACK,
                sun_halo_color: Color::BLACK,
                cloud_emissive_color: Color::BLACK,
                cloud_layer1_ambient_color: Color::BLACK,
                cloud_layer2_ambient_color: Color::BLACK,
                ocean_close_color: Color::BLACK,
                ocean_far_color: Color::BLACK,
                river_close_color: Color::BLACK,
                river_far_color: Color::BLACK,
                horizon_ambient_color: Color::BLACK,
                ground_ambient_color: Color::BLACK,
                fog_end: 0.0,
                fog_start: 0.0,
                glow: 0.0,
                cloud_density: 0.0,
                unk1: 0.0,
                unk2: 0.0,
            },
            LightDataRow {
                time: 1440.0,
                direct_color: Color::WHITE,
                ambient_color: Color::WHITE,
                sky_top: Color::linear_rgb(1.0, 1.0, 1.0),
                sky_middle: Color::WHITE,
                sky_band1: Color::WHITE,
                sky_band2: Color::WHITE,
                sky_smog: Color::WHITE,
                fog_color: Color::WHITE,
                sun_color: Color::WHITE,
                sun_halo_color: Color::WHITE,
                cloud_emissive_color: Color::WHITE,
                cloud_layer1_ambient_color: Color::WHITE,
                cloud_layer2_ambient_color: Color::WHITE,
                ocean_close_color: Color::WHITE,
                ocean_far_color: Color::WHITE,
                river_close_color: Color::WHITE,
                river_far_color: Color::WHITE,
                horizon_ambient_color: Color::WHITE,
                ground_ambient_color: Color::WHITE,
                fog_end: 0.0,
                fog_start: 0.0,
                glow: 0.0,
                cloud_density: 0.0,
                unk1: 0.0,
                unk2: 0.0,
            },
        ];
        let result = interpolate_colors(&rows, 2160.0);
        let top = result.sky_top.to_linear();
        assert!((top.red - 0.5).abs() < 0.05);
        assert!((top.green - 0.5).abs() < 0.05);
        assert!((top.blue - 0.5).abs() < 0.05);
    }

    #[test]
    fn noon_light_params_12_fog_range_is_in_yards() {
        // Noon FogEnd 18000 (yards × 36) with FogScaler 0.25.
        let rows = load_light_data("data/LightData.ron", 12);
        let noon = interpolate_colors(&rows, 1440.0);
        assert!(
            (noon.fog_end - 500.0).abs() < 0.01,
            "fog end {}",
            noon.fog_end
        );
        assert!(
            (noon.fog_start - 125.0).abs() < 0.01,
            "fog start {}",
            noon.fog_start
        );
    }

    fn constant_row(sky_top: Color, fog_end: f32) -> LightDataRow {
        let colors = default_sky_colors();
        LightDataRow {
            time: 0.0,
            direct_color: colors.direct_color,
            ambient_color: colors.ambient_color,
            sky_top,
            sky_middle: colors.sky_middle,
            sky_band1: colors.sky_band1,
            sky_band2: colors.sky_band2,
            sky_smog: colors.sky_smog,
            fog_color: colors.fog_color,
            sun_color: colors.sun_color,
            sun_halo_color: colors.sun_halo_color,
            cloud_emissive_color: colors.cloud_emissive_color,
            cloud_layer1_ambient_color: colors.cloud_layer1_ambient_color,
            cloud_layer2_ambient_color: colors.cloud_layer2_ambient_color,
            ocean_close_color: colors.ocean_close_color,
            ocean_far_color: colors.ocean_far_color,
            river_close_color: colors.river_close_color,
            river_far_color: colors.river_far_color,
            horizon_ambient_color: colors.horizon_ambient_color,
            ground_ambient_color: colors.ground_ambient_color,
            fog_end,
            fog_start: 0.0,
            glow: 0.0,
            cloud_density: 0.0,
            unk1: 0.0,
            unk2: 0.0,
        }
    }

    #[test]
    fn light_blend_overlays_each_local_on_the_result_so_far() {
        use crate::light_lookup::WeightedLightParams;
        let rows = HashMap::from([
            (
                12,
                vec![constant_row(Color::linear_rgb(0.0, 0.0, 0.0), 3600.0)],
            ),
            (
                30,
                vec![constant_row(Color::linear_rgb(1.0, 1.0, 1.0), 7200.0)],
            ),
            (
                31,
                vec![constant_row(Color::linear_rgb(0.0, 0.0, 1.0), 0.0)],
            ),
        ]);
        let weighted = |light_params_id, weight| WeightedLightParams {
            light_params_id,
            weight,
        };
        let blend = [weighted(12, 1.0), weighted(30, 0.5), weighted(31, 0.25)];
        let colors = sample_light_blend(&rows, &blend, 1440.0);
        // Global 0 -> local 30 at 0.5 gives 0.5, then local 31 at 0.25 pulls towards (0,0,1).
        let top = colors.sky_top.to_linear();
        assert!((top.red - 0.375).abs() < 1e-5, "{top:?}");
        assert!((top.blue - 0.625).abs() < 1e-5, "{top:?}");
        // Fog: 100 yd -> 200 yd at 0.5 = 150, then -> 0 at 0.25 = 112.5.
        assert!((colors.fog_end - 112.5).abs() < 1e-3, "{}", colors.fog_end);
        let global_only = sample_light_blend(&rows, &blend[..1], 1440.0);
        assert_eq!(global_only.sky_top.to_linear().red, 0.0);
    }

    #[test]
    fn load_light_data_real() {
        let rows = load_light_data("data/LightData.ron", 12);
        assert!(!rows.is_empty(), "Should find rows for LightParamID 12");
        for w in rows.windows(2) {
            assert!(w[0].time <= w[1].time, "Rows should be sorted by time");
        }
        assert!(rows_have_extended_color_data(&rows));
        assert!(rows.iter().any(|row| row.fog_end > 0.0));
    }
}
