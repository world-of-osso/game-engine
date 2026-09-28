//! Shared creation rules (`src/scenes/char_create/logic.rs`) over the real local catalog.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::OnceLock,
};

use game_engine_core::{
    customization_data::CustomizationDb, npc_appearance_assets::load_customization_db,
};
use game_engine_ui_model::char_create_component::{CharCreateAction, CustomizationOptionUi};
use game_engine_ui_model::char_create_data::race_can_be_class;

use super::{CharCreateState, build_ui_state, reduce};

const HUMAN: u8 = 1;
const WARRIOR: u8 = 1;
const DEATH_KNIGHT: u8 = 6;
const HUMAN_MALE_SKIN_COLOR: u32 = 9;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn db() -> &'static CustomizationDb {
    static DB: OnceLock<CustomizationDb> = OnceLock::new();
    DB.get_or_init(|| load_customization_db(&data_root()).expect("local customization catalog"))
}

fn state(race: u8, sex: u8, class: u8) -> CharCreateState {
    let mut state = CharCreateState {
        selected_race: race,
        selected_sex: sex,
        selected_class: class,
        ..CharCreateState::default()
    };
    super::randomize_appearance_with_seed(&mut state, db(), 1);
    state
}

/// Every offered row across all categories, as the Customize screen lists them.
fn offered_rows(state: &CharCreateState) -> Vec<CustomizationOptionUi> {
    let mut probe = CharCreateState {
        appearance: state.appearance.clone(),
        selected_race: state.selected_race,
        selected_sex: state.selected_sex,
        selected_class: state.selected_class,
        ..CharCreateState::default()
    };
    let categories = build_ui_state(&probe, db()).categories;
    categories
        .into_iter()
        .flat_map(|category| {
            probe.selected_category = category.id;
            build_ui_state(&probe, db()).options
        })
        .collect()
}

fn offered_choice_ids(state: &CharCreateState, option_id: u32) -> Vec<u32> {
    offered_rows(state)
        .into_iter()
        .find(|row| row.id == option_id)
        .map(|row| row.choices.iter().map(|choice| choice.id).collect())
        .unwrap_or_default()
}

/// ChrCustomizationReq rows: ID -> (ReqType, ClassMask), read independently of the catalog.
fn requirement_rows() -> &'static HashMap<u32, (u32, i32)> {
    static ROWS: OnceLock<HashMap<u32, (u32, i32)>> = OnceLock::new();
    ROWS.get_or_init(|| {
        let mut reader = csv::Reader::from_path(data_root().join("ChrCustomizationReq.csv"))
            .expect("ChrCustomizationReq.csv");
        let headers = reader.headers().unwrap().clone();
        let column = |name: &str| headers.iter().position(|header| header == name).unwrap();
        let (id, req_type, class_mask) = (column("ID"), column("ReqType"), column("ClassMask"));
        reader
            .records()
            .map(|record| {
                let record = record.unwrap();
                (
                    record[id].parse().unwrap(),
                    (
                        record[req_type].parse().unwrap(),
                        record[class_mask].parse().unwrap(),
                    ),
                )
            })
            .collect()
    })
}

fn authored_requirement(race: u8, sex: u8, choice_id: u32) -> u32 {
    db().choice_by_id(race, sex, choice_id)
        .expect("offered choice is authored for the model")
        .requirement_id
}

#[test]
fn human_male_warrior_is_offered_only_player_skin_colors() {
    let warrior = state(HUMAN, 0, WARRIOR);
    assert_eq!(
        offered_choice_ids(&warrior, HUMAN_MALE_SKIN_COLOR),
        [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 4957, 4958, 4972, 4975, 4978, 4979
        ]
    );
    // Req 53 (ClassMask 32) skins belong to Death Knights; NPC (ReqType 2) and
    // transmog (ReqType 4) skins 11, 12, 16, 17 and 18 are never player choices.
    let death_knight = state(HUMAN, 0, DEATH_KNIGHT);
    assert_eq!(
        offered_choice_ids(&death_knight, HUMAN_MALE_SKIN_COLOR),
        [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 13, 14, 15, 4957, 4958, 4972, 4975, 4978, 4979
        ]
    );
}

#[test]
fn warrior_cannot_select_a_death_knight_skin() {
    let mut warrior = state(HUMAN, 0, WARRIOR);
    let before = warrior.appearance.clone();
    reduce(
        &mut warrior,
        CharCreateAction::SelectOptionChoice(HUMAN_MALE_SKIN_COLOR, 13),
        db(),
        Err("no names"),
        "",
        7,
    );
    assert_eq!(warrior.appearance, before);
    assert!(warrior.error_text.is_some());
}

/// Races and body types whose skin swatches describe the dominant body colour.
const SWEEP_RACES: [u8; 6] = [1, 2, 3, 4, 10, 22];

fn sweep_states() -> Vec<CharCreateState> {
    SWEEP_RACES
        .into_iter()
        .flat_map(|race| [0, 1].map(|sex| (race, sex)))
        .flat_map(|(race, sex)| {
            (1..=13)
                .filter(move |&class| race_can_be_class(race, class))
                .map(move |class| state(race, sex, class))
        })
        .collect()
}

