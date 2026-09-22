use super::*;
use crate::ui::frame::{Frame, WidgetData};
use crate::ui::registry::FrameRegistry;
use crate::ui::widgets::button::ButtonState;
#[path = "../menu_character_layout_test_support.rs"]
mod layout_support;

struct ScreenHarness {
    screen: ui_toolkit::screen::Screen,
    shared: SharedContext,
    reg: FrameRegistry,
}

impl ScreenHarness {
    fn new(state: CharCreateUiState) -> Self {
        let mut reg = FrameRegistry::new(state.viewport_width as f32, state.viewport_height as f32);
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut screen = ui_toolkit::screen::Screen::new(char_create_screen);
        screen.sync(&shared, &mut reg);
        layout_support::compute_layout(&mut reg);
        Self {
            screen,
            shared,
            reg,
        }
    }

    fn sync(&mut self, state: CharCreateUiState) {
        self.shared.insert(state);
        self.screen.sync(&self.shared, &mut self.reg);
        layout_support::compute_layout(&mut self.reg);
    }
}

fn frame<'a>(reg: &'a FrameRegistry, name: &str) -> &'a Frame {
    reg.get_by_name(name)
        .and_then(|id| reg.get(id))
        .unwrap_or_else(|| panic!("{name} should exist"))
}

fn rect(reg: &FrameRegistry, name: &str) -> crate::ui::layout::LayoutRect {
    frame(reg, name)
        .layout_rect
        .clone()
        .unwrap_or_else(|| panic!("{name} should have native bounds"))
}

fn assert_rect(reg: &FrameRegistry, name: &str, expected: [f32; 4]) {
    let r = rect(reg, name);
    for (actual, expected) in [r.x, r.y, r.width, r.height].into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.02,
            "{name}: {r:?} vs {expected}"
        );
    }
}

fn font_text(reg: &FrameRegistry, name: &str) -> &str {
    match &frame(reg, name).widget_data {
        Some(WidgetData::FontString(data)) => &data.text,
        _ => panic!("{name} should be text"),
    }
}

fn action(reg: &FrameRegistry, name: &str) -> CharCreateAction {
    CharCreateAction::parse(frame(reg, name).onclick.as_deref().unwrap_or(""))
        .unwrap_or_else(|| panic!("{name} should dispatch an action"))
}

fn choice(id: u32, label: &str) -> CustomizationChoiceUi {
    CustomizationChoiceUi {
        id,
        label: label.to_string(),
        swatch: None,
        secondary_swatch: None,
        enabled: true,
    }
}

fn option(id: u32, label: &str) -> CustomizationOptionUi {
    CustomizationOptionUi {
        id,
        label: label.to_string(),
        ui_type: 0,
        selected_choice_id: 70001,
        choices: vec![
            choice(70001, "Amber"),
            choice(70005, "Blue"),
            choice(70100, "Violet"),
        ],
        enabled: true,
        disabled_reason: None,
    }
}

fn customize_state() -> CharCreateUiState {
    CharCreateUiState {
        mode: CharCreateMode::Customize,
        categories: vec![
            CustomizationCategoryUi {
                id: 3,
                label: "Hair".into(),
                icon_atlas: Some("charactercreate-icon-customize-hair".into()),
                selected_icon_atlas: Some("charactercreate-icon-customize-hair-selected".into()),
            },
            CustomizationCategoryUi {
                id: 23,
                label: "Eyes".into(),
                icon_atlas: Some("charactercreate-icon-customize-head".into()),
                selected_icon_atlas: Some("charactercreate-icon-customize-head-selected".into()),
            },
        ],
        selected_category: 23,
        options: vec![
            option(22, "Eye Color"),
            option(8789, "Ears"),
            option(525, "Face Shape"),
        ],
        name: "Aeloria".into(),
        ..Default::default()
    }
}

#[test]
fn retail_creation_reference_navigation_and_tiles() {
    let harness = ScreenHarness::new(CharCreateUiState::default());
    let reg = &harness.reg;
    assert_rect(reg, "Race_1", [68.0, 106.0, 79.0, 79.0]);
    let class = rect(reg, "Class_1");
    assert_eq!((class.width, class.height), (67.0, 67.0));
    assert_eq!(class.y + class.height, 1080.0 - 62.0);
    assert_rect(reg, BACK_BUTTON.0, [46.0, 986.0, 250.0, 66.0]);
    assert_rect(reg, NEXT_BUTTON.0, [1624.0, 986.0, 250.0, 66.0]);
}

#[test]
fn retail_creation_reference_customization_column() {
    let harness = ScreenHarness::new(customize_state());
    let panel = rect(&harness.reg, "CustomizePanel");
    assert_eq!(panel.x + panel.width, 1920.0 - 33.0);
    assert_eq!(panel.y, 297.0);
    assert_rect(&harness.reg, "Camera_reset", [40.0, 30.0, 48.0, 48.0]);
    assert_rect(&harness.reg, "Category_23", [1796.0, 166.0, 104.0, 105.0]);
}

