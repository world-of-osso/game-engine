use super::{CharCreateState, CharCreateUiState, build_class_availability};
use game_engine::appearance_options;
use game_engine::customization_data::{CustomizationDb, CustomizationOption};
use game_engine::ui::screens::char_create_component::{
    CustomizationCategoryUi, CustomizationChoiceUi, CustomizationOptionUi,
};

pub(super) fn build_ui_state(state: &CharCreateState, db: &CustomizationDb) -> CharCreateUiState {
    let options = db
        .options_for(state.selected_race, state.selected_sex)
        .unwrap_or(&[]);
    let mut categories = Vec::new();
    for option in options {
        if categories
            .iter()
            .any(|category: &CustomizationCategoryUi| category.id == option.category_id)
        {
            continue;
        }
        categories.push(CustomizationCategoryUi {
            id: option.category_id,
            label: option.category_name.clone(),
            icon_atlas: atlas_name(option.category_icon),
            selected_icon_atlas: atlas_name(option.category_selected_icon),
        });
    }
    let selected_category = categories
        .iter()
        .find(|category| category.id == state.selected_category)
        .or_else(|| categories.first())
        .map_or(0, |category| category.id);
    let rows = options
        .iter()
        .filter(|option| option.category_id == selected_category)
        .map(|option| build_option(state, db, option))
        .collect();
    let partial_labels: Vec<_> = options
        .iter()
        .filter(|option| option.category_id == selected_category)
        .filter(|option| {
            db.choices_for_option(
                state.selected_race,
                state.selected_sex,
                state.selected_class,
                option.id,
            )
            .iter()
            .any(|choice| {
                choice.has_unsupported_effects && appearance_options::choice_can_render(choice)
            })
        })
        .map(|option| option.display_name.as_str())
        .collect();
    CharCreateUiState {
        support_notice: (!partial_labels.is_empty()).then(|| {
            format!(
                "Some effects are not rendered for: {}.",
                partial_labels.join(", ")
            )
        }),
        mode: state.mode,
        selected_race: state.selected_race,
        selected_class: state.selected_class,
        selected_sex: state.selected_sex,
        categories,
        selected_category,
        options: rows,
        open_dropdown: state.open_dropdown,
        error_text: state.error_text.clone(),
        class_availability: build_class_availability(state.selected_race),
        ..Default::default()
    }
}

fn atlas_name(id: u32) -> Option<String> {
    ui_toolkit::atlas::get_name_by_element_id(id).map(str::to_owned)
}

fn rgb(raw: i32) -> Option<[u8; 3]> {
    (raw != 0).then(|| {
        let argb = raw as u32;
        [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8]
    })
}

fn build_option(
    state: &CharCreateState,
    db: &CustomizationDb,
    option: &CustomizationOption,
) -> CustomizationOptionUi {
    let choices = db.choices_for_option(
        state.selected_race,
        state.selected_sex,
        state.selected_class,
        option.id,
    );
    let selected = appearance_options::selected_choice(
        db,
        state.selected_race,
        state.selected_sex,
        state.selected_class,
        &state.appearance,
        option,
    );
    let compatible_faces = (option.option_type
        == game_engine::customization_data::OptionType::Face
        && appearance_options::is_core_option(db, state.selected_race, state.selected_sex, option))
    .then(|| {
        super::appearance::compatible_face_indices(
            db,
            state.selected_race,
            state.selected_sex,
            state.selected_class,
            state.appearance.skin_color,
        )
    });
    let supported = choices
        .iter()
        .any(|choice| appearance_options::choice_can_render(choice));
    let disabled_reason = if choices.is_empty() {
        Some("No choices available for this character".to_owned())
    } else if !supported {
        Some("These appearance effects are not supported yet".to_owned())
    } else if option.ui_type > 1 {
        Some("This customization control type is not supported yet".to_owned())
    } else {
        None
    };
    CustomizationOptionUi {
        id: option.id,
        label: option.display_name.clone(),
        ui_type: option.ui_type,
        selected_choice_id: selected.map_or(0, |choice| choice.id),
        choices: choices
            .iter()
            .enumerate()
            .map(|(index, choice)| CustomizationChoiceUi {
                id: choice.id,
                label: if choice.display_name.is_empty() {
                    (index + 1).to_string()
                } else {
                    choice.display_name.clone()
                },
                swatch: rgb(choice.swatch_colors[0]),
                secondary_swatch: rgb(choice.swatch_colors[1]),
                enabled: appearance_options::choice_can_render(choice)
                    && compatible_faces.as_ref().is_none_or(|indices| {
                        indices.iter().any(|&valid| usize::from(valid) == index)
                    }),
            })
            .collect(),
        enabled: disabled_reason.is_none(),
        disabled_reason,
    }
}
