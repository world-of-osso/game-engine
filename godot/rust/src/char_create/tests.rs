//! Shared creation rules (`src/scenes/char_create/logic.rs`) over the real local catalog.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::OnceLock,
};

use game_engine_core::{
    customization_data::{CustomizationDb, OptionType},
    npc_appearance_assets::load_customization_db,
};
use game_engine_ui_model::char_create_component::{CharCreateAction, CustomizationOptionUi};
use game_engine_ui_model::char_create_data::race_can_be_class;
use shared::components::CharacterAppearance;

use super::{CharCreateState, build_ui_state, reduce};

#[path = "forever_flow_tests.rs"]
mod forever_flow_tests;

#[test]
fn skyborne_race_selection_uses_default_class_and_only_player_options() {
    for (race, class) in [(95, 8), (96, 7)] {
        for sex in [0, 1] {
            let mut selected = state_with_race(HUMAN, sex, WARRIOR);
            super::apply_race_change_with_seed(&mut selected, race, db(), 1);
            assert_eq!(selected.selected_class, class);
            let raw = db().options_for(race, sex).unwrap();
            assert_eq!(raw.len(), 18 + usize::from(sex));
            let eye_style = raw
                .iter()
                .find(|option| option.display_name == "Eye Style")
                .unwrap();
            assert!(
                db().offered_choices(race, sex, class, eye_style.id)
                    .is_empty()
            );
            let rows = offered_rows(&selected);
            assert!(!rows.iter().any(|row| row.id == eye_style.id));
            assert_eq!(rows.len(), raw.len() - 1);
        }
    }
}

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
        visage_active: state.visage_active,
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

fn appearance(
    sex: u8,
    [skin, face, eye, hair_style, hair_color, facial]: [u8; 6],
) -> CharacterAppearance {
    CharacterAppearance {
        sex,
        skin_color: skin,
        face,
        eye_color: eye,
        hair_style,
        hair_color,
        facial_style: facial,
        customization_choices: Vec::new(),
        visage: None,
    }
}

fn core_choice_ids(
    race: u8,
    class: u8,
    appearance: &CharacterAppearance,
) -> Vec<(OptionType, u32)> {
    [
        OptionType::SkinColor,
        OptionType::Face,
        OptionType::EyeColor,
        OptionType::HairStyle,
        OptionType::HairColor,
        OptionType::FacialHair,
    ]
    .into_iter()
    .filter_map(|kind| {
        let index = match kind {
            OptionType::SkinColor => appearance.skin_color,
            OptionType::Face => appearance.face,
            OptionType::EyeColor => appearance.eye_color,
            OptionType::HairStyle => appearance.hair_style,
            OptionType::HairColor => appearance.hair_color,
            _ => appearance.facial_style,
        };
        db().get_choice_for_class(race, appearance.sex, class, kind, index)
            .map(|choice| (kind, choice.id))
    })
    .collect()
}

/// Dev-server roster appearances (account admin: Theron, Elara; fb_camera: Fbcamera, all
/// Human Warriors) keep resolving to the choice IDs they had before requirement filtering:
/// positions in the authored (OrderIndex, ID) choice list of each core option.
#[test]
fn saved_character_appearances_resolve_to_their_original_choice_ids() {
    use OptionType::*;
    let theron = appearance(0, [0, 1, 0, 1, 2, 1]);
    assert_eq!(
        core_choice_ids(HUMAN, WARRIOR, &theron),
        [
            (SkinColor, 1),
            (Face, 21),
            (EyeColor, 4126),
            (HairStyle, 45),
            (HairColor, 63),
            (FacialHair, 77)
        ]
    );
    let elara = appearance(1, [2, 3, 0, 2, 4, 0]);
    assert_eq!(
        core_choice_ids(HUMAN, WARRIOR, &elara),
        [
            (SkinColor, 87),
            (Face, 105),
            (EyeColor, 4150),
            (HairStyle, 134),
            (HairColor, 160)
        ]
    );
    let fbcamera = appearance(0, [0; 6]);
    assert_eq!(
        core_choice_ids(HUMAN, WARRIOR, &fbcamera),
        [
            (SkinColor, 1),
            (Face, 20),
            (EyeColor, 4126),
            (HairStyle, 44),
            (HairColor, 61),
            (FacialHair, 76)
        ]
    );
}

/// Hidden choices keep their stored positions: Human male skin index 22 is still choice
/// 4978 and index 12 still the Death Knight skin 13, which a warrior is not offered.
#[test]
fn filtering_changes_offers_but_not_stored_skin_indices() {
    let mut warrior = state(HUMAN, 0, WARRIOR);
    reduce(
        &mut warrior,
        CharCreateAction::SelectOptionChoice(HUMAN_MALE_SKIN_COLOR, 4978),
        db(),
        Err("no names"),
        "",
        5,
    );
    assert_eq!(warrior.appearance.skin_color, 22);
    let death_knight_skin = db()
        .get_choice_for_class(HUMAN, 0, WARRIOR, OptionType::SkinColor, 12)
        .unwrap();
    assert_eq!(death_knight_skin.id, 13);
    // Night Elf/Blood Elf faces keep the original per-class split at their old indices.
    let night_elf_warrior_face = db()
        .get_choice_for_class(4, 0, WARRIOR, OptionType::Face, 0)
        .unwrap();
    assert_eq!(night_elf_warrior_face.requirement_id, 142);
}

