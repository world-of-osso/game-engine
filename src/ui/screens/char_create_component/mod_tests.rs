#![allow(clippy::field_reassign_with_default)]

use super::*;
use crate::ui::frame::{Dimension, WidgetData};
use crate::ui::layout::{recompute_layouts, resolve_frame_layout};

fn rect_for_name(
    reg: &crate::ui::registry::FrameRegistry,
    name: &str,
) -> crate::ui::layout::LayoutRect {
    let id = reg
        .get_by_name(name)
        .unwrap_or_else(|| panic!("{name} frame should exist"));
    reg.get(id)
        .and_then(|frame| frame.layout_rect.clone())
        .or_else(|| resolve_frame_layout(reg, id))
        .unwrap_or_else(|| panic!("{name} should have a layout rect"))
}

struct ScreenHarness {
    screen: ui_toolkit::screen::Screen,
    shared: ui_toolkit::screen::SharedContext,
    reg: crate::ui::registry::FrameRegistry,
}

impl ScreenHarness {
    fn new(state: CharCreateUiState) -> Self {
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(state);
        let mut reg = crate::ui::registry::FrameRegistry::new(1920.0, 1080.0);
        let mut screen = ui_toolkit::screen::Screen::new(char_create_screen);
        screen.sync(&shared, &mut reg);
        let root = reg
            .get_by_name(CHAR_CREATE_ROOT.0)
            .expect("CharCreateRoot should exist");
        let (screen_width, screen_height) = (reg.screen_width, reg.screen_height);
        if let Some(frame) = reg.get_mut(root) {
            frame.width = Dimension::Fixed(screen_width);
            frame.height = Dimension::Fixed(screen_height);
        }
        reg.mark_all_rects_dirty();
        recompute_layouts(&mut reg);
        Self {
            screen,
            shared,
            reg,
        }
    }

    fn sync(&mut self, state: CharCreateUiState) {
        self.shared.insert(state);
        self.screen.sync(&self.shared, &mut self.reg);
        self.reg.mark_all_rects_dirty();
        recompute_layouts(&mut self.reg);
    }
}

fn build_screen(state: CharCreateUiState) -> crate::ui::registry::FrameRegistry {
    ScreenHarness::new(state).reg
}

fn font_text(reg: &crate::ui::registry::FrameRegistry, name: &str) -> String {
    let id = reg
        .get_by_name(name)
        .unwrap_or_else(|| panic!("{name} frame should exist"));
    reg.get(id)
        .and_then(|frame| match &frame.widget_data {
            Some(WidgetData::FontString(data)) => Some(data.text.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} should be a fontstring"))
}

#[test]
fn action_roundtrip() {
    let actions = [
        CharCreateAction::SelectRace(2),
        CharCreateAction::SelectClass(5),
        CharCreateAction::ToggleSex,
        CharCreateAction::Randomize,
        CharCreateAction::NextMode,
        CharCreateAction::Back,
        CharCreateAction::AppearanceInc(AppearanceField::HairStyle),
        CharCreateAction::AppearanceDec(AppearanceField::Face),
        CharCreateAction::ToggleDropdown(AppearanceField::SkinColor),
        CharCreateAction::SelectChoice(AppearanceField::HairColor, 5),
        CharCreateAction::CreateConfirm,
    ];
    for action in &actions {
        let s = action.to_string();
        let parsed = CharCreateAction::parse(&s).unwrap_or_else(|| panic!("failed to parse '{s}'"));
        assert_eq!(&parsed, action);
    }
}

#[test]
fn screen_builds_with_default_state() {
    let reg = build_screen(CharCreateUiState::default());
    assert!(reg.get_by_name("CharCreateRoot").is_some());
    assert!(reg.get_by_name("CharCreateBack").is_some());
    assert!(reg.get_by_name("CharCreateRandomize").is_some());
}

#[test]
fn customize_mode_shows_appearance_options() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    let reg = build_screen(state);
    assert!(reg.get_by_name("CustomizePanel").is_some());
    assert!(reg.get_by_name("CharCreateNameInput").is_some());
    assert!(reg.get_by_name("CharCreateRandomize").is_some());
}

#[test]
fn customize_mode_shows_create_error_text() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    state.error_text = Some("Name already exists".to_string());

    let reg = build_screen(state);
    let error = reg
        .get_by_name(ERROR_TEXT.0)
        .and_then(|id| reg.get(id))
        .expect("CharCreateError should exist");

    assert!(!error.hidden, "expected create error label to be visible");
    assert_eq!(font_text(&reg, ERROR_TEXT.0), "Name already exists");
}

