use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

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

impl fmt::Display for CharCreateAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectRace(id) => write!(f, "select_race:{id}"),
            Self::SelectClass(id) => write!(f, "select_class:{id}"),
            Self::ToggleSex => f.write_str("toggle_sex"),
            Self::NextMode => f.write_str("next_mode"),
            Self::Back => f.write_str("back"),
            Self::AppearanceInc(field) => write!(f, "appearance_inc:{}", field_str(*field)),
            Self::AppearanceDec(field) => write!(f, "appearance_dec:{}", field_str(*field)),
            Self::CreateConfirm => f.write_str("create_confirm"),
        }
    }
}

fn field_str(f: AppearanceField) -> &'static str {
    match f {
        AppearanceField::SkinColor => "skin",
        AppearanceField::Face => "face",
        AppearanceField::HairStyle => "hair_style",
        AppearanceField::HairColor => "hair_color",
        AppearanceField::FacialStyle => "facial",
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
    pub name: String,
    pub error_text: Option<String>,
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

// --- Constants ---

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);
const COLOR_WHITE: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);
const COLOR_DISABLED: FontColor = FontColor::new(0.4, 0.4, 0.4, 1.0);
const COLOR_SELECTED: FontColor = FontColor::new(1.0, 0.92, 0.72, 1.0);

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

struct DynName(String);

fn dyn_name(s: String) -> DynName {
    DynName(s)
}

// --- Race grid ---

fn race_button(race_id: u8, short_name: &str, name: &str, is_selected: bool) -> Element {
    let color = if is_selected { COLOR_SELECTED } else { COLOR_SUBTITLE };
    let border = if is_selected {
        "2px solid 1.0,0.82,0.0,1.0"
    } else {
        "1px solid 0.45,0.38,0.22,0.6"
    };
    let bg = if is_selected { "0.2,0.16,0.08,0.9" } else { "0.1,0.08,0.05,0.7" };
    rsx! {
        r#frame {
            name: dyn_name(format!("Race_{race_id}")),
            width: 52.0, height: 60.0,
            onclick: CharCreateAction::SelectRace(race_id),
            border: border, background_color: bg,
            fontstring {
                name: dyn_name(format!("Race_{race_id}_Short")),
                width: 44.0, height: 24.0,
                text: short_name,
                font: GameFont::FrizQuadrata, font_size: 16.0,
                font_color: color,
                anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, y: "-4" }
            }
            fontstring {
                name: dyn_name(format!("Race_{race_id}_Label")),
                width: 50.0, height: 16.0,
                text: name,
                font: GameFont::FrizQuadrata, font_size: 9.0,
                font_color: color,
                anchor { point: AnchorPoint::Bottom, relative_point: AnchorPoint::Bottom, y: "4" }
            }
        }
    }
}

fn faction_column(label: &str, col_name: &str, x_offset: &str, races: Element) -> Element {
    rsx! {
        fontstring {
            name: dyn_name(format!("{col_name}Label")),
            width: 140.0, height: 24.0,
            text: label,
            font: GameFont::FrizQuadrata, font_size: 16.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: x_offset, y: "-4",
            }
        }
        r#frame {
            name: dyn_name(format!("{col_name}Races")),
            width: 150.0, height: 400.0,
            layout: "flex-row-wrap", gap: 6.0,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: x_offset, y: "-30",
            }
            {races}
        }
    }
}

fn race_grid(selected_race: u8) -> Element {
    use crate::char_create_data::{Faction, RACES};
    let alliance: Element = RACES
        .iter()
        .filter(|r| r.faction == Faction::Alliance)
        .flat_map(|r| race_button(r.id, r.short_name, r.name, r.id == selected_race))
        .collect();
    let horde: Element = RACES
        .iter()
        .filter(|r| r.faction == Faction::Horde)
        .flat_map(|r| race_button(r.id, r.short_name, r.name, r.id == selected_race))
        .collect();
    rsx! {
        r#frame {
            name: "RaceGrid",
            width: 320.0, height: 500.0,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "20", y: "-80",
            }
            {faction_column("Alliance", "Alliance", "5", alliance)}
            {faction_column("Horde", "Horde", "165", horde)}
        }
    }
}

// --- Class grid ---

fn class_button_style(is_selected: bool, available: bool) -> (FontColor, &'static str, &'static str) {
    let color = if !available {
        COLOR_DISABLED
    } else if is_selected {
        COLOR_SELECTED
    } else {
        COLOR_SUBTITLE
    };
    let border = if is_selected && available {
        "2px solid 1.0,0.82,0.0,1.0"
    } else {
        "1px solid 0.45,0.38,0.22,0.4"
    };
    let bg = if is_selected && available { "0.2,0.16,0.08,0.9" } else { "0.1,0.08,0.05,0.7" };
    (color, border, bg)
}

