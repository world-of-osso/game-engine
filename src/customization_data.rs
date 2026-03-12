//! Character customization data from ChrCustomization* DB2 CSVs.
//!
//! Parses the CSV chain at startup to build a lookup structure:
//! (race, sex) -> ChrModelID -> options -> choices -> materials + geosets.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;

use crate::asset::{blp::load_blp_rgba, casc_resolver::ensure_texture};

/// Hardcoded (race, sex) -> ChrModelID mapping.
/// Derived from ChrModel.csv: IDs 1-22 cover the 11 original races (male=odd, female=even).
fn race_sex_to_chr_model_id(race: u8, sex: u8) -> Option<u32> {
    let race_index: u32 = match race {
        1 => 0,                                            // Human
        2 => 1,                                            // Orc
        3 => 2,                                            // Dwarf
        4 => 3,                                            // NightElf
        5 => 4,                                            // Undead
        6 => 5,                                            // Tauren
        7 => 6,                                            // Gnome
        8 => 7,                                            // Troll
        9 => 8,                                            // Goblin
        10 => 9,                                           // BloodElf
        11 => 10,                                          // Draenei
        22 => return Some(if sex == 0 { 43 } else { 44 }), // Worgen
        25 => return Some(if sex == 0 { 47 } else { 48 }), // Pandaren
        27 => return Some(if sex == 0 { 37 } else { 38 }), // Nightborne
        28 => return Some(if sex == 0 { 39 } else { 40 }), // Highmountain Tauren
        29 => return Some(if sex == 0 { 33 } else { 34 }), // Void Elf
        30 => return Some(if sex == 0 { 35 } else { 36 }), // Lightforged Draenei
        31 => return Some(if sex == 0 { 31 } else { 32 }), // Zandalari Troll
        34 => return Some(if sex == 0 { 41 } else { 42 }), // Dark Iron Dwarf
        35 => return Some(if sex == 0 { 53 } else { 54 }), // Vulpera
        36 => return Some(if sex == 0 { 45 } else { 46 }), // Mag'har Orc
        37 => return Some(if sex == 0 { 55 } else { 56 }), // Mechagnome
        _ => return None,
    };
    Some(race_index * 2 + sex as u32 + 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionType {
    SkinColor,
    Face,
    HairStyle,
    HairColor,
    FacialHair,
}

impl OptionType {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "Skin Color" | "Fur Color" => Some(Self::SkinColor),
            "Face" => Some(Self::Face),
            "Hair Style" => Some(Self::HairStyle),
            "Hair Color" => Some(Self::HairColor),
            "Beard" | "Facial Hair" | "Mustache" | "Sideburns" => Some(Self::FacialHair),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CustomizationChoice {
    /// (ChrModelTextureTargetID, resolved FDID)
    pub materials: Vec<(u16, u32)>,
    /// (GeosetType, GeosetID)
    pub geosets: Vec<(u16, u16)>,
    /// Representative RGB color sampled from the primary texture (center pixel).
    pub swatch_color: Option<[u8; 3]>,
}

#[derive(Debug, Clone)]
pub struct CustomizationOption {
    pub option_type: OptionType,
    pub choices: Vec<CustomizationChoice>,
}

#[derive(Resource, Default, Debug)]
pub struct CustomizationDb {
    options_by_model: HashMap<u32, Vec<CustomizationOption>>,
    pub layout_by_model: HashMap<u32, u32>,
}

impl CustomizationDb {
    pub fn load(data_dir: &Path) -> Self {
        match Self::try_load(data_dir) {
            Ok(db) => {
                info!(
                    "CustomizationDb loaded: {} models",
                    db.options_by_model.len()
                );
                db
            }
            Err(e) => {
                warn!("Failed to load customization data: {e}");
                Self::default()
            }
        }
    }

    fn try_load(data_dir: &Path) -> Result<Self, String> {
        let raw = RawData::parse(data_dir)?;
        let mut db = CustomizationDb::default();
        for cm in &raw.chr_models {
            db.layout_by_model.insert(cm.id, cm.layout_id);
        }
        let indexed = IndexedData::build(&raw);
        for (model_id, opts) in &indexed.opts_by_model {
            db.options_by_model
                .insert(*model_id, build_model_options(opts, &indexed, &raw));
        }
        Ok(db)
    }

    pub fn options_for(&self, race: u8, sex: u8) -> Option<&[CustomizationOption]> {
        let model_id = race_sex_to_chr_model_id(race, sex)?;
        self.options_by_model.get(&model_id).map(|v| v.as_slice())
    }

    pub fn choice_count(&self, race: u8, sex: u8, opt_type: OptionType) -> u8 {
        self.options_for(race, sex)
            .and_then(|opts| opts.iter().find(|o| o.option_type == opt_type))
            .map(|o| o.choices.len().min(255) as u8)
            .unwrap_or(0)
    }

    pub fn get_choice(
        &self,
        race: u8,
        sex: u8,
        opt_type: OptionType,
        index: u8,
    ) -> Option<&CustomizationChoice> {
        self.options_for(race, sex)?
            .iter()
            .find(|o| o.option_type == opt_type)?
            .choices
            .get(index as usize)
    }

    pub fn swatch_color(
        &self,
        race: u8,
        sex: u8,
        opt_type: OptionType,
        index: u8,
    ) -> Option<[u8; 3]> {
        self.get_choice(race, sex, opt_type, index)?.swatch_color
    }

    pub fn all_swatch_colors(
        &self,
        race: u8,
        sex: u8,
        opt_type: OptionType,
    ) -> Vec<Option<[u8; 3]>> {
        self.options_for(race, sex)
            .and_then(|opts| opts.iter().find(|o| o.option_type == opt_type))
            .map(|o| o.choices.iter().map(|c| c.swatch_color).collect())
            .unwrap_or_default()
    }

    pub fn layout_id(&self, race: u8, sex: u8) -> Option<u32> {
        let model_id = race_sex_to_chr_model_id(race, sex)?;
        self.layout_by_model.get(&model_id).copied()
    }
}

// --- Indexed data for join resolution ---

struct IndexedData<'a> {
    opts_by_model: HashMap<u32, Vec<&'a RawOption>>,
    choices_by_option: HashMap<u32, Vec<&'a RawChoice>>,
    elements_by_choice: HashMap<u32, Vec<&'a RawElement>>,
}

impl<'a> IndexedData<'a> {
    fn build(raw: &'a RawData) -> Self {
        let mut opts_by_model: HashMap<u32, Vec<&RawOption>> = HashMap::new();
        for opt in &raw.options {
            if opt.chr_model_id > 0 {
                opts_by_model.entry(opt.chr_model_id).or_default().push(opt);
            }
        }
        let mut choices_by_option: HashMap<u32, Vec<&RawChoice>> = HashMap::new();
        for ch in &raw.choices {
            choices_by_option.entry(ch.option_id).or_default().push(ch);
        }
        let mut elements_by_choice: HashMap<u32, Vec<&RawElement>> = HashMap::new();
        for el in &raw.elements {
            elements_by_choice.entry(el.choice_id).or_default().push(el);
        }
        Self {
            opts_by_model,
            choices_by_option,
            elements_by_choice,
        }
    }
}

fn build_model_options(
    opts: &[&RawOption],
    indexed: &IndexedData<'_>,
    raw: &RawData,
) -> Vec<CustomizationOption> {
    opts.iter()
        .filter_map(|opt| {
            let opt_type = OptionType::from_name(&opt.name)?;
            let sample_swatch = matches!(opt_type, OptionType::SkinColor | OptionType::HairColor);
            Some(CustomizationOption {
                option_type: opt_type,
                choices: resolve_option_choices(opt.id, indexed, raw, sample_swatch),
            })
        })
        .collect()
}

fn resolve_option_choices(
    option_id: u32,
    indexed: &IndexedData<'_>,
    raw: &RawData,
    sample_swatch: bool,
) -> Vec<CustomizationChoice> {
    let Some(raw_choices) = indexed.choices_by_option.get(&option_id) else {
        return Vec::new();
    };
    let mut sorted: Vec<_> = raw_choices
        .iter()
        .map(|ch| {
            let (materials, geosets) = resolve_choice_elements(ch.id, indexed, raw);
            let swatch_color = if sample_swatch {
                sample_swatch_color(&materials)
            } else {
                None
            };
            (
                ch.order_index,
                CustomizationChoice {
                    materials,
                    geosets,
                    swatch_color,
                },
            )
        })
        .collect();
    sorted.sort_by_key(|(idx, _)| *idx);
    sorted.into_iter().map(|(_, c)| c).collect()
}

/// Sample the center pixel of the first texture to get a representative color.
fn sample_swatch_color(materials: &[(u16, u32)]) -> Option<[u8; 3]> {
    let &(_, fdid) = materials.first()?;
    let path = ensure_texture(fdid)?;
    let (rgba, w, h) = load_blp_rgba(&path).ok()?;
    let cx = w / 2;
    let cy = h / 2;
    let idx = ((cy * w + cx) * 4) as usize;
    if idx + 2 < rgba.len() {
        Some([rgba[idx], rgba[idx + 1], rgba[idx + 2]])
    } else {
        None
    }
}

type ChoiceElements = (Vec<(u16, u32)>, Vec<(u16, u16)>);

fn resolve_choice_elements(
    choice_id: u32,
    indexed: &IndexedData<'_>,
    raw: &RawData,
) -> ChoiceElements {
    let mut materials = Vec::new();
    let mut geosets = Vec::new();
    let Some(elements) = indexed.elements_by_choice.get(&choice_id) else {
        return (materials, geosets);
    };
    for el in elements {
        if el.material_id > 0
            && let Some(mat) = raw.materials.get(&el.material_id)
            && let Some(&fdid) = raw.texture_fdids.get(&mat.material_resources_id)
        {
            materials.push((mat.texture_target_id, fdid));
        }
        if el.geoset_id > 0
            && let Some(geo) = raw.geosets.get(&el.geoset_id)
        {
            geosets.push((geo.geoset_type, geo.geoset_id));
        }
    }
    (materials, geosets)
}

// --- CSV parsing (manual, no csv crate) ---

struct RawData {
    chr_models: Vec<RawChrModel>,
    options: Vec<RawOption>,
    choices: Vec<RawChoice>,
    elements: Vec<RawElement>,
    materials: HashMap<u32, RawMaterial>,
    geosets: HashMap<u32, RawGeoset>,
    texture_fdids: HashMap<u32, u32>,
}

impl RawData {
    fn parse(data_dir: &Path) -> Result<Self, String> {
        Ok(Self {
            chr_models: parse_chr_models(&data_dir.join("ChrModel.csv"))?,
            options: parse_options(&data_dir.join("ChrCustomizationOption.csv"))?,
            choices: parse_choices(&data_dir.join("ChrCustomizationChoice.csv"))?,
            elements: parse_elements(&data_dir.join("ChrCustomizationElement.csv"))?,
            materials: parse_materials(&data_dir.join("ChrCustomizationMaterial.csv"))?,
            geosets: parse_geosets(&data_dir.join("ChrCustomizationGeoset.csv"))?,
            texture_fdids: parse_texture_file_data(&data_dir.join("TextureFileData.csv"))?,
        })
    }
}

struct RawChrModel {
    id: u32,
    layout_id: u32,
}
struct RawOption {
    id: u32,
    name: String,
    chr_model_id: u32,
}
struct RawChoice {
    id: u32,
    option_id: u32,
    order_index: u32,
}
struct RawElement {
    choice_id: u32,
    geoset_id: u32,
    material_id: u32,
}
struct RawMaterial {
    texture_target_id: u16,
    material_resources_id: u32,
}
struct RawGeoset {
    geoset_type: u16,
    geoset_id: u16,
}

/// Parse a CSV with quoted fields. Returns (headers, rows).
pub fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut lines = data.lines();
    let header_line = lines.next().ok_or("empty CSV")?;
    let headers = parse_csv_line(header_line);
    let rows: Vec<_> = lines.map(parse_csv_line).collect();
    Ok((headers, rows))
}

/// Parse a single CSV line handling quoted fields (double-quote escaping).
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == ',' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

fn col_idx(headers: &[String], name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| format!("Column '{name}' not found"))
}

