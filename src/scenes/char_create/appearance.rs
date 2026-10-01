//! Appearance selection rules: every choice comes from the catalog's offered list and the
//! combined selection satisfies each choice's ChrCustomizationReqChoice groups. Picks are
//! stored by choice ID through `set_choice`, which maps core selectors to their stored index.

use std::collections::HashSet;

use super::deps::{
    CustomizationChoice, CustomizationDb, OptionType, RequiredChoices, appearance_options,
};
use shared::components::CharacterAppearance;

use super::{CharCreateState, clamp_appearance_field, mix_seed};

const CORE_SELECTORS: [OptionType; 6] = [
    OptionType::SkinColor,
    OptionType::Face,
    OptionType::EyeColor,
    OptionType::HairStyle,
    OptionType::HairColor,
    OptionType::FacialHair,
];

pub fn randomize_appearance_with_seed(
    state: &mut CharCreateState,
    db: &CustomizationDb,
    seed: u64,
) {
    let (race, sex, class) = (
        state.customization_race(),
        state.selected_sex,
        state.selected_class,
    );
    let mut seed = seed ^ ((race as u64) << 40) ^ ((sex as u64) << 32) ^ ((class as u64) << 24);
    // Only the edited form is randomized; the other form is kept.
    state.appearance = CharacterAppearance {
        sex,
        visage: state.appearance.visage.take(),
        ..CharacterAppearance::default()
    };
    for option in db.options_for(race, sex).into_iter().flatten() {
        let choices = selectable_choices(state, db, option.id);
        if choices.is_empty() {
            continue;
        }
        seed = mix_seed(seed);
        let choice = choices[(seed % choices.len() as u64) as usize];
        apply_choice(state, db, option.id, choice.id);
    }
    repair_required_choices(state, db, None);
    state.open_dropdown = None;
    state.selected_category = 0;
}

pub fn normalize_appearance(state: &mut CharCreateState, db: &CustomizationDb) {
    let (race, sex, class) = (
        state.customization_race(),
        state.selected_sex,
        state.selected_class,
    );
    for option_type in CORE_SELECTORS {
        let count = db.choice_count_for_class(race, sex, class, option_type);
        clamp_appearance_field(core_field(&mut state.appearance, option_type), count);
    }
    appearance_options::normalize_additional_choices(db, race, sex, class, &mut state.appearance);
    replace_unoffered_choices(state, db);
    repair_required_choices(state, db, None);
}

/// Selections outside the offered list (NPC, other-class or locked) become the first
/// offered choice. Only what is selected changes; stored indices keep their meaning.
fn replace_unoffered_choices(state: &mut CharCreateState, db: &CustomizationDb) {
    for (option_id, choice) in selected_option_choices(state, db) {
        let offered = selectable_choices(state, db, option_id);
        if let Some(first) = offered.first()
            && !offered.iter().any(|candidate| candidate.id == choice.id)
        {
            apply_choice(state, db, option_id, first.id);
        }
    }
}

fn core_field(appearance: &mut CharacterAppearance, option_type: OptionType) -> &mut u8 {
    match option_type {
        OptionType::SkinColor => &mut appearance.skin_color,
        OptionType::Face => &mut appearance.face,
        OptionType::EyeColor => &mut appearance.eye_color,
        OptionType::HairStyle => &mut appearance.hair_style,
        OptionType::HairColor => &mut appearance.hair_color,
        _ => &mut appearance.facial_style,
    }
}

pub fn adjust_appearance(
    state: &mut CharCreateState,
    option_id: u32,
    delta: i8,
    db: &CustomizationDb,
) {
    let Some(option) = db.option_by_id(state.customization_race(), state.selected_sex, option_id)
    else {
        return;
    };
    let choices = selectable_choices(state, db, option_id);
    if choices.is_empty() {
        return;
    }
    let selected = appearance_options::selected_choice(
        db,
        state.customization_race(),
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

/// Select `choice_id`, then change other options as its required choices demand
/// (for example the face after a skin color, or a skin color after a face).
pub fn select_choice(
    state: &mut CharCreateState,
    option_id: u32,
    choice_id: u32,
    db: &CustomizationDb,
) {
    if !selectable_choices(state, db, option_id)
        .iter()
        .any(|choice| choice.id == choice_id)
    {
        state.error_text = Some(format!(
            "Choice {choice_id} is not available for this character"
        ));
        return;
    }
    let result = appearance_options::set_choice(
        db,
        state.customization_race(),
        state.selected_sex,
        state.selected_class,
        &mut state.appearance,
        option_id,
        choice_id,
    );
    match result {
        Ok(()) => {
            repair_required_choices(state, db, Some(option_id));
            state.error_text = None;
            state.open_dropdown = None;
        }
        Err(error) => state.error_text = Some(error),
    }
}

fn selectable_choices<'a>(
    state: &CharCreateState,
    db: &'a CustomizationDb,
    option_id: u32,
) -> Vec<&'a CustomizationChoice> {
    db.offered_choices(
        state.customization_race(),
        state.selected_sex,
        state.selected_class,
        option_id,
    )
    .into_iter()
    .filter(|choice| appearance_options::choice_can_render(choice))
    .collect()
}

