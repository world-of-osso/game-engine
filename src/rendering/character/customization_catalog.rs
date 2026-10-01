//! Character customization data from ChrCustomization* DB2 CSVs.
//!
//! Parses the CSV chain at startup to build a lookup structure:
//! (race, sex) -> ChrModelID -> full choice IDs -> materials + geosets.
//! UI options retain authored IDs, category metadata and ordering for every local option.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

#[path = "customization_catalog_support.rs"]
mod support;

/// (race, sex) -> ChrModelID from ChrRaceXChrModel.csv, plus each race's
/// ChrRaces.UnalteredVisualRaceID (e.g. Worgen 22 -> Human 1 for the Gilnean form).
#[derive(Debug, Default, Clone)]
pub(crate) struct RaceModels {
    pub(crate) chr_model_by_race_sex: HashMap<(u8, u8), u32>,
    pub(crate) unaltered_race: HashMap<u8, u8>,
}

impl RaceModels {
    pub(crate) fn load(data_dir: &Path) -> Result<Self, String> {
        let mut models = RaceModels::default();
        for_each_csv_row(&data_dir.join("ChrRaceXChrModel.csv"), |row| {
            let race = row.u8("ChrRacesID")?;
            let sex = row.u8("Sex")?;
            let model = row.u32("ChrModelID")?;
            models.chr_model_by_race_sex.insert((race, sex), model);
            Ok(())
        })?;
        for_each_csv_row(&data_dir.join("ChrRaces.csv"), |row| {
            let unaltered = row.u8("UnalteredVisualRaceID")?;
            if unaltered != 0 {
                models.unaltered_race.insert(row.u8("ID")?, unaltered);
            }
            Ok(())
        })?;
        Ok(models)
    }

    pub(crate) fn chr_model_id(&self, race: u8, sex: u8) -> Option<u32> {
        self.chr_model_by_race_sex.get(&(race, sex)).copied()
    }

    fn unaltered_chr_model_id(&self, race: u8, sex: u8) -> Option<u32> {
        let unaltered = *self.unaltered_race.get(&race)?;
        self.chr_model_id(unaltered, sex)
    }

    /// The race whose ChrCustomizationReq masks gate `race`'s options: an unplayable
    /// unaltered form (Dracthyr visage 75 / 76) answers for the playable race that
    /// names it, as TrinityCore's DB2Manager registers its options under that
    /// parent race (`parentRaces`).
    fn requirement_race(&self, race: u8) -> u8 {
        if support::race_mask_bit(race).is_some() {
            return race;
        }
        self.unaltered_race
            .iter()
            .find_map(|(&parent, &form)| (form == race).then_some(parent))
            .unwrap_or(race)
    }
}

/// One ChrCustomizationReq row, which gates who may select an option or choice.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CustomizationRequirement {
    req_type: u32,
    class_mask: i32,
    race_masks: [i32; 2],
    /// Achievement, quest or item-appearance unlocks are account collection state.
    unlock_gated: bool,
}

impl CustomizationRequirement {
    /// TrinityCore `WorldSession::MeetsChrCustomizationReq` for a new character with no
    /// account unlocks. ReqType bit 0 marks player rows; WoWDBDefs names the remaining
    /// ReqType values NPC (2) and Transmog (4), which players cannot select.
    fn allows_new_character(&self, race: u8, class: u8) -> bool {
        let class_allowed = self.class_mask == 0
            || class
                .checked_sub(1)
                .is_some_and(|bit| bit < 32 && (self.class_mask as u32) & (1 << bit) != 0);
        let race_allowed = self.race_masks == [0, 0]
            || support::race_mask_bit(race).is_some_and(|bit| {
                (self.race_masks[(bit / 32) as usize] as u32) & (1 << (bit % 32)) != 0
            });
        self.req_type & 1 != 0 && !self.unlock_gated && class_allowed && race_allowed
    }
}