fn field_u32(row: &[String], col: usize) -> u32 {
    row.get(col)
        .and_then(|s| {
            s.parse::<u32>()
                .ok()
                .or_else(|| s.parse::<i32>().ok().map(|v| v as u32))
        })
        .unwrap_or(0)
}

fn field_str(row: &[String], col: usize) -> String {
    row.get(col).cloned().unwrap_or_default()
}

fn parse_chr_models(path: &Path) -> Result<Vec<RawChrModel>, String> {
    let (h, rows) = read_csv(path)?;
    let id = col_idx(&h, "ID")?;
    let layout = col_idx(&h, "CharComponentTextureLayoutID")?;
    Ok(rows
        .iter()
        .map(|r| RawChrModel {
            id: field_u32(r, id),
            layout_id: field_u32(r, layout),
        })
        .collect())
}

fn parse_options(path: &Path) -> Result<Vec<RawOption>, String> {
    let (h, rows) = read_csv(path)?;
    let id = col_idx(&h, "ID")?;
    let name = col_idx(&h, "Name_lang")?;
    let model = col_idx(&h, "ChrModelID")?;
    Ok(rows
        .iter()
        .map(|r| RawOption {
            id: field_u32(r, id),
            name: field_str(r, name),
            chr_model_id: field_u32(r, model),
        })
        .collect())
}

