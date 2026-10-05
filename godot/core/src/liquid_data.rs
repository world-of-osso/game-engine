//! Retail liquid material inputs from the local-CASC DB2 exports (`scripts/export_db2_csv.py`):
//! an MH2O layer's `LiquidObject` (or, below 42, its LiquidType) selects a `LiquidType`, whose
//! `LiquidMaterial` gives the vertex format and shader and whose `LiquidTypeXTexture` rows fill
//! the material texture slots. Ports WebWowViewerCpp `LiquidMaterialManager.cpp`
//! (`getLiquidMaterial`, `createLiquidMaterial`, `assignLiquidTextures`) and the per-material
//! `create*LiquidData` packing (`LiquidWater.cpp` wave periods).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::csv_util::parse_csv_records;

pub use crate::asset::adt_format::adt_tex::FIRST_LIQUID_OBJECT;

/// LiquidType 2 "Ocean". Its LiquidObject layers (all on object 42, which has no DB2 row)
/// are flat at sea level and store LVF 2 depth-only vertices, not their material's LVF 0
/// (wowdev ADT/v18 SMLiquidInstance: "≥ WoD ... assumes both 0.0 for LVF = 2"; noggit3
/// `liquid_layer.cpp`: "lvf 2 is only used for flat water at height 0"; WebWowViewerCpp
/// `LiquidInstance.cpp` `createAdtVertexData` singles out liquid_type 2). Across the active
/// build's 52,882 root ADTs every type 2 vertex block is 81 bytes, and every other block,
/// the Kul Tiras, Zandalar and Nazjatar oceans on object 42 included, has its material's
/// LVF size.
pub const OCEAN_LIQUID_TYPE: u32 = 2;
const OCEAN_LVF: u8 = 2;

/// Texture slots per LiquidType (`FrameCountTexture[6]`).
pub const TEXTURE_SLOTS: usize = 6;
pub const PBR_WATER_MATERIAL: u8 = 130;
const LEGACY_WATER_TYPE: u16 = 5;

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
    /// Named source fields: 18 for legacy materials, all 38 for PBR water.
    pub floats: Vec<f32>,
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
    floats: Vec<f32>,
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

/// Separate product catalogs: only maps absent by ID and directory in Retail use Forever.
/// A broken Forever export is retained as an error without disabling Retail materials.
pub struct MapLiquidCatalog {
    retail: LiquidCatalog,
    forever_maps: HashSet<u32>,
    forever: Result<LiquidCatalog, String>,
}

impl MapLiquidCatalog {
    pub fn read(data: &Path) -> Result<Self, String> {
        let retail_dir = data.join("db2/12.1.0.69933");
        let forever_dir = data.join("db2/1.60.1.70205");
        let retail_text = std::fs::read_to_string(retail_dir.join("Map.csv"))
            .map_err(|error| format!("Retail Map.csv: {error}"))?;
        let retail_maps = crate::map_catalog::MapCatalog::parse(&retail_text, &retail_text)?;
        let merged_maps = crate::map_catalog::MapCatalog::read(data)?;
        let forever_maps = read_table(&forever_dir, "Map", |_| Ok(()))?
            .into_keys()
            .filter(|id| retail_maps.by_id(*id).is_none() && merged_maps.by_id(*id).is_some())
            .collect();
        Ok(Self {
            retail: LiquidCatalog::read(&retail_dir)?,
            forever_maps,
            forever: LiquidCatalog::read(&forever_dir),
        })
    }