#[test]
fn customize_mode_shows_choice_names_for_non_color_options() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    state.face_label = "Calm".to_string();
    state.hair_style_label = "Bald".to_string();
    state.facial_style_label = "Goatee".to_string();

    let reg = build_screen(state);

    assert_eq!(font_text(&reg, "AppVal_face"), "Calm");
    assert_eq!(font_text(&reg, "AppVal_hair_style"), "Bald");
    assert_eq!(font_text(&reg, "AppVal_facial"), "Goatee");
}

#[test]
fn dropdown_panel_background_is_fully_opaque() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    state.open_dropdown = Some(AppearanceField::SkinColor);
    state.skin_color_swatches = vec![Some([64, 32, 16])];

    let reg = build_screen(state);
    let dropdown = reg
        .get_by_name("Dropdown_skin")
        .and_then(|id| reg.get(id))
        .expect("Dropdown_skin frame should exist");
    let bg = dropdown
        .background_color
        .expect("Dropdown_skin should have a background color");

    assert!(
        (bg[3] - 1.0).abs() < f32::EPSILON,
        "expected opaque dropdown bg, got {bg:?}"
    );
}

#[test]
fn swatch_preview_is_centered_between_stepper_buttons() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    state.skin_color_swatches = vec![Some([64, 32, 16])];

    let reg = build_screen(state);
    let dec = rect_for_name(&reg, "AppDec_skin");
    let swatch = rect_for_name(&reg, "AppSwatchArea_skin");
    let inc = rect_for_name(&reg, "AppInc_skin");

    let button_gap_center = ((dec.x + dec.width) + inc.x) * 0.5;
    let swatch_center = swatch.x + swatch.width * 0.5;

    assert!(
        (swatch_center - button_gap_center).abs() < 0.01,
        "expected swatch center {swatch_center} to match button gap center {button_gap_center}"
    );
}

fn assert_rect(
    reg: &crate::ui::registry::FrameRegistry,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    let r = rect_for_name(reg, name);
    assert_eq!((r.x, r.y, r.width, r.height), (x, y, w, h), "{name}");
}

fn build_skin_dropdown_screen() -> crate::ui::registry::FrameRegistry {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    state.open_dropdown = Some(AppearanceField::SkinColor);
    state.skin_color_swatches = vec![Some([64, 32, 16]), Some([96, 48, 24]), Some([128, 64, 32])];
    state.skin_color = 1;
    build_screen(state)
}

#[test]
fn skin_dropdown_positions_match_expected_layout() {
    let reg = build_skin_dropdown_screen();

    assert_rect(&reg, "Dropdown_skin", 20.0, 156.0, 282.0, 36.0);
    assert_rect(&reg, "DropChoice_skin_0", 24.0, 160.0, 44.0, 28.0);
    assert_rect(&reg, "DropChoice_skin_1", 70.0, 160.0, 44.0, 28.0);
    assert_rect(&reg, "DropChoice_skin_2", 116.0, 160.0, 44.0, 28.0);
    assert_rect(&reg, "DropSwatch_skin_1", 72.0, 164.0, 40.0, 20.0);
    assert_rect(&reg, "DropSel_skin_1", 64.0, 160.0, 48.0, 28.0);
}

#[test]
fn dropdown_children_inherit_dialog_strata() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    state.open_dropdown = Some(AppearanceField::SkinColor);
    state.skin_color_swatches = vec![Some([64, 32, 16])];

    let reg = build_screen(state);
    let dropdown = reg
        .get_by_name("Dropdown_skin")
        .and_then(|id| reg.get(id))
        .expect("Dropdown_skin should exist");
    let choice = reg
        .get_by_name("DropChoice_skin_0")
        .and_then(|id| reg.get(id))
        .expect("DropChoice_skin_0 should exist");
    let swatch = reg
        .get_by_name("DropSwatch_skin_0")
        .and_then(|id| reg.get(id))
        .expect("DropSwatch_skin_0 should exist");

    assert_eq!(dropdown.strata, crate::ui::strata::FrameStrata::Dialog);
    assert_eq!(choice.strata, crate::ui::strata::FrameStrata::Dialog);
    assert_eq!(swatch.strata, crate::ui::strata::FrameStrata::Dialog);
}

#[test]
fn dropdown_background_expands_to_cover_all_swatch_choices() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    state.open_dropdown = Some(AppearanceField::SkinColor);
    state.skin_color_swatches = (0..40)
        .map(|i| Some([(20 + i) as u8, (40 + i) as u8, (60 + i) as u8]))
        .collect();

    let reg = build_screen(state);
    let dropdown = rect_for_name(&reg, "Dropdown_skin");
    let last_choice = rect_for_name(&reg, "DropChoice_skin_39");

    assert!(
        last_choice.y + last_choice.height <= dropdown.y + dropdown.height,
        "expected dropdown to cover all choices, got dropdown={dropdown:?} last_choice={last_choice:?}"
    );
}

