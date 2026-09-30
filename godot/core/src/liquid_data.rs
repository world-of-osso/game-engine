//! Retail water material inputs from the local-CASC DB2 exports (`scripts/export_db2_csv.py`):
//! an MH2O layer's `LiquidObject` (or, below 42, its LiquidType) selects a `LiquidType`, whose
//! `LiquidMaterial` gives the vertex format and whose `LiquidTypeXTexture` rows fill the
//! material texture slots. Ports WebWowViewerCpp `LiquidMaterialManager.cpp`
//! (`getLiquidMaterial`, `assignLiquidTextures`) and `LiquidWater.cpp` (`createWaterLiquidData`).

use std::collections::HashMap;
use std::path::Path;

use crate::csv_util::parse_csv_records;

/// `liquid_object_or_lvf` below this is a vertex format, not a `LiquidObject` ID
/// (LiquidInstance.cpp `createAdtVertexData`: `< 42` is an LVF override).
pub const FIRST_LIQUID_OBJECT: u16 = 42;
/// Texture slots per LiquidType (`FrameCountTexture[6]`).
const TEXTURE_SLOTS: usize = 6;

/// Which LightData close/far colours and LightParams alphas a water surface uses: the
/// procedural depth texture row of its LiquidTypeXTexture set (`Type` 0 ocean, 1 river,
/// 2 WMO; LiquidMaterialManager.cpp `assignLiquidTextures`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterColorSource {
    Ocean = 0,
    River = 1,
    Wmo = 2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaterMaterial {
    pub liquid_type: u32,
    pub material_id: u8,
    /// Vertex format of the material (`LiquidMaterial.LVF`).
    pub lvf: u8,
    /// `LiquidType.Float[0..18]`, the shader's f0..f17.
    pub floats: [f32; 18],
    /// Cubic depth-to-colour polynomial `LiquidType.Coefficient[0..4]`.
    pub depth_coefficients: [f32; 4],
    pub flow_direction: f32,
    pub flow_speed: f32,
    pub color_source: WaterColorSource,
    /// Frames of slot 2 (normal/bump) and slot 3 (foam), each cycled once per second.
    pub bump_frames: Vec<u32>,
    pub foam_frames: Vec<u32>,
    /// `LiquidWater.cpp` CalculateWavePeriod of the two wave scales; zero without wave speed.
    pub wave_periods: [f32; 2],
}

struct LiquidTypeRow {
    material_id: u8,
    frame_counts: [u8; TEXTURE_SLOTS],
    floats: [f32; 18],
    coefficients: [f32; 4],
}

struct LiquidObjectRow {
    flow_direction: f32,
    flow_speed: f32,
    liquid_type: u32,
}

struct TextureRow {
    order: i32,
    fdid: u32,
    kind: i32,
}

pub struct LiquidCatalog {
    types: HashMap<u32, LiquidTypeRow>,
    objects: HashMap<u32, LiquidObjectRow>,
    material_lvf: HashMap<u32, u8>,
    textures: HashMap<u32, Vec<TextureRow>>,
}

impl LiquidCatalog {
    /// Reads `LiquidType`, `LiquidObject`, `LiquidMaterial` and `LiquidTypeXTexture` CSVs.
    pub fn read(db2_dir: &Path) -> Result<Self, String> {
        let types = read_table(db2_dir, "LiquidType", |row| {
            Ok(LiquidTypeRow {
                material_id: row.number("MaterialID")?,
                frame_counts: row.array("FrameCountTexture")?,
                floats: row.array("Float")?,
                coefficients: row.array("Coefficient")?,
            })
        })?;
        let objects = read_table(db2_dir, "LiquidObject", |row| {
            Ok(LiquidObjectRow {
                flow_direction: row.number("FlowDirection")?,
                flow_speed: row.number("FlowSpeed")?,
                liquid_type: row.number("LiquidTypeID")?,
            })
        })?;
        let material_lvf = read_table(db2_dir, "LiquidMaterial", |row| row.number("LVF"))?;
        let textures = read_textures(db2_dir)?;
        Ok(Self {
            types,
            objects,
            material_lvf,
            textures,
        })
    }

    /// The water material of an MH2O instance (`liquid_type`, `liquid_object_or_lvf`).
    pub fn water_material(
        &self,
        liquid_type: u16,
        liquid_object: u16,
    ) -> Result<WaterMaterial, String> {
        let (type_id, flow_direction, flow_speed) = if liquid_object >= FIRST_LIQUID_OBJECT {
            let object = self
                .objects
                .get(&u32::from(liquid_object))
                .ok_or_else(|| format!("LiquidObject {liquid_object} has no DB2 row"))?;
            (object.liquid_type, object.flow_direction, object.flow_speed)
        } else {
            (u32::from(liquid_type), 0.0, 0.0)
        };
        let row = self
            .types
            .get(&type_id)
            .ok_or_else(|| format!("LiquidType {type_id} has no DB2 row"))?;
        let lvf = *self
            .material_lvf
            .get(&u32::from(row.material_id))
            .ok_or_else(|| format!("LiquidMaterial {} has no DB2 row", row.material_id))?;
        let (slots, color_source) = self.texture_slots(type_id, row)?;
        let [_, _, bump_frames, foam_frames, ..] = slots;
        Ok(WaterMaterial {
            liquid_type: type_id,
            material_id: row.material_id,
            lvf,
            floats: row.floats,
            depth_coefficients: row.coefficients,
            flow_direction,
            flow_speed,
            color_source,
            bump_frames,
            foam_frames,
            wave_periods: wave_periods(&row.floats),
        })
    }