/// Read `ChrCustomizationReq.csv` (build-pinned export; see the character-creation spec).
pub(crate) fn load_requirements(
    data_dir: &Path,
) -> Result<HashMap<u32, CustomizationRequirement>, String> {
    let mut requirements = HashMap::new();
    for_each_csv_row(&data_dir.join("ChrCustomizationReq.csv"), |row| {
        let unlock_gated = [
            "ReqAchievementID",
            "ReqQuestID",
            "ReqItemModifiedAppearanceID",
        ]
        .into_iter()
        .map(|column| row.i32(column))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|id| id != 0);
        requirements.insert(
            row.u32("ID")?,
            CustomizationRequirement {
                req_type: row.u32("ReqType")?,
                class_mask: row.i32("ClassMask")?,
                race_masks: [row.i32("RaceMasks_0")?, row.i32("RaceMasks_1")?],
                unlock_gated,
            },
        );
        Ok(())
    })?;
    Ok(requirements)
}

/// Read `ChrCustomizationReqChoice.csv`: requirement ID -> required choice IDs.
fn load_required_choice_ids(data_dir: &Path) -> Result<HashMap<u32, Vec<u32>>, String> {
    let mut required: HashMap<u32, Vec<u32>> = HashMap::new();
    for_each_csv_row(&data_dir.join("ChrCustomizationReqChoice.csv"), |row| {
        required
            .entry(row.u32("ChrCustomizationReqID")?)
            .or_default()
            .push(row.u32("ChrCustomizationChoiceID")?);
        Ok(())
    })?;
    Ok(required)
}

/// One ChrCustomizationReqChoice group: the selection must contain one of `choice_ids`
/// for `option_id` (TrinityCore `MeetsChrCustomizationReq` required dependent choices).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredChoices {
    pub option_id: u32,
    pub choice_ids: Vec<u32>,
}

struct CsvRow<'a> {
    headers: &'a [String],
    fields: Vec<String>,
    path: &'a Path,
}

impl CsvRow<'_> {
    fn u32(&self, column: &str) -> Result<u32, String> {
        let index = crate::csv_util::header_index(self.headers, column, self.path)?;
        let value = self.fields.get(index).map(String::as_str).unwrap_or("");
        value.parse().map_err(|err| {
            format!(
                "invalid {column} {value:?} in {}: {err}",
                self.path.display()
            )
        })
    }

    fn i32(&self, column: &str) -> Result<i32, String> {
        let index = crate::csv_util::header_index(self.headers, column, self.path)?;
        let value = self.fields.get(index).map(String::as_str).unwrap_or("");
        value.parse().map_err(|err| {
            format!(
                "invalid {column} {value:?} in {}: {err}",
                self.path.display()
            )
        })
    }

    fn u8(&self, column: &str) -> Result<u8, String> {
        let value = self.u32(column)?;
        u8::try_from(value)
            .map_err(|_| format!("{column} {value} out of range in {}", self.path.display()))
    }
}

