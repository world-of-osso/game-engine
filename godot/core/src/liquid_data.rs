//! Retail liquid material inputs from the local-CASC DB2 exports (`scripts/export_db2_csv.py`):
//! an MH2O layer's `LiquidObject` (or, below 42, its LiquidType) selects a `LiquidType`, whose
//! `LiquidMaterial` gives the vertex format and shader and whose `LiquidTypeXTexture` rows fill
//! the material texture slots. Ports WebWowViewerCpp `LiquidMaterialManager.cpp`
//! (`getLiquidMaterial`, `createLiquidMaterial`, `assignLiquidTextures`) and the per-material
//! `create*LiquidData` packing (`LiquidWater.cpp` wave periods).

use std::collections::HashMap;
use std::path::Path;

use crate::csv_util::parse_csv_records;

pub use crate::asset::adt_format::adt_tex::FIRST_LIQUID_OBJECT;

/// LiquidType 2 "Ocean". Its LiquidObject layers (all on object 42, which has no DB2 row)
/// are flat at sea level and store LVF 2 depth-only vertices, not their material's LVF 0
/// (wowdev ADT/v18 SMLiquidInstance: "≥ WoD ... assumes both 0.0 for LVF = 2"; noggit3
/// `liquid_layer.cpp`: "lvf 2 is only used for flat water at height 0"; WebWowViewerCpp
/// `LiquidInstance.cpp` `createAdtVertexData` singles out liquid_type 2). In the cached tiles
/// every type 2 vertex block is 81 bytes, and every other LiquidObject block, Kul Tiras Ocean
/// 947 on object 42 included, has its material's LVF size.
pub const OCEAN_LIQUID_TYPE: u32 = 2;
const OCEAN_LVF: u8 = 2;

/// Texture slots per LiquidType (`FrameCountTexture[6]`).
pub const TEXTURE_SLOTS: usize = 6;

/// The pixel shader of a LiquidMaterial (LiquidMaterialManager.cpp `createLiquidMaterial`;
/// IDs without a case, such as 8, use water).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LiquidShader {
    Water,
    Magma,
    Mercury,
    Fog,
    LeyLine,
    Fel,
    Swamp,
    Azerite,
}

impl LiquidShader {
    pub fn for_material(material_id: u8) -> Self {
        match material_id {
            2 | 4 => Self::Magma,
            5 => Self::Mercury,
            10 => Self::Fog,
            12 => Self::LeyLine,
            13 => Self::Fel,
            14 => Self::Swamp,
            18 => Self::Azerite,
            _ => Self::Water,
        }
    }

    /// Slots the pixel shader samples (forwardLiquidShader_text.slang `calc*LiquidMat` calls).
    pub fn texture_slots(self) -> &'static [usize] {
        match self {
            Self::Water => &[2, 3],
            Self::Magma => &[1, 2, 3],
            Self::Mercury => &[2],
            Self::Fog => &[0],
            Self::LeyLine => &[0, 1, 3, 4],
            Self::Fel => &[0, 1, 2, 3, 4, 5],
            Self::Swamp => &[0, 1, 2, 3, 4],
            Self::Azerite => &[0, 1, 2, 4, 5],
        }
    }

    /// The normal-map slot crossfaded over `Float[index]` seconds, with the next frame bound
    /// as the reference's slot 7 (LiquidFel.cpp, LiquidSwamp.cpp, LiquidAzerithe.cpp
    /// `resolveAnimatedTextures`). Other slots advance one frame per second.
    pub fn crossfade(self) -> Option<(usize, usize)> {
        match self {
            Self::Fel | Self::Azerite => Some((5, 12)),
            Self::Swamp => Some((4, 5)),
            _ => None,
        }
    }

    /// Engine-global files the material binds beside its LiquidType textures: shader
    /// uniform name and FDID (LiquidMaterialManager.h magma noise volume 768431, a 128x128x16
    /// RGBA blob; LiquidAzerithe.cpp environment 1797551 and shore foam 1844666 BLPs).
    pub fn global_textures(self) -> &'static [(&'static str, u32)] {
        match self {
            Self::Magma => &[("noise_volume", MAGMA_NOISE_FDID)],
            Self::Azerite => &[
                ("environment_texture", 1_797_551),
                ("shore_foam", 1_844_666),
            ],
            _ => &[],
        }
    }
}

