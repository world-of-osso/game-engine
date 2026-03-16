use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::campsite_component::{campsite_panel, campsite_tab};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharSelectAction {
    SelectChar(usize),
    EnterWorld,
    CreateToggle,
    DeleteChar,
    Back,
    CampsiteToggle,
    SelectCampsite(u32),
}

impl fmt::Display for CharSelectAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectChar(i) => write!(f, "select_char:{i}"),
            Self::EnterWorld => f.write_str("enter_world"),
            Self::CreateToggle => f.write_str("create_toggle"),
            Self::DeleteChar => f.write_str("delete_char"),
            Self::Back => f.write_str("back"),
            Self::CampsiteToggle => f.write_str("campsite_toggle"),
            Self::SelectCampsite(id) => write!(f, "select_campsite:{id}"),
        }
    }
}

impl CharSelectAction {
    pub fn parse(s: &str) -> Option<Self> {
        if let Some(idx_str) = s.strip_prefix("select_char:") {
            return idx_str.parse().ok().map(Self::SelectChar);
        }
        if let Some(id_str) = s.strip_prefix("select_campsite:") {
            return id_str.parse().ok().map(Self::SelectCampsite);
        }
        match s {
            "enter_world" => Some(Self::EnterWorld),
            "create_toggle" => Some(Self::CreateToggle),
            "delete_char" => Some(Self::DeleteChar),
            "back" => Some(Self::Back),
            "campsite_toggle" => Some(Self::CampsiteToggle),
            _ => None,
        }
    }
}

// --- Context types ---

pub struct CharSelectState {
    pub characters: Vec<CharDisplayEntry>,
    pub selected_index: Option<usize>,
    pub selected_name: String,
    pub status_text: String,
}

impl Default for CharSelectState {
    fn default() -> Self {
        Self {
            characters: Vec::new(),
            selected_index: None,
            selected_name: "Character Selection".to_string(),
            status_text: String::new(),
        }
    }
}

pub struct CharDisplayEntry {
    pub name: String,
    pub info: String,
    pub status: String,
}

pub struct CampsiteEntry {
    pub id: u32,
    pub name: String,
}

#[derive(Default)]
pub struct CampsiteState {
    pub scenes: Vec<CampsiteEntry>,
    pub panel_visible: bool,
    pub selected_id: Option<u32>,
}

// --- Frame names ---

pub const CHAR_SELECT_ROOT: FrameName = FrameName("CharSelectRoot");
pub const CHAR_LIST_PANEL: FrameName = FrameName("CharacterListPanel");
pub const ENTER_WORLD_BUTTON: FrameName = FrameName("EnterWorld");
pub const CREATE_CHAR_BUTTON: FrameName = FrameName("CreateChar");
pub const DELETE_CHAR_BUTTON: FrameName = FrameName("DeleteChar");
pub const BACK_BUTTON: FrameName = FrameName("BackToLogin");
pub const STATUS_TEXT: FrameName = FrameName("CSStatus");
pub const SELECTED_NAME_TEXT: FrameName = FrameName("CharSelectCharacterName");

// --- Constants ---

const REALM_NAME: &str = "World of Osso";

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);
const COLOR_MUTED: FontColor = FontColor::new(0.75, 0.72, 0.65, 1.0);

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const BIG_BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BIG_BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BIG_BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BIG_BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

const TOP_HUD_LEFT_ATLAS: &str = "glues-characterselect-tophud-left-bg";
const TOP_HUD_MIDDLE_ATLAS: &str = "glues-characterselect-tophud-middle-bg";
const TOP_HUD_RIGHT_ATLAS: &str = "glues-characterselect-tophud-right-bg";
const NAME_BG_ATLAS: &str = "custom-nameplate-bg";
const LIST_REALM_BG_ATLAS: &str = "glues-characterselect-listrealm-bg";
const CARD_BACKDROP_ATLAS: &str = "glues-characterselect-card-singles";
const CARD_SELECTED_ATLAS: &str = "glues-characterselect-card-selected";
const EMPTY_CARD_ATLAS: &str = "glues-characterselect-card-empty";
const CARD_BACKDROP_TINT: &str = "0.76,0.70,0.57,0.96";
const CARD_SELECTED_TINT: &str = "0.82,0.74,0.46,0.9";

// --- Card frame name helpers ---

/// Wrapper for dynamic frame names (the rsx! macro calls `.0.to_string()` on name exprs).
struct DynName(String);

pub fn card_frame_name(index: usize) -> String {
    format!("CharCard_{index}")
}

fn dyn_name(s: String) -> DynName {
    DynName(s)
}

fn card_selected_dyn(index: usize) -> DynName {
    DynName(format!("CharCard_{index}Selected"))
}

// --- Background & Logo ---

fn cs_background() -> Element {
    rsx! {
        r#frame {
            name: "CharSelectBackground",
            stretch: true,
            background_color: "0,0,0,0",
            strata: FrameStrata::Background,
        }
    }
}

fn cs_logo() -> Element {
    Vec::new()
}

