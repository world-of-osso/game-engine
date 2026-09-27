//! Bevy-free character-creation state and action rules shared by the Bevy and Godot hosts.
//! The including module provides a sibling `deps` module exposing the customization catalog
//! (`CustomizationDb`, `CustomizationOption`, `CustomizationChoice`, `OptionType`),
//! `appearance_options`, `char_create_component` and `char_create_data`.

use std::time::{SystemTime, UNIX_EPOCH};

use shared::components::CharacterAppearance;
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
    randomize_appearance_with_seed(&mut state, db, fresh_random_seed());
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
        CharCreateAction::RandomizeName => {
            match names {
                Ok(catalog) => {
                    if let Some(name) = apply_random_name_with_seed(state, catalog, name, seed) {
                        effects.push(CharCreateEffect::SetNameText(name));
                    }
                }
                Err(_) => {
                    state.error_text = Some("Authored random names are unavailable".to_string())
                }
            }
            return effects;
        }
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
        CharCreateAction::SelectCategory(id) => {
            if db
                .options_for(state.selected_race, state.selected_sex)
                .into_iter()
                .flatten()
                .any(|option| option.category_id == id)
            {
                state.selected_category = id;
                state.open_dropdown = None;
            }
        }
        CharCreateAction::Camera(action) => state.camera_action = Some(action),
        CharCreateAction::CreateConfirm => {
            effects.extend(create_request(state, name).map(CharCreateEffect::SendCreate));
            effects.push(CharCreateEffect::FocusNameInput);
        }
    }
    appearance::normalize_appearance(state, db);
    effects
}

fn create_request(state: &mut CharCreateState, name: &str) -> Option<CreateCharacter> {
    if name.is_empty() {
        state.error_text = Some("Please enter a name".to_string());
        return None;
    }
    state.error_text = None;
    Some(CreateCharacter {
        name: name.to_string(),
        race: state.selected_race,
        class: state.selected_class,
        appearance: state.appearance.clone(),
    })
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
    if !race_can_be_class(race_id, state.selected_class) {
        state.selected_class = first_available_class(race_id);
    }
    randomize_appearance_with_seed(state, db, seed);
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
    }
}

pub fn apply_sex_toggle_with_seed(state: &mut CharCreateState, db: &CustomizationDb, seed: u64) {
    state.selected_sex = if state.selected_sex == MALE {
        FEMALE
    } else {
        MALE
    };
    randomize_appearance_with_seed(state, db, seed);
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

fn pick_random_choice(seed: &mut u64, count: u8) -> u8 {
    if count == 0 {
        return 0;
    }
    *seed = mix_seed(*seed);
    (*seed % count as u64) as u8
}
