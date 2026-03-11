//! LightData keyframe loading and sky color interpolation.

use std::collections::BTreeMap;

use bevy::prelude::Color;
use serde::Deserialize;

/// One keyframe row filtered to a single LightParamID.
#[derive(Debug, Clone)]
pub struct LightDataRow {
    pub time: f32,
    pub direct_color: Color,
    pub ambient_color: Color,
    pub sky_top: Color,
    pub sky_middle: Color,
    pub sky_band1: Color,
    pub sky_band2: Color,
    pub sky_smog: Color,
    #[allow(dead_code)]
    pub fog_color: Color,
}

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
}

/// Decode a BGR32 integer (as stored in LightData exports) to linear Color.
pub fn decode_bgr32(val: u32) -> Color {
    let r = (val & 0xFF) as f32 / 255.0;
    let g = ((val >> 8) & 0xFF) as f32 / 255.0;
    let b = ((val >> 16) & 0xFF) as f32 / 255.0;
    Color::linear_rgb(r, g, b)
}

/// Interpolated sky color set for the current time of day.
#[derive(Debug, Clone)]
pub struct SkyColorSet {
    pub sky_top: Color,
    pub sky_middle: Color,
    pub sky_band1: Color,
    pub sky_band2: Color,
    pub sky_smog: Color,
    pub direct_color: Color,
    pub ambient_color: Color,
    #[allow(dead_code)]
    pub fog_color: Color,
}

fn deserialize_light_row(row: LightDataSerializedRow) -> LightDataRow {
    LightDataRow {
        time: row.time,
        direct_color: decode_bgr32(row.direct_color),
        ambient_color: decode_bgr32(row.ambient_color),
        sky_top: decode_bgr32(row.sky_top),
        sky_middle: decode_bgr32(row.sky_middle),
        sky_band1: decode_bgr32(row.sky_band1),
        sky_band2: decode_bgr32(row.sky_band2),
        sky_smog: decode_bgr32(row.sky_smog),
        fog_color: decode_bgr32(row.fog_color),
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
fn resolve_csv_fallback_column_indices(header: &str) -> [usize; 10] {
    let cols: Vec<&str> = header.split(',').collect();
    let idx =
        |name: &str, fallback: usize| cols.iter().position(|c| *c == name).unwrap_or(fallback);
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
    ]
}

fn parse_csv_fallback_light_row(
    line: &str,
    ci: &[usize; 10],
    param_id: u32,
) -> Option<LightDataRow> {
    let fields: Vec<&str> = line.split(',').collect();
    let pid: u32 = fields.get(ci[0])?.parse().ok()?;
    if pid != param_id {
        return None;
    }
    let p = |i: usize| -> u32 { fields.get(ci[i]).and_then(|s| s.parse().ok()).unwrap_or(0) };
    Some(LightDataRow {
        time: p(1) as f32,
        direct_color: decode_bgr32(p(2)),
        ambient_color: decode_bgr32(p(3)),
        sky_top: decode_bgr32(p(4)),
        sky_middle: decode_bgr32(p(5)),
        sky_band1: decode_bgr32(p(6)),
        sky_band2: decode_bgr32(p(7)),
        sky_smog: decode_bgr32(p(8)),
        fog_color: decode_bgr32(p(9)),
    })
}

fn load_light_data_csv_fallback(path: &str, param_id: u32) -> Vec<LightDataRow> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read fallback {path}: {e}");
            return Vec::new();
        }
    };
    let mut lines = contents.lines();
    let header = match lines.next() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let ci = resolve_csv_fallback_column_indices(header);
    let mut rows: Vec<LightDataRow> = lines
        .filter_map(|l| parse_csv_fallback_light_row(l, &ci, param_id))
        .collect();
    rows.sort_by(|a, b| a.time.total_cmp(&b.time));
    rows
}