#[test]
fn actions_preserve_full_option_and_choice_ids() {
    let actions = [
        CharCreateAction::SelectRace(29),
        CharCreateAction::SelectClass(5),
        CharCreateAction::SelectSex(1),
        CharCreateAction::Randomize,
        CharCreateAction::NextMode,
        CharCreateAction::Back,
        CharCreateAction::SelectCategory(23),
        CharCreateAction::AdjustOption(8789, -1),
        CharCreateAction::AdjustOption(8789, 1),
        CharCreateAction::ToggleOption(525),
        CharCreateAction::SelectOptionChoice(8789, 70005),
        CharCreateAction::CreateConfirm,
        CharCreateAction::Camera(CameraControl::Reset),
        CharCreateAction::Camera(CameraControl::ZoomIn),
        CharCreateAction::Camera(CameraControl::ZoomOut),
        CharCreateAction::Camera(CameraControl::RotateLeft),
        CharCreateAction::Camera(CameraControl::RotateRight),
    ];
    for value in actions {
        assert_eq!(CharCreateAction::parse(&value.to_string()), Some(value));
    }
    for malformed in [
        "select_sex:2",
        "adjust_option:1:0",
        "adjust_option:1:300",
        "select_category:3:extra",
        "appearance_inc:skin",
        "select_option_choice:1",
        "camera:unknown",
    ] {
        assert!(CharCreateAction::parse(malformed).is_none(), "{malformed}");
    }
}

#[test]
fn every_supported_race_and_class_remains_present_and_hittable() {
    let harness = ScreenHarness::new(CharCreateUiState::default());
    for race in crate::char_create_data::RACES {
        let name = format!("Race_{}", race.id);
        assert_eq!(
            action(&harness.reg, &name),
            CharCreateAction::SelectRace(race.id)
        );
        let r = rect(&harness.reg, &name);
        let hit = ui_toolkit::input::find_frame_at(
            &harness.reg,
            r.x + r.width / 2.0,
            r.y + r.height / 2.0,
        )
        .expect("race center should receive input");
        assert_eq!(hit, frame(&harness.reg, &name).id);
    }
    for class in crate::char_create_data::CLASSES {
        assert!(
            harness
                .reg
                .get_by_name(&format!("Class_{}", class.id))
                .is_some()
        );
    }
}

#[test]
fn unavailable_classes_are_disabled_and_selected_race_updates_in_place() {
    let mut harness = ScreenHarness::new(CharCreateUiState::default());
    let id = frame(&harness.reg, "Race_2").id;
    let disabled_class = frame(&harness.reg, "Class_11");
    assert!(
        matches!(&disabled_class.widget_data, Some(WidgetData::Button(button)) if button.state == ButtonState::Disabled)
    );
    harness.sync(CharCreateUiState {
        selected_race: 2,
        ..Default::default()
    });
    assert_eq!(frame(&harness.reg, "Race_2").id, id);
    assert!(!frame(&harness.reg, "Race_2_Selected").hidden);
    assert!(frame(&harness.reg, "Race_1_Selected").hidden);
    assert_eq!(
        action(&harness.reg, "Race_2"),
        CharCreateAction::SelectRace(2)
    );
}

#[test]
fn class_buttons_wrap_to_two_rows_without_overlapping_navigation() {
    let harness = ScreenHarness::new(CharCreateUiState {
        viewport_width: 1280,
        viewport_height: 900,
        ..Default::default()
    });
    let back = rect(&harness.reg, BACK_BUTTON.0);
    let next = rect(&harness.reg, NEXT_BUTTON.0);
    let first = rect(&harness.reg, "Class_1");
    let last = rect(&harness.reg, "Class_11");
    assert!(
        last.y > first.y,
        "ten classes should span two rows at this width"
    );
    for class in crate::char_create_data::CLASSES {
        let r = rect(&harness.reg, &format!("Class_{}", class.id));
        assert!(r.x > back.x + back.width && r.x + r.width < next.x);
        assert!(r.y + r.height <= 900.0 - 62.0);
    }
}

