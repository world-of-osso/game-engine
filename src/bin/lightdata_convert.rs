use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Serialize)]
struct LightDataFile {
    by_param: BTreeMap<u32, Vec<LightDataRow>>,
}

#[derive(Debug, Serialize)]
struct LightDataRow {
    time: f32,
    direct_color: u32,
    ambient_color: u32,
    sky_top: u32,
    sky_middle: u32,
    sky_band1: u32,
    sky_band2: u32,
    sky_smog: u32,
    fog_color: u32,
    fog_end: f32,
    fog_start: f32,
    glow: f32,
    cloud_density: f32,
    unk1: f32,
    unk2: f32,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let input = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/LightData.csv"));
    let output = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/LightData.ron"));

    if let Err(err) = convert_csv_to_ron(&input, &output) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn convert_csv_to_ron(input: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let contents = std::fs::read_to_string(input)
        .map_err(|e| format!("failed to read {}: {e}", input.display()))?;
    let mut lines = contents.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{} is empty", input.display()))?;
    let ci = resolve_column_indices(header);

    let mut by_param: BTreeMap<u32, Vec<LightDataRow>> = BTreeMap::new();
    for line in lines {
        if let Some((param_id, row)) = parse_row(line, &ci) {
            by_param.entry(param_id).or_default().push(row);
        }
    }
    for rows in by_param.values_mut() {
        rows.sort_by(|a, b| a.time.total_cmp(&b.time));
    }

    let file = LightDataFile { by_param };
    let pretty = ron::ser::PrettyConfig::new().compact_arrays(true);
    let ron_text = ron::ser::to_string_pretty(&file, pretty)
        .map_err(|e| format!("failed to serialize RON: {e}"))?;

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    std::fs::write(output, ron_text)
        .map_err(|e| format!("failed to write {}: {e}", output.display()))?;

    eprintln!(
        "Wrote {} with {} LightParamID groups",
        output.display(),
        file.by_param.len()
    );
    Ok(())
}

fn resolve_column_indices(header: &str) -> [usize; 16] {
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
        idx("FogEnd", 21),
        idx("FogScaler", 22),
        idx("SunFogStrength", 40),
        idx("CloudDensity", 31),
        idx("Field_10_0_0_44649_042", 43),
        idx("Field_12_0_0_63854_043", 44),
    ]
}

fn parse_row(line: &str, ci: &[usize; 16]) -> Option<(u32, LightDataRow)> {
    let fields: Vec<&str> = line.split(',').collect();
    let p_u32 = |i: usize| -> u32 {
        fields
            .get(ci[i])
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let p_f32 = |i: usize| -> f32 {
        fields
            .get(ci[i])
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0)
    };

    let param_id = p_u32(0);
    let row = LightDataRow {
        time: p_f32(1),
        direct_color: p_u32(2),
        ambient_color: p_u32(3),
        sky_top: p_u32(4),
        sky_middle: p_u32(5),
        sky_band1: p_u32(6),
        sky_band2: p_u32(7),
        sky_smog: p_u32(8),
        fog_color: p_u32(9),
        fog_end: p_f32(10),
        fog_start: p_f32(10) * p_f32(11),
        glow: p_f32(12),
        cloud_density: p_f32(13),
        unk1: p_f32(14),
        unk2: p_f32(15),
    };

    Some((param_id, row))
}