fn for_each_csv_row(
    path: &Path,
    mut visit: impl FnMut(&CsvRow<'_>) -> Result<(), String>,
) -> Result<(), String> {
    let text =
        std::fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let headers = crate::csv_util::parse_csv_line_trimmed(lines.next().unwrap_or(""));
    for line in lines.filter(|line| !line.is_empty()) {
        visit(&CsvRow {
            headers: &headers,
            fields: crate::csv_util::parse_csv_line_trimmed(line),
            path,
        })?;
    }
    Ok(())
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
    Additional(u32),
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

/// One `ChrCustomizationSkinnedModel` row of a choice: submesh
/// `geoset_type * 100 + geoset_id` of collection M2 `collection_fdid`, bound to
/// the character skeleton (wow.export `DBCharacterCustomization.get_skinned_model_for_choice`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceSkinnedModel {
    /// Nonzero: shown only while that other choice is selected.
    pub related_choice_id: u32,
    pub collection_fdid: u32,
    pub geoset_type: u16,
    pub geoset_id: u16,
}

impl ChoiceSkinnedModel {
    /// The collection submesh ID this row shows.
    pub fn mesh_part_id(&self) -> u16 {
        self.geoset_type * 100 + self.geoset_id
    }
}

#[derive(Debug, Clone)]
pub struct CustomizationChoice {
    pub id: u32,
    pub display_name: String,
    pub requirement_id: u32,
    pub visibility_requirement_id: u32,
    /// Authored signed ARGB values; zero is retained rather than replaced by sampled colors.
    pub swatch_colors: [i32; 2],
    pub has_unsupported_effects: bool,
    /// (ChrModelTextureTargetID, resolved FDID)
    pub materials: Vec<(u16, u32)>,
    /// Materials gated by another selected customization choice.
    pub related_materials: Vec<ChoiceMaterial>,
    /// (GeosetType, GeosetID)
    pub geosets: Vec<(u16, u16)>,
    /// Geosets gated by another selected customization choice.
    pub related_geosets: Vec<ChoiceGeoset>,
    /// Collection models bound to the character skeleton.
    pub skinned_models: Vec<ChoiceSkinnedModel>,
    pub shows_scalp: bool,
    pub(super) sample_swatch: bool,
    /// Representative RGB color sampled from the primary texture (center pixel).
    pub(super) swatch_color_cache: Arc<OnceLock<Option<[u8; 3]>>>,
}

impl CustomizationChoice {
    pub fn sample_swatch_color_with(
        &self,
        loader: impl FnOnce(&[(u16, u32)]) -> Option<[u8; 3]>,
    ) -> Option<[u8; 3]> {
        if !self.sample_swatch {
            return None;
        }
        *self
            .swatch_color_cache
            .get_or_init(|| loader(&self.materials))
    }
}

#[derive(Debug, Clone)]
pub struct CustomizationOption {
    pub id: u32,
    pub display_name: String,
    pub category_id: u32,
    pub category_name: String,
    pub category_order_index: u32,
    pub category_icon: u32,
    pub category_selected_icon: u32,
    pub order_index: u32,
    /// Authored CSV OptionType; distinct from the recognized selector enum.
    pub ui_type: u32,
    /// Authored requirement ID, not an inferred eligibility decision.
    pub requirement_id: u32,
    pub option_type: OptionType,
    pub choices: Vec<CustomizationChoice>,
}

/// Bevy-free catalog name shared by both clients; the Bevy crate wraps it as a resource.
pub type CustomizationCatalog = CustomizationDb;

#[derive(Default, Debug)]
pub struct CustomizationDb {
    pub(super) options_by_model: HashMap<u32, Vec<CustomizationOption>>,
    choices_by_model: HashMap<u32, HashMap<u32, CustomizationChoice>>,
    pub layout_by_model: HashMap<u32, u32>,
    presentation_by_model: HashMap<u32, ModelPresentation>,
    hair_scalp_fallback_by_model: HashMap<u32, u16>,
    race_models: RaceModels,
    requirements: HashMap<u32, CustomizationRequirement>,
    required_choices: HashMap<u32, Vec<RequiredChoices>>,
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
    pub(crate) fn from_raw(raw: &RawData) -> Self {
        let mut db = CustomizationDb {
            race_models: raw.race_models.clone(),
            ..CustomizationDb::default()
        };
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
        let indexed = IndexedData::build(raw);
        for (model_id, opts) in &indexed.opts_by_model {
            let choices = build_model_choices(*model_id, opts, &indexed, raw);
            db.options_by_model.insert(
                *model_id,
                build_model_options(opts, &indexed, &choices, raw),
            );
            db.choices_by_model.insert(*model_id, choices);
        }
        db
    }

    pub fn chr_model_id(&self, race: u8, sex: u8) -> Option<u32> {
        self.race_models.chr_model_id(race, sex)
    }

    /// Resolve authored full IDs, including options that the player UI does not expose.
    pub fn choice_by_id(&self, race: u8, sex: u8, choice_id: u32) -> Option<&CustomizationChoice> {
        let model_id = self.chr_model_id(race, sex)?;
        self.choices_by_model.get(&model_id)?.get(&choice_id)
    }

    /// A choice of the race's unaltered visual form (ChrRaces.UnalteredVisualRaceID),
    /// e.g. a Worgen profile's Gilnean-form choices on the Human model.
    pub fn unaltered_form_choice_by_id(
        &self,
        race: u8,
        sex: u8,
        choice_id: u32,
    ) -> Option<&CustomizationChoice> {
        let model_id = self.race_models.unaltered_chr_model_id(race, sex)?;
        self.choices_by_model.get(&model_id)?.get(&choice_id)
    }

    pub fn options_for(&self, race: u8, sex: u8) -> Option<&[CustomizationOption]> {
        let model_id = self.chr_model_id(race, sex)?;
        self.options_by_model.get(&model_id).map(|v| v.as_slice())
    }

    pub fn option_by_id(&self, race: u8, sex: u8, option_id: u32) -> Option<&CustomizationOption> {
        self.options_for(race, sex)?
            .iter()
            .find(|option| option.id == option_id)
    }

    /// Stored-index space: the choices a core selector index addresses, in authored order.
    /// Persisted `CharacterAppearance` indices mean positions in this list, so it never
    /// depends on requirement data; use `offered_choices` for what creation may select.
    pub fn choices_for_option(
        &self,
        race: u8,
        sex: u8,
        class: u8,
        option_id: u32,
    ) -> Vec<&CustomizationChoice> {
        self.option_by_id(race, sex, option_id)
            .map(|option| stored_choices(race, class, option).collect())
            .unwrap_or_default()
    }

    /// Choices a new `class` character may select: requirement-allowed members of the
    /// stored list whose required-choice groups can be met.
    pub fn offered_choices(
        &self,
        race: u8,
        sex: u8,
        class: u8,
        option_id: u32,
    ) -> Vec<&CustomizationChoice> {
        let Some(option) = self.option_by_id(race, sex, option_id) else {
            return Vec::new();
        };
        if !self.requirement_allows(option.requirement_id, race, class) {
            return Vec::new();
        }
        stored_choices(race, class, option)
            .filter(|choice| {
                self.requirement_allows(choice.requirement_id, race, class)
                    && self.required_choices(choice).iter().all(|group| {
                        group.choice_ids.iter().any(|&id| {
                            self.choice_id_available(race, sex, class, group.option_id, id)
                        })
                    })
            })
            .collect()
    }

    fn option_available(&self, race: u8, class: u8, option: &CustomizationOption) -> bool {
        support::option_visible_for_class(race, class, option.option_type)
            && self.requirement_allows(option.requirement_id, race, class)
    }

    /// Option and choice requirements only; required-choice groups are not followed.
    fn choice_id_available(
        &self,
        race: u8,
        sex: u8,
        class: u8,
        option_id: u32,
        choice_id: u32,
    ) -> bool {
        self.option_by_id(race, sex, option_id)
            .is_some_and(|option| self.option_available(race, class, option))
            && self
                .choice_by_id(race, sex, choice_id)
                .is_some_and(|choice| self.requirement_allows(choice.requirement_id, race, class))
    }

    /// Requirement 0 is ungated; an unknown requirement ID is not selectable.
    fn requirement_allows(&self, requirement_id: u32, race: u8, class: u8) -> bool {
        requirement_id == 0
            || self
                .requirements
                .get(&requirement_id)
                .is_some_and(|requirement| {
                    requirement.allows_new_character(self.race_models.requirement_race(race), class)
                })
    }

    /// Other options' choices this choice's requirement needs selected alongside it.
    pub fn required_choices(&self, choice: &CustomizationChoice) -> &[RequiredChoices] {
        self.required_choices
            .get(&choice.requirement_id)
            .map_or(&[], Vec::as_slice)
    }

    /// Load `ChrCustomizationReq` and `ChrCustomizationReqChoice` from `data_dir`.
    pub(crate) fn load_requirements(&mut self, data_dir: &Path) -> Result<(), String> {
        self.requirements = load_requirements(data_dir)?;
        let option_by_choice: HashMap<u32, u32> = self
            .options_by_model
            .values()
            .flatten()
            .flat_map(|option| option.choices.iter().map(|choice| (choice.id, option.id)))
            .collect();
        self.required_choices = load_required_choice_ids(data_dir)?
            .into_iter()
            .map(|(requirement_id, choice_ids)| {
                (
                    requirement_id,
                    group_by_option(&choice_ids, &option_by_choice),
                )
            })
            .collect();
        Ok(())
    }

    pub(super) fn option_for_type(
        &self,
        race: u8,
        sex: u8,
        opt_type: OptionType,
    ) -> Option<&CustomizationOption> {
        // Preserve the original lowest-ID selection when multiple authored labels
        // share a core selector (for example Beard, Mustache and Sideburns).
        self.options_for(race, sex)?
            .iter()
            .filter(|option| option.option_type == opt_type)
            .min_by_key(|option| option.id)
    }

    pub fn choice_count(&self, race: u8, sex: u8, opt_type: OptionType) -> u8 {
        self.option_for_type(race, sex, opt_type)
            .map(|o| o.choices.len().min(255) as u8)
            .unwrap_or(0)
    }

    pub fn choice_count_for_class(&self, race: u8, sex: u8, class: u8, opt_type: OptionType) -> u8 {
        self.option_for_type(race, sex, opt_type)
            .map(|option| stored_choices(race, class, option).count().min(255) as u8)
            .unwrap_or(0)
    }

    pub fn get_choice(
        &self,
        race: u8,
        sex: u8,
        opt_type: OptionType,
        index: u8,
    ) -> Option<&CustomizationChoice> {
        self.option_for_type(race, sex, opt_type)?
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
        let option = self.option_for_type(race, sex, opt_type)?;
        stored_choices(race, class, option).nth(index as usize)
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

    pub fn layout_id(&self, race: u8, sex: u8) -> Option<u32> {
        let model_id = self.chr_model_id(race, sex)?;
        self.layout_by_model.get(&model_id).copied()
    }

    pub fn presentation_for(&self, race: u8, sex: u8) -> ModelPresentation {
        let Some(model_id) = self.chr_model_id(race, sex) else {
            return ModelPresentation::default();
        };
        self.presentation_by_model
            .get(&model_id)
            .copied()
            .unwrap_or_default()
    }

    pub fn scalp_fallback_hair_geoset(&self, race: u8, sex: u8) -> Option<u16> {
        let model_id = self.chr_model_id(race, sex)?;
        self.hair_scalp_fallback_by_model.get(&model_id).copied()
    }
}

/// Choices the original class filter showed keep their persisted indices; choices it hid
/// (Night Elf/Blood Elf faces of other classes) follow them, so old indices stay stable
/// while every authored choice gets an index. Whole-option class gates still apply.
fn stored_choices<'a>(
    race: u8,
    class: u8,
    option: &'a CustomizationOption,
) -> impl Iterator<Item = &'a CustomizationChoice> {
    let option_visible = support::option_visible_for_class(race, class, option.option_type);
    let legacy = move |choice: &&CustomizationChoice| {
        support::legacy_choice_visible(race, class, option.option_type, choice)
    };
    let visible = option.choices.iter().filter(legacy);
    let hidden = option.choices.iter().filter(move |choice| !legacy(choice));
    visible.chain(hidden).filter(move |_| option_visible)
}

