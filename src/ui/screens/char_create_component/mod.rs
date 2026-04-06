mod appearance_widgets;
mod char_create_widgets;

use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

use appearance_widgets::{customization_row, dropdown_panel};
use char_create_widgets::{
    bottom_buttons, class_button, create_confirm_button, error_label, faction_column,
    name_input_field, race_buttons_for_faction,
};

// --- Actions ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharCreateAction {
    SelectRace(u8),
    SelectClass(u8),
    ToggleSex,
    Randomize,
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
            Self::Randomize => f.write_str("randomize"),
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
            "randomize" => Some(Self::Randomize),
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
    pub face_label: String,
    pub hair_style_label: String,
    pub facial_style_label: String,
    pub skin_color_swatches: Vec<Option<[u8; 3]>>,
    pub hair_color_swatches: Vec<Option<[u8; 3]>>,
    pub open_dropdown: Option<AppearanceField>,
    pub name: String,
    pub error_text: Option<String>,
    pub name_input_focused: bool,
    /// (class_id, class_name, icon_file, available_for_race)
    pub class_availability: Vec<(u8, &'static str, &'static str, bool)>,
}

impl Default for CharCreateUiState {
    fn default() -> Self {
        use crate::char_create_data::{CLASSES, race_can_be_class};
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
            face_label: String::new(),
            hair_style_label: String::new(),
            facial_style_label: String::new(),
            skin_color_swatches: Vec::new(),
            hair_color_swatches: Vec::new(),
            open_dropdown: None,
            name: String::new(),
            error_text: None,
            name_input_focused: false,
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
pub const RANDOMIZE_BUTTON: FrameName = FrameName("CharCreateRandomize");
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
        {skin_color_row(state)}
        {face_row(state)}
        {hair_style_row(state)}
        {hair_color_row(state)}
        {facial_style_row(state)}
    }
}

fn skin_color_row(state: &CharCreateUiState) -> Element {
    customization_row(
        "Skin Color",
        state.skin_color,
        "",
        &state.skin_color_swatches,
        AppearanceField::SkinColor,
    )
}

fn face_row(state: &CharCreateUiState) -> Element {
    customization_row(
        "Face",
        state.face,
        &state.face_label,
        &[],
        AppearanceField::Face,
    )
}

fn hair_style_row(state: &CharCreateUiState) -> Element {
    customization_row(
        "Hair Style",
        state.hair_style,
        &state.hair_style_label,
        &[],
        AppearanceField::HairStyle,
    )
}

fn hair_color_row(state: &CharCreateUiState) -> Element {
    customization_row(
        "Hair Color",
        state.hair_color,
        "",
        &state.hair_color_swatches,
        AppearanceField::HairColor,
    )
}

fn facial_style_row(state: &CharCreateUiState) -> Element {
    customization_row(
        "Facial Style",
        state.facial_style,
        &state.facial_style_label,
        &[],
        AppearanceField::FacialStyle,
    )
}

// row_height(44) + gap(8) = 52 per row; dropdowns should open below the active row.
fn active_dropdown(state: &CharCreateUiState) -> Element {
    match state.open_dropdown {
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
    }
}

fn customize_panel(state: &CharCreateUiState) -> Element {
    let dropdown = active_dropdown(state);
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
            {name_input_field(state.name_input_focused)}
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
#[path = "mod_tests.rs"]
mod tests;