/// Load LightData.ron rows for a specific LightParamID, with CSV fallback.
pub fn load_light_data(path: &str, param_id: u32) -> Vec<LightDataRow> {
    match load_light_data_ron(path, param_id) {
        Ok(rows) => rows,
        Err(err) => {
            eprintln!("{err}");
            if let Some(base) = path.strip_suffix(".ron") {
                let csv_path = format!("{base}.csv");
                eprintln!("Falling back to legacy CSV: {csv_path}");
                return load_light_data_csv_fallback(&csv_path, param_id);
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
    }
}

fn lerp_rows(a: &LightDataRow, b: &LightDataRow, t: f32) -> SkyColorSet {
    SkyColorSet {
        sky_top: lerp_color(a.sky_top, b.sky_top, t),
        sky_middle: lerp_color(a.sky_middle, b.sky_middle, t),
        sky_band1: lerp_color(a.sky_band1, b.sky_band1, t),
        sky_band2: lerp_color(a.sky_band2, b.sky_band2, t),
        sky_smog: lerp_color(a.sky_smog, b.sky_smog, t),
        direct_color: lerp_color(a.direct_color, b.direct_color, t),
        ambient_color: lerp_color(a.ambient_color, b.ambient_color, t),
        fog_color: lerp_color(a.fog_color, b.fog_color, t),
    }
}

fn find_bracket(rows: &[LightDataRow], m: f32) -> (&LightDataRow, &LightDataRow, f32) {
    for i in 0..rows.len() {
        let next = (i + 1) % rows.len();
        let t0 = rows[i].time;
        let t1 = if next == 0 {
            rows[next].time + 2880.0
        } else {
            rows[next].time
        };
        let m_adj = if next == 0 && m < t0 { m + 2880.0 } else { m };
        if m_adj >= t0 && m_adj <= t1 {
            let span = t1 - t0;
            let t = if span > 0.0 { (m_adj - t0) / span } else { 0.0 };
            return (&rows[i], &rows[next], t);
        }
    }
    let last = &rows[rows.len() - 1];
    let first = &rows[0];
    let span = (first.time + 2880.0) - last.time;
    let t = if span > 0.0 {
        (m + 2880.0 - last.time) / span
    } else {
        0.0
    };
    (last, first, t)
}

/// Interpolate between LightData keyframes at the given time (0–2880).
pub fn interpolate_colors(rows: &[LightDataRow], minutes: f32) -> SkyColorSet {
    match rows.len() {
        0 => default_sky_colors(),
        1 => lerp_rows(&rows[0], &rows[0], 0.0),
        _ => {
            let m = minutes.rem_euclid(2880.0);
            let (a, b, t) = find_bracket(rows, m);
            lerp_rows(a, b, t)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_bgr32_red() {
        let c = decode_bgr32(0x000000FF);
        let lin = c.to_linear();
        assert!((lin.red - 1.0).abs() < 0.01);
        assert!(lin.green < 0.01);
        assert!(lin.blue < 0.01);
    }

    #[test]
    fn decode_bgr32_blue() {
        let c = decode_bgr32(0x00FF0000);
        let lin = c.to_linear();
        assert!(lin.red < 0.01);
        assert!(lin.green < 0.01);
        assert!((lin.blue - 1.0).abs() < 0.01);
    }

    #[test]
    fn decode_bgr32_white() {
        let c = decode_bgr32(0x00FFFFFF);
        let lin = c.to_linear();
        assert!((lin.red - 1.0).abs() < 0.01);
        assert!((lin.green - 1.0).abs() < 0.01);
        assert!((lin.blue - 1.0).abs() < 0.01);
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
            },
        ];
        let result = interpolate_colors(&rows, 720.0);
        let top = result.sky_top.to_linear();
        assert!((top.red - 0.5).abs() < 0.05);
    }

    #[test]
    fn load_light_data_real() {
        let rows = load_light_data("data/LightData.ron", 12);
        assert!(!rows.is_empty(), "Should find rows for LightParamID 12");
        for w in rows.windows(2) {
            assert!(w[0].time <= w[1].time, "Rows should be sorted by time");
        }
    }
}