/// Group required choice IDs by their option, keeping authored option and choice order.
/// Choices without a known option cannot be selected and form their own unsatisfiable group.
fn group_by_option(
    choice_ids: &[u32],
    option_by_choice: &HashMap<u32, u32>,
) -> Vec<RequiredChoices> {
    let mut groups: Vec<RequiredChoices> = Vec::new();
    for &choice_id in choice_ids {
        let option_id = option_by_choice.get(&choice_id).copied().unwrap_or(0);
        match groups.iter_mut().find(|group| group.option_id == option_id) {
            Some(group) => group.choice_ids.push(choice_id),
            None => groups.push(RequiredChoices {
                option_id,
                choice_ids: vec![choice_id],
            }),
        }
    }
    groups
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

fn build_model_choices(
    model_id: u32,
    opts: &[&RawOption],
    indexed: &IndexedData<'_>,
    raw: &RawData,
) -> HashMap<u32, CustomizationChoice> {
    opts.iter()
        .flat_map(|opt| {
            resolve_option_choices(
                model_id,
                OptionType::from_name(&opt.name),
                opt.id,
                indexed,
                raw,
            )
        })
        .map(|choice| (choice.id, choice))
        .collect()
}

fn build_model_options(
    opts: &[&RawOption],
    indexed: &IndexedData<'_>,
    choices: &HashMap<u32, CustomizationChoice>,
    raw: &RawData,
) -> Vec<CustomizationOption> {
    let mut options: Vec<_> = opts
        .iter()
        .map(|opt| {
            let opt_type =
                OptionType::from_name(&opt.name).unwrap_or(OptionType::Additional(opt.id));
            let category = raw.categories.get(&opt.category_id);
            let mut ordered = indexed
                .choices_by_option
                .get(&opt.id)
                .cloned()
                .unwrap_or_default();
            ordered.sort_by_key(|choice| (choice.order_index, choice.id));
            CustomizationOption {
                id: opt.id,
                display_name: opt.name.clone(),
                category_id: opt.category_id,
                category_name: category
                    .map(|category| category.name.clone())
                    .unwrap_or_default(),
                category_order_index: category
                    .map(|category| category.order_index)
                    .unwrap_or_default(),
                category_icon: category.map(|category| category.icon).unwrap_or_default(),
                category_selected_icon: category
                    .map(|category| category.selected_icon)
                    .unwrap_or_default(),
                order_index: opt.order_index,
                ui_type: opt.ui_type,
                requirement_id: opt.requirement_id,
                option_type: opt_type,
                choices: ordered
                    .iter()
                    .map(|choice| choices[&choice.id].clone())
                    .collect(),
            }
        })
        .collect();
    options.sort_by_key(|option| (option.category_order_index, option.order_index, option.id));
    options
}

fn resolve_option_choices(
    model_id: u32,
    opt_type: Option<OptionType>,
    option_id: u32,
    indexed: &IndexedData<'_>,
    raw: &RawData,
) -> Vec<CustomizationChoice> {
    let sample_swatch = matches!(
        opt_type,
        Some(OptionType::SkinColor | OptionType::HairColor)
    );
    let Some(raw_choices) = indexed.choices_by_option.get(&option_id) else {
        return Vec::new();
    };
    raw_choices
        .iter()
        .map(|ch| {
            let (materials, related_materials, geosets, related_geosets) =
                resolve_choice_elements(ch.id, indexed, raw);
            let shows_scalp =
                choice_shows_scalp(opt_type, model_id, &geosets, &related_geosets, raw);
            CustomizationChoice {
                id: ch.id,
                display_name: ch.name.clone(),
                requirement_id: ch.requirement_id,
                visibility_requirement_id: ch.visibility_requirement_id,
                swatch_colors: ch.swatch_colors,
                has_unsupported_effects: indexed.elements_by_choice.get(&ch.id).is_some_and(
                    |elements| {
                        elements
                            .iter()
                            .any(|element| element.has_unsupported_effects)
                    },
                ),
                materials,
                related_materials,
                geosets,
                related_geosets,
                skinned_models: resolve_choice_skinned_models(ch.id, indexed, raw),
                shows_scalp,
                sample_swatch,
                swatch_color_cache: Arc::new(OnceLock::new()),
            }
        })
        .collect()
}

fn choice_shows_scalp(
    opt_type: Option<OptionType>,
    model_id: u32,
    geosets: &[(u16, u16)],
    related_geosets: &[ChoiceGeoset],
    raw: &RawData,
) -> bool {
    opt_type == Some(OptionType::HairStyle)
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

fn build_hair_scalp_fallbacks(hair_geosets: &HashMap<(u32, u16, u16), bool>) -> HashMap<u32, u16> {
    let mut fallbacks = HashMap::new();
    let mut entries: Vec<_> = hair_geosets.iter().collect();
    entries.sort_by_key(|((model_id, geoset_type, geoset_id), _)| {
        (*model_id, *geoset_type, *geoset_id)
    });
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
    let Some(elements) = indexed.elements_by_choice.get(&choice_id) else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };
    let (materials, related_materials) = resolve_choice_materials(elements, raw);
    let (geosets, related_geosets) = resolve_choice_geosets(elements, raw);
    (materials, related_materials, geosets, related_geosets)
}

fn resolve_choice_materials(
    elements: &[&RawElement],
    raw: &RawData,
) -> (Vec<(u16, u32)>, Vec<ChoiceMaterial>) {
    let (related_materials, materials): (Vec<_>, Vec<_>) = elements
        .iter()
        .filter_map(|el| {
            let mat = raw.materials.get(&el.material_id)?;
            let &fdid = raw.texture_fdids.get(&mat.material_resources_id)?;
            Some((
                el.related_choice_id > 0,
                ChoiceMaterial {
                    related_choice_id: el.related_choice_id,
                    target_id: mat.texture_target_id,
                    fdid,
                },
            ))
        })
        .partition(|(is_related, _)| *is_related);
    let related_materials = related_materials
        .into_iter()
        .map(|(_, material)| material)
        .collect();
    let materials = materials
        .into_iter()
        .map(|(_, material)| (material.target_id, material.fdid))
        .collect();
    (materials, related_materials)
}

fn resolve_choice_geosets(
    elements: &[&RawElement],
    raw: &RawData,
) -> (Vec<(u16, u16)>, Vec<ChoiceGeoset>) {
    let (related_geosets, geosets): (Vec<_>, Vec<_>) = elements
        .iter()
        .filter_map(|el| {
            let geo = raw.geosets.get(&el.geoset_id)?;
            Some((
                el.related_choice_id > 0,
                ChoiceGeoset {
                    related_choice_id: el.related_choice_id,
                    geoset_type: geo.geoset_type,
                    geoset_id: geo.geoset_id,
                },
            ))
        })
        .partition(|(is_related, _)| *is_related);
    let related_geosets = related_geosets
        .into_iter()
        .map(|(_, geoset)| geoset)
        .collect();
    let geosets = geosets
        .into_iter()
        .map(|(_, geoset)| (geoset.geoset_type, geoset.geoset_id))
        .collect();
    (geosets, related_geosets)
}

fn resolve_choice_skinned_models(
    choice_id: u32,
    indexed: &IndexedData<'_>,
    raw: &RawData,
) -> Vec<ChoiceSkinnedModel> {
    let Some(elements) = indexed.elements_by_choice.get(&choice_id) else {
        return Vec::new();
    };
    elements
        .iter()
        .filter_map(|element| {
            let row = raw.skinned_models.get(&element.skinned_model_id)?;
            Some(ChoiceSkinnedModel {
                related_choice_id: element.related_choice_id,
                collection_fdid: row.collection_fdid,
                geoset_type: row.geoset_type,
                geoset_id: row.geoset_id,
            })
        })
        .collect()
}

// --- CSV parsing (manual, no csv crate) ---

pub(crate) struct RawData {
    pub(crate) chr_models: Vec<RawChrModel>,
    pub(crate) options: Vec<RawOption>,
    pub(crate) categories: HashMap<u32, RawCategory>,
    pub(crate) choices: Vec<RawChoice>,
    pub(crate) elements: Vec<RawElement>,
    pub(crate) materials: HashMap<u32, RawMaterial>,
    pub(crate) geosets: HashMap<u32, RawGeoset>,
    /// `ChrCustomizationSkinnedModel` rows by ID.
    pub(crate) skinned_models: HashMap<u32, RawSkinnedModel>,
    pub(crate) hair_geosets: HashMap<(u32, u16, u16), bool>,
    pub(crate) texture_fdids: HashMap<u32, u32>,
    pub(crate) race_models: RaceModels,
}

pub(crate) struct RawChrModel {
    pub(crate) id: u32,
    pub(crate) layout_id: u32,
    pub(crate) customize_scale: f32,
    pub(crate) camera_distance_offset: f32,
}
#[derive(Default)]
pub(crate) struct RawOption {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) chr_model_id: u32,
    pub(crate) category_id: u32,
    pub(crate) order_index: u32,
    pub(crate) ui_type: u32,
    pub(crate) requirement_id: u32,
}
#[derive(Default)]
pub(crate) struct RawCategory {
    pub(crate) name: String,
    pub(crate) order_index: u32,
    pub(crate) icon: u32,
    pub(crate) selected_icon: u32,
}
pub(crate) struct RawChoice {
    pub(crate) id: u32,
    pub(crate) option_id: u32,
    pub(crate) name: String,
    pub(crate) requirement_id: u32,
    pub(crate) visibility_requirement_id: u32,
    pub(crate) swatch_colors: [i32; 2],
    pub(crate) order_index: u32,
}
pub(crate) struct RawElement {
    pub(crate) choice_id: u32,
    pub(crate) related_choice_id: u32,
    pub(crate) geoset_id: u32,
    pub(crate) material_id: u32,
    pub(crate) skinned_model_id: u32,
    pub(crate) has_unsupported_effects: bool,
}
pub(crate) struct RawSkinnedModel {
    pub(crate) collection_fdid: u32,
    pub(crate) geoset_type: u16,
    pub(crate) geoset_id: u16,
}
pub(crate) struct RawMaterial {
    pub(crate) texture_target_id: u16,
    pub(crate) material_resources_id: u32,
}
pub(crate) struct RawGeoset {
    pub(crate) geoset_type: u16,
    pub(crate) geoset_id: u16,
}
