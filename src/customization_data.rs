//! Character customization data from ChrCustomization* DB2 CSVs.
//!
//! Parses the CSV chain at startup to build a lookup structure:
//! (race, sex) -> ChrModelID -> options -> choices -> materials + geosets.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use bevy::prelude::*;

#[path = "customization_data_support.rs"]
mod support;
#[path = "customization_csv.rs"]
mod csv;

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
    EyeColor,
    HairStyle,
    HairColor,
    FacialHair,
    Ears,
    Horns,
    Blindfold,
    EyeStyle,
    Eyesight,
}

impl OptionType {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "Skin Color" | "Fur Color" => Some(Self::SkinColor),
            "Face" => Some(Self::Face),
            "Eye Color" | "Eye Color Style" => Some(Self::EyeColor),
            "Hair Style" => Some(Self::HairStyle),
            "Hair Color" => Some(Self::HairColor),
            "Beard" | "Facial Hair" | "Mustache" | "Sideburns" => Some(Self::FacialHair),
            "Ears" => Some(Self::Ears),
            "Horns" => Some(Self::Horns),
            "Blindfold" => Some(Self::Blindfold),
            "Eye Style" => Some(Self::EyeStyle),
            "Eyesight" => Some(Self::Eyesight),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChoiceMaterial {
    pub related_choice_id: u32,
    pub target_id: u16,
    pub fdid: u32,
}

#[derive(Debug, Clone)]
pub struct ChoiceGeoset {
    pub related_choice_id: u32,
    pub geoset_type: u16,
    pub geoset_id: u16,
}

#[derive(Debug, Clone)]
pub struct CustomizationChoice {
    pub id: u32,
    pub display_name: String,
    pub requirement_id: u32,
    /// (ChrModelTextureTargetID, resolved FDID)
    pub materials: Vec<(u16, u32)>,
    /// Materials gated by another selected customization choice.
    pub related_materials: Vec<ChoiceMaterial>,
    /// (GeosetType, GeosetID)
    pub geosets: Vec<(u16, u16)>,
    /// Geosets gated by another selected customization choice.
    pub related_geosets: Vec<ChoiceGeoset>,
    pub shows_scalp: bool,
    sample_swatch: bool,
    /// Representative RGB color sampled from the primary texture (center pixel).
    swatch_color_cache: Arc<OnceLock<Option<[u8; 3]>>>,
}

impl CustomizationChoice {
    fn swatch_color(&self) -> Option<[u8; 3]> {
        if !self.sample_swatch {
            return None;
        }
        *self
            .swatch_color_cache
            .get_or_init(|| support::sample_swatch_color(&self.materials))
    }
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
    presentation_by_model: HashMap<u32, ModelPresentation>,
    hair_scalp_fallback_by_model: HashMap<u32, u16>,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelPresentation {
    pub customize_scale: f32,
    pub camera_distance_offset: f32,
}

impl Default for ModelPresentation {
    fn default() -> Self {
        Self {
            customize_scale: 1.0,
            camera_distance_offset: 0.0,
        }
    }
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
            db.presentation_by_model.insert(
                cm.id,
                ModelPresentation {
                    customize_scale: cm.customize_scale,
                    camera_distance_offset: cm.camera_distance_offset,
                },
            );
        }
        db.hair_scalp_fallback_by_model = build_hair_scalp_fallbacks(&raw.hair_geosets);
        let indexed = IndexedData::build(&raw);
        for (model_id, opts) in &indexed.opts_by_model {
            db.options_by_model
                .insert(*model_id, build_model_options(*model_id, opts, &indexed, &raw));
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

    pub fn choice_count_for_class(&self, race: u8, sex: u8, class: u8, opt_type: OptionType) -> u8 {
        self.options_for(race, sex)
            .and_then(|opts| opts.iter().find(|o| o.option_type == opt_type))
            .map(|o| {
                o.choices
                    .iter()
                    .filter(|choice| support::choice_visible_for_class(race, class, opt_type, choice))
                    .count()
                    .min(255) as u8
            })
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

    pub fn get_choice_for_class(
        &self,
        race: u8,
        sex: u8,
        class: u8,
        opt_type: OptionType,
        index: u8,
    ) -> Option<&CustomizationChoice> {
        self.options_for(race, sex)?
            .iter()
            .find(|o| o.option_type == opt_type)?
            .choices
            .iter()
            .filter(|choice| support::choice_visible_for_class(race, class, opt_type, choice))
            .nth(index as usize)
    }

    pub fn swatch_color(
        &self,
        race: u8,
        sex: u8,
        opt_type: OptionType,
        index: u8,
    ) -> Option<[u8; 3]> {
        self.get_choice(race, sex, opt_type, index)?.swatch_color()
    }

    pub fn choice_name(&self, race: u8, sex: u8, opt_type: OptionType, index: u8) -> Option<&str> {
        let name = self
            .get_choice(race, sex, opt_type, index)?
            .display_name
            .as_str();
        (!name.is_empty()).then_some(name)
    }

    pub fn choice_name_for_class(
        &self,
        race: u8,
        sex: u8,
        class: u8,
        opt_type: OptionType,
        index: u8,
    ) -> Option<&str> {
        let name = self
            .get_choice_for_class(race, sex, class, opt_type, index)?
            .display_name
            .as_str();
        (!name.is_empty()).then_some(name)
    }

    pub fn all_swatch_colors(
        &self,
        race: u8,
        sex: u8,
        opt_type: OptionType,
    ) -> Vec<Option<[u8; 3]>> {
        self.options_for(race, sex)
            .and_then(|opts| opts.iter().find(|o| o.option_type == opt_type))
            .map(|o| {
                o.choices
                    .iter()
                    .map(CustomizationChoice::swatch_color)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn layout_id(&self, race: u8, sex: u8) -> Option<u32> {
        let model_id = race_sex_to_chr_model_id(race, sex)?;
        self.layout_by_model.get(&model_id).copied()
    }

    pub fn presentation_for(&self, race: u8, sex: u8) -> ModelPresentation {
        let Some(model_id) = race_sex_to_chr_model_id(race, sex) else {
            return ModelPresentation::default();
        };
        self.presentation_by_model
            .get(&model_id)
            .copied()
            .unwrap_or_default()
    }

    pub fn scalp_fallback_hair_geoset(&self, race: u8, sex: u8) -> Option<u16> {
        let model_id = race_sex_to_chr_model_id(race, sex)?;
        self.hair_scalp_fallback_by_model.get(&model_id).copied()
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
    model_id: u32,
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
                choices: resolve_option_choices(model_id, opt_type, opt.id, indexed, raw, sample_swatch),
            })
        })
        .collect()
}

fn resolve_option_choices(
    model_id: u32,
    opt_type: OptionType,
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
            let (materials, related_materials, geosets, related_geosets) =
                resolve_choice_elements(ch.id, indexed, raw);
            let shows_scalp =
                choice_shows_scalp(opt_type, model_id, &geosets, &related_geosets, raw);
            (
                ch.order_index,
                CustomizationChoice {
                    id: ch.id,
                    display_name: ch.name.clone(),
                    requirement_id: ch.requirement_id,
                    materials,
                    related_materials,
                    geosets,
                    related_geosets,
                    shows_scalp,
                    sample_swatch,
                    swatch_color_cache: Arc::new(OnceLock::new()),
                },
            )
        })
        .collect();
    sorted.sort_by_key(|(idx, _)| *idx);
    sorted.into_iter().map(|(_, c)| c).collect()
}

fn choice_shows_scalp(
    opt_type: OptionType,
    model_id: u32,
    geosets: &[(u16, u16)],
    related_geosets: &[ChoiceGeoset],
    raw: &RawData,
) -> bool {
    opt_type == OptionType::HairStyle
        && geosets
            .iter()
            .copied()
            .chain(related_geosets.iter().map(|g| (g.geoset_type, g.geoset_id)))
            .any(|(geoset_type, geoset_id)| {
                raw.hair_geosets
                    .get(&(model_id, geoset_type, geoset_id))
                    .copied()
                    .unwrap_or(false)
            })
}

fn build_hair_scalp_fallbacks(
    hair_geosets: &HashMap<(u32, u16, u16), bool>,
) -> HashMap<u32, u16> {
    let mut fallbacks = HashMap::new();
    let mut entries: Vec<_> = hair_geosets.iter().collect();
    entries.sort_by_key(|((model_id, geoset_type, geoset_id), _)| (*model_id, *geoset_type, *geoset_id));
    for (&(model_id, geoset_type, geoset_id), &shows_scalp) in entries {
        if shows_scalp && geoset_type == 0 {
            fallbacks.entry(model_id).or_insert(geoset_id);
        }
    }
    fallbacks
}

type ChoiceElements = (
    Vec<(u16, u32)>,
    Vec<ChoiceMaterial>,
    Vec<(u16, u16)>,
    Vec<ChoiceGeoset>,
);

fn resolve_choice_elements(
    choice_id: u32,
    indexed: &IndexedData<'_>,
    raw: &RawData,
) -> ChoiceElements {
    let mut materials = Vec::new();
    let mut related_materials = Vec::new();
    let mut geosets = Vec::new();
    let mut related_geosets = Vec::new();
    let Some(elements) = indexed.elements_by_choice.get(&choice_id) else {
        return (materials, related_materials, geosets, related_geosets);
    };
    for el in elements {
        if el.material_id > 0
            && let Some(mat) = raw.materials.get(&el.material_id)
            && let Some(&fdid) = raw.texture_fdids.get(&mat.material_resources_id)
        {
            if el.related_choice_id > 0 {
                related_materials.push(ChoiceMaterial {
                    related_choice_id: el.related_choice_id,
                    target_id: mat.texture_target_id,
                    fdid,
                });
            } else {
                materials.push((mat.texture_target_id, fdid));
            }
        }
        if el.geoset_id > 0
            && let Some(geo) = raw.geosets.get(&el.geoset_id)
        {
            if el.related_choice_id > 0 {
                related_geosets.push(ChoiceGeoset {
                    related_choice_id: el.related_choice_id,
                    geoset_type: geo.geoset_type,
                    geoset_id: geo.geoset_id,
                });
            } else {
                geosets.push((geo.geoset_type, geo.geoset_id));
            }
        }
    }
    (materials, related_materials, geosets, related_geosets)
}

// --- CSV parsing (manual, no csv crate) ---

struct RawData {
    chr_models: Vec<RawChrModel>,
    options: Vec<RawOption>,
    choices: Vec<RawChoice>,
    elements: Vec<RawElement>,
    materials: HashMap<u32, RawMaterial>,
    geosets: HashMap<u32, RawGeoset>,
    hair_geosets: HashMap<(u32, u16, u16), bool>,
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
            hair_geosets: parse_hair_geosets(&data_dir.join("CharHairGeosets.csv"))?,
            texture_fdids: parse_texture_file_data(&data_dir.join("TextureFileData.csv"))?,
        })
    }
}

