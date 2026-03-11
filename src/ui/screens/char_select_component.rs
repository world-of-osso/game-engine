use ui_toolkit::rsx;
use ui_toolkit::screen::ScreenContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

// --- Context types ---

pub struct CharSelectState {
    pub characters: Vec<CharDisplayEntry>,
    pub selected_index: Option<usize>,
    pub create_panel_visible: bool,
    pub selected_name: String,
    pub status_text: String,
}

impl Default for CharSelectState {
    fn default() -> Self {
        Self {
            characters: Vec::new(),
            selected_index: None,
            create_panel_visible: false,
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

// --- Frame names ---

pub const CHAR_SELECT_ROOT: FrameName = FrameName("CharSelectRoot");
pub const CHAR_LIST_PANEL: FrameName = FrameName("CharacterListPanel");
pub const ENTER_WORLD_BUTTON: FrameName = FrameName("EnterWorld");
pub const CREATE_CHAR_BUTTON: FrameName = FrameName("CreateChar");
pub const DELETE_CHAR_BUTTON: FrameName = FrameName("DeleteChar");
pub const BACK_BUTTON: FrameName = FrameName("BackToLogin");
pub const CREATE_PANEL: FrameName = FrameName("CreatePanel");
pub const CREATE_NAME_INPUT: FrameName = FrameName("CreateNameInput");
pub const CREATE_CONFIRM_BUTTON: FrameName = FrameName("CreateConfirm");
pub const STATUS_TEXT: FrameName = FrameName("CSStatus");
pub const SELECTED_NAME_TEXT: FrameName = FrameName("CharSelectCharacterName");

// --- Constants ---

const TEX_GAME_LOGO: &str = "data/glues/common/Glues-WoW-TheWarWithinLogo.blp";
const REALM_NAME: &str = "World of Osso";

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);
const COLOR_MUTED: FontColor = FontColor::new(0.75, 0.72, 0.65, 1.0);

const BUTTON_ATLAS_UP: &str = "128-redbutton-up";
const BUTTON_ATLAS_PRESSED: &str = "128-redbutton-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "128-redbutton-highlight";
const BUTTON_ATLAS_DISABLED: &str = "128-redbutton-disable";
const BIG_BUTTON_ATLAS_UP: &str = "glue-bigbutton-brown-up";
const BIG_BUTTON_ATLAS_PRESSED: &str = "glue-bigbutton-brown-down";
const BIG_BUTTON_ATLAS_HIGHLIGHT: &str = "glue-bigbutton-brown-highlight";
const BIG_BUTTON_ATLAS_DISABLED: &str = "glue-bigbutton-brown-disable";

const TOP_HUD_LEFT_ATLAS: &str = "glues-characterselect-gs-tophud-left";
const TOP_HUD_MIDDLE_ATLAS: &str = "glues-characterselect-gs-tophud-middle";
const TOP_HUD_RIGHT_ATLAS: &str = "glues-characterselect-gs-tophud-right";
const TOP_HUD_LEFT_SELECTED: &str = "glues-characterselect-gs-tophud-left-selected";
const TOP_HUD_MIDDLE_SELECTED: &str = "glues-characterselect-gs-tophud-middle-selected";
const TOP_HUD_RIGHT_SELECTED: &str = "glues-characterselect-gs-tophud-right-selected";
const NAME_BG_ATLAS: &str = "glues-characterselect-namebg";
const LIST_BG_ATLAS: &str = "glues-characterselect-card-all-bg";
const LIST_REALM_BG_ATLAS: &str = "glues-characterselect-listrealm-bg";
const CARD_BACKDROP_ATLAS: &str = "glues-characterselect-card-singles";
const CARD_SELECTED_ATLAS: &str = "glues-characterselect-card-selected";
const EMPTY_CARD_ATLAS: &str = "glues-characterselect-card-empty";

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
            background_color: "0.01,0.01,0.01,1.0",
            strata: FrameStrata::Background,
        }
    }
}

fn cs_logo() -> Element {
    rsx! {
        texture {
            name: "CharSelectLogo",
            width: 256.0, height: 128.0,
            texture_file: TEX_GAME_LOGO,
            strata: FrameStrata::High,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "3", y: "-32",
            }
        }
    }
}

// --- Top HUD banner (3 atlas pieces) ---

fn cs_hud_left(atlas: &str) -> Element {
    rsx! {
        texture {
            name: "CharSelectTopHudLeft",
            width: 220.0, height: 43.0,
            texture_atlas: atlas,
            anchor {
                point: AnchorPoint::TopRight, relative_point: AnchorPoint::Top,
                x: "-59", y: "-22",
            }
        }
    }
}

fn cs_hud_middle(atlas: &str) -> Element {
    rsx! {
        texture {
            name: "CharSelectTopHudMiddle",
            width: 118.0, height: 43.0,
            texture_atlas: atlas,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::Top,
                x: "-59", y: "-22",
            }
        }
    }
}