// --- Top HUD banner (3 atlas pieces) ---

fn cs_top_hud() -> Element {
    rsx! {
        texture {
            name: "CharSelectTopHudLeft",
            width: 212.0,
            height: 51.0,
            texture_atlas: TOP_HUD_LEFT_ATLAS,
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::Top,
                x: "-15",
                y: "-22",
            }
        }
        texture {
            name: "CharSelectTopHudMiddle",
            width: 30.0,
            height: 51.0,
            texture_atlas: TOP_HUD_MIDDLE_ATLAS,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::Top,
                x: "-15",
                y: "-22",
            }
        }
        texture {
            name: "CharSelectTopHudRight",
            width: 212.0,
            height: 51.0,
            texture_atlas: TOP_HUD_RIGHT_ATLAS,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::Top,
                x: "15",
                y: "-22",
            }
        }
    }
}

// --- Name area (below top HUD) ---

fn cs_name_area(selected_name: &str, has_selection: bool) -> Element {
    let hide_name_bg = !has_selection;
    rsx! {
        texture {
            name: "CharSelectNameBg",
            width: 300.0,
            height: 60.0,
            texture_atlas: NAME_BG_ATLAS,
            hidden: hide_name_bg,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-80",
            }
        }
        fontstring {
            name: SELECTED_NAME_TEXT,
            width: 520.0,
            height: 36.0,
            text: selected_name,
            font: GameFont::FrizQuadrata,
            font_size: 27.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-90",
            }
        }
    }
}

// --- Character card ---

fn card_textures(index: usize, is_selected: bool) -> Element {
    let backdrop_name = dyn_name(format!("CharCard_{index}Backdrop"));
    let sel_name = card_selected_dyn(index);
    let hide_selected = !is_selected;
    rsx! {
        texture {
            name: backdrop_name,
            width: 310.0,
            height: 89.0,
            texture_atlas: CARD_BACKDROP_ATLAS,
            vertex_color: CARD_BACKDROP_TINT,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
        }
        texture {
            name: sel_name,
            width: 342.0,
            height: 122.0,
            texture_atlas: CARD_SELECTED_ATLAS,
            vertex_color: CARD_SELECTED_TINT,
            hidden: hide_selected,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "7",
                y: "14",
            }
        }
    }
}

fn card_name_label(index: usize, name: &str) -> Element {
    let label_name = dyn_name(format!("CharCard_{index}Name"));
    rsx! {
        fontstring {
            name: label_name,
            width: 260.0,
            height: 24.0,
            text: name,
            font: GameFont::FrizQuadrata,
            font_size: 24.0,
            font_color: COLOR_GOLD,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "40",
                y: "-16",
            }
        }
    }
}

fn card_info_label(index: usize, info: &str) -> Element {
    let label_name = dyn_name(format!("CharCard_{index}Info"));
    rsx! {
        fontstring {
            name: label_name,
            width: 260.0,
            height: 18.0,
            text: info,
            font: GameFont::FrizQuadrata,
            font_size: 15.0,
            font_color: COLOR_SUBTITLE,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "40",
                y: "-43",
            }
        }
    }
}

fn card_status_label(index: usize, status: &str) -> Element {
    let label_name = dyn_name(format!("CharCard_{index}Status"));
    rsx! {
        fontstring {
            name: label_name,
            width: 240.0,
            height: 18.0,
            text: status,
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: COLOR_MUTED,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "40",
                y: "-67",
            }
        }
    }
}

fn character_card(index: usize, ch: &CharDisplayEntry, is_selected: bool) -> Element {
    let frame_name = dyn_name(card_frame_name(index));
    let onclick = CharSelectAction::SelectChar(index);
    let texts = [
        card_name_label(index, &ch.name),
        card_info_label(index, &ch.info),
        card_status_label(index, &ch.status),
    ]
    .into_iter()
    .flatten()
    .collect::<Element>();

    rsx! {
        r#frame {
            name: frame_name,
            width: 347.0,
            height: 95.0,
            onclick,
            {card_textures(index, is_selected)}
            {texts}
        }
    }
}

// --- Empty card ---

fn empty_card() -> Element {
    rsx! {
        r#frame {
            name: "CharSelectEmptyCard",
            width: 347.0,
            height: 95.0,
            onclick: CharSelectAction::CreateToggle,
            texture {
                name: "CharSelectEmptyCardBackdrop",
                width: 316.0,
                height: 95.0,
                texture_atlas: EMPTY_CARD_ATLAS,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "20",
                }
            }
        }
    }
}

// --- Character list panel ---

fn list_backdrop() -> Element {
    Vec::new()
}

fn list_realm_header() -> Element {
    rsx! {
        texture {
            name: "CharacterListRealmBackdrop",
            width: 281.0,
            height: 23.0,
            texture_atlas: LIST_REALM_BG_ATLAS,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "52",
                y: "-16",
            }
        }
        fontstring {
            name: "CharacterListRealmLabel",
            width: 281.0,
            height: 28.0,
            text: REALM_NAME,
            font: GameFont::FrizQuadrata,
            font_size: 20.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "50",
                y: "-14",
            }
        }
    }
}

