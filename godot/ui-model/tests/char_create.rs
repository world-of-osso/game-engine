use game_engine_ui_model::CharacterCreateModel;
use game_engine_ui_model::char_create_component::{
    BACK_BUTTON, CREATE_BUTTON, CREATE_NAME_INPUT, CharCreateAction, CharCreateMode,
    CharCreateUiState, CustomizationCategoryUi, CustomizationChoiceUi, CustomizationOptionUi,
    NEXT_BUTTON,
};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::widgets::texture::TextureSource;

fn frame<'a>(model: &'a CharacterCreateModel, name: &str) -> &'a ui_toolkit::frame::Frame {
    model
        .registry
        .get(model.registry.get_by_name(name).unwrap())
        .unwrap()
}

fn action(model: &CharacterCreateModel, name: &str) -> Option<CharCreateAction> {
    frame(model, name)
        .onclick
        .as_deref()
        .and_then(CharCreateAction::parse)
}

fn customize() -> CharCreateUiState {
    CharCreateUiState {
        mode: CharCreateMode::Customize,
        selected_category: 23,
        categories: vec![CustomizationCategoryUi {
            id: 23,
            label: "Eyes".into(),
            icon_atlas: Some("charactercreate-icon-customize-head".into()),
            selected_icon_atlas: Some("charactercreate-icon-customize-head-selected".into()),
        }],
        options: vec![CustomizationOptionUi {
            id: 22,
            label: "Eye Color".into(),
            ui_type: 0,
            selected_choice_id: 70001,
            choices: vec![
                CustomizationChoiceUi {
                    id: 70001,
                    label: "Amber".into(),
                    swatch: None,
                    secondary_swatch: None,
                    enabled: true,
                },
                CustomizationChoiceUi {
                    id: 70005,
                    label: "Blue".into(),
                    swatch: None,
                    secondary_swatch: None,
                    enabled: true,
                },
            ],
            enabled: true,
            disabled_reason: None,
        }],
        open_dropdown: Some(22),
        name: "Aeloria".into(),
        random_name_available: true,
        ..Default::default()
    }
}

#[test]
fn authored_race_class_mode_retains_named_actions_and_navigation() {
    let mut model = CharacterCreateModel::new(1920.0, 1080.0);
    model.sync();
    assert_eq!(
        action(&model, "Race_1"),
        Some(CharCreateAction::SelectRace(1))
    );
    assert_eq!(
        action(&model, "Class_2"),
        Some(CharCreateAction::SelectClass(2))
    );
    assert_eq!(action(&model, "Class_7"), None); // Human Shaman is unavailable.
    assert_eq!(action(&model, BACK_BUTTON.0), Some(CharCreateAction::Back));
    assert_eq!(
        action(&model, NEXT_BUTTON.0),
        Some(CharCreateAction::NextMode)
    );
    assert!(model.registry.get_by_name(CREATE_NAME_INPUT.0).is_none());
    let back = frame(&model, BACK_BUTTON.0);
    assert_eq!(
        (back.width, back.height),
        (Dimension::Fixed(250.0), Dimension::Fixed(66.0))
    );
    let root = frame(&model, "CharCreateRoot");
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(1920.0), Dimension::Fixed(1080.0))
    );
    assert_eq!(frame(&model, "Race_1").position.left, Val::Px(68.0));
    model.registry.screen_width = 1280.0;
    model.registry.screen_height = 900.0;
    model.sync();
    let root = frame(&model, "CharCreateRoot");
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(1280.0), Dimension::Fixed(900.0))
    );
}

#[test]
fn authored_customize_mode_keeps_dropdown_choices_name_and_postsetup() {
    // Closed dropdown labels are measured with the client's FrizQuadrata.
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut model = CharacterCreateModel::new(1920.0, 1080.0);
    model.shared.insert(customize());
    model.sync();
    assert_eq!(
        action(&model, "Category_23"),
        Some(CharCreateAction::SelectCategory(23))
    );
    assert_eq!(
        action(&model, "OptionToggle_22"),
        Some(CharCreateAction::ToggleOption(22))
    );
    assert_eq!(
        action(&model, "OptionChoice_22_70005"),
        Some(CharCreateAction::SelectOptionChoice(22, 70005))
    );
    assert_eq!(
        action(&model, CREATE_BUTTON.0),
        Some(CharCreateAction::CreateConfirm)
    );
    assert_eq!(
        action(&model, "CharCreateRandomName"),
        Some(CharCreateAction::RandomizeName)
    );
    assert_eq!(
        action(&model, "Camera_zoom_in"),
        Some(CharCreateAction::Camera(
            game_engine_ui_model::char_create_component::CameraControl::ZoomIn
        ))
    );
    assert!(
        matches!(&frame(&model, CREATE_NAME_INPUT.0).widget_data, Some(WidgetData::EditBox(edit)) if edit.text == "Aeloria")
    );
    let popup = frame(&model, "Dropdown_22");
    assert_eq!(
        (popup.width, popup.height),
        (Dimension::Fixed(150.0), Dimension::Fixed(53.0))
    );
    assert_eq!(
        (popup.position.left, popup.position.top),
        (Val::Px(1700.5), Val::Px(335.0))
    );
    let backdrop = frame(&model, "Dropdown_22_Background");
    assert!(
        matches!(&backdrop.widget_data, Some(WidgetData::Texture(texture)) if texture.source == TextureSource::Atlas("common-dropdown-c-bg".into()))
    );
    assert_eq!(
        backdrop.nine_slice.as_ref().unwrap().edge_sizes,
        Some([23.0, 18.0, 23.0, 28.0])
    );
    assert!(model.registry.get_by_name(NEXT_BUTTON.0).is_none());
}

/// At 1280x720 both race columns, with six Horde allied races (Dracthyr 70
/// included), end above the 66 px navigation buttons 28 px from the bottom.
#[test]
fn race_columns_end_above_the_navigation_buttons_at_720p() {
    let mut model = CharacterCreateModel::new(1280.0, 720.0);
    model.sync();
    let nav_top = 720.0 - 28.0 - 66.0;
    for race in game_engine_ui_model::char_create_data::RACES {
        let Val::Px(top) = frame(&model, &format!("Race_{}", race.id)).position.top else {
            panic!("Race_{} has no pixel top", race.id);
        };
        assert!(top + 79.0 <= nav_top, "{} ends at {}", race.name, top + 79.0);
    }
}