fn cs_hud_right(atlas: &str) -> Element {
    rsx! {
        texture {
            name: "CharSelectTopHudRight",
            width: 220.0, height: 43.0,
            texture_atlas: atlas,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::Top,
                x: "59", y: "-22",
            }
        }
    }
}

fn cs_top_hud(has_selection: bool) -> Element {
    let (l, m, r) = if has_selection {
        (TOP_HUD_LEFT_SELECTED, TOP_HUD_MIDDLE_SELECTED, TOP_HUD_RIGHT_SELECTED)
    } else {
        (TOP_HUD_LEFT_ATLAS, TOP_HUD_MIDDLE_ATLAS, TOP_HUD_RIGHT_ATLAS)
    };
    [cs_hud_left(l), cs_hud_middle(m), cs_hud_right(r)]
        .into_iter()
        .flatten()
        .collect()
}

// --- Name area (below top HUD) ---

fn cs_name_area(selected_name: &str, has_selection: bool) -> Element {
    let hide_name_bg = !has_selection;
    rsx! {
        texture {
            name: "CharSelectNameBg",
            width: 194.0, height: 61.0,
            texture_atlas: NAME_BG_ATLAS,
            hidden: hide_name_bg,
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, y: "-43" }
        }
        fontstring {
            name: SELECTED_NAME_TEXT,
            width: 520.0, height: 36.0,
            text: selected_name,
            font: GameFont::FrizQuadrata, font_size: 27.0,
            font_color: COLOR_GOLD,
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, y: "-50" }
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
            width: 310.0, height: 89.0,
            texture_atlas: CARD_BACKDROP_ATLAS,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "20", y: "-3",
            }
        }
        texture {
            name: sel_name,
            width: 342.0, height: 122.0,
            texture_atlas: CARD_SELECTED_ATLAS,
            hidden: hide_selected,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "7", y: "11",
            }
        }
    }
}

fn card_name_label(index: usize, name: &str) -> Element {
    let label_name = dyn_name(format!("CharCard_{index}Name"));
    rsx! {
        fontstring {
            name: label_name,
            width: 260.0, height: 24.0,
            text: name,
            font: GameFont::FrizQuadrata, font_size: 24.0,
            font_color: COLOR_GOLD, justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "40", y: "-16",
            }
        }
    }
}

fn card_info_label(index: usize, info: &str) -> Element {
    let label_name = dyn_name(format!("CharCard_{index}Info"));
    rsx! {
        fontstring {
            name: label_name,
            width: 260.0, height: 18.0,
            text: info,
            font: GameFont::FrizQuadrata, font_size: 15.0,
            font_color: COLOR_SUBTITLE, justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "40", y: "-43",
            }
        }
    }
}

fn card_status_label(index: usize, status: &str) -> Element {
    let label_name = dyn_name(format!("CharCard_{index}Status"));
    rsx! {
        fontstring {
            name: label_name,
            width: 240.0, height: 18.0,
            text: status,
            font: GameFont::FrizQuadrata, font_size: 14.0,
            font_color: COLOR_MUTED, justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "40", y: "-67",
            }
        }
    }
}

fn character_card(index: usize, ch: &CharDisplayEntry, is_selected: bool) -> Element {
    let frame_name = dyn_name(card_frame_name(index));
    let onclick = format!("select_char:{index}");
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
            width: 347.0, height: 95.0,
            onclick: onclick,
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
            width: 347.0, height: 95.0,
            onclick: "create_toggle",
            texture {
                name: "CharSelectEmptyCardBackdrop",
                width: 316.0, height: 95.0,
                texture_atlas: EMPTY_CARD_ATLAS,
                anchor {
                    point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                    x: "20",
                }
            }
        }
    }
}

// --- Character list panel ---

fn list_backdrop() -> Element {
    rsx! {
        texture {
            name: "CharacterListBackdrop",
            width: 386.0, height: 520.0,
            texture_atlas: LIST_BG_ATLAS,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn list_realm_header() -> Element {
    rsx! {
        texture {
            name: "CharacterListRealmBackdrop",
            width: 281.0, height: 23.0,
            texture_atlas: LIST_REALM_BG_ATLAS,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "52", y: "-16",
            }
        }
        fontstring {
            name: "CharacterListRealmLabel",
            width: 281.0, height: 28.0,
            text: REALM_NAME,
            font: GameFont::FrizQuadrata, font_size: 20.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "50", y: "-14",
            }
        }
    }
}

fn list_helper_and_divider() -> Element {
    rsx! {
        fontstring {
            name: "CharacterListHelperText",
            width: 346.0, height: 18.0,
            text: "Select a character to enter the world",
            font: GameFont::FrizQuadrata, font_size: 13.0,
            font_color: COLOR_MUTED,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "20", y: "-51",
            }
        }
        r#frame {
            name: "CharacterListDivider",
            width: 346.0, height: 1.0,
            background_color: "1.0,0.9,0.65,0.12",
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "20", y: "-80",
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
    let empty = if characters.is_empty() { empty_card() } else { Vec::new() };
    rsx! {
        r#frame {
            name: "CharacterListCards",
            width: 347.0, height: 420.0,
            layout: "flex-col", gap: 1.0,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "19", y: "-94",
            }
            {cards}
            {empty}
        }
    }
}