#[test]
fn dynamic_categories_and_all_option_rows_dispatch_by_identity() {
    let harness = ScreenHarness::new(customize_state());
    assert_eq!(
        action(&harness.reg, "Category_3"),
        CharCreateAction::SelectCategory(3)
    );
    for (id, label) in [(22, "Eye Color"), (8789, "Ears"), (525, "Face Shape")] {
        assert_eq!(font_text(&harness.reg, &format!("OptionLabel_{id}")), label);
        assert_eq!(
            action(&harness.reg, &format!("OptionToggle_{id}")),
            CharCreateAction::ToggleOption(id)
        );
        assert_eq!(
            action(&harness.reg, &format!("OptionInc_{id}")),
            CharCreateAction::AdjustOption(id, 1)
        );
    }
    for control in [
        CameraControl::Reset,
        CameraControl::ZoomIn,
        CameraControl::ZoomOut,
        CameraControl::RotateLeft,
        CameraControl::RotateRight,
    ] {
        assert_eq!(
            action(&harness.reg, &format!("Camera_{}", control.as_str())),
            CharCreateAction::Camera(control)
        );
    }
    assert_eq!(
        action(&harness.reg, "CharCreateRandomize"),
        CharCreateAction::Randomize
    );
    assert_eq!(
        action(&harness.reg, "CharCreateSex_1"),
        CharCreateAction::SelectSex(1)
    );
}

#[test]
fn category_change_removes_old_controls_without_replacing_name_input() {
    let state = customize_state();
    let mut harness = ScreenHarness::new(state.clone());
    let name_id = frame(&harness.reg, CREATE_NAME_INPUT.0).id;
    harness.sync(CharCreateUiState {
        selected_category: 3,
        options: vec![option(890, "Eyebrows")],
        ..state
    });
    assert!(harness.reg.get_by_name("Option_22").is_none());
    assert!(harness.reg.get_by_name("Option_890").is_some());
    assert_eq!(frame(&harness.reg, CREATE_NAME_INPUT.0).id, name_id);
    assert!(!frame(&harness.reg, "Category_3_Selected").hidden);
}

#[test]
fn non_color_dropdown_preserves_authored_order_and_choice_ids() {
    let harness = ScreenHarness::new(CharCreateUiState {
        open_dropdown: Some(22),
        ..customize_state()
    });
    for (index, (id, label)) in [(70001, "Amber"), (70005, "Blue"), (70100, "Violet")]
        .into_iter()
        .enumerate()
    {
        let name = format!("OptionChoice_22_{id}");
        assert_eq!(
            action(&harness.reg, &name),
            CharCreateAction::SelectOptionChoice(22, id)
        );
        assert_eq!(font_text(&harness.reg, &format!("{name}_Text")), label);
        assert_eq!(
            rect(&harness.reg, &name).y,
            rect(&harness.reg, "Dropdown_22").y + index as f32 * 20.0
        );
        assert_eq!(frame(&harness.reg, &name).strata, FrameStrata::Dialog);
    }
}

#[test]
fn unnamed_choices_remain_numbered_and_split_swatches_keep_authored_colors() {
    let mut opt = option(22, "Eye Color");
    opt.choices[0] = CustomizationChoiceUi {
        id: 70001,
        label: String::new(),
        swatch: Some([128, 32, 16]),
        secondary_swatch: Some([20, 80, 160]),
        enabled: true,
    };
    let harness = ScreenHarness::new(CharCreateUiState {
        options: vec![opt],
        open_dropdown: Some(22),
        ..customize_state()
    });
    assert_eq!(font_text(&harness.reg, "OptionValue_22_Text"), "1");
    for (suffix, expected) in [
        ("Swatch", [128.0 / 255.0, 32.0 / 255.0, 16.0 / 255.0, 1.0]),
        (
            "SecondarySwatch",
            [20.0 / 255.0, 80.0 / 255.0, 160.0 / 255.0, 1.0],
        ),
    ] {
        let data = &frame(&harness.reg, &format!("OptionChoice_22_70001_{suffix}")).widget_data;
        assert!(
            matches!(data, Some(WidgetData::Texture(texture)) if texture.vertex_color == expected)
        );
    }
}

#[test]
fn long_popup_uses_columns_and_keeps_every_choice_on_screen() {
    let mut opt = option(22, "Eye Color");
    opt.choices = (0..130)
        .map(|index| choice(80000 + index, &format!("Color {index}")))
        .collect();
    let harness = ScreenHarness::new(CharCreateUiState {
        options: vec![opt],
        open_dropdown: Some(22),
        ..customize_state()
    });
    let first = rect(&harness.reg, "OptionChoice_22_80000");
    let last = rect(&harness.reg, "OptionChoice_22_80129");
    assert!(
        last.x > first.x,
        "fixture must actually exercise additional columns"
    );
    for index in 0..130 {
        let r = rect(&harness.reg, &format!("OptionChoice_22_{}", 80000 + index));
        assert!(
            r.x >= 0.0 && r.y >= 0.0 && r.x + r.width <= 1920.0 && r.y + r.height <= 1080.0,
            "choice {index}: {r:?}"
        );
    }
}