fn class_button(class_id: u8, name: &str, icon: &str, is_selected: bool, available: bool) -> Element {
    let (color, border, bg) = class_button_style(is_selected, available);
    let onclick = if available {
        CharCreateAction::SelectClass(class_id).to_string()
    } else {
        String::new()
    };
    let alpha = if available { "1.0" } else { "0.3" };
    rsx! {
        r#frame {
            name: dyn_name(format!("Class_{class_id}")),
            width: 52.0, height: 60.0,
            onclick: onclick,
            border: border, background_color: bg,
            texture {
                name: dyn_name(format!("Class_{class_id}_Icon")),
                width: 36.0, height: 36.0,
                texture_file: icon,
                alpha: alpha,
                anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, y: "-2" }
            }
            fontstring {
                name: dyn_name(format!("Class_{class_id}_Label")),
                width: 50.0, height: 16.0,
                text: name,
                font: GameFont::FrizQuadrata, font_size: 9.0,
                font_color: color,
                anchor { point: AnchorPoint::Bottom, relative_point: AnchorPoint::Bottom, y: "2" }
            }
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
            width: 180.0, height: 500.0,
            layout: "flex-row-wrap", gap: 6.0,
            anchor {
                point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight,
                x: "-20", y: "-80",
            }
            fontstring {
                name: "ClassLabel",
                width: 160.0, height: 24.0,
                text: "Class",
                font: GameFont::FrizQuadrata, font_size: 16.0,
                font_color: COLOR_GOLD,
            }
            {classes}
        }
    }
}

// --- Appearance row helpers ---

fn appearance_dec_button(field: AppearanceField) -> Element {
    rsx! {
        button {
            name: dyn_name(format!("AppDec_{}", field_str(field))),
            width: 32.0, height: 28.0,
            text: "<", font_size: 14.0,
            onclick: CharCreateAction::AppearanceDec(field),
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right, x: "-70" }
        }
    }
}

fn appearance_inc_button(field: AppearanceField) -> Element {
    rsx! {
        button {
            name: dyn_name(format!("AppInc_{}", field_str(field))),
            width: 32.0, height: 28.0,
            text: ">", font_size: 14.0,
            onclick: CharCreateAction::AppearanceInc(field),
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right, x: "-10" }
        }
    }
}

fn appearance_row(label: &str, value: u8, field: AppearanceField) -> Element {
    let val_text = format!("{}", value + 1);
    rsx! {
        r#frame {
            name: dyn_name(format!("Appearance_{}", field_str(field))),
            width: 280.0, height: 32.0,
            fontstring {
                name: dyn_name(format!("AppLabel_{}", field_str(field))),
                width: 120.0, height: 24.0,
                text: label,
                font: GameFont::FrizQuadrata, font_size: 13.0,
                font_color: COLOR_SUBTITLE, justify_h: JustifyH::Left,
                anchor { point: AnchorPoint::Left, relative_point: AnchorPoint::Left, x: "10" }
            }
            {appearance_dec_button(field)}
            fontstring {
                name: dyn_name(format!("AppVal_{}", field_str(field))),
                width: 30.0, height: 24.0,
                text: val_text,
                font: GameFont::FrizQuadrata, font_size: 13.0,
                font_color: COLOR_WHITE,
                anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right, x: "-40" }
            }
            {appearance_inc_button(field)}
        }
    }
}

fn customize_panel(state: &CharCreateUiState) -> Element {
    rsx! {
        r#frame {
            name: "CustomizePanel",
            width: 300.0, height: 500.0,
            layout: "flex-col", gap: 8.0,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "20", y: "-80",
            }
            fontstring {
                name: "CustomizeLabel",
                width: 280.0, height: 24.0,
                text: "Customize Appearance",
                font: GameFont::FrizQuadrata, font_size: 16.0,
                font_color: COLOR_GOLD,
            }
            {appearance_row("Skin Color", state.skin_color, AppearanceField::SkinColor)}
            {appearance_row("Face", state.face, AppearanceField::Face)}
            {appearance_row("Hair Style", state.hair_style, AppearanceField::HairStyle)}
            {appearance_row("Hair Color", state.hair_color, AppearanceField::HairColor)}
            {appearance_row("Facial Style", state.facial_style, AppearanceField::FacialStyle)}
        }
    }
}

// --- Name input + create button ---