/// `xtextures/fx/noise.blob`: 16 slices of 128x128 RGBA, bound as a 128x2048 atlas.
pub const MAGMA_NOISE_FDID: u32 = 768_431;
pub const MAGMA_NOISE_SIZE: (u32, u32) = (128, 128 * 16);

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
pub struct LiquidMaterial {
    pub liquid_type: u32,
    pub material_id: u8,
    pub shader: LiquidShader,
    /// Vertex format of the material (`LiquidMaterial.LVF`).
    pub lvf: u8,
    /// `LiquidType.Float[0..18]`, the shader's f0..f17.
    pub floats: [f32; 18],
    /// Cubic depth-to-colour polynomial `LiquidType.Coefficient[0..4]`.
    pub depth_coefficients: [f32; 4],
    /// `LiquidType.Color[0..3]` as CSqliteDB.cpp `getFloatFromInt<0..2>`: bytes 0, 1, 2 / 255.
    pub colors: [[f32; 3]; 3],
    /// `LiquidType.Int[0..4]`.
    pub ints: [i32; 4],
    pub flow_direction: f32,
    pub flow_speed: f32,
    pub color_source: WaterColorSource,
    /// Texture FDIDs of each slot in frame order; 0 is a procedural (black) texture.
    pub texture_slots: [Vec<u32>; TEXTURE_SLOTS],
    /// `LiquidWater.cpp` CalculateWavePeriod of the two wave scales; zero without wave speed.
    pub wave_periods: [f32; 2],
}

struct LiquidTypeRow {
    material_id: u8,
    frame_counts: [u8; TEXTURE_SLOTS],
    floats: [f32; 18],
    coefficients: [f32; 4],
    colors: [i64; 3],
    ints: [i32; 4],
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
                colors: row.array("Color")?,
                ints: row.array("Int")?,
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

    /// The liquid material of an MH2O instance (`liquid_type`, `liquid_object_or_lvf`).
    /// A LiquidObject without a DB2 row (ocean object 42 and a few authored IDs the shipped
    /// table omits) takes the instance's own `liquid_type` with no flow, as WebWowViewerCpp
    /// `CSqliteDB::getLiquidObjectData` does.
    pub fn liquid_material(
        &self,
        liquid_type: u16,
        liquid_object: u16,
    ) -> Result<LiquidMaterial, String> {
        let object = (liquid_object >= FIRST_LIQUID_OBJECT)
            .then(|| self.objects.get(&u32::from(liquid_object)))
            .flatten();
        let (type_id, flow_direction, flow_speed) = match object {
            Some(object) => (object.liquid_type, object.flow_direction, object.flow_speed),
            None => (u32::from(liquid_type), 0.0, 0.0),
        };
        let row = self
            .types
            .get(&type_id)
            .ok_or_else(|| format!("LiquidType {type_id} has no DB2 row"))?;
        let material_lvf = *self
            .material_lvf
            .get(&u32::from(row.material_id))
            .ok_or_else(|| format!("LiquidMaterial {} has no DB2 row", row.material_id))?;
        let lvf = if type_id == OCEAN_LIQUID_TYPE && liquid_object >= FIRST_LIQUID_OBJECT {
            OCEAN_LVF
        } else {
            material_lvf
        };
        let (texture_slots, color_source) = self.texture_slots(type_id, row)?;
        Ok(LiquidMaterial {
            liquid_type: type_id,
            material_id: row.material_id,
            shader: LiquidShader::for_material(row.material_id),
            lvf,
            floats: row.floats,
            depth_coefficients: row.coefficients,
            flow_direction,
            flow_speed,
            colors: row
                .colors
                .map(|color| [0, 8, 16].map(|shift| ((color >> shift) & 0xff) as f32 / 255.0)),
            ints: row.ints,
            color_source,
            texture_slots,
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