struct RawChrModel {
    id: u32,
    layout_id: u32,
    customize_scale: f32,
    camera_distance_offset: f32,
}
struct RawOption {
    id: u32,
    name: String,
    chr_model_id: u32,
}
struct RawChoice {
    id: u32,
    option_id: u32,
    name: String,
    requirement_id: u32,
    order_index: u32,
}
struct RawElement {
    choice_id: u32,
    related_choice_id: u32,
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

fn chr_model_id_for_hair_row(race: u8, sex: u8) -> Option<u32> {
    race_sex_to_chr_model_id(race, sex)
}

fn parse_chr_models(path: &Path) -> Result<Vec<RawChrModel>, String> {
    let (h, rows) = csv::read_csv(path)?;
    let id = csv::col_idx(&h, "ID")?;
    let layout = csv::col_idx(&h, "CharComponentTextureLayoutID")?;
    let customize_scale = csv::col_idx(&h, "CustomizeScale")?;
    let camera_distance_offset = csv::col_idx(&h, "CameraDistanceOffset")?;
    Ok(rows
        .iter()
        .map(|r| RawChrModel {
            id: csv::field_u32(r, id),
            layout_id: csv::field_u32(r, layout),
            customize_scale: csv::field_f32(r, customize_scale),
            camera_distance_offset: csv::field_f32(r, camera_distance_offset),
        })
        .collect())
}

fn parse_options(path: &Path) -> Result<Vec<RawOption>, String> {
    let (h, rows) = csv::read_csv(path)?;
    let id = csv::col_idx(&h, "ID")?;
    let name = csv::col_idx(&h, "Name_lang")?;
    let model = csv::col_idx(&h, "ChrModelID")?;
    Ok(rows
        .iter()
        .map(|r| RawOption {
            id: csv::field_u32(r, id),
            name: csv::field_str(r, name),
            chr_model_id: csv::field_u32(r, model),
        })
        .collect())
}

fn parse_choices(path: &Path) -> Result<Vec<RawChoice>, String> {
    let (h, rows) = csv::read_csv(path)?;
    let name = csv::col_idx(&h, "Name_lang")?;
    let id = csv::col_idx(&h, "ID")?;
    let opt = csv::col_idx(&h, "ChrCustomizationOptionID")?;
    let requirement = csv::col_idx(&h, "ChrCustomizationReqID")?;
    let order = csv::col_idx(&h, "OrderIndex")?;
    Ok(rows
        .iter()
        .map(|r| RawChoice {
            name: csv::field_str(r, name),
            id: csv::field_u32(r, id),
            option_id: csv::field_u32(r, opt),
            requirement_id: csv::field_u32(r, requirement),
            order_index: csv::field_u32(r, order),
        })
        .collect())
}

fn parse_elements(path: &Path) -> Result<Vec<RawElement>, String> {
    let (h, rows) = csv::read_csv(path)?;
    let choice = csv::col_idx(&h, "ChrCustomizationChoiceID")?;
    let related_choice = csv::col_idx(&h, "RelatedChrCustomizationChoiceID")?;
    let geoset = csv::col_idx(&h, "ChrCustomizationGeosetID")?;
    let material = csv::col_idx(&h, "ChrCustomizationMaterialID")?;
    Ok(rows
        .iter()
        .map(|r| RawElement {
            choice_id: csv::field_u32(r, choice),
            related_choice_id: csv::field_u32(r, related_choice),
            geoset_id: csv::field_u32(r, geoset),
            material_id: csv::field_u32(r, material),
        })
        .collect())
}

fn parse_materials(path: &Path) -> Result<HashMap<u32, RawMaterial>, String> {
    let (h, rows) = csv::read_csv(path)?;
    let id = csv::col_idx(&h, "ID")?;
    let target = csv::col_idx(&h, "ChrModelTextureTargetID")?;
    let res = csv::col_idx(&h, "MaterialResourcesID")?;
    Ok(rows
        .iter()
        .map(|r| {
            let k = csv::field_u32(r, id);
            let v = RawMaterial {
                texture_target_id: csv::field_u32(r, target) as u16,
                material_resources_id: csv::field_u32(r, res),
            };
            (k, v)
        })
        .collect())
}

fn parse_geosets(path: &Path) -> Result<HashMap<u32, RawGeoset>, String> {
    let (h, rows) = csv::read_csv(path)?;
    let id = csv::col_idx(&h, "ID")?;
    let gtype = csv::col_idx(&h, "GeosetType")?;
    let gid = csv::col_idx(&h, "GeosetID")?;
    Ok(rows
        .iter()
        .map(|r| {
            (
                csv::field_u32(r, id),
                RawGeoset {
                    geoset_type: csv::field_u32(r, gtype) as u16,
                    geoset_id: csv::field_u32(r, gid) as u16,
                },
            )
        })
        .collect())
}

fn parse_hair_geosets(path: &Path) -> Result<HashMap<(u32, u16, u16), bool>, String> {
    let (h, rows) = csv::read_csv(path)?;
    let race = csv::col_idx(&h, "RaceID")?;
    let sex = csv::col_idx(&h, "SexID")?;
    let gtype = csv::col_idx(&h, "GeosetType")?;
    let gid = csv::col_idx(&h, "GeosetID")?;
    let shows_scalp = csv::col_idx(&h, "Showscalp")?;
    let mut hair_geosets = HashMap::new();
    for row in &rows {
        let Some(model_id) = chr_model_id_for_hair_row(
            csv::field_u32(row, race) as u8,
            csv::field_u32(row, sex) as u8,
        ) else {
            continue;
        };
        hair_geosets.insert(
            (
                model_id,
                csv::field_u32(row, gtype) as u16,
                csv::field_u32(row, gid) as u16,
            ),
            csv::field_u32(row, shows_scalp) != 0,
        );
    }
    Ok(hair_geosets)
}

fn parse_texture_file_data(path: &Path) -> Result<HashMap<u32, u32>, String> {
    let (h, rows) = csv::read_csv(path)?;
    let fdid = csv::col_idx(&h, "FileDataID")?;
    let res = csv::col_idx(&h, "MaterialResourcesID")?;
    Ok(rows
        .iter()
        .map(|r| (csv::field_u32(r, res), csv::field_u32(r, fdid)))
        .collect())
}

#[cfg(test)]
#[path = "customization_data_tests.rs"]
mod tests;