#[test]
fn skin_dropdown_opens_below_its_label_row() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    state.open_dropdown = Some(AppearanceField::SkinColor);
    state.skin_color_swatches = vec![Some([64, 32, 16])];

    let reg = build_screen(state);
    let label = rect_for_name(&reg, "AppLabel_skin");
    let dropdown = rect_for_name(&reg, "Dropdown_skin");

    assert!(
        dropdown.y >= label.y + label.height,
        "expected dropdown below label row, got label={label:?} dropdown={dropdown:?}"
    );
}

#[test]
fn hair_dropdown_aligns_with_customize_panel_and_opens_below_row() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;
    state.open_dropdown = Some(AppearanceField::HairColor);
    state.hair_color_swatches = vec![Some([64, 32, 16]), Some([96, 48, 24])];

    let reg = build_screen(state);
    let panel = rect_for_name(&reg, "CustomizePanel");
    let label = rect_for_name(&reg, "AppLabel_hair_color");
    let dropdown = rect_for_name(&reg, "Dropdown_hair_color");

    assert_eq!(dropdown.x, panel.x);
    assert!(
        dropdown.y >= label.y + label.height,
        "expected hair dropdown below label row, got label={label:?} dropdown={dropdown:?}"
    );
}

#[test]
fn name_panel_controls_stack_with_spacing() {
    let mut state = CharCreateUiState::default();
    state.mode = CharCreateMode::Customize;

    let reg = build_screen(state);
    let name_label = rect_for_name(&reg, "NameLabel");
    let name_input = rect_for_name(&reg, "CharCreateNameInput");
    let create_button = rect_for_name(&reg, "CharCreateButton");

    assert!(
        name_input.y >= name_label.y + name_label.height,
        "expected name input below label, got label={name_label:?} input={name_input:?}"
    );
    assert!(
        create_button.y >= name_input.y + name_input.height,
        "expected create button below input, got input={name_input:?} button={create_button:?}"
    );
    assert!(
        create_button.y > name_input.y + name_input.height,
        "expected visible gap between input and create button, got input={name_input:?} button={create_button:?}"
    );
}

#[test]
fn name_input_exists_in_all_dropdown_states() {
    // Verify the editbox frame exists for every possible open_dropdown value
    let dropdowns: Vec<Option<AppearanceField>> = vec![
        None,
        Some(AppearanceField::SkinColor),
        Some(AppearanceField::Face),
        Some(AppearanceField::HairStyle),
        Some(AppearanceField::HairColor),
        Some(AppearanceField::FacialStyle),
    ];
    for dropdown in &dropdowns {
        let mut state = CharCreateUiState::default();
        state.mode = CharCreateMode::Customize;
        state.open_dropdown = *dropdown;
        state.skin_color_swatches = vec![Some([64, 32, 16])];
        state.hair_color_swatches = vec![Some([64, 32, 16])];
        let reg = build_screen(state);
        assert!(
            reg.get_by_name("CharCreateNameInput").is_some(),
            "name input should exist when open_dropdown={dropdown:?}"
        );
        assert!(
            reg.get_by_name("NameLabel").is_some(),
            "name label should exist when open_dropdown={dropdown:?}"
        );
        let input_rect = rect_for_name(&reg, "CharCreateNameInput");
        assert!(
            input_rect.width > 0.0 && input_rect.height > 0.0,
            "name input should have size when open_dropdown={dropdown:?}, got {input_rect:?}"
        );
    }
}

#[test]
fn name_panel_stays_on_screen_after_dropdown_toggle() {
    let mut harness = ScreenHarness::new({
        let mut s = CharCreateUiState::default();
        s.mode = CharCreateMode::Customize;
        s
    });

    // Check initial position
    let initial_panel = rect_for_name(&harness.reg, "NamePanel");
    let screen_h = harness.reg.screen_height;
    assert!(
        initial_panel.y + initial_panel.height <= screen_h,
        "NamePanel should be on screen initially, got {initial_panel:?} screen_h={screen_h}"
    );

    // Toggle face dropdown and re-sync
    harness.sync({
        let mut s = CharCreateUiState::default();
        s.mode = CharCreateMode::Customize;
        s.open_dropdown = Some(AppearanceField::Face);
        s
    });

    let after_panel = rect_for_name(&harness.reg, "NamePanel");
    assert!(
        after_panel.y + after_panel.height <= screen_h,
        "NamePanel should stay on screen after dropdown toggle, got {after_panel:?} screen_h={screen_h}"
    );
    assert!(
        after_panel.y >= 0.0,
        "NamePanel should not go above screen, got {after_panel:?}"
    );

    // Also check the root frame size hasn't changed
    let root_id = harness.reg.get_by_name(CHAR_CREATE_ROOT.0).unwrap();
    let root = harness.reg.get(root_id).unwrap();
    assert_eq!(
        root.resolved_width(),
        1920.0,
        "root width should stay at screen width"
    );
    assert_eq!(
        root.resolved_height(),
        1080.0,
        "root height should stay at screen height"
    );
}