#[test]
fn option_spacing_compresses_without_clipping_at_shorter_viewport() {
    let options = (0..12)
        .map(|id| option(100 + id, "Customization"))
        .collect();
    let harness = ScreenHarness::new(CharCreateUiState {
        options,
        viewport_height: 900,
        ..customize_state()
    });
    assert_eq!(rect(&harness.reg, "CustomizePanel").y, 267.0);
    let mut bottom = 0.0;
    for id in 100..112 {
        let r = rect(&harness.reg, &format!("Option_{id}"));
        assert!(r.y >= bottom);
        assert!(r.y + r.height < 900.0 - 94.0);
        bottom = r.y + r.height;
    }
}

#[test]
fn checkbox_uses_two_actual_choice_ids_and_selected_mark() {
    let opt = CustomizationOptionUi {
        ui_type: 1,
        choices: vec![choice(3, "No"), choice(9, "Yes")],
        selected_choice_id: 9,
        ..option(24, "Upright")
    };
    let state = CharCreateUiState {
        options: vec![opt.clone()],
        ..customize_state()
    };
    let mut harness = ScreenHarness::new(state.clone());
    assert_eq!(
        action(&harness.reg, "OptionCheck_24"),
        CharCreateAction::SelectOptionChoice(24, 3)
    );
    assert!(!frame(&harness.reg, "OptionCheck_24_Mark").hidden);
    harness.sync(CharCreateUiState {
        options: vec![CustomizationOptionUi {
            selected_choice_id: 3,
            ..opt
        }],
        ..state
    });
    assert_eq!(
        action(&harness.reg, "OptionCheck_24"),
        CharCreateAction::SelectOptionChoice(24, 9)
    );
    assert!(frame(&harness.reg, "OptionCheck_24_Mark").hidden);
}

#[test]
fn disabled_options_explain_unavailable_controls_and_disabled_choices_stay_disabled() {
    let blocked = CustomizationOptionUi {
        enabled: false,
        disabled_reason: Some("Unsupported model effect".into()),
        ..option(24, "Posture")
    };
    let mut selectable = option(22, "Eye Color");
    selectable.choices[1].enabled = false;
    let harness = ScreenHarness::new(CharCreateUiState {
        options: vec![blocked, selectable],
        open_dropdown: Some(22),
        ..customize_state()
    });
    assert_eq!(
        font_text(&harness.reg, "OptionReason_24"),
        "Unsupported model effect"
    );
    assert!(harness.reg.get_by_name("OptionToggle_24").is_none());
    assert!(
        matches!(&frame(&harness.reg, "OptionChoice_22_70005").widget_data, Some(WidgetData::Button(button)) if button.state == ButtonState::Disabled)
    );
}

#[test]
fn missing_control_support_is_explicit_not_a_silent_dropdown() {
    let harness = ScreenHarness::new(CharCreateUiState {
        options: vec![CustomizationOptionUi {
            ui_type: 2,
            ..option(77, "Height")
        }],
        ..customize_state()
    });
    assert_eq!(
        font_text(&harness.reg, "OptionReason_77"),
        "This control type is not supported"
    );
    assert!(harness.reg.get_by_name("OptionToggle_77").is_none());
}

#[test]
fn name_focus_text_and_errors_survive_popup_rebuilds() {
    let state = CharCreateUiState {
        error_text: Some("Name already exists".into()),
        ..customize_state()
    };
    let mut harness = ScreenHarness::new(state.clone());
    let name_id = frame(&harness.reg, CREATE_NAME_INPUT.0).id;
    harness.sync(CharCreateUiState {
        open_dropdown: Some(22),
        name_input_focused: true,
        ..state
    });
    let input = frame(&harness.reg, CREATE_NAME_INPUT.0);
    assert_eq!(input.id, name_id);
    assert!(
        matches!(&input.widget_data, Some(WidgetData::EditBox(edit)) if edit.text == "Aeloria")
    );
    assert_eq!(font_text(&harness.reg, ERROR_TEXT.0), "Name already exists");
    assert_rect(&harness.reg, "NamePanel", [760.0, 34.0, 400.0, 100.0]);
    assert_eq!(
        action(&harness.reg, CREATE_BUTTON.0),
        CharCreateAction::CreateConfirm
    );
}

#[test]
fn name_entry_is_only_created_in_customize_mode() {
    let mut harness = ScreenHarness::new(CharCreateUiState::default());
    assert!(harness.reg.get_by_name(CREATE_NAME_INPUT.0).is_none());
    harness.sync(customize_state());
    assert!(harness.reg.get_by_name(CREATE_NAME_INPUT.0).is_some());
    assert!(harness.reg.get_by_name(NEXT_BUTTON.0).is_none());
    harness.sync(CharCreateUiState::default());
    assert!(harness.reg.get_by_name(CREATE_NAME_INPUT.0).is_none());
    assert!(harness.reg.get_by_name(CREATE_BUTTON.0).is_none());
}
