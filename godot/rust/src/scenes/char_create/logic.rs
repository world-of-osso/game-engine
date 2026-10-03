//! Bevy-free character-creation state and action rules shared by the Bevy and Godot hosts.
//! The including module provides a sibling `deps` module exposing the customization catalog
//! (`CustomizationDb`, `CustomizationOption`, `CustomizationChoice`, `OptionType`),
//! `appearance_options`, `char_create_component` and `char_create_data`.

use std::time::{SystemTime, UNIX_EPOCH};

use shared::components::{CharacterAppearance, FormAppearance};
use shared::protocol::CreateCharacter;

use super::deps;
use deps::CustomizationDb;
use deps::char_create_component::{
    CameraControl, CharCreateAction, CharCreateMode, CharCreateUiState,
};
use deps::char_create_data::{CLASSES, first_available_class, race_can_be_class};

#[path = "appearance.rs"]
pub mod appearance;
#[path = "customization_view.rs"]
pub mod customization_view;
#[path = "name_catalog.rs"]
pub mod name_catalog;

pub use customization_view::build_ui_state;
pub use name_catalog::NameCatalog;

const MALE: u8 = 0;
const FEMALE: u8 = 1;
/// Dracthyr (52 Alliance, 70 Horde) create a visage form beside the dragon form.
pub fn has_visage_form(race: u8) -> bool {
    matches!(race, 52 | 70)
}

pub struct CharCreateState {
    pub selected_race: u8,
    pub selected_class: u8,
    pub selected_sex: u8,
    pub name: String,
    pub appearance: CharacterAppearance,
    pub mode: CharCreateMode,
    pub error_text: Option<String>,
    pub open_dropdown: Option<u32>,
    pub selected_category: u32,
    pub camera_action: Option<CameraControl>,
    /// The visage form is shown and edited. `appearance` always holds the form being
    /// edited and `appearance.visage` the other one; `create_request` restores the
    /// dragon form to `appearance` (Retail `SetViewingAlteredForm`).
    pub visage_active: bool,
}

impl CharCreateState {
    /// The race whose models and options the edited form uses: the visage form is
    /// ChrRaces.UnalteredVisualRaceID of Dracthyr 52 / 70 (75 / 76, ChrModel 127/128).
    pub fn customization_race(&self) -> u8 {
        match (self.visage_active, self.selected_race) {
            (true, 52) => 75,
            (true, 70) => 76,
            (_, race) => race,
        }
    }
}

impl Default for CharCreateState {
    fn default() -> Self {
        Self {
            selected_race: 1,
            selected_class: 1,
            selected_sex: MALE,
            name: String::new(),
            appearance: CharacterAppearance::default(),
            mode: CharCreateMode::RaceClass,
            error_text: None,
            open_dropdown: None,
            selected_category: 0,
            camera_action: None,
            visage_active: false,
        }
    }
}

/// Host work requested by a character-creation action.
#[derive(Debug)]
pub enum CharCreateEffect {
    ExitToCharSelect,
    SetNameText(String),
    SendCreate(CreateCharacter),
    FocusNameInput,
}

/// Default Human Warrior, optionally in a startup mode, with a random appearance.
pub fn initial_state(mode: Option<CharCreateMode>, db: &CustomizationDb) -> CharCreateState {
    let mut state = CharCreateState::default();
    if let Some(mode) = mode {
        state.mode = mode;
    }
    let seed = fresh_random_seed();
    randomize_appearance_with_seed(&mut state, db, seed);
    ensure_visage_form(&mut state, db, seed);
    state
}

