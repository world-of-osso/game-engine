//! Strict numeric CSV decoding for authored Light and LightData assets.
//! LightData colors are linear RGB; fog distances remain raw until keyframe sampling.

use std::collections::HashMap;
use std::str::FromStr;

use crate::light_lookup_data::LightEntry;
use crate::sky_lightdata_data::LightDataRow;

pub type LightKeyframes = HashMap<u32, Vec<LightDataRow<[f32; 3]>>>;

pub fn parse_light_csv(source: &str) -> Result<Vec<LightEntry>, String> {
    let (columns, lines) = csv_rows(source)?;
    let mut entries = Vec::new();
    for (index, line) in lines {
        if line.is_empty() {
            continue;
        }
        let row = CsvRow::new(&columns, line, index + 2);
        let mut light_params_ids = [0; 8];
        for (slot, value) in light_params_ids.iter_mut().enumerate() {
            *value = row.parse(&format!("LightParamsID_{slot}"))?;
        }
        entries.push(LightEntry {
            id: row.parse("ID")?,
            map_id: row.parse("ContinentID")?,
            position: [
                row.number("GameCoords_0")?,
                row.number("GameCoords_1")?,
                row.number("GameCoords_2")?,
            ],
            falloff_start: row.number("GameFalloffStart")?,
            falloff_end: row.number("GameFalloffEnd")?,
            light_params_ids,
        });
    }
    if entries.is_empty() {
        return Err("Light.csv contains no authored rows".into());
    }
    Ok(entries)
}

pub fn parse_light_data_csv(source: &str) -> Result<LightKeyframes, String> {
    let (columns, lines) = csv_rows(source)?;
    let mut keyframes = LightKeyframes::new();
    for (index, line) in lines {
        if line.is_empty() {
            continue;
        }
        let row = CsvRow::new(&columns, line, index + 2);
        let param_id = row.parse("LightParamID")?;
        keyframes
            .entry(param_id)
            .or_default()
            .push(read_light_data_row(&row)?);
    }
    if keyframes.is_empty() {
        return Err("LightData.csv contains no authored rows".into());
    }
    for rows in keyframes.values_mut() {
        rows.sort_by(|a, b| a.time.total_cmp(&b.time));
    }
    Ok(keyframes)
}

fn read_light_data_row(row: &CsvRow<'_>) -> Result<LightDataRow<[f32; 3]>, String> {
    let fog_end = row.number("FogEnd")?;
    Ok(LightDataRow {
        time: row.number("Time")?,
        direct_color: row.color("DirectColor")?,
        ambient_color: row.color("AmbientColor")?,
        sky_top: row.color("SkyTopColor")?,
        sky_middle: row.color("SkyMiddleColor")?,
        sky_band1: row.color("SkyBand1Color")?,
        sky_band2: row.color("SkyBand2Color")?,
        sky_smog: row.color("SkySmogColor")?,
        fog_color: row.color("SkyFogColor")?,
        sun_color: row.color("SunColor")?,
        sun_halo_color: row.color("CloudSunColor")?,
        cloud_emissive_color: row.color("CloudEmissiveColor")?,
        cloud_layer1_ambient_color: row.color("CloudLayer1AmbientColor")?,
        cloud_layer2_ambient_color: row.color("CloudLayer2AmbientColor")?,
        ocean_close_color: row.color("OceanCloseColor")?,
        ocean_far_color: row.color("OceanFarColor")?,
        river_close_color: row.color("RiverCloseColor")?,
        river_far_color: row.color("RiverFarColor")?,
        horizon_ambient_color: row.color("HorizonAmbientColor")?,
        ground_ambient_color: row.color("GroundAmbientColor")?,
        fog_end,
        fog_start: fog_end * row.number("FogScaler")?,
        fog_scaler: row.number("FogScaler")?,
        fog_density: row.number("FogDensity")?,
        glow: row.number("SunFogStrength")?,
        cloud_density: row.number("CloudDensity")?,
        unk1: row.number("Field_10_0_0_44649_042")?,
        unk2: row.number("Field_12_0_0_63854_043")?,
    })
}

pub(crate) fn csv_rows(
    source: &str,
) -> Result<(HashMap<String, usize>, impl Iterator<Item = (usize, &str)>), String> {
    let mut lines = source.lines();
    let header = lines
        .next()
        .filter(|header| !header.is_empty())
        .ok_or("Missing CSV header")?;
    let columns = header
        .split(',')
        .enumerate()
        .map(|(index, name)| (name.to_owned(), index))
        .collect();
    Ok((columns, lines.enumerate()))
}

pub(crate) struct CsvRow<'a> {
    columns: &'a HashMap<String, usize>,
    values: Vec<&'a str>,
    line: usize,
}

impl<'a> CsvRow<'a> {
    pub(crate) fn new(columns: &'a HashMap<String, usize>, line: &'a str, number: usize) -> Self {
        Self {
            columns,
            values: line.split(',').collect(),
            line: number,
        }
    }

    pub(crate) fn parse<T: FromStr>(&self, name: &str) -> Result<T, String>
    where
        T::Err: std::fmt::Display,
    {
        let index = self
            .columns
            .get(name)
            .ok_or_else(|| format!("Missing CSV column {name}"))?;
        let value = self
            .values
            .get(*index)
            .ok_or_else(|| format!("CSV row {} missing {name}", self.line))?;
        value
            .parse()
            .map_err(|error| format!("CSV row {} invalid {name}: {error}", self.line))
    }

    pub(crate) fn number(&self, name: &str) -> Result<f32, String> {
        let value: f32 = self.parse(name)?;
        if !value.is_finite() {
            return Err(format!("CSV row {} non-finite {name}", self.line));
        }
        Ok(value)
    }

    pub(crate) fn color(&self, name: &str) -> Result<[f32; 3], String> {
        // CSV exports the packed 32-bit color as either signed or unsigned decimal.
        let value: i64 = self.parse(name)?;
        if !(i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(&value) {
            return Err(format!(
                "CSV row {} {name} exceeds packed 32-bit color range",
                self.line
            ));
        }
        let [_, red, green, blue] = (value as u32).to_be_bytes();
        Ok(authored_to_linear_rgb(
            [red, green, blue].map(|channel| f32::from(channel) / 255.0),
        ))
    }
}

/// Authored (sRGB-encoded) RGB to the linear RGB keyframes interpolate in.
pub fn authored_to_linear_rgb(authored: [f32; 3]) -> [f32; 3] {
    authored.map(|channel| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// Retail shading consumes authored RGB even though keyframes interpolate in linear RGB.
pub fn linear_to_authored_rgb(linear: [f32; 3]) -> [f32; 3] {
    linear.map(|channel| {
        if channel <= 0.0031308 {
            channel * 12.92
        } else {
            1.055 * channel.powf(1.0 / 2.4) - 0.055
        }
    })
}
