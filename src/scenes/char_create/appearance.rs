use std::collections::HashSet;

use game_engine::customization_data::{CustomizationDb, OptionType};
use shared::components::CharacterAppearance;

use super::{CharCreateState, clamp_appearance_field, mix_seed, pick_random_choice};

pub(super) fn randomize_appearance_with_seed(
    state: &mut CharCreateState,
    db: &CustomizationDb,
    seed: u64,
) {
    let (race, sex, class) = (
        state.selected_race,
        state.selected_sex,
        state.selected_class,
    );
    let mut seed = seed ^ ((race as u64) << 40) ^ ((sex as u64) << 32) ^ ((class as u64) << 24);
    let skin_color = random_skin_index(db, race, sex, class, &mut seed);
    let face = random_face_index(db, race, sex, class, skin_color, &mut seed);

    state.appearance = CharacterAppearance {
        sex,
        skin_color,
        face,
        eye_color: pick_random_choice(
            &mut seed,
            db.choice_count_for_class(race, sex, class, OptionType::EyeColor),
        ),
        hair_style: pick_random_choice(
            &mut seed,
            db.choice_count_for_class(race, sex, class, OptionType::HairStyle),
        ),
        hair_color: pick_random_choice(
            &mut seed,
            db.choice_count_for_class(race, sex, class, OptionType::HairColor),
        ),
        facial_style: pick_random_choice(
            &mut seed,
            db.choice_count_for_class(race, sex, class, OptionType::FacialHair),
        ),
        customization_choices: Vec::new(),
    };
    randomize_additional_choices(state, db, &mut seed);
    state.open_dropdown = None;
    state.selected_category = 0;
}

fn random_skin_index(db: &CustomizationDb, race: u8, sex: u8, class: u8, seed: &mut u64) -> u8 {
    let compatible = compatible_skin_indices(db, race, sex, class);
    if compatible.is_empty() {
        return pick_random_choice(
            seed,
            db.choice_count_for_class(race, sex, class, OptionType::SkinColor),
        );
    }
    *seed = mix_seed(*seed);
    compatible[(*seed % compatible.len() as u64) as usize]
}

pub(super) fn normalize_appearance(state: &mut CharCreateState, db: &CustomizationDb) {
    let (race, sex, class) = (
        state.selected_race,
        state.selected_sex,
        state.selected_class,
    );

    clamp_appearance_field(
        &mut state.appearance.skin_color,
        db.choice_count_for_class(race, sex, class, OptionType::SkinColor),
    );
    normalize_face_choice(state, db);
    clamp_appearance_field(
        &mut state.appearance.eye_color,
        db.choice_count_for_class(race, sex, class, OptionType::EyeColor),
    );
    clamp_appearance_field(
        &mut state.appearance.hair_style,
        db.choice_count_for_class(race, sex, class, OptionType::HairStyle),
    );
    clamp_appearance_field(
        &mut state.appearance.hair_color,
        db.choice_count_for_class(race, sex, class, OptionType::HairColor),
    );
    clamp_appearance_field(
        &mut state.appearance.facial_style,
        db.choice_count_for_class(race, sex, class, OptionType::FacialHair),
    );
    game_engine::appearance_options::normalize_additional_choices(
        db,
        race,
        sex,
        class,
        &mut state.appearance,
    );
}

pub(super) fn adjust_appearance(
    state: &mut CharCreateState,
    option_id: u32,
    delta: i8,
    db: &CustomizationDb,
) {
    let Some(option) = db.option_by_id(state.selected_race, state.selected_sex, option_id) else {
        return;
    };
    if option.option_type == OptionType::Face
        && game_engine::appearance_options::is_core_option(
            db,
            state.selected_race,
            state.selected_sex,
            option,
        )
    {
        cycle_face_choice(state, db, delta);
        state.open_dropdown = None;
        return;
    }
    let choices: Vec<_> = db
        .choices_for_option(
            state.selected_race,
            state.selected_sex,
            state.selected_class,
            option_id,
        )
        .into_iter()
        .filter(|choice| !choice.has_unsupported_effects)
        .collect();
    if choices.is_empty() {
        return;
    }
    let selected = game_engine::appearance_options::selected_choice(
        db,
        state.selected_race,
        state.selected_sex,
        state.selected_class,
        &state.appearance,
        option,
    );
    let current = selected
        .and_then(|selected| choices.iter().position(|choice| choice.id == selected.id))
        .unwrap_or(0);
    let next = (current as isize + delta as isize).rem_euclid(choices.len() as isize) as usize;
    select_choice(state, option_id, choices[next].id, db);
}

pub(super) fn select_choice(
    state: &mut CharCreateState,
    option_id: u32,
    choice_id: u32,
    db: &CustomizationDb,
) {
    if let Some(option) = db.option_by_id(state.selected_race, state.selected_sex, option_id)
        && option.option_type == OptionType::Face
        && game_engine::appearance_options::is_core_option(
            db,
            state.selected_race,
            state.selected_sex,
            option,
        )
    {
        let choices = db.choices_for_option(
            state.selected_race,
            state.selected_sex,
            state.selected_class,
            option_id,
        );
        let compatible = compatible_face_indices(
            db,
            state.selected_race,
            state.selected_sex,
            state.selected_class,
            state.appearance.skin_color,
        );
        if !choices.iter().enumerate().any(|(index, choice)| {
            choice.id == choice_id && compatible.iter().any(|&valid| usize::from(valid) == index)
        }) {
            state.error_text =
                Some("That face is unavailable for the selected skin color".to_owned());
            return;
        }
    }
    let result = game_engine::appearance_options::set_choice(
        db,
        state.selected_race,
        state.selected_sex,
        state.selected_class,
        &mut state.appearance,
        option_id,
        choice_id,
    );
    match result {
        Ok(()) => {
            normalize_face_choice(state, db);
            state.error_text = None;
            state.open_dropdown = None;
        }
        Err(error) => state.error_text = Some(error),
    }
}

