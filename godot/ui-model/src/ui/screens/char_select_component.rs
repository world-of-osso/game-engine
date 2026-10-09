use std::fmt;

use crate::minimal_scroll_bar::{
    BAR_W, MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr,
};
use ui_toolkit::frame::{Dimension, NineSlice};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::TextureSource;

pub use super::char_select_delete_confirm_component::{
    DELETE_CANCEL_BUTTON, DELETE_CONFIRM_BUTTON, DELETE_CONFIRM_DELAY_SECS, DELETE_CONFIRM_DIALOG,
    DELETE_CONFIRM_INPUT, DeleteCharacterTarget, DeleteConfirmUiState, DeleteConfirmation,
    delete_confirmation_modal,
};
use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::campsite_component::campsite_panel;
use super::char_select_top_nav_component::{char_select_top_nav, sync_top_nav_tabs};
use super::default_button_atlas::{
    DISABLED as BUTTON_ATLAS_DISABLED, HIGHLIGHT as BUTTON_ATLAS_HIGHLIGHT,
    PRESSED as BUTTON_ATLAS_PRESSED, UP as BUTTON_ATLAS_UP,
};
use super::trash_button_component::trash_icon_button;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharSelectAction {
    SelectChar(usize),
    EnterWorld,
    CreateToggle,
    DeleteChar,
    ConfirmDeleteChar,
    CancelDeleteChar,
    Back,
    Menu,
    CampsiteToggle,
    SelectCampsite(u32),
    CampsitePage(usize),
}

impl fmt::Display for CharSelectAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectChar(i) => write!(f, "select_char:{i}"),
            Self::EnterWorld => f.write_str("enter_world"),
            Self::CreateToggle => f.write_str("create_toggle"),
            Self::DeleteChar => f.write_str("delete_char"),
            Self::ConfirmDeleteChar => f.write_str("confirm_delete_char"),
            Self::CancelDeleteChar => f.write_str("cancel_delete_char"),
            Self::Back => f.write_str("back"),
            Self::Menu => f.write_str("menu"),
            Self::CampsiteToggle => f.write_str("campsite_toggle"),
            Self::SelectCampsite(id) => write!(f, "select_campsite:{id}"),
            Self::CampsitePage(page) => write!(f, "campsite_page:{page}"),
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
        if let Some(page) = s.strip_prefix("campsite_page:") {
            return page.parse().ok().map(Self::CampsitePage);
        }
        match s {
            "enter_world" => Some(Self::EnterWorld),
            "create_toggle" => Some(Self::CreateToggle),
            "delete_char" => Some(Self::DeleteChar),
            "confirm_delete_char" => Some(Self::ConfirmDeleteChar),
            "cancel_delete_char" => Some(Self::CancelDeleteChar),
            "back" => Some(Self::Back),
            "menu" => Some(Self::Menu),
            "campsite_toggle" => Some(Self::CampsiteToggle),
            _ => None,
        }
    }
}

// --- Context types ---

#[derive(PartialEq)]
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

#[derive(PartialEq)]
pub struct CharDisplayEntry {
    pub name: String,
    pub info: String,
    pub status: String,
}

#[derive(Clone, PartialEq)]
pub struct CampsiteEntry {
    pub id: u32,
    pub name: String,
    pub preview_image: Option<CampsitePreview>,
}

/// Campsite card art: the scene texture kit's atlas member (texture FileDataID and its
/// normalized `[left, right, top, bottom]` crop).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CampsitePreview {
    pub fdid: u32,
    pub tex_coords: [f32; 4],
}

#[derive(Clone, Default, PartialEq)]
pub struct CampsiteState {
    pub scenes: Vec<CampsiteEntry>,
    pub panel_visible: bool,
    pub selected_id: Option<u32>,
    /// Zero-based campsite page; the panel clamps it to the available pages.
    pub page: usize,
}

// --- Frame names ---

pub const CHAR_SELECT_ROOT: FrameName = FrameName("CharSelectRoot");

pub fn size_char_select_root(registry: &mut FrameRegistry) {
    let (width, height) = (registry.screen_width, registry.screen_height);
    if let Some(root) = registry.get_by_name(CHAR_SELECT_ROOT.0)
        && let Some(frame) = registry.get_mut(root)
    {
        frame.width = Dimension::Fixed(width);
        frame.height = Dimension::Fixed(height);
    }
}
/// Original Up/Down roster navigation: wraps, and starts from the far end when unselected.
pub fn step_selection(selected: Option<usize>, count: usize, forward: bool) -> Option<usize> {
    if count == 0 {
        return selected;
    }
    Some(match (selected, forward) {
        (Some(index), true) if index + 1 < count => index + 1,
        (Some(_) | None, true) => 0,
        (Some(0) | None, false) => count - 1,
        (Some(index), false) => index - 1,
    })
}

const LIST_PANEL_BG_ATLAS: &str = "glues-characterselect-card-all-bg";