#[test]
fn name_input_not_present_in_race_class_mode() {
    // The editbox only exists in Customize mode. Starting in RaceClass means
    // any one-time setup targeting the editbox by ID will miss it.
    let harness = ScreenHarness::new(CharCreateUiState::default());
    assert!(
        harness.reg.get_by_name("CharCreateNameInput").is_none(),
        "name input should not exist in RaceClass mode"
    );
}

#[test]
fn name_input_created_on_customize_mode_switch() {
    let mut harness = ScreenHarness::new(CharCreateUiState::default());
    harness.sync({
        let mut s = CharCreateUiState::default();
        s.mode = CharCreateMode::Customize;
        s
    });
    let id = harness
        .reg
        .get_by_name("CharCreateNameInput")
        .expect("name input should exist after switching to Customize mode");
    let frame = harness.reg.get(id).unwrap();
    assert!(
        frame.nine_slice.is_some(),
        "name input should have nine_slice backdrop from RSX after mode switch"
    );
}

#[test]
fn name_input_survives_dropdown_toggle() {
    let mut harness = ScreenHarness::new({
        let mut s = CharCreateUiState::default();
        s.mode = CharCreateMode::Customize;
        s
    });

    // Verify editbox exists initially
    assert!(
        harness.reg.get_by_name("CharCreateNameInput").is_some(),
        "name input should exist before dropdown toggle"
    );

    // Open face dropdown and re-sync
    harness.sync({
        let mut s = CharCreateUiState::default();
        s.mode = CharCreateMode::Customize;
        s.open_dropdown = Some(AppearanceField::Face);
        s
    });

    // Editbox must still exist
    assert!(
        harness.reg.get_by_name("CharCreateNameInput").is_some(),
        "name input should survive face dropdown toggle"
    );
    let name_input = rect_for_name(&harness.reg, "CharCreateNameInput");
    assert!(
        name_input.width > 0.0 && name_input.height > 0.0,
        "name input should have non-zero size after dropdown toggle, got {name_input:?}"
    );
}

fn editbox_background_color(reg: &crate::ui::registry::FrameRegistry) -> Option<[f32; 4]> {
    let id = reg.get_by_name("CharCreateNameInput")?;
    let frame = reg.get(id)?;
    frame.background_color
}

fn editbox_center_texture(reg: &crate::ui::registry::FrameRegistry) -> Option<String> {
    let id = reg.get_by_name("CharCreateNameInput")?;
    let frame = reg.get(id)?;
    let ns = frame.nine_slice.as_ref()?;
    let parts = ns.part_textures.as_ref()?;
    match &parts[4] {
        crate::ui::widgets::texture::TextureSource::File(path) => Some(path.clone()),
        _ => None,
    }
}

#[test]
fn editbox_background_color_changes_on_focus() {
    let mut harness = ScreenHarness::new({
        let mut s = CharCreateUiState::default();
        s.mode = CharCreateMode::Customize;
        s
    });

    let unfocused_center =
        editbox_center_texture(&harness.reg).expect("editbox should have a center texture");
    assert!(
        unfocused_center.ends_with("Common-Input-Border-M.blp"),
        "unfocused state should match pre-77f891b center texture, got {unfocused_center}"
    );
    assert!(
        editbox_background_color(&harness.reg).is_none(),
        "unfocused state should not use a frame background_color"
    );

    // Switch to focused
    harness.sync({
        let mut s = CharCreateUiState::default();
        s.mode = CharCreateMode::Customize;
        s.name_input_focused = true;
        s
    });

    let focused_center = editbox_center_texture(&harness.reg)
        .expect("editbox should have a center texture after focus");
    assert!(
        focused_center.ends_with("editbox-white-fill.ktx2"),
        "focused state should use the white-fill center texture, got {focused_center}"
    );
    let focused_background = editbox_background_color(&harness.reg)
        .expect("editbox should have a backdrop color after focus");
    assert!(
        focused_background[0] > 0.1,
        "focused background should be visibly warm, got {focused_background:?}"
    );
    assert!(
        !focused_center.ends_with("Common-Input-Border-M.blp"),
        "focused state should differ from the unfocused original center texture"
    );
}