fn randomize_additional_choices(state: &mut CharCreateState, db: &CustomizationDb, seed: &mut u64) {
    for option in db
        .options_for(state.selected_race, state.selected_sex)
        .into_iter()
        .flatten()
    {
        if game_engine::appearance_options::is_core_option(
            db,
            state.selected_race,
            state.selected_sex,
            option,
        ) {
            continue;
        }
        let choices: Vec<_> = db
            .choices_for_option(
                state.selected_race,
                state.selected_sex,
                state.selected_class,
                option.id,
            )
            .into_iter()
            .filter(|choice| !choice.has_unsupported_effects)
            .collect();
        if choices.is_empty() {
            continue;
        }
        *seed = mix_seed(*seed);
        let choice = choices[(*seed % choices.len() as u64) as usize];
        state.appearance.customization_choices.push(
            shared::components::CustomizationChoiceSelection {
                option_id: option.id,
                choice_id: choice.id,
            },
        );
    }
    state
        .appearance
        .customization_choices
        .sort_by_key(|selection| selection.option_id);
}

fn random_face_index(
    db: &CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    skin_color: u8,
    seed: &mut u64,
) -> u8 {
    let compatible = compatible_face_indices(db, race, sex, class, skin_color);
    if compatible.is_empty() {
        return 0;
    }
    *seed = mix_seed(*seed);
    compatible[(*seed % compatible.len() as u64) as usize]
}

fn normalize_face_choice(state: &mut CharCreateState, db: &CustomizationDb) {
    let compatible = compatible_face_indices(
        db,
        state.selected_race,
        state.selected_sex,
        state.selected_class,
        state.appearance.skin_color,
    );
    if compatible.is_empty() {
        state.appearance.face = 0;
        return;
    }
    if !compatible.contains(&state.appearance.face) {
        state.appearance.face = compatible[0];
    }
}

fn cycle_face_choice(state: &mut CharCreateState, db: &CustomizationDb, delta: i8) {
    let compatible = compatible_face_indices(
        db,
        state.selected_race,
        state.selected_sex,
        state.selected_class,
        state.appearance.skin_color,
    );
    if compatible.is_empty() {
        state.appearance.face = 0;
        return;
    }
    let current = compatible
        .iter()
        .position(|&index| index == state.appearance.face)
        .unwrap_or(0);
    let next = next_index(current, compatible.len(), delta);
    state.appearance.face = compatible[next];
}

pub(super) fn compatible_face_indices(
    db: &CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    skin_color: u8,
) -> Vec<u8> {
    let Some(selected_skin_id) = db
        .get_choice_for_class(race, sex, class, OptionType::SkinColor, skin_color)
        .map(|choice| choice.id)
    else {
        return Vec::new();
    };
    let skin_choice_ids = skin_choice_ids(db, race, sex, class);
    let face_count = db.choice_count_for_class(race, sex, class, OptionType::Face);

    (0..face_count)
        .filter(|&index| {
            face_matches_skin(
                db,
                race,
                sex,
                class,
                index,
                selected_skin_id,
                &skin_choice_ids,
            )
        })
        .collect()
}

fn compatible_skin_indices(db: &CustomizationDb, race: u8, sex: u8, class: u8) -> Vec<u8> {
    let skin_count = db.choice_count_for_class(race, sex, class, OptionType::SkinColor);
    if db.choice_count_for_class(race, sex, class, OptionType::Face) == 0 {
        return (0..skin_count).collect();
    }
    (0..skin_count)
        .filter(|&skin_color| !compatible_face_indices(db, race, sex, class, skin_color).is_empty())
        .collect()
}

fn face_matches_skin(
    db: &CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    face: u8,
    selected_skin_id: u32,
    skin_choice_ids: &HashSet<u32>,
) -> bool {
    let Some(choice) = db.get_choice_for_class(race, sex, class, OptionType::Face, face) else {
        return false;
    };
    let related_skin_ids = related_skin_ids(choice, skin_choice_ids);
    related_skin_ids.is_empty() || related_skin_ids.contains(&selected_skin_id)
}

fn related_skin_ids(
    choice: &game_engine::customization_data::CustomizationChoice,
    skin_choice_ids: &HashSet<u32>,
) -> HashSet<u32> {
    choice
        .related_materials
        .iter()
        .map(|material| material.related_choice_id)
        .chain(
            choice
                .related_geosets
                .iter()
                .map(|geoset| geoset.related_choice_id),
        )
        .filter(|choice_id| skin_choice_ids.contains(choice_id))
        .collect()
}

fn skin_choice_ids(db: &CustomizationDb, race: u8, sex: u8, class: u8) -> HashSet<u32> {
    let count = db.choice_count_for_class(race, sex, class, OptionType::SkinColor);
    (0..count)
        .filter_map(|index| {
            db.get_choice_for_class(race, sex, class, OptionType::SkinColor, index)
                .map(|choice| choice.id)
        })
        .collect()
}

fn next_index(current: usize, len: usize, delta: i8) -> usize {
    if delta > 0 {
        (current + 1) % len
    } else if current == 0 {
        len - 1
    } else {
        current - 1
    }
}