/// Chrome applied after every sync: viewport-sized root, character list backdrop and
/// top-navigation tab states.
pub fn apply_char_select_postsetup(registry: &mut FrameRegistry) {
    size_char_select_root(registry);
    sync_top_nav_tabs(registry);
    if let Some(id) = registry.get_by_name(CHAR_LIST_PANEL.0)
        && let Some(frame) = registry.get_mut(id)
    {
        frame.nine_slice = atlas_nine_slice(LIST_PANEL_BG_ATLAS);
    }
}

/// Nine-slice of an atlas region drawn with its authored margins at native size.
pub fn atlas_nine_slice(name: &str) -> Option<NineSlice> {
    let edges = ui_toolkit::atlas::nine_slice_margins(name)?;
    Some(NineSlice {
        edge_size: edges[0],
        edge_size_v: Some(edges[1]),
        edge_sizes: Some(edges),
        uv_edge_size: Some(edges[0]),
        uv_edge_sizes: Some(edges),
        texture: Some(TextureSource::Atlas(name.to_string())),
        bg_color: [1.0, 1.0, 1.0, 1.0],
        border_color: [1.0, 1.0, 1.0, 1.0],
        ..Default::default()
    })
}

pub const CHAR_LIST_PANEL: FrameName = FrameName("CharacterListPanel");
pub const CHARACTER_LIST_SCROLL: &str = "CharacterListCards";
pub const CHARACTER_CARD_HEIGHT: f32 = 95.0;
const CARD_GAP: f32 = 2.0; // Retail CharacterSelectList.lua:193.
const LIST_HEIGHT: f32 = 343.0; // Panel520 - header94 - Retail footer reservation83.
const BAR_LEFT: f32 = 351.0;

/// Retail CharacterSelectList.lua:198 and CharacterSelectListUtil.lua: character-height pan.
pub const CHARACTER_LIST_PAN: usize = (CHARACTER_CARD_HEIGHT + CARD_GAP) as usize;

#[derive(PartialEq)]
struct ScrolledSelection(Option<usize>);

/// Scroll only when selection changes, not when a wheel/drag rebuilds the same selection.
pub fn sync_char_select_screen(
    screen: &mut Screen,
    shared: &mut SharedContext,
    registry: &mut FrameRegistry,
) {
    screen.sync(shared, registry);
    let selected = shared.get::<CharSelectState>().and_then(|state| {
        state
            .selected_index
            .filter(|index| *index < state.characters.len())
    });
    if shared
        .get::<ScrolledSelection>()
        .is_some_and(|previous| previous.0 == selected)
    {
        return;
    }
    shared.insert(ScrolledSelection(selected));
    let Some(index) = selected else { return };
    let Some(scroll) = registry.scroll_lists.get(CHARACTER_LIST_SCROLL) else {
        return;
    };
    let offset = scroll.first_row;
    let top = index * (CHARACTER_CARD_HEIGHT + CARD_GAP) as usize;
    let bottom = top + CHARACTER_CARD_HEIGHT as usize;
    let target = if top < offset {
        top
    } else {
        offset.max(bottom.saturating_sub(LIST_HEIGHT as usize))
    };
    if registry
        .scroll_lists
        .scroll_to(CHARACTER_LIST_SCROLL, target)
    {
        screen.sync(shared, registry);
    }
}
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

const NAME_BG_ATLAS: &str = "custom-nameplate-bg";
const LIST_REALM_BG_ATLAS: &str = "glues-characterselect-listrealm-bg";
const CARD_BACKDROP_ATLAS: &str = "glues-characterselect-card-singles";
const CARD_SELECTED_ATLAS: &str = "glues-characterselect-card-selected";
const EMPTY_CARD_ATLAS: &str = "glues-characterselect-card-empty";
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

// --- Name area (below top navigation) ---

fn cs_name_area(selected_name: &str, has_selection: bool) -> Element {
    let hide_name_bg = !has_selection;
    rsx! {
        texture {
            name: "CharSelectNameBg",
            width: 300.0,
            height: 60.0,
            texture_atlas: NAME_BG_ATLAS,
            hidden: hide_name_bg,
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
            margin_top: {80.0},
        }
        fontstring {
            name: SELECTED_NAME_TEXT,
            width: 520.0,
            height: 36.0,
            text: selected_name,
            font: GameFont::FrizQuadrata,
            font_size: 27.0,
            font_color: COLOR_GOLD,
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
            margin_top: {90.0},
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
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
        }
        texture {
            name: sel_name,
            width: 342.0,
            height: 122.0,
            texture_atlas: CARD_SELECTED_ATLAS,
            vertex_color: CARD_SELECTED_TINT,
            hidden: hide_selected,
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {7},
            margin_top: {-14.0},
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
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {40},
            margin_top: {16.0},
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
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {40},
            margin_top: {43.0},
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
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {40},
            margin_top: {67.0},
        }
    }
}