    /// Material130 has no implemented PBR renderer. This explicitly borrows LiquidType5
    /// legacy inputs, not a client-authored fallback. Retire when the PBR contract is ported.
    /// Source metadata remains intact in `liquid_material`.
    pub fn render_material(
        &self,
        map_id: u32,
        liquid_type: u16,
        liquid_object: u16,
    ) -> Result<LiquidMaterial, String> {
        let source = self.liquid_material(map_id, liquid_type, liquid_object)?;
        if source.material_id != PBR_WATER_MATERIAL {
            return Ok(source);
        }
        let catalog = if self.uses_forever(map_id) {
            self.forever.as_ref().map_err(Clone::clone)?
        } else {
            &self.retail
        };
        let mut borrowed = catalog.liquid_material(LEGACY_WATER_TYPE, 0)
            .map_err(|error| format!("LiquidType {liquid_type} Material130 requires borrowed LiquidType5 inputs: {error}"))?;
        if borrowed.material_id != 1 {
            return Err("Borrowed LiquidType5 must identify legacy Water material1".into());
        }
        for &slot in LiquidShader::Water.texture_slots() {
            let frames = &borrowed.texture_slots[slot];
            if frames.is_empty() || frames.contains(&0) {
                return Err(format!(
                    "LiquidType {liquid_type} Material130 borrowed LiquidType5 has no valid texture for required legacy slot {slot}"
                ));
            }
        }
        borrowed.liquid_type = source.liquid_type;
        borrowed.material_id = source.material_id;
        borrowed.lvf = source.lvf;
        borrowed.flow_direction = source.flow_direction;
        borrowed.flow_speed = source.flow_speed;
        Ok(borrowed)
    }

    pub fn uses_forever(&self, map_id: u32) -> bool {
        self.forever_maps.contains(&map_id)
    }

    pub fn liquid_material(
        &self,
        map_id: u32,
        liquid_type: u16,
        liquid_object: u16,
    ) -> Result<LiquidMaterial, String> {
        if !self.uses_forever(map_id) {
            return self.retail.liquid_material(liquid_type, liquid_object);
        }
        let catalog = self
            .forever
            .as_ref()
            .map_err(|error| format!("Forever map {map_id}: {error}"))?;
        if liquid_object >= FIRST_LIQUID_OBJECT
            && !catalog.objects.contains_key(&u32::from(liquid_object))
        {
            return Err(format!(
                "Forever LiquidObject {liquid_object} has no DB2 row"
            ));
        }
        let material = catalog
            .liquid_material(liquid_type, liquid_object)
            .map_err(|error| format!("Forever map {map_id}: {error}"))?;
        for &slot in material.shader.texture_slots() {
            if material.texture_slots[slot].is_empty() {
                return Err(format!(
                    "Forever LiquidType {} has no textures for slot {slot}",
                    material.liquid_type
                ));
            }
        }
        Ok(material)
    }
}

impl LiquidCatalog {
    /// Reads `LiquidType`, `LiquidObject`, `LiquidMaterial` and `LiquidTypeXTexture` CSVs.
    pub fn read(db2_dir: &Path) -> Result<Self, String> {
        let types = read_table(db2_dir, "LiquidType", |row| {
            let material_id = row.number("MaterialID")?;
            let float_count = if material_id == PBR_WATER_MATERIAL {
                38
            } else {
                18
            };
            Ok(LiquidTypeRow {
                material_id,
                frame_counts: row.array("FrameCountTexture")?,
                floats: row.numbers("Float", float_count)?,
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
            floats: row.floats.clone(),
            depth_coefficients: row.coefficients,
            flow_direction,
            flow_speed,
            colors: row
                .colors
                .map(|color| [0, 8, 16].map(|shift| ((color >> shift) & 0xff) as f32 / 255.0)),
            ints: row.ints,
            color_source,
            texture_slots,
            // These are legacy Water periods, not PBR/FFT parameters.
            wave_periods: if row.material_id == PBR_WATER_MATERIAL {
                [0.0; 2]
            } else {
                wave_periods(&row.floats)
            },
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
fn wave_periods(floats: &[f32]) -> [f32; 2] {
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

    fn numbers<T: std::str::FromStr>(&self, column: &str, count: usize) -> Result<Vec<T>, String> {
        (0..count)
            .map(|index| self.number(&format!("{column}_{index}")))
            .collect()
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