/// Apply one authored action. `name` is the current name-input text and `names` the
/// authored name catalog (or why it could not be loaded).
pub fn reduce(
    state: &mut CharCreateState,
    action: CharCreateAction,
    db: &CustomizationDb,
    names: Result<&NameCatalog, &str>,
    name: &str,
    seed: u64,
) -> Vec<CharCreateEffect> {
    let mut effects = Vec::new();
    match action {
        CharCreateAction::SelectRace(id) => apply_race_change_with_seed(state, id, db, seed),
        CharCreateAction::SelectClass(id) => apply_class_change_with_seed(state, id, db, seed),
        CharCreateAction::SelectSex(sex) => {
            if sex <= 1 && sex != state.selected_sex {
                apply_sex_toggle_with_seed(state, db, seed);
            }
        }
        CharCreateAction::Randomize => randomize_appearance_with_seed(state, db, seed),
        CharCreateAction::SetForm(visage) => set_visage_active(state, visage),
        // Naming leaves the appearance untouched, so it skips normalization.
        CharCreateAction::RandomizeName => return randomize_name(state, names, name, seed),
        CharCreateAction::NextMode => state.mode = CharCreateMode::Customize,
        CharCreateAction::Back => {
            if state.mode == CharCreateMode::Customize {
                state.mode = CharCreateMode::RaceClass;
            } else {
                effects.push(CharCreateEffect::ExitToCharSelect);
            }
        }
        CharCreateAction::AdjustOption(id, delta) => {
            appearance::adjust_appearance(state, id, delta, db)
        }
        CharCreateAction::ToggleOption(id) => toggle_dropdown(state, id),
        CharCreateAction::SelectOptionChoice(id, choice) => {
            appearance::select_choice(state, id, choice, db)
        }
        CharCreateAction::SelectCategory(id) => select_category(state, id, db),
        CharCreateAction::Camera(action) => state.camera_action = Some(action),
        CharCreateAction::CreateConfirm => {
            effects.extend(create_request(state, name).map(CharCreateEffect::SendCreate));
            effects.push(CharCreateEffect::FocusNameInput);
        }
    }
    appearance::normalize_appearance(state, db);
    effects
}

fn randomize_name(
    state: &mut CharCreateState,
    names: Result<&NameCatalog, &str>,
    current: &str,
    seed: u64,
) -> Vec<CharCreateEffect> {
    let Ok(catalog) = names else {
        state.error_text = Some("Authored random names are unavailable".to_string());
        return Vec::new();
    };
    apply_random_name_with_seed(state, catalog, current, seed)
        .map(CharCreateEffect::SetNameText)
        .into_iter()
        .collect()
}

/// Only categories with offered options for the current race, body type and class are selectable.
fn select_category(state: &mut CharCreateState, id: u32, db: &CustomizationDb) {
    let has_options = customization_view::offered_options(state, db)
        .into_iter()
        .any(|option| option.category_id == id);
    if has_options {
        state.selected_category = id;
        state.open_dropdown = None;
    }
}

fn create_request(state: &mut CharCreateState, name: &str) -> Option<CreateCharacter> {
    if name.is_empty() {
        state.error_text = Some("Please enter a name".to_string());
        return None;
    }
    state.error_text = None;
    let mut appearance = state.appearance.clone();
    if state.visage_active {
        swap_forms(&mut appearance);
    }
    Some(CreateCharacter {
        name: name.to_string(),
        race: state.selected_race,
        class: state.selected_class,
        appearance,
    })
}

/// Show and edit the visage (`true`) or dragon form of a Dracthyr.
fn set_visage_active(state: &mut CharCreateState, visage: bool) {
    if !has_visage_form(state.selected_race) || state.visage_active == visage {
        return;
    }
    swap_forms(&mut state.appearance);
    state.visage_active = visage;
    state.open_dropdown = None;
    state.selected_category = 0;
}

/// Exchange the edited form's selections with the stored other form's.
fn swap_forms(appearance: &mut CharacterAppearance) {
    let other = appearance.visage.take().unwrap_or_default();
    let current = FormAppearance {
        skin_color: appearance.skin_color,
        face: appearance.face,
        eye_color: appearance.eye_color,
        hair_style: appearance.hair_style,
        hair_color: appearance.hair_color,
        facial_style: appearance.facial_style,
        customization_choices: std::mem::take(&mut appearance.customization_choices),
    };
    *appearance = CharacterAppearance {
        sex: appearance.sex,
        skin_color: other.skin_color,
        face: other.face,
        eye_color: other.eye_color,
        hair_style: other.hair_style,
        hair_color: other.hair_color,
        facial_style: other.facial_style,
        customization_choices: other.customization_choices,
        visage: Some(current),
    };
}

/// A Dracthyr gets a random other form when it has none; any other race has none.
fn ensure_visage_form(state: &mut CharCreateState, db: &CustomizationDb, seed: u64) {
    if !has_visage_form(state.selected_race) {
        state.appearance.visage = None;
        state.visage_active = false;
        return;
    }
    if state.appearance.visage.is_some() {
        return;
    }
    let mut other = CharCreateState {
        selected_race: state.selected_race,
        selected_class: state.selected_class,
        selected_sex: state.selected_sex,
        visage_active: !state.visage_active,
        ..CharCreateState::default()
    };
    randomize_appearance_with_seed(&mut other, db, seed.rotate_left(17));
    swap_forms(&mut other.appearance);
    state.appearance.visage = other.appearance.visage;
}