#[test]
fn every_offered_choice_meets_its_player_class_requirement() {
    for state in sweep_states() {
        for row in offered_rows(&state) {
            for choice in &row.choices {
                let requirement =
                    authored_requirement(state.selected_race, state.selected_sex, choice.id);
                if requirement == 0 {
                    continue;
                }
                let (req_type, class_mask) = requirement_rows()[&requirement];
                let class_bit = 1i32 << (state.selected_class - 1);
                assert!(
                    req_type & 1 == 1 && (class_mask == 0 || class_mask & class_bit != 0),
                    "race {} sex {} class {} option {} offers choice {} with requirement {requirement} (type {req_type}, classes {class_mask})",
                    state.selected_race,
                    state.selected_sex,
                    state.selected_class,
                    row.label,
                    choice.id,
                );
            }
        }
    }
}

/// Every selected choice's ChrCustomizationReqChoice groups are met by the selection.
fn assert_required_choices_met(state: &CharCreateState, context: &str) {
    let selected = super::appearance::selected_option_choices(state, db());
    let ids: HashSet<u32> = selected.iter().map(|(_, choice)| choice.id).collect();
    for (option_id, choice) in &selected {
        for group in db().required_choices(choice) {
            assert!(
                group.choice_ids.iter().any(|id| ids.contains(id)),
                "{context}: option {option_id} choice {} needs option {} in {:?}",
                choice.id,
                group.option_id,
                group.choice_ids
            );
        }
    }
}

#[test]
fn randomize_only_produces_offered_combinations_that_meet_required_choices() {
    for state in sweep_states() {
        let (race, sex, class) = (
            state.selected_race,
            state.selected_sex,
            state.selected_class,
        );
        for seed in 0..48 {
            let mut randomized = state_with_race(race, sex, class);
            reduce(
                &mut randomized,
                CharCreateAction::Randomize,
                db(),
                Err("no names"),
                "",
                seed,
            );
            let context = format!("race {race} sex {sex} class {class} seed {seed}");
            assert_required_choices_met(&randomized, &context);
            let rows = offered_rows(&randomized);
            let offered: HashSet<(u32, u32)> = rows
                .iter()
                .flat_map(|row| {
                    row.choices
                        .iter()
                        .filter(|choice| choice.enabled)
                        .map(|choice| (row.id, choice.id))
                })
                .collect();
            for row in rows {
                assert!(
                    offered.contains(&(row.id, row.selected_choice_id)),
                    "{context}: {} selects unoffered or disabled choice {}",
                    row.label,
                    row.selected_choice_id
                );
            }
        }
    }
}

/// Selecting any offered choice keeps it and repairs the other options into a valid combination.
#[test]
fn selecting_any_offered_choice_keeps_it_and_meets_required_choices() {
    for state in sweep_states() {
        for row in offered_rows(&state) {
            for choice in row.choices.iter().filter(|choice| choice.enabled) {
                let mut selected = state_with_race(
                    state.selected_race,
                    state.selected_sex,
                    state.selected_class,
                );
                selected.appearance = state.appearance.clone();
                reduce(
                    &mut selected,
                    CharCreateAction::SelectOptionChoice(row.id, choice.id),
                    db(),
                    Err("no names"),
                    "",
                    3,
                );
                let context = format!(
                    "race {} sex {} class {} select {} = {}",
                    state.selected_race,
                    state.selected_sex,
                    state.selected_class,
                    row.label,
                    choice.id
                );
                let kept = offered_rows(&selected)
                    .into_iter()
                    .find(|after| after.id == row.id)
                    .map(|after| after.selected_choice_id);
                assert_eq!(kept, Some(choice.id), "{context}");
                assert_required_choices_met(&selected, &context);
            }
        }
    }
}

/// The reported combination: Human male skin 4978 (tan swatch) with face 27 (choice 15430).
#[test]
fn reported_skin_and_face_27_select_together() {
    let mut warrior = state(HUMAN, 0, WARRIOR);
    for (option, choice) in [(HUMAN_MALE_SKIN_COLOR, 4978), (10, 15430)] {
        reduce(
            &mut warrior,
            CharCreateAction::SelectOptionChoice(option, choice),
            db(),
            Err("no names"),
            "",
            5,
        );
    }
    let rows = offered_rows(&warrior);
    let selected = |option_id| rows.iter().find(|row| row.id == option_id).unwrap();
    assert_eq!(selected(HUMAN_MALE_SKIN_COLOR).selected_choice_id, 4978);
    let face = selected(10);
    assert_eq!(face.selected_choice_id, 15430);
    assert_eq!(
        face.choices
            .iter()
            .position(|choice| choice.id == 15430)
            .map(|index| index + 1),
        Some(27)
    );
    // A legacy face requires a legacy skin, so choosing it changes the skin color.
    reduce(
        &mut warrior,
        CharCreateAction::SelectOptionChoice(10, 20),
        db(),
        Err("no names"),
        "",
        5,
    );
    let rows = offered_rows(&warrior);
    let skin = rows
        .iter()
        .find(|row| row.id == HUMAN_MALE_SKIN_COLOR)
        .unwrap();
    assert_eq!(skin.selected_choice_id, 1);
}

fn state_with_race(race: u8, sex: u8, class: u8) -> CharCreateState {
    CharCreateState {
        selected_race: race,
        selected_sex: sex,
        selected_class: class,
        ..CharCreateState::default()
    }
}