fn parse_choices(path: &Path) -> Result<Vec<RawChoice>, String> {
    let (h, rows) = read_csv(path)?;
    let id = col_idx(&h, "ID")?;
    let opt = col_idx(&h, "ChrCustomizationOptionID")?;
    let order = col_idx(&h, "OrderIndex")?;
    Ok(rows
        .iter()
        .map(|r| RawChoice {
            id: field_u32(r, id),
            option_id: field_u32(r, opt),
            order_index: field_u32(r, order),
        })
        .collect())
}

fn parse_elements(path: &Path) -> Result<Vec<RawElement>, String> {
    let (h, rows) = read_csv(path)?;
    let choice = col_idx(&h, "ChrCustomizationChoiceID")?;
    let geoset = col_idx(&h, "ChrCustomizationGeosetID")?;
    let material = col_idx(&h, "ChrCustomizationMaterialID")?;
    Ok(rows
        .iter()
        .map(|r| RawElement {
            choice_id: field_u32(r, choice),
            geoset_id: field_u32(r, geoset),
            material_id: field_u32(r, material),
        })
        .collect())
}

fn parse_materials(path: &Path) -> Result<HashMap<u32, RawMaterial>, String> {
    let (h, rows) = read_csv(path)?;
    let id = col_idx(&h, "ID")?;
    let target = col_idx(&h, "ChrModelTextureTargetID")?;
    let res = col_idx(&h, "MaterialResourcesID")?;
    Ok(rows
        .iter()
        .map(|r| {
            let k = field_u32(r, id);
            let v = RawMaterial {
                texture_target_id: field_u32(r, target) as u16,
                material_resources_id: field_u32(r, res),
            };
            (k, v)
        })
        .collect())
}