/// Server creation result: success leaves creation; failure shows the server error.
pub fn receive_create_result(
    state: &mut CharCreateState,
    success: bool,
    error: Option<String>,
) -> Option<CharCreateEffect> {
    if success {
        return Some(CharCreateEffect::ExitToCharSelect);
    }
    state.error_text = Some(error.unwrap_or_else(|| "Creation failed".to_string()));
    None
}

pub fn apply_race_change_with_seed(
    state: &mut CharCreateState,
    race_id: u8,
    db: &CustomizationDb,
    seed: u64,
) {
    state.selected_race = race_id;
    state.visage_active = false;
    state.appearance.visage = None;
    if !race_can_be_class(race_id, state.selected_class) {
        state.selected_class = first_available_class(race_id);
    }
    randomize_appearance_with_seed(state, db, seed);
    ensure_visage_form(state, db, seed);
}

pub fn apply_class_change_with_seed(
    state: &mut CharCreateState,
    class_id: u8,
    db: &CustomizationDb,
    seed: u64,
) {
    if race_can_be_class(state.selected_race, class_id) {
        state.selected_class = class_id;
        randomize_appearance_with_seed(state, db, seed);
        ensure_visage_form(state, db, seed);
    }
}

pub fn apply_sex_toggle_with_seed(state: &mut CharCreateState, db: &CustomizationDb, seed: u64) {
    state.selected_sex = if state.selected_sex == MALE {
        FEMALE
    } else {
        MALE
    };
    state.appearance.visage = None;
    randomize_appearance_with_seed(state, db, seed);
    ensure_visage_form(state, db, seed);
}

/// Pick an authored name different from `current`; returns it for the name input.
pub fn apply_random_name_with_seed(
    state: &mut CharCreateState,
    catalog: &NameCatalog,
    current: &str,
    seed: u64,
) -> Option<String> {
    let Some(name) = catalog.pick_name(state.selected_race, state.selected_sex, current, seed)
    else {
        state.error_text = Some("No authored names for this race and body type".to_string());
        return None;
    };
    state.name = name.to_string();
    state.error_text = None;
    Some(state.name.clone())
}

pub fn randomize_appearance_with_seed(
    state: &mut CharCreateState,
    db: &CustomizationDb,
    seed: u64,
) {
    appearance::randomize_appearance_with_seed(state, db, seed);
}

pub fn clamp_appearance_field(value: &mut u8, count: u8) {
    if count == 0 {
        *value = 0;
    } else if *value >= count {
        *value = count - 1;
    }
}

fn toggle_dropdown(state: &mut CharCreateState, field: u32) {
    state.open_dropdown = if state.open_dropdown == Some(field) {
        None
    } else {
        Some(field)
    };
}

pub fn build_class_availability(race: u8) -> Vec<(u8, &'static str, u32, bool)> {
    CLASSES
        .iter()
        .map(|c| (c.id, c.name, c.icon_fdid, race_can_be_class(race, c.id)))
        .collect()
}

/// UI state for the current creation state plus host-owned name/viewport facts.
pub fn ui_state(
    state: &CharCreateState,
    db: &CustomizationDb,
    names: Result<&NameCatalog, &str>,
    name_input_focused: bool,
    viewport: (u32, u32),
) -> CharCreateUiState {
    let mut ui = build_ui_state(state, db);
    ui.random_name_available =
        names.is_ok_and(|catalog| catalog.has_names(state.selected_race, state.selected_sex));
    ui.name_input_focused = name_input_focused;
    (ui.viewport_width, ui.viewport_height) = viewport;
    ui
}

const CHAR_CREATE_RANDOM_SEED_MIX: u64 = 0x9e37_79b9_7f4a_7c15;
const CHAR_CREATE_RANDOM_SEED_MUL_1: u64 = 0xbf58_476d_1ce4_e5b9;
const CHAR_CREATE_RANDOM_SEED_MUL_2: u64 = 0x94d0_49bb_1331_11eb;

pub fn fresh_random_seed() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos() as u64,
        Err(_) => CHAR_CREATE_RANDOM_SEED_MIX,
    }
}

fn mix_seed(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(CHAR_CREATE_RANDOM_SEED_MIX);
    z = (z ^ (z >> 30)).wrapping_mul(CHAR_CREATE_RANDOM_SEED_MUL_1);
    z = (z ^ (z >> 27)).wrapping_mul(CHAR_CREATE_RANDOM_SEED_MUL_2);
    z ^ (z >> 31)
}
