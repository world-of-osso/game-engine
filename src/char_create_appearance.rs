use std::collections::HashSet;

use game_engine::customization_data::{CustomizationDb, OptionType};
use shared::components::CharacterAppearance;

use super::{
    AppearanceField, CharCreateState, clamp_appearance_field, mix_seed, pick_random_choice,
};

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
    };
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
}

pub(super) fn adjust_appearance(
    state: &mut CharCreateState,
    field: AppearanceField,
    delta: i8,
    db: &CustomizationDb,
) {
    match field {
        AppearanceField::SkinColor => {
            let max = choice_count(state, db, OptionType::SkinColor);
            cycle_choice(&mut state.appearance.skin_color, max, delta);
            normalize_face_choice(state, db);
        }
        AppearanceField::Face => cycle_face_choice(state, db, delta),
        AppearanceField::HairStyle => {
            let max = choice_count(state, db, OptionType::HairStyle);
            cycle_choice(&mut state.appearance.hair_style, max, delta);
        }
        AppearanceField::HairColor => {
            let max = choice_count(state, db, OptionType::HairColor);
            cycle_choice(&mut state.appearance.hair_color, max, delta);
        }
        AppearanceField::FacialStyle => {
            let max = choice_count(state, db, OptionType::FacialHair);
            cycle_choice(&mut state.appearance.facial_style, max, delta);
        }
    }
}

pub(super) fn select_choice(
    state: &mut CharCreateState,
    field: AppearanceField,
    idx: u8,
    db: &CustomizationDb,
) {
    match field {
        AppearanceField::SkinColor => {
            state.appearance.skin_color = idx;
            normalize_face_choice(state, db);
        }
        AppearanceField::Face => state.appearance.face = idx,
        AppearanceField::HairStyle => state.appearance.hair_style = idx,
        AppearanceField::HairColor => state.appearance.hair_color = idx,
        AppearanceField::FacialStyle => state.appearance.facial_style = idx,
    }
    state.open_dropdown = None;
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

fn compatible_face_indices(
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

fn choice_count(state: &CharCreateState, db: &CustomizationDb, opt_type: OptionType) -> u8 {
    db.choice_count_for_class(
        state.selected_race,
        state.selected_sex,
        state.selected_class,
        opt_type,
    )
}

fn cycle_choice(value: &mut u8, count: u8, delta: i8) {
    if count == 0 {
        return;
    }
    let current = (*value).min(count - 1) as usize;
    *value = next_index(current, count as usize, delta) as u8;
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