fn cs_character_list(characters: &[CharDisplayEntry], selected: Option<usize>) -> Element {
    let chrome = [list_backdrop(), list_realm_header(), list_helper_and_divider()]
        .into_iter()
        .flatten()
        .collect::<Element>();
    rsx! {
        r#frame {
            name: CHAR_LIST_PANEL,
            width: 386.0, height: 520.0,
            anchor {
                point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight,
                x: "-22", y: "-164",
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
            width: 256.0, height: 64.0,
            text: "Enter World", font_size: 18.0,
            onclick: "enter_world",
            button_atlas_up: BIG_BUTTON_ATLAS_UP,
            button_atlas_pressed: BIG_BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BIG_BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BIG_BUTTON_ATLAS_DISABLED,
            anchor { point: AnchorPoint::Bottom, relative_point: AnchorPoint::Bottom, y: "111" }
        }
    }
}

fn create_char_button() -> Element {
    rsx! {
        button {
            name: CREATE_CHAR_BUTTON,
            width: 205.0, height: 42.0,
            text: "Create New Character", font_size: 14.0,
            onclick: "create_toggle",
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomLeft, relative_to: CHAR_LIST_PANEL,
                relative_point: AnchorPoint::BottomLeft,
                x: "18", y: "64",
            }
        }
    }
}

fn delete_char_button() -> Element {
    rsx! {
        button {
            name: DELETE_CHAR_BUTTON,
            width: 128.0, height: 42.0,
            text: "Delete", font_size: 14.0,
            onclick: "delete_char",
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomRight, relative_to: CHAR_LIST_PANEL,
                relative_point: AnchorPoint::BottomRight,
                x: "-18", y: "64",
            }
        }
    }
}

fn back_button() -> Element {
    rsx! {
        button {
            name: BACK_BUTTON,
            width: 188.0, height: 42.0,
            text: "Back", font_size: 14.0,
            onclick: "back",
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

fn cs_action_buttons() -> Element {
    [enter_world_button(), create_char_button(), delete_char_button(), back_button()]
        .into_iter()
        .flatten()
        .collect()
}

// --- Create character panel ---

fn create_panel_labels() -> Element {
    rsx! {
        fontstring {
            name: "CreateNameLabel",
            width: 300.0, height: 24.0,
            text: "Create New Character",
            font: GameFont::FrizQuadrata, font_size: 18.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "16", y: "-18",
            }
        }
        fontstring {
            name: "CreateNameSubtitle",
            width: 300.0, height: 18.0,
            text: "Enter a name for your new adventurer",
            font: GameFont::FrizQuadrata, font_size: 12.0,
            font_color: COLOR_MUTED,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "16", y: "-46",
            }
        }
    }
}

fn create_panel_input() -> Element {
    rsx! {
        editbox {
            name: CREATE_NAME_INPUT,
            width: 300.0, height: 38.0,
            font_size: 16.0,
            anchor {
                point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft,
                x: "16", y: "-74",
            }
        }
        button {
            name: CREATE_CONFIRM_BUTTON,
            width: 205.0, height: 42.0,
            text: "Create Character", font_size: 14.0,
            onclick: "create_confirm",
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, y: "-118" }
        }
    }
}

fn cs_create_panel(visible: bool) -> Element {
    let hide = !visible;
    rsx! {
        r#frame {
            name: CREATE_PANEL,
            width: 332.0, height: 164.0,
            hidden: hide,
            nine_slice: "12.0,0.03,0.03,0.04,0.94,0.65,0.48,0.16,1.0",
            anchor { point: AnchorPoint::Center, relative_point: AnchorPoint::Center, y: "-40" }
            {create_panel_labels()}
            {create_panel_input()}
        }
    }
}

// --- Status text ---

fn cs_status(text: &str) -> Element {
    rsx! {
        fontstring {
            name: STATUS_TEXT,
            width: 720.0, height: 24.0,
            text: text,
            font: GameFont::FrizQuadrata, font_size: 13.0,
            font_color: COLOR_SUBTITLE,
            anchor { point: AnchorPoint::Bottom, relative_point: AnchorPoint::Bottom, y: "188" }
        }
    }
}

// --- Main screen ---

pub fn char_select_screen(ctx: &ScreenContext) -> Element {
    let state = ctx
        .get::<CharSelectState>()
        .expect("CharSelectState must be in ScreenContext");
    let has_selection = state.selected_index.is_some();

    rsx! {
        r#frame { name: CHAR_SELECT_ROOT, strata: FrameStrata::Fullscreen,
            {cs_background()}
            {cs_logo()}
            {cs_top_hud(has_selection)}
            {cs_name_area(&state.selected_name, has_selection)}
            {cs_character_list(&state.characters, state.selected_index)}
            {cs_action_buttons()}
            {cs_create_panel(state.create_panel_visible)}
            {cs_status(&state.status_text)}
        }
    }
}