fn apply_choice(state: &mut CharCreateState, db: &CustomizationDb, option_id: u32, choice_id: u32) {
    appearance_options::set_choice(
        db,
        state.customization_race(),
        state.selected_sex,
        state.selected_class,
        &mut state.appearance,
        option_id,
        choice_id,
    )
    .expect("selectable choices are available and renderable");
}

/// (option ID, selected choice) for every option with a selection.
pub fn selected_option_choices<'a>(
    state: &CharCreateState,
    db: &'a CustomizationDb,
) -> Vec<(u32, &'a CustomizationChoice)> {
    db.options_for(state.customization_race(), state.selected_sex)
        .into_iter()
        .flatten()
        .filter_map(|option| {
            appearance_options::selected_choice(
                db,
                state.customization_race(),
                state.selected_sex,
                state.selected_class,
                &state.appearance,
                option,
            )
            .map(|choice| (option.id, choice))
        })
        .collect()
}

/// Repair unmet ChrCustomizationReqChoice groups without changing `kept_option`.
fn repair_required_choices(
    state: &mut CharCreateState,
    db: &CustomizationDb,
    kept_option: Option<u32>,
) {
    let option_count = db
        .options_for(state.customization_race(), state.selected_sex)
        .map_or(0, <[_]>::len);
    for _ in 0..option_count * 2 {
        let Some((option_id, choice_id)) = next_required_choice_fix(state, db, kept_option) else {
            return;
        };
        apply_choice(state, db, option_id, choice_id);
    }
}

fn next_required_choice_fix(
    state: &CharCreateState,
    db: &CustomizationDb,
    kept_option: Option<u32>,
) -> Option<(u32, u32)> {
    let selected = selected_option_choices(state, db);
    let selected_ids: HashSet<u32> = selected.iter().map(|(_, choice)| choice.id).collect();
    selected.iter().find_map(|&(dependent_option, choice)| {
        db.required_choices(choice)
            .iter()
            .filter(|group| !group.choice_ids.iter().any(|id| selected_ids.contains(id)))
            .find_map(|group| {
                let unmet = UnmetGroup {
                    dependent_option,
                    group,
                    selected_ids: &selected_ids,
                };
                unmet.fix(state, db, kept_option)
            })
    })
}

struct UnmetGroup<'a> {
    dependent_option: u32,
    group: &'a RequiredChoices,
    selected_ids: &'a HashSet<u32>,
}

impl UnmetGroup<'_> {
    /// Prefer changing the required option to a listed choice; otherwise change the
    /// dependent option to a choice whose own groups the selection already meets.
    fn fix(
        &self,
        state: &CharCreateState,
        db: &CustomizationDb,
        kept_option: Option<u32>,
    ) -> Option<(u32, u32)> {
        let required_option = self.group.option_id;
        let required_fix = (kept_option != Some(required_option))
            .then(|| {
                first_selectable(state, db, required_option, |candidate| {
                    self.group.choice_ids.contains(&candidate.id)
                })
            })
            .flatten()
            .map(|id| (required_option, id));
        required_fix.or_else(|| {
            (kept_option != Some(self.dependent_option))
                .then(|| {
                    first_selectable(state, db, self.dependent_option, |candidate| {
                        requirements_met(db, candidate, self.selected_ids)
                    })
                })
                .flatten()
                .map(|id| (self.dependent_option, id))
        })
    }
}

fn first_selectable(
    state: &CharCreateState,
    db: &CustomizationDb,
    option_id: u32,
    accepts: impl Fn(&CustomizationChoice) -> bool,
) -> Option<u32> {
    selectable_choices(state, db, option_id)
        .into_iter()
        .find(|choice| accepts(choice))
        .map(|choice| choice.id)
}

/// Whether `selected_ids` contains one listed choice for each of `choice`'s groups.
pub fn requirements_met(
    db: &CustomizationDb,
    choice: &CustomizationChoice,
    selected_ids: &HashSet<u32>,
) -> bool {
    db.required_choices(choice)
        .iter()
        .all(|group| group.choice_ids.iter().any(|id| selected_ids.contains(id)))
}