fn character_card(
    index: usize,
    ch: &CharDisplayEntry,
    is_selected: bool,
    offset: usize,
) -> Element {
    let top = index as f32 * (CHARACTER_CARD_HEIGHT + CARD_GAP) - offset as f32;
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
            height: CHARACTER_CARD_HEIGHT,
            pos_type: "absolute",
            left: 0.0,
            top,
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
                pos_type: "absolute",
                left: "0%",
                top: "0%",
                margin_left: {20},
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
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {52},
            margin_top: {16.0},
        }
        fontstring {
            name: "CharacterListRealmLabel",
            width: 281.0,
            height: 28.0,
            text: REALM_NAME,
            font: GameFont::FrizQuadrata,
            font_size: 20.0,
            font_color: COLOR_GOLD,
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {50},
            margin_top: {14.0},
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
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {20},
            margin_top: {51.0},
        }
        r#frame {
            name: "CharacterListDivider",
            width: 346.0,
            height: 1.0,
            background_color: "1.0,0.9,0.65,0.12",
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {20},
            margin_top: {80.0},
        }
    }
}

fn card_list(
    ctx: &SharedContext,
    characters: &[CharDisplayEntry],
    selected: Option<usize>,
) -> Element {
    let count = characters.len().max(1);
    let content_height = count as f32 * (CHARACTER_CARD_HEIGHT + CARD_GAP) - CARD_GAP;
    let geometry = pixel_geometry(LIST_HEIGHT, content_height, LIST_HEIGHT - 6.0);
    let offset = geometry.clamp(ctx.scroll_first_row(CHARACTER_LIST_SCROLL));
    let config = scroll_list_attr(&geometry);
    let bar = MinimalScrollBar {
        list: CHARACTER_LIST_SCROLL,
        left: BAR_LEFT,
        top: 2.0,
        height: LIST_HEIGHT - 6.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideBar,
    };
    let cards: Element = characters
        .iter()
        .enumerate()
        .flat_map(|(i, ch)| character_card(i, ch, selected == Some(i), offset))
        .collect();
    let empty = if characters.is_empty() {
        empty_card()
    } else {
        Vec::new()
    };
    rsx! {
        r#frame {
            name: {DynName(CHARACTER_LIST_SCROLL.into())},
            width: {BAR_LEFT + BAR_W},
            height: LIST_HEIGHT,
            mouse_enabled: true,
            scroll_list: config,
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {19},
            margin_top: {94.0},
            {cards}
            {empty}
            {bar.element()}
        }
    }
}

fn cs_character_list(
    ctx: &SharedContext,
    characters: &[CharDisplayEntry],
    selected: Option<usize>,
) -> Element {
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
            pos_type: "absolute",
            left: "100%",
            top: "0%",
            translate_x: "-100%",
            margin_left: {-22},
            margin_top: {164.0},
            {chrome}
            {card_list(ctx, characters, selected)}
            {create_char_button()}
            {delete_char_button_if_selected(selected.is_some())}
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
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            pos_type: "absolute",
            left: "50%",
            top: "100%",
            translate_x: "-50%",
            translate_y: "-100%",
            margin_top: {-111.0},
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
            pos_type: "absolute",
            left: "0%",
            top: "100%",
            translate_y: "-100%",
            margin_left: {18},
            margin_top: {-23.0},
        }
    }
}

fn delete_char_button() -> Element {
    trash_icon_button(
        DELETE_CHAR_BUTTON,
        FrameName("DeleteCharIcon"),
        CharSelectAction::DeleteChar,
        crate::ui::screens::trash_button_component::ButtonPosition {
            right: 18.0,
            bottom: 23.0,
        },
    )
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
            pos_type: "absolute",
            left: "0%",
            top: "100%",
            translate_y: "-100%",
            margin_left: {12},
            margin_top: {-60.0},
        }
    }
}

fn delete_char_button_if_selected(has_selection: bool) -> Element {
    if has_selection {
        delete_char_button()
    } else {
        Vec::new()
    }
}

fn cs_action_buttons() -> Element {
    [enter_world_button(), back_button()]
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
            pos_type: "absolute",
            left: "50%",
            top: "100%",
            translate_x: "-50%",
            translate_y: "-100%",
            margin_top: {-188.0},
        }
    }
}

// --- Main screen ---

pub fn char_select_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CharSelectState>()
        .expect("CharSelectState must be in SharedContext");
    let campsite = ctx.get::<CampsiteState>();
    let delete_confirm = ctx
        .get::<DeleteConfirmUiState>()
        .cloned()
        .unwrap_or_default();
    let has_selection = state.selected_index.is_some();

    rsx! {
        r#frame { name: CHAR_SELECT_ROOT, strata: FrameStrata::Background,
            {cs_background()}
            {cs_logo()}
            {char_select_top_nav()}
            {cs_name_area(&state.selected_name, has_selection)}
            {cs_character_list(ctx, &state.characters, state.selected_index)}
            {cs_action_buttons()}
            {cs_status(&state.status_text)}
            {campsite_ui(campsite)}
            {delete_confirmation_modal(&delete_confirm)}
        }
    }
}

fn campsite_ui(campsite: Option<&CampsiteState>) -> Element {
    campsite.map(campsite_panel).unwrap_or_default()
}