    /// LiquidMaterialManager.cpp `assignLiquidTextures`: rows in `OrderIndex` order fill a
    /// slot until it holds `FrameCountTexture[slot]` frames (0 means one). A zero-FDID row is
    /// the procedural depth texture that selects the colour source; it binds black.
    fn texture_slots(
        &self,
        type_id: u32,
        row: &LiquidTypeRow,
    ) -> Result<([Vec<u32>; TEXTURE_SLOTS], WaterColorSource), String> {
        let mut slots: [Vec<u32>; TEXTURE_SLOTS] = Default::default();
        let mut color_source = WaterColorSource::Ocean;
        let mut slot = 0;
        for texture in self.textures.get(&type_id).into_iter().flatten() {
            if slot >= TEXTURE_SLOTS {
                return Err(format!("LiquidType {type_id} has more textures than slots"));
            }
            if texture.fdid == 0 {
                color_source = match texture.kind {
                    0 => WaterColorSource::Ocean,
                    1 => WaterColorSource::River,
                    2 => WaterColorSource::Wmo,
                    kind => {
                        return Err(format!(
                            "LiquidType {type_id} procedural texture type {kind} is unknown"
                        ));
                    }
                };
            }
            slots[slot].push(texture.fdid);
            let frames = row.frame_counts[slot];
            if frames == 0 || slots[slot].len() == usize::from(frames) {
                slot += 1;
            }
        }
        Ok((slots, color_source))
    }
}

/// LiquidTypeXTexture rows per LiquidType in `OrderIndex` order.
fn read_textures(db2_dir: &Path) -> Result<HashMap<u32, Vec<TextureRow>>, String> {
    let mut textures: HashMap<u32, Vec<TextureRow>> = HashMap::new();
    let rows = read_table(db2_dir, "LiquidTypeXTexture", |row| {
        Ok((
            row.number::<u32>("LiquidTypeID")?,
            TextureRow {
                order: row.number("OrderIndex")?,
                fdid: row.number("FileDataID")?,
                kind: row.number("Type")?,
            },
        ))
    })?;
    for (liquid_type, texture) in rows.into_values() {
        textures.entry(liquid_type).or_default().push(texture);
    }
    for rows in textures.values_mut() {
        rows.sort_by_key(|row| row.order);
    }
    Ok(textures)
}

/// LiquidWater.cpp `createWaterLiquidData`: wave periods only when `Float[16]` animates waves.
fn wave_periods(floats: &[f32; 18]) -> [f32; 2] {
    if floats[16] * 0.001_875 == 0.0 {
        return [0.0; 2];
    }
    [
        wave_period(floats[3], floats[8]),
        wave_period(floats[4], floats[8]),
    ]
}

/// LiquidWater.cpp `CalculateWavePeriod`: LCM of the 1/scale and 1/(32·scale·amplitude)
/// periods at 1/10000 resolution. A zero scale or amplitude gives an infinite period, whose
/// integer cast is undefined in the reference; it contributes no period here.
fn wave_period(scale: f32, amplitude: f32) -> f32 {
    let periods: Vec<u128> = [1.0 / scale, 1.0 / (scale * 32.0 * amplitude)]
        .into_iter()
        .filter(|period| *period != 0.0 && period.is_finite())
        .map(|period| (period * 10_000.0).abs() as u128)
        .collect();
    let Some((&first, rest)) = periods.split_first() else {
        return 0.0;
    };
    let lcm = rest
        .iter()
        .filter(|&&period| period != 0)
        .fold(first, |lcm, &period| lcm * period / gcd(lcm, period));
    lcm as f32 / 10_000.0
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

struct CsvRow<'a> {
    table: &'a str,
    headers: &'a [String],
    values: &'a [String],
}

impl CsvRow<'_> {
    fn number<T: std::str::FromStr>(&self, column: &str) -> Result<T, String> {
        let index = self
            .headers
            .iter()
            .position(|header| header == column)
            .ok_or_else(|| format!("{}.csv has no {column} column", self.table))?;
        let value = self.values.get(index).map(String::as_str).unwrap_or("");
        value
            .parse()
            .map_err(|_| format!("{}.csv {column} {value:?} is not a number", self.table))
    }

    fn array<T: std::str::FromStr + Copy + Default, const N: usize>(
        &self,
        column: &str,
    ) -> Result<[T; N], String> {
        let mut values = [T::default(); N];
        for (index, value) in values.iter_mut().enumerate() {
            *value = self.number(&format!("{column}_{index}"))?;
        }
        Ok(values)
    }
}

fn read_table<T>(
    dir: &Path,
    table: &str,
    parse: impl Fn(&CsvRow) -> Result<T, String>,
) -> Result<HashMap<u32, T>, String> {
    let path = dir.join(format!("{table}.csv"));
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    let mut records = parse_csv_records(&text).into_iter();
    let headers = records
        .next()
        .ok_or_else(|| format!("{} is empty", path.display()))?;
    records
        .filter(|values| values.len() > 1)
        .map(|values| {
            let row = CsvRow {
                table,
                headers: &headers,
                values: &values,
            };
            Ok((row.number("ID")?, parse(&row)?))
        })
        .collect()
}
