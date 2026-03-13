mod char_create_widgets;

use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

use char_create_widgets::{
    bottom_buttons, class_button, create_confirm_button, customization_row, dropdown_panel,
    error_label, faction_column, name_input_field, race_buttons_for_faction,
};

// --- Actions ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharCreateAction {
    SelectRace(u8),
    SelectClass(u8),
    ToggleSex,
    NextMode,
    Back,
    AppearanceInc(AppearanceField),
    AppearanceDec(AppearanceField),
    ToggleDropdown(AppearanceField),
    SelectChoice(AppearanceField, u8),
    CreateConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceField {
    SkinColor,
    Face,
    HairStyle,
    HairColor,
    FacialStyle,
}

impl AppearanceField {
    pub fn as_str(self) -> &'static str {
        match self {
            AppearanceField::SkinColor => "skin",
            AppearanceField::Face => "face",
            AppearanceField::HairStyle => "hair_style",
            AppearanceField::HairColor => "hair_color",
            AppearanceField::FacialStyle => "facial",
        }
    }
}

impl fmt::Display for CharCreateAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectRace(id) => write!(f, "select_race:{id}"),
            Self::SelectClass(id) => write!(f, "select_class:{id}"),
            Self::ToggleSex => f.write_str("toggle_sex"),
            Self::NextMode => f.write_str("next_mode"),
            Self::Back => f.write_str("back"),
            Self::AppearanceInc(field) => write!(f, "appearance_inc:{}", field.as_str()),
            Self::AppearanceDec(field) => write!(f, "appearance_dec:{}", field.as_str()),
            Self::ToggleDropdown(field) => write!(f, "toggle_dropdown:{}", field.as_str()),
            Self::SelectChoice(field, idx) => {
                write!(f, "select_choice:{}:{idx}", field.as_str())
            }
            Self::CreateConfirm => f.write_str("create_confirm"),
        }
    }
}

fn parse_field(s: &str) -> Option<AppearanceField> {
    match s {
        "skin" => Some(AppearanceField::SkinColor),
        "face" => Some(AppearanceField::Face),
        "hair_style" => Some(AppearanceField::HairStyle),
        "hair_color" => Some(AppearanceField::HairColor),
        "facial" => Some(AppearanceField::FacialStyle),
        _ => None,
    }
}

impl CharCreateAction {
    pub fn parse(s: &str) -> Option<Self> {
        if let Some(id) = s.strip_prefix("select_race:") {
            return id.parse().ok().map(Self::SelectRace);
        }
        if let Some(id) = s.strip_prefix("select_class:") {
            return id.parse().ok().map(Self::SelectClass);
        }
        if let Some(field) = s.strip_prefix("appearance_inc:") {
            return parse_field(field).map(Self::AppearanceInc);
        }
        if let Some(field) = s.strip_prefix("appearance_dec:") {
            return parse_field(field).map(Self::AppearanceDec);
        }
        if let Some(field) = s.strip_prefix("toggle_dropdown:") {
            return parse_field(field).map(Self::ToggleDropdown);
        }
        if let Some(rest) = s.strip_prefix("select_choice:") {
            let mut parts = rest.splitn(2, ':');
            let field = parts.next().and_then(parse_field)?;
            let idx = parts.next().and_then(|s| s.parse().ok())?;
            return Some(Self::SelectChoice(field, idx));
        }
        match s {
            "toggle_sex" => Some(Self::ToggleSex),
            "next_mode" => Some(Self::NextMode),
            "back" => Some(Self::Back),
            "create_confirm" => Some(Self::CreateConfirm),
            _ => None,
        }
    }
}

// --- State ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharCreateMode {
    RaceClass,
    Customize,
}

pub struct CharCreateUiState {
    pub mode: CharCreateMode,
    pub selected_race: u8,
    pub selected_class: u8,
    pub selected_sex: u8,
    pub skin_color: u8,
    pub face: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub facial_style: u8,
    pub skin_color_swatches: Vec<Option<[u8; 3]>>,
    pub hair_color_swatches: Vec<Option<[u8; 3]>>,
    pub open_dropdown: Option<AppearanceField>,
    pub name: String,
    pub error_text: Option<String>,
    /// (class_id, class_name, icon_file, available_for_race)
    pub class_availability: Vec<(u8, &'static str, &'static str, bool)>,
}

impl Default for CharCreateUiState {
    fn default() -> Self {
        use crate::char_create_data::{race_can_be_class, CLASSES};
        let race = 1;
        let class_availability: Vec<_> = CLASSES
            .iter()
            .map(|c| (c.id, c.name, c.icon_file, race_can_be_class(race, c.id)))
            .collect();
        Self {
            mode: CharCreateMode::RaceClass,
            selected_race: race,
            selected_class: 1,
            selected_sex: 0,
            skin_color: 0,
            face: 0,
            hair_style: 0,
            hair_color: 0,
            facial_style: 0,
            skin_color_swatches: Vec::new(),
            hair_color_swatches: Vec::new(),
            open_dropdown: None,
            name: String::new(),
            error_text: None,
            class_availability,
        }
    }
}

