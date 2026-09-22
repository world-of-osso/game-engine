//! Resolve core selectors and disjoint additional option IDs into one choice set.

use shared::components::{CharacterAppearance, CustomizationChoiceSelection};

use crate::customization_data::{
    CustomizationChoice, CustomizationDb, CustomizationOption, OptionType,
};

fn core_value(appearance: &CharacterAppearance, kind: OptionType) -> Option<u8> {
    match kind {
        OptionType::SkinColor => Some(appearance.skin_color),
        OptionType::Face => Some(appearance.face),
        OptionType::EyeColor => Some(appearance.eye_color),
        OptionType::HairStyle => Some(appearance.hair_style),
        OptionType::HairColor => Some(appearance.hair_color),
        OptionType::FacialHair => Some(appearance.facial_style),
        _ => None,
    }
}

pub fn is_core_option(
    db: &CustomizationDb,
    race: u8,
    sex: u8,
    option: &CustomizationOption,
) -> bool {
    core_value(&CharacterAppearance::default(), option.option_type).is_some()
        && db
            .options_for(race, sex)
            .into_iter()
            .flatten()
            .filter(|candidate| candidate.option_type == option.option_type)
            .map(|candidate| candidate.id)
            .min()
            == Some(option.id)
}

/// Unassigned additional options use their first supported authored choice.
/// Explicit invalid choices are not replaced with a different choice.
pub fn selected_choice<'a>(
    db: &'a CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    appearance: &CharacterAppearance,
    option: &CustomizationOption,
) -> Option<&'a CustomizationChoice> {
    let choices = db.choices_for_option(race, sex, class, option.id);
    if is_core_option(db, race, sex, option) {
        return choices
            .get(core_value(appearance, option.option_type)? as usize)
            .copied();
    }
    if let Some(selection) = appearance
        .customization_choices
        .iter()
        .find(|selection| selection.option_id == option.id)
    {
        return choices
            .into_iter()
            .find(|choice| choice.id == selection.choice_id);
    }
    choices
        .into_iter()
        .find(|choice| !choice.has_unsupported_effects)
}

pub fn selected_choices<'a>(
    db: &'a CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    appearance: &CharacterAppearance,
) -> Vec<&'a CustomizationChoice> {
    db.options_for(race, sex)
        .into_iter()
        .flatten()
        .filter_map(|option| selected_choice(db, race, sex, class, appearance, option))
        .collect()
}

pub fn set_choice(
    db: &CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    appearance: &mut CharacterAppearance,
    option_id: u32,
    choice_id: u32,
) -> Result<(), String> {
    let option = db.option_by_id(race, sex, option_id).ok_or_else(|| {
        format!("Customization option {option_id} is unavailable for this body type")
    })?;
    let choices = db.choices_for_option(race, sex, class, option_id);
    let index = choices
        .iter()
        .position(|choice| choice.id == choice_id)
        .ok_or_else(|| format!("Choice {choice_id} is unavailable for option {option_id}"))?;
    if choices[index].has_unsupported_effects {
        return Err(format!(
            "{} uses appearance effects not supported yet",
            option.display_name
        ));
    }
    if is_core_option(db, race, sex, option) {
        let value = u8::try_from(index)
            .map_err(|_| format!("Choice index {index} exceeds the core selector range"))?;
        match option.option_type {
            OptionType::SkinColor => appearance.skin_color = value,
            OptionType::Face => appearance.face = value,
            OptionType::EyeColor => appearance.eye_color = value,
            OptionType::HairStyle => appearance.hair_style = value,
            OptionType::HairColor => appearance.hair_color = value,
            OptionType::FacialHair => appearance.facial_style = value,
            _ => unreachable!("is_core_option accepts only the six core selectors"),
        }
    } else if let Some(selection) = appearance
        .customization_choices
        .iter_mut()
        .find(|selection| selection.option_id == option_id)
    {
        selection.choice_id = choice_id;
    } else {
        appearance
            .customization_choices
            .push(CustomizationChoiceSelection {
                option_id,
                choice_id,
            });
        appearance
            .customization_choices
            .sort_by_key(|selection| selection.option_id);
    }
    Ok(())
}

/// Race/class/body-type changes discard invalid overrides, not authored defaults.
pub fn normalize_additional_choices(
    db: &CustomizationDb,
    race: u8,
    sex: u8,
    class: u8,
    appearance: &mut CharacterAppearance,
) {
    appearance.customization_choices.retain(|selection| {
        db.option_by_id(race, sex, selection.option_id)
            .is_some_and(|option| {
                !is_core_option(db, race, sex, option)
                    && db
                        .choices_for_option(race, sex, class, option.id)
                        .iter()
                        .any(|choice| {
                            choice.id == selection.choice_id && !choice.has_unsupported_effects
                        })
            })
    });
    appearance
        .customization_choices
        .sort_by_key(|selection| selection.option_id);
    appearance
        .customization_choices
        .dedup_by_key(|selection| selection.option_id);
}