fn parse_geosets(path: &Path) -> Result<HashMap<u32, RawGeoset>, String> {
    let (h, rows) = read_csv(path)?;
    let id = col_idx(&h, "ID")?;
    let gtype = col_idx(&h, "GeosetType")?;
    let gid = col_idx(&h, "GeosetID")?;
    Ok(rows
        .iter()
        .map(|r| {
            (
                field_u32(r, id),
                RawGeoset {
                    geoset_type: field_u32(r, gtype) as u16,
                    geoset_id: field_u32(r, gid) as u16,
                },
            )
        })
        .collect())
}

fn parse_texture_file_data(path: &Path) -> Result<HashMap<u32, u32>, String> {
    let (h, rows) = read_csv(path)?;
    let fdid = col_idx(&h, "FileDataID")?;
    let res = col_idx(&h, "MaterialResourcesID")?;
    Ok(rows
        .iter()
        .map(|r| (field_u32(r, res), field_u32(r, fdid)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chr_model_id_human() {
        assert_eq!(race_sex_to_chr_model_id(1, 0), Some(1));
        assert_eq!(race_sex_to_chr_model_id(1, 1), Some(2));
    }

    #[test]
    fn chr_model_id_draenei() {
        assert_eq!(race_sex_to_chr_model_id(11, 0), Some(21));
        assert_eq!(race_sex_to_chr_model_id(11, 1), Some(22));
    }

    #[test]
    fn load_customization_db() {
        let db = CustomizationDb::load(Path::new("data"));
        let count = db.choice_count(1, 0, OptionType::SkinColor);
        assert!(count > 0, "Human Male skin colors: {count}");
        let count = db.choice_count(1, 0, OptionType::HairStyle);
        assert!(count > 0, "Human Male hair styles: {count}");
    }

    #[test]
    fn human_male_skin_has_materials() {
        let db = CustomizationDb::load(Path::new("data"));
        let choice = db.get_choice(1, 0, OptionType::SkinColor, 0).unwrap();
        assert!(
            !choice.materials.is_empty(),
            "Skin should have materials: {choice:?}"
        );
    }
}