// --- Frame names ---

pub const CHAR_CREATE_ROOT: FrameName = FrameName("CharCreateRoot");
pub const CREATE_NAME_INPUT: FrameName = FrameName("CharCreateNameInput");
pub const CREATE_BUTTON: FrameName = FrameName("CharCreateButton");
pub const BACK_BUTTON: FrameName = FrameName("CharCreateBack");
pub const NEXT_BUTTON: FrameName = FrameName("CharCreateNext");
pub const SEX_TOGGLE_BUTTON: FrameName = FrameName("CharCreateSexToggle");
pub const ERROR_TEXT: FrameName = FrameName("CharCreateError");

// --- Shared constants (used by widgets submodule) ---

pub(crate) const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
pub(crate) const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);
pub(crate) const COLOR_WHITE: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);
pub(crate) const COLOR_DISABLED: FontColor = FontColor::new(0.4, 0.4, 0.4, 1.0);
pub(crate) const COLOR_SELECTED: FontColor = FontColor::new(1.0, 0.92, 0.72, 1.0);

pub(crate) const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
pub(crate) const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
pub(crate) const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
pub(crate) const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

pub(crate) struct DynName(pub String);

// --- Layout panels (call into widgets submodule) ---

fn race_grid(selected_race: u8) -> Element {
    use crate::char_create_data::Faction;
    let alliance = race_buttons_for_faction(Faction::Alliance, selected_race);
    let horde = race_buttons_for_faction(Faction::Horde, selected_race);
    rsx! {
        r#frame { name: "RaceGrid", width: 320.0, height: 500.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "20",
                y: "-80",
            }
            {faction_column("Alliance", "Alliance", "5", alliance)}
            {faction_column("Horde", "Horde", "165", horde)}
        }
    }
}

fn class_grid(state: &CharCreateUiState) -> Element {
    let classes: Element = state
        .class_availability
        .iter()
        .flat_map(|&(id, name, icon, avail)| {
            class_button(id, name, icon, id == state.selected_class, avail)
        })
        .collect();
    rsx! {
        r#frame {
            name: "ClassGrid",
            width: 180.0,
            height: 500.0,
            layout: "flex-row-wrap",
            gap: 6.0,
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-20",
                y: "-80",
            }
            fontstring {
                name: "ClassLabel",
                width: 160.0,
                height: 24.0,
                text: "Class",
                font: GameFont::FrizQuadrata,
                font_size: 16.0,
                font_color: COLOR_GOLD,
            }
            {classes}
        }
    }
}

fn customize_rows(state: &CharCreateUiState) -> Element {
    rsx! {
        {customization_row("Skin Color", state.skin_color, &state.skin_color_swatches, AppearanceField::SkinColor)}
        {customization_row("Face", state.face, &[], AppearanceField::Face)}
        {customization_row("Hair Style", state.hair_style, &[], AppearanceField::HairStyle)}
        {customization_row("Hair Color", state.hair_color, &state.hair_color_swatches, AppearanceField::HairColor)}
        {customization_row("Facial Style", state.facial_style, &[], AppearanceField::FacialStyle)}
    }
}

fn customize_panel(state: &CharCreateUiState) -> Element {
    // row_height(44) + gap(8) = 52 per row; dropdowns should open below the active row.
    let dropdown = match state.open_dropdown {
        Some(AppearanceField::SkinColor) => dropdown_panel(
            AppearanceField::SkinColor,
            &state.skin_color_swatches,
            state.skin_color,
            -156.0,
        ),
        Some(AppearanceField::HairColor) => dropdown_panel(
            AppearanceField::HairColor,
            &state.hair_color_swatches,
            state.hair_color,
            -312.0,
        ),
        _ => Element::default(),
    };
    rsx! {
        r#frame {
            name: "CustomizePanel",
            width: 300.0,
            height: 500.0,
            layout: "flex-col",
            gap: 8.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "20",
                y: "-80",
            }
            fontstring {
                name: "CustomizeLabel",
                width: 280.0,
                height: 24.0,
                text: "Customize Appearance",
                font: GameFont::FrizQuadrata,
                font_size: 16.0,
                font_color: COLOR_GOLD,
            }
            {customize_rows(state)}
        }
        {dropdown}
    }
}

fn name_and_create(state: &CharCreateUiState) -> Element {
    rsx! {
        r#frame { name: "NamePanel", width: 400.0, height: 120.0,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "140",
            }
            {name_input_field()}
            {error_label(state.error_text.as_deref())}
            {create_confirm_button()}
        }
    }
}