fn offered_choice_count(race: u8, class: u8, label: &str) -> usize {
    offered_rows(&state(race, 0, class))
        .iter()
        .find(|row| row.label == label)
        .map_or(0, |row| row.choices.len())
}

/// Night Elf male Horns 47, Blindfold 48 and Tattoo 376 choices carry
/// ChrCustomizationReq ClassMask 2048 (Demon Hunter): six or more extra choices each.
#[test]
fn demon_hunter_offers_its_class_horns_blindfolds_and_tattoos() {
    const NIGHT_ELF: u8 = 4;
    const DEMON_HUNTER: u8 = 12;
    for label in ["Horns", "Blindfold", "Tattoo"] {
        let hunter = offered_choice_count(NIGHT_ELF, DEMON_HUNTER, label);
        let warrior = offered_choice_count(NIGHT_ELF, WARRIOR, label);
        assert!(
            hunter >= warrior + 6,
            "{label}: DH {hunter}, warrior {warrior}"
        );
    }
}

/// Dracthyr (52) customize the dragon form, ChrModel 89.
#[test]
fn dracthyr_evoker_offers_dragon_form_options() {
    for label in ["Horns", "Tail", "Body Size", "Snout"] {
        assert!(offered_choice_count(52, 13, label) > 1, "{label}");
    }
}

fn act(state: &mut CharCreateState, action: CharCreateAction) -> Vec<super::CharCreateEffect> {
    reduce(state, action, db(), Err("no names"), "Scalesong", 5)
}

/// A Dracthyr gets both forms: the dragon form is edited first, the visage form
/// (ChrRaces 75) is stored beside it, `SetForm` swaps which one is edited and
/// previewed, and the create request always carries the dragon form first.
#[test]
fn dracthyr_creates_a_visage_form_beside_the_dragon_form() {
    let mut state = CharCreateState::default();
    act(&mut state, CharCreateAction::SelectRace(52));
    assert_eq!(state.customization_race(), 52);
    let dragon = state.appearance.clone();
    let visage = dragon.visage.clone().expect("Dracthyr visage form");
    let visage_options = db().options_for(75, state.selected_sex).unwrap();
    let visage_choices: HashSet<u32> = visage_options
        .iter()
        .flat_map(|option| option.choices.iter().map(|choice| choice.id))
        .collect();
    assert!(
        visage
            .customization_choices
            .iter()
            .all(|selection| visage_choices.contains(&selection.choice_id)),
        "visage choices come from race 75 options"
    );

    act(&mut state, CharCreateAction::SetForm(true));
    assert_eq!(state.customization_race(), 75);
    assert_eq!(
        state.appearance.customization_choices,
        visage.customization_choices
    );
    assert_eq!(
        state
            .appearance
            .visage
            .as_ref()
            .unwrap()
            .customization_choices,
        dragon.customization_choices
    );
    let ui = build_ui_state(&state, db());
    assert_eq!(ui.altered_form, Some(true));

    let effects = act(&mut state, CharCreateAction::CreateConfirm);
    let request = effects
        .into_iter()
        .find_map(|effect| match effect {
            super::CharCreateEffect::SendCreate(request) => Some(request),
            _ => None,
        })
        .unwrap();
    assert_eq!(request.race, 52);
    assert_eq!(
        request.appearance.customization_choices,
        dragon.customization_choices
    );
    assert_eq!(
        request.appearance.visage.unwrap().customization_choices,
        visage.customization_choices
    );

    act(&mut state, CharCreateAction::SelectRace(1));
    assert!(state.appearance.visage.is_none());
    assert_eq!(build_ui_state(&state, db()).altered_form, None);
}

/// The visage form edits ChrRaces 75 (Alliance) / 76 (Horde), the
/// UnalteredVisualRaceID of 52 / 70. Neither has a PlayableRaceBit, so its
/// options' ChrCustomizationReq rows are checked against the Dracthyr race,
/// as TrinityCore registers alt-form options under the parent race.
#[test]
fn dracthyr_visage_form_offers_its_customization_options() {
    for (race, visage_race) in [(52, 75), (70, 76)] {
        let mut state = CharCreateState::default();
        act(&mut state, CharCreateAction::SelectRace(race));
        act(&mut state, CharCreateAction::SetForm(true));
        assert_eq!(state.customization_race(), visage_race);
        let labels: Vec<String> = offered_rows(&state)
            .iter()
            .map(|row| row.label.clone())
            .collect();
        for label in ["Face", "Skin Color", "Hair Style"] {
            assert!(labels.iter().any(|l| l == label), "race {race}: {labels:?}");
        }
    }
}