fn name_input_field() -> Element {
    rsx! {
        fontstring {
            name: "NameLabel",
            width: 300.0, height: 24.0,
            text: "Character Name",
            font: GameFont::FrizQuadrata, font_size: 14.0,
            font_color: COLOR_GOLD,
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top }
        }
        editbox {
            name: CREATE_NAME_INPUT,
            width: 300.0, height: 38.0,
            font_size: 16.0,
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, y: "-28" }
        }
    }
}

fn error_label(error_text: Option<&str>) -> Element {
    let error_hidden = error_text.is_none();
    let text = error_text.unwrap_or("");
    rsx! {
        fontstring {
            name: ERROR_TEXT,
            width: 300.0, height: 20.0,
            text: text,
            hidden: error_hidden,
            font: GameFont::FrizQuadrata, font_size: 12.0,
            font_color: FontColor::new(1.0, 0.2, 0.2, 1.0),
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, y: "-70" }
        }
    }
}

fn create_confirm_button() -> Element {
    rsx! {
        button {
            name: CREATE_BUTTON,
            width: 205.0, height: 42.0,
            text: "Create Character", font_size: 14.0,
            onclick: CharCreateAction::CreateConfirm,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, y: "-90" }
        }
    }
}

fn name_and_create(state: &CharCreateUiState) -> Element {
    rsx! {
        r#frame {
            name: "NamePanel",
            width: 400.0, height: 120.0,
            anchor { point: AnchorPoint::Bottom, relative_point: AnchorPoint::Bottom, y: "140" }
            {name_input_field()}
            {error_label(state.error_text.as_deref())}
            {create_confirm_button()}
        }
    }
}

// --- Bottom buttons ---

fn back_button() -> Element {
    rsx! {
        button {
            name: BACK_BUTTON,
            width: 188.0, height: 42.0,
            text: "Back", font_size: 14.0,
            onclick: CharCreateAction::Back,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomLeft, relative_point: AnchorPoint::BottomLeft,
                x: "12", y: "60",
            }
        }
    }
}

fn next_button(hidden: bool) -> Element {
    rsx! {
        button {
            name: NEXT_BUTTON,
            width: 188.0, height: 42.0,
            text: "Next", font_size: 14.0,
            hidden: hidden,
            onclick: CharCreateAction::NextMode,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomRight, relative_point: AnchorPoint::BottomRight,
                x: "-12", y: "60",
            }
        }
    }
}

fn sex_toggle_button() -> Element {
    rsx! {
        button {
            name: SEX_TOGGLE_BUTTON,
            width: 140.0, height: 42.0,
            text: "Toggle Sex", font_size: 14.0,
            onclick: CharCreateAction::ToggleSex,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor { point: AnchorPoint::Bottom, relative_point: AnchorPoint::Bottom, y: "60" }
        }
    }
}

fn bottom_buttons(mode: CharCreateMode) -> Element {
    let hide_next = mode != CharCreateMode::RaceClass;
    [back_button(), next_button(hide_next), sex_toggle_button()]
        .into_iter()
        .flatten()
        .collect()
}

// --- Title ---

fn title_area(state: &CharCreateUiState) -> Element {
    use crate::char_create_data::{class_by_id, race_by_id};
    let race_name = race_by_id(state.selected_race).map(|r| r.name).unwrap_or("Unknown");
    let class_name = class_by_id(state.selected_class).map(|c| c.name).unwrap_or("Unknown");
    let sex_str = if state.selected_sex == 0 { "Male" } else { "Female" };
    let title = format!("{sex_str} {race_name} {class_name}");
    rsx! {
        fontstring {
            name: "CharCreateTitle",
            width: 520.0, height: 36.0,
            text: title,
            font: GameFont::FrizQuadrata, font_size: 24.0,
            font_color: COLOR_GOLD,
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, y: "-30" }
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
        let state = CharCreateUiState::default();
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(state);
        let mut reg = crate::ui::registry::FrameRegistry::new(1920.0, 1080.0);
        let mut screen = ui_toolkit::screen::Screen::new(char_create_screen);
        screen.sync(&shared, &mut reg);
        assert!(reg.get_by_name("CharCreateRoot").is_some());
        assert!(reg.get_by_name("CharCreateBack").is_some());
    }

    #[test]
    fn customize_mode_shows_appearance_options() {
        let mut state = CharCreateUiState::default();
        state.mode = CharCreateMode::Customize;
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(state);
        let mut reg = crate::ui::registry::FrameRegistry::new(1920.0, 1080.0);
        let mut screen = ui_toolkit::screen::Screen::new(char_create_screen);
        screen.sync(&shared, &mut reg);
        assert!(reg.get_by_name("CustomizePanel").is_some());
        assert!(reg.get_by_name("CharCreateNameInput").is_some());
    }
}