fn title_area(state: &CharCreateUiState) -> Element {
    use crate::char_create_data::{class_by_id, race_by_id};
    let race_name = race_by_id(state.selected_race)
        .map(|r| r.name)
        .unwrap_or("Unknown");
    let class_name = class_by_id(state.selected_class)
        .map(|c| c.name)
        .unwrap_or("Unknown");
    let sex_str = if state.selected_sex == 0 {
        "Male"
    } else {
        "Female"
    };
    let title = format!("{sex_str} {race_name} {class_name}");
    rsx! {
        fontstring {
            name: "CharCreateTitle",
            width: 520.0,
            height: 36.0,
            text: title,
            font: GameFont::FrizQuadrata,
            font_size: 24.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-30",
            }
        }
    }
}

// --- Main screen ---

pub fn char_create_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CharCreateUiState>()
        .expect("CharCreateUiState must be in SharedContext");
    let mode_content = match state.mode {
        CharCreateMode::RaceClass => {
            let mut elems = race_grid(state.selected_race);
            elems.extend(class_grid(state));
            elems
        }
        CharCreateMode::Customize => {
            let mut elems = customize_panel(state);
            elems.extend(name_and_create(state));
            elems
        }
    };
    rsx! {
        r#frame { name: CHAR_CREATE_ROOT, strata: FrameStrata::Background,
            r#frame {
                name: "CharCreateBackground",
                stretch: true,
                background_color: "0,0,0,0",
                strata: FrameStrata::Background,
            }
            {title_area(state)}
            {mode_content}
            {bottom_buttons(state.mode)}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::frame::Dimension;
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

    fn build_screen(state: CharCreateUiState) -> crate::ui::registry::FrameRegistry {
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
        reg
    }

    #[test]
    fn action_roundtrip() {
        let actions = [
            CharCreateAction::SelectRace(2),
            CharCreateAction::SelectClass(5),
            CharCreateAction::ToggleSex,
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
            let parsed =
                CharCreateAction::parse(&s).unwrap_or_else(|| panic!("failed to parse '{s}'"));
            assert_eq!(&parsed, action);
        }
    }

    #[test]
    fn screen_builds_with_default_state() {
        let reg = build_screen(CharCreateUiState::default());
        assert!(reg.get_by_name("CharCreateRoot").is_some());
        assert!(reg.get_by_name("CharCreateBack").is_some());
    }

    #[test]
    fn customize_mode_shows_appearance_options() {
        let mut state = CharCreateUiState::default();
        state.mode = CharCreateMode::Customize;
        let reg = build_screen(state);
        assert!(reg.get_by_name("CustomizePanel").is_some());
        assert!(reg.get_by_name("CharCreateNameInput").is_some());
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

    #[test]
    fn swatch_selection_overlay_is_shifted_left_to_match_palette_art() {
        let mut state = CharCreateUiState::default();
        state.mode = CharCreateMode::Customize;
        state.skin_color_swatches = vec![Some([64, 32, 16])];

        let reg = build_screen(state);
        let swatch = rect_for_name(&reg, "AppSwatch_skin");
        let selection = rect_for_name(&reg, "AppSwatchSel_skin");
        let swatch_center = swatch.x + swatch.width * 0.5;
        let selection_center = selection.x + selection.width * 0.5;

        assert!(
            (selection_center - (swatch_center - 8.0)).abs() < 0.01,
            "expected selection center {selection_center} to be 8px left of swatch center {swatch_center}"
        );
    }

    #[test]
    fn skin_dropdown_positions_match_expected_layout() {
        let mut state = CharCreateUiState::default();
        state.mode = CharCreateMode::Customize;
        state.open_dropdown = Some(AppearanceField::SkinColor);
        state.skin_color_swatches =
            vec![Some([64, 32, 16]), Some([96, 48, 24]), Some([128, 64, 32])];
        state.skin_color = 1;

        let reg = build_screen(state);

        let dropdown = rect_for_name(&reg, "Dropdown_skin");
        assert_eq!(dropdown.x, 20.0);
        assert_eq!(dropdown.y, 156.0);
        assert_eq!(dropdown.width, 280.0);
        assert_eq!(dropdown.height, 36.0);

        let choice0 = rect_for_name(&reg, "DropChoice_skin_0");
        let choice1 = rect_for_name(&reg, "DropChoice_skin_1");
        let choice2 = rect_for_name(&reg, "DropChoice_skin_2");
        assert_eq!(
            (choice0.x, choice0.y, choice0.width, choice0.height),
            (24.0, 160.0, 44.0, 28.0)
        );
        assert_eq!(
            (choice1.x, choice1.y, choice1.width, choice1.height),
            (70.0, 160.0, 44.0, 28.0)
        );
        assert_eq!(
            (choice2.x, choice2.y, choice2.width, choice2.height),
            (116.0, 160.0, 44.0, 28.0)
        );

        let swatch1 = rect_for_name(&reg, "DropSwatch_skin_1");
        let selection1 = rect_for_name(&reg, "DropSel_skin_1");
        assert_eq!(
            (swatch1.x, swatch1.y, swatch1.width, swatch1.height),
            (72.0, 164.0, 40.0, 20.0)
        );
        assert_eq!(
            (
                selection1.x,
                selection1.y,
                selection1.width,
                selection1.height
            ),
            (64.0, 160.0, 48.0, 28.0)
        );
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
}