fn list_helper_and_divider() -> Element {
    rsx! {
        fontstring {
            name: "CharacterListHelperText",
            width: 346.0,
            height: 18.0,
            text: "Select a character to enter the world",
            font: GameFont::FrizQuadrata,
            font_size: 13.0,
            font_color: COLOR_MUTED,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "20",
                y: "-51",
            }
        }
        r#frame {
            name: "CharacterListDivider",
            width: 346.0,
            height: 1.0,
            background_color: "1.0,0.9,0.65,0.12",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "20",
                y: "-80",
            }
        }
    }
}

fn card_list(characters: &[CharDisplayEntry], selected: Option<usize>) -> Element {
    let cards: Element = characters
        .iter()
        .enumerate()
        .flat_map(|(i, ch)| character_card(i, ch, selected == Some(i)))
        .collect();
    let empty = if characters.is_empty() {
        empty_card()
    } else {
        Vec::new()
    };
    rsx! {
        r#frame {
            name: "CharacterListCards",
            width: 347.0,
            height: 420.0,
            layout: "flex-col",
            gap: 10.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "19",
                y: "-94",
            }
            {cards}
            {empty}
        }
    }
}

fn cs_character_list(characters: &[CharDisplayEntry], selected: Option<usize>) -> Element {
    let chrome = [
        list_backdrop(),
        list_realm_header(),
        list_helper_and_divider(),
    ]
    .into_iter()
    .flatten()
    .collect::<Element>();
    rsx! {
        r#frame { name: CHAR_LIST_PANEL, width: 386.0, height: 520.0,
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-22",
                y: "-164",
            }
            {chrome}
            {card_list(characters, selected)}
        }
    }
}

// --- Action buttons ---

fn enter_world_button() -> Element {
    rsx! {
        button {
            name: ENTER_WORLD_BUTTON,
            width: 256.0,
            height: 64.0,
            text: "Enter World",
            font_size: 18.0,
            onclick: CharSelectAction::EnterWorld,
            button_atlas_up: BIG_BUTTON_ATLAS_UP,
            button_atlas_pressed: BIG_BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BIG_BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BIG_BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "111",
            }
        }
    }
}

fn create_char_button() -> Element {
    rsx! {
        button {
            name: CREATE_CHAR_BUTTON,
            width: 205.0,
            height: 42.0,
            text: "Create New Character",
            font_size: 14.0,
            onclick: CharSelectAction::CreateToggle,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomLeft,
                relative_to: CHAR_LIST_PANEL,
                relative_point: AnchorPoint::BottomLeft,
                x: "18",
                y: "64",
            }
        }
    }
}

fn delete_char_button() -> Element {
    rsx! {
        button {
            name: DELETE_CHAR_BUTTON,
            width: 46.0,
            height: 42.0,
            text: "X",
            font_size: 18.0,
            onclick: CharSelectAction::DeleteChar,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomRight,
                relative_to: CHAR_LIST_PANEL,
                relative_point: AnchorPoint::BottomRight,
                x: "-18",
                y: "64",
            }
        }
    }
}

fn back_button() -> Element {
    rsx! {
        button {
            name: BACK_BUTTON,
            width: 188.0,
            height: 42.0,
            text: "Back",
            font_size: 14.0,
            onclick: CharSelectAction::Back,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomLeft,
                relative_point: AnchorPoint::BottomLeft,
                x: "12",
                y: "60",
            }
        }
    }
}

fn cs_action_buttons(has_selection: bool) -> Element {
    let delete_button: Element = if has_selection {
        delete_char_button()
    } else {
        Vec::new()
    };

    [
        enter_world_button(),
        create_char_button(),
        delete_button,
        back_button(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

// --- Status text ---

fn cs_status(text: &str) -> Element {
    rsx! {
        fontstring {
            name: STATUS_TEXT,
            width: 720.0,
            height: 24.0,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 13.0,
            font_color: COLOR_SUBTITLE,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "188",
            }
        }
    }
}

// --- Main screen ---

pub fn char_select_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CharSelectState>()
        .expect("CharSelectState must be in SharedContext");
    let campsite = ctx.get::<CampsiteState>();
    let has_selection = state.selected_index.is_some();
    let top_hud: Element = if campsite.is_some() {
        Vec::new()
    } else {
        cs_top_hud()
    };

    let campsite_ui: Element = if let Some(cs) = &campsite {
        [campsite_tab(cs.panel_visible), campsite_panel(cs)]
            .into_iter()
            .flatten()
            .collect()
    } else {
        Vec::new()
    };

    rsx! {
        r#frame { name: CHAR_SELECT_ROOT, strata: FrameStrata::Background,
            {cs_background()}
            {cs_logo()}
            {top_hud}
            {cs_name_area(&state.selected_name, has_selection)}
            {cs_character_list(&state.characters, state.selected_index)}
            {cs_action_buttons(has_selection)}
            {cs_status(&state.status_text)}
            {campsite_ui}
        }
    }
}
