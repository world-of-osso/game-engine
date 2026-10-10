//! Retail 12.x spellbook (`Blizzard_PlayerSpells/SpellBook`): `SpellBookFrameTemplate`
//! (1612×856 open book, evergreen art, category tabs, two 680×650 page views, paging
//! controls), `SpellBookHeaderTemplate` and `SpellBookItemTemplate`, laid out by
//! `PagedCondensedVerticalGridContentFrameTemplate` (3 columns filled column-first,
//! `viewsPerPage` 2, `spacerSize` 20, `xPadding` 15, `yPadding` 10), inside the
//! `PlayerSpellsFrame` `PortraitFrameTemplate` window (title, spec portrait, close
//! button). The frame scales down to fit the viewport.

use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::screens::quest_art::window_chrome;

#[path = "spellbook_frame_component/items.rs"]
mod items;
#[path = "spellbook_frame_component/layout.rs"]
mod layout;
#[path = "spellbook_frame_component/pagination.rs"]
mod pagination;
#[path = "spellbook_frame_component/paging.rs"]
mod paging;
#[path = "spellbook_frame_component/player_spells_pages.rs"]
mod player_spells_pages;
#[path = "spellbook_frame_component/talents_page.rs"]
mod talents_page;

use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::GameFont;
pub use items::apply_spellbook_postsetup;
use items::view;
use layout::{background, category_tabs, tab_width};
pub use pagination::{Placement, paginate};
use paging::paging;

pub const SPELLBOOK_FRAME: FrameName = FrameName("SpellBookRoot");
/// `"{ACTION_SPELLBOOK_TAB}{index}"` selects a category tab.
pub const ACTION_SPELLBOOK_TAB: &str = "spellbook_tab:";
/// `"{ACTION_SPELLBOOK_CAST}{spell_id}"`: a known active spell's icon was clicked.
pub const ACTION_SPELLBOOK_CAST: &str = "spellbook_cast:";
pub const ACTION_SPELLBOOK_PREV_PAGE: &str = "spellbook_page:prev";
pub const ACTION_SPELLBOOK_NEXT_PAGE: &str = "spellbook_page:next";
pub const ACTION_SPELLBOOK_CLOSE: &str = "spellbook_close";
pub const ACTION_PLAYER_SPELLS_TAB: &str = "player_spells_tab:";
pub const ACTION_ACTIVATE_SPEC: &str = "player_spells_activate:";

/// Blizzard_PlayerSpellsFrame.lua:14-16: independent pages, in this order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerSpellsTab {
    Specialization,
    Talents,
    #[default]
    Spellbook,
}

impl PlayerSpellsTab {
    pub const ALL: [Self; 3] = [Self::Specialization, Self::Talents, Self::Spellbook];

    pub fn title(self) -> &'static str {
        match self {
            Self::Specialization => "Specialization",
            Self::Talents => "Talents",
            Self::Spellbook => "Spellbook",
        }
    }
}

pub fn pressed_player_spells_tab(
    bindings: &game_engine_core::input_bindings_data::InputBindingsData,
    input: &impl game_engine_core::input_bindings_data::InputState,
) -> Option<PlayerSpellsTab> {
    use game_engine_core::input_bindings_data::InputAction;
    [
        (InputAction::ToggleSpellbook, PlayerSpellsTab::Spellbook),
        (InputAction::ToggleTalents, PlayerSpellsTab::Talents),
        (
            InputAction::ToggleSpecialization,
            PlayerSpellsTab::Specialization,
        ),
    ]
    .into_iter()
    .find_map(|(action, tab)| bindings.is_just_pressed(action, input).then_some(tab))
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpecializationChoice {
    pub id: u32,
    pub name: String,
    pub icon_fdid: u32,
    pub role: String,
    pub description: String,
    pub thumbnail: String,
    pub active: bool,
}

pub fn specialization_choices(
    data: &game_engine_core::spell_catalog::SpellbookTabIndex,
    class_id: u32,
    active_spec: Option<u32>,
) -> Vec<SpecializationChoice> {
    let mut specs: Vec<_> = data
        .specs
        .iter()
        .filter(|(_, spec)| spec.class_id == class_id && !spec.initial)
        .collect();
    specs.sort_by_key(|(id, spec)| (spec.order_index, **id));
    let class = data
        .class_names
        .get(&class_id)
        .map(String::as_str)
        .unwrap_or("")
        .to_lowercase()
        .replace(' ', "");
    specs
        .into_iter()
        .map(|(&id, spec)| specialization_choice(id, spec, &class, active_spec))
        .collect()
}

fn specialization_choice(
    id: u32,
    spec: &game_engine_core::spell_catalog::SpecTabInfo,
    class: &str,
    active_spec: Option<u32>,
) -> SpecializationChoice {
    SpecializationChoice {
        id,
        name: spec.name.clone(),
        icon_fdid: spec.icon_fdid,
        role: match spec.role {
            0 => "Tank",
            1 => "Healer",
            2 => "Damage",
            role => panic!("Unsupported ChrSpecialization role {role} for spec {id}"),
        }
        .into(),
        description: spec.description.clone(),
        // Blizzard_ClassSpecializationsFrame.lua:13-54, SPEC_FORMAT_STRINGS.
        thumbnail: format!(
            "spec-thumbnail-{class}-{}",
            spec.name.to_lowercase().replace(' ', "")
        ),
        active: active_spec == Some(id),
    }
}

/// `PlayerSpellsFrame` (`PortraitFrameTemplate`, Blizzard_PlayerSpellsFrame.xml:5-10):
/// the window holding the book, its title, portrait and close button.
pub const FRAME_W: f32 = 1618.0;
pub const FRAME_H: f32 = 883.0;
/// Bottom tabs extend beyond the portrait window (PlayerSpellsFrame.xml:27).
pub const FRAME_TOTAL_H: f32 = FRAME_H + 36.0;
/// `SpellBookFrameTemplate` content size (`PlayerSpellsFrame` 1612×856 tab page),
/// anchored BOTTOMLEFT 0,4 (Blizzard_PlayerSpellsFrame.xml:49-57).
const BOOK_W: f32 = 1612.0;
const BOOK_H: f32 = 856.0;
const BOOK_Y: f32 = FRAME_H - 4.0 - BOOK_H;
/// `SetTitle(SPELLBOOK)` (Blizzard_PlayerSpellsFrame.lua:139), GlobalStrings `SPELLBOOK`.
/// `PortraitFrameBaseTemplate` portrait: 62×62 at TOPLEFT -5,7
/// (SharedUIPanelTemplates.xml:558-562).
const PORTRAIT: [f32; 4] = [-5.0, -7.0, 62.0, 62.0];
const SCREEN_MARGIN: f32 = 16.0;
/// `TopBar` 1612×54; the book halves start 51 below the top.
const TOP_BAR_H: f32 = 54.0;
const BOOK_TOP: f32 = 51.0;
/// `View1` TOPLEFT 85,-65 and `View2` TOPRIGHT -50,-65 of `PagedSpellsFrame` (TOPLEFT y -50).
pub const VIEW_W: f32 = 680.0;
pub const VIEW_H: f32 = 650.0;
const VIEW_TOP: f32 = 50.0 + 65.0;
const VIEW1_LEFT: f32 = 85.0;
const VIEW2_LEFT: f32 = BOOK_W - 50.0 - VIEW_W;
/// Grid: 3 columns, `xPadding` 15, `yPadding` 10, `spacerSize` 20.
pub const COLUMNS: usize = 3;
const X_PADDING: f32 = 15.0;
const Y_PADDING: f32 = 10.0;
const SPACER: f32 = 20.0;
/// `SpellBookItemTemplate` 220×60, widened to the column (`autoExpandElements`).
pub const ITEM_W: f32 = (VIEW_W - (COLUMNS as f32 - 1.0) * X_PADDING) / COLUMNS as f32;
pub const ITEM_H: f32 = 60.0;
/// `SpellBookHeaderTemplate` 300×51, widened to the view (`autoExpandHeaders`).
pub const HEADER_H: f32 = 51.0;
/// Item `Button` 40×40 at LEFT; `Icon` 36×36 centred; text from x 50.
const BUTTON_SIZE: f32 = 40.0;
const ICON_SIZE: f32 = 36.0;
const TEXT_LEFT: f32 = 50.0;
/// `SystemFont_Large` 16, `SystemFont_Med1` 12, `SystemFont_Huge2` 24, `GameFontNormalSmall` 10.
const NAME_SIZE: f32 = 16.0;
const SUBTEXT_SIZE: f32 = 12.0;
const HEADER_SIZE: f32 = 24.0;
const TAB_TEXT_SIZE: f32 = 10.0;
const PAGE_TEXT_SIZE: f32 = 14.0;
/// GlobalColor `SPELLBOOK_FONT_COLOR` (0xFF2E1B0F).
const FONT_COLOR: &str = "0.180,0.106,0.059,1.0";
/// `unlearnedTextAlpha` 0.6.
const UNLEARNED_FONT_COLOR: &str = "0.180,0.106,0.059,0.6";
/// GlobalColor `SPELLBOOK_UNLEARNED_TINT_COLOR` (0xFFAC885D) at `unlearnedIconAlpha` 0.6.
const UNLEARNED_TINT: &str = "0.675,0.533,0.365,0.6";
/// `GameFontNormalSmall` gold and `GameFontHighlightSmall` white.
const TAB_TEXT: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_SELECTED: &str = "1.0,1.0,1.0,1.0";
/// Category tabs: TOPLEFT 70,-19, 32 high, 100..150 wide, 1 apart.
const TABS_LEFT: f32 = 70.0;
const TABS_TOP: f32 = 19.0;
const TAB_H: f32 = 32.0;
const TAB_MIN_W: f32 = 100.0;
const TAB_MAX_W: f32 = 150.0;
const TAB_SPACING: f32 = 1.0;
/// `uiframe-tab-left` / `uiframe-tab-right` atlas widths (TabSystemTemplates.xml:41-52).
const TAB_LEFT_W: f32 = 35.0;
const TAB_RIGHT_W: f32 = 37.0;
/// `textPadding`-free width estimate of `GameFontNormalSmall` glyphs.
const TAB_GLYPH_W: f32 = 6.5;
/// `PagingControls` BOTTOMRIGHT -75,40 of `PagedSpellsFrame`: page text, then 32×32
/// previous and next buttons, 8 apart.
const PAGING_RIGHT: f32 = BOOK_W - 75.0;
const PAGING_BOTTOM: f32 = BOOK_H - 40.0;
const PAGE_BUTTON: f32 = 32.0;
const PAGING_SPACING: f32 = 8.0;
const PAGE_TEXT_W: f32 = 90.0;
/// `SPELLBOOK_AVAILABLE_AT` and `PAGE_NUMBER_WITH_MAX` (GlobalStrings).
const AVAILABLE_AT: &str = "Level ";

/// `interface/spellbook/spellbookbackgroundevergreen.blp` (FDID 5834697, 2048×1024).
const fn evergreen(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: 5_834_697,
        atlas: (2048.0, 1024.0),
        rect,
    }
}

/// `interface/spellbook/spellbookelements.blp` (FDID 5506565, 1024×1024).
const fn elements(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: 5_506_565,
        atlas: (1024.0, 1024.0),
        rect,
    }
}

/// `interface/common/uiframetabs` (FDID 4707839, 64×256).
const fn tabs(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: 4_707_839,
        atlas: (64.0, 256.0),
        rect,
    }
}

const TOP_BAR: AtlasArt = evergreen((1.0, 1615.0, 1.0, 59.0));
const BOOK_LEFT: AtlasArt = evergreen((914.0, 1720.0, 61.0, 866.0));
const BOOK_RIGHT: AtlasArt = evergreen((1.0, 808.0, 61.0, 866.0));
const BOOKMARK: AtlasArt = evergreen((810.0, 912.0, 61.0, 618.0));
/// First of the 4×2 `spellbook-corner-flipbook-evergreen` frames.
const CORNER: AtlasArt = elements((1.0, 151.0, 1.0, 156.0));
const HEADER_BACKPLATE: AtlasArt = elements((1.0, 317.0, 313.0, 419.0));
const DIVIDER: AtlasArt = elements((255.0, 912.0, 421.0, 432.0));
const ITEM_BACKPLATE: AtlasArt = elements((319.0, 575.0, 313.0, 377.0));
const ICON_FRAME: AtlasArt = elements((875.0, 1013.0, 140.0, 271.0));
const ICON_FRAME_INACTIVE: AtlasArt = elements((1.0, 137.0, 554.0, 681.0));
const PASSIVE_FRAME_INACTIVE: AtlasArt = elements((140.0, 252.0, 538.0, 650.0));
/// `talents-node-circle-gray` (UiTextureAtlas 1970, FDID 4556093, 2048×1024).
const PASSIVE_FRAME: AtlasArt = AtlasArt {
    fdid: 4_556_093,
    atlas: (2048.0, 1024.0),
    rect: (219.0, 269.0, 569.0, 619.0),
};
const TAB_LEFT: AtlasArt = tabs((1.0, 36.0, 209.0, 245.0));
const TAB_RIGHT: AtlasArt = tabs((1.0, 38.0, 171.0, 207.0));
const TAB_MIDDLE: AtlasArt = tabs((0.0, 1.0, 45.0, 81.0));
const ACTIVE_TAB_LEFT: AtlasArt = tabs((1.0, 36.0, 127.0, 169.0));
const ACTIVE_TAB_RIGHT: AtlasArt = tabs((1.0, 38.0, 83.0, 125.0));
const ACTIVE_TAB_MIDDLE: AtlasArt = tabs((0.0, 1.0, 1.0, 43.0));
/// `Interface\Buttons\UI-SpellbookIcon-PrevPage-*` / `NextPage-*`.
const PREV_PAGE_UP: u32 = 130_869;
const PREV_PAGE_DISABLED: u32 = 130_867;
const NEXT_PAGE_UP: u32 = 130_866;
const NEXT_PAGE_DISABLED: u32 = 130_864;

/// One spell entry of a category.
#[derive(Clone, Debug, PartialEq)]
pub struct SpellbookItemView {
    pub spell_id: u32,
    pub name: String,
    pub subtext: String,
    pub icon_fdid: u32,
    pub passive: bool,
    /// Learned later: greyed with "Level N" (`FutureSpell`).
    pub available_at: Option<u32>,
    /// Remaining fraction of the effective spell's cooldown/GCD.
    pub cooldown_fraction: f32,
}

/// A skill line of the category: its header and spells.
#[derive(Clone, Debug, PartialEq)]
pub struct SpellbookGroup {
    pub name: String,
    pub items: Vec<SpellbookItemView>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpellbookCategory {
    pub name: String,
    pub groups: Vec<SpellbookGroup>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellbookFrameState {
    pub viewport: [f32; 2],
    pub categories: Vec<SpellbookCategory>,
    pub selected: usize,
    /// 0-based page of the selected category.
    pub page: usize,
    /// The active specialization's icon (`PlayerSpellsFrameMixin:UpdatePortrait`,
    /// Blizzard_PlayerSpellsFrame.lua:332-342); 0 draws an empty ring.
    pub portrait_fdid: u32,
    pub tab: PlayerSpellsTab,
    pub specializations: Vec<SpecializationChoice>,
    pub can_activate_spec: bool,
    pub talents: Option<crate::talents::TalentView>,
}

impl SpellbookFrameState {
    /// PlayerSpellsUtil.lua:119-130: same page closes, another page selects and shows.
    pub fn toggle_tab(&mut self, open: bool, tab: PlayerSpellsTab) -> bool {
        let shown = !open || self.tab != tab;
        self.tab = tab;
        shown
    }

    pub fn select_frame_tab(&mut self, action: &str) -> Result<bool, String> {
        let Some(raw) = action.strip_prefix(ACTION_PLAYER_SPELLS_TAB) else {
            return Ok(false);
        };
        let index: usize = raw
            .parse()
            .map_err(|_| format!("Bad PlayerSpells tab: {action}"))?;
        self.tab = *PlayerSpellsTab::ALL
            .get(index)
            .ok_or_else(|| format!("Unknown PlayerSpells tab: {index}"))?;
        Ok(true)
    }

    pub fn selected_category(&self) -> Option<&SpellbookCategory> {
        self.categories.get(self.selected)
    }

    pub fn page_count(&self) -> usize {
        self.selected_category().map_or(1, |category| {
            paginate(&category.groups).len().div_ceil(2).max(1)
        })
    }
}

/// Frame scale and top-left origin for a viewport.
pub fn frame_layout(viewport: [f32; 2]) -> (f32, [f32; 2]) {
    let [width, height] = viewport;
    let fit_w = (width - 2.0 * SCREEN_MARGIN) / FRAME_W;
    let fit_h = (height - 2.0 * SCREEN_MARGIN) / FRAME_TOTAL_H;
    let scale = fit_w.min(fit_h).clamp(0.1, 1.0);
    let origin = [
        ((width - FRAME_W * scale) / 2.0).round(),
        ((height - FRAME_TOTAL_H * scale) / 2.0).round(),
    ];
    (scale, origin)
}

pub fn spell_item_name(spell_id: u32) -> String {
    format!("SpellBookItem{spell_id}")
}

pub fn category_tab_name(index: usize) -> String {
    format!("SpellBookCategoryTab{}", index + 1)
}

struct DynName(String);

fn art(name: String, art: &AtlasArt, rect: [f32; 4], s: f32) -> Element {
    art_colored(name, art, rect, s, "1.0,1.0,1.0,1.0", false)
}

fn art_colored(
    name: String,
    art: &AtlasArt,
    rect: [f32; 4],
    s: f32,
    color: &str,
    flip_v: bool,
) -> Element {
    let [x, y, width, height] = rect;
    let (left, right, top, bottom) = art.rect;
    let (w, h) = art.atlas;
    let (top, bottom) = if flip_v { (bottom, top) } else { (top, bottom) };
    let coords = format!("{},{},{},{}", left / w, right / w, top / h, bottom / h);
    rsx! {
        texture {
            name: {DynName(name)},
            width: {width * s},
            height: {height * s},
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            vertex_color: color,
            pos_type: "absolute",
            pos_x: {x * s},
            pos_y: {y * s},
        }
    }
}

fn file_texture(name: String, fdid: u32, rect: [f32; 4], s: f32) -> Element {
    let [x, y, width, height] = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width: {width * s},
            height: {height * s},
            texture_fdid: fdid,
            pos_type: "absolute",
            pos_x: {x * s},
            pos_y: {y * s},
        }
    }
}

struct Label<'a> {
    name: String,
    text: &'a str,
    rect: [f32; 4],
    size: f32,
    color: &'a str,
    justify: &'a str,
}

fn label(spec: Label<'_>, s: f32) -> Element {
    let [x, y, width, height] = spec.rect;
    rsx! {
        fontstring {
            name: {DynName(spec.name)},
            width: {width * s},
            height: {height * s},
            text: spec.text,
            font: GameFont::FrizQuadrata,
            font_size: {(spec.size * s).max(6.0)},
            font_color: spec.color,
            justify_h: spec.justify,
            pos_type: "absolute",
            pos_x: {x * s},
            pos_y: {y * s},
        }
    }
}

pub fn spellbook_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<SpellbookFrameState>()
        .expect("SpellbookFrameState must be in SharedContext");
    let (s, [x, y]) = frame_layout(state.viewport);
    let chrome = window_chrome(
        "SpellBook",
        (FRAME_W * s, FRAME_H * s),
        state.tab.title(),
        ACTION_SPELLBOOK_CLOSE,
    );
    let content = frame_content(state, s);
    rsx! {
        r#frame {
            name: SPELLBOOK_FRAME,
            width: {FRAME_W * s},
            height: {FRAME_H * s},
            mouse_enabled: true,
            strata: FrameStrata::High,
            pos_type: "absolute",
            left: x,
            top: y,
            {chrome}
            {content}
            {player_spells_pages::bottom_tabs(state, s)}
            {frame_portrait(state, s)}
        }
    }
}

fn frame_content(state: &SpellbookFrameState, s: f32) -> Element {
    match state.tab {
        PlayerSpellsTab::Spellbook => book_page(state, s),
        PlayerSpellsTab::Specialization => player_spells_pages::specializations(state, s),
        PlayerSpellsTab::Talents => talents_page::talents(state, s),
    }
}

fn book_page(state: &SpellbookFrameState, s: f32) -> Element {
    let views = state
        .selected_category()
        .map(|category| paginate(&category.groups))
        .unwrap_or_default();
    let page = state.page.min(state.page_count() - 1);
    rsx! {
        r#frame {
            name: "SpellBookFrame",
            width: {BOOK_W * s}, height: {BOOK_H * s},
            pos_type: "absolute", pos_x: 0.0, pos_y: {BOOK_Y * s},
            {background(s)}
            {category_tabs(state, s)}
            {view(state, views.get(page * 2), 0, s)}
            {view(state, views.get(page * 2 + 1), 1, s)}
            {paging(state, s)}
        }
    }
}

fn frame_portrait(state: &SpellbookFrameState, s: f32) -> Element {
    let [portrait_x, portrait_y, portrait_w, portrait_h] = PORTRAIT;
    rsx! {
            // `PortraitContainer` frameLevel 400, above the book's 100
            // (SharedUIPanelTemplates.xml:551, Blizzard_PlayerSpellsFrame.xml:49).
            texture {
                name: "SpellBookPortrait",
                frame_level: 400.0,
                width: {portrait_w * s},
                height: {portrait_h * s},
                texture_fdid: {state.portrait_fdid},
                pos_type: "absolute",
                pos_x: {portrait_x * s},
                pos_y: {portrait_y * s},
            }
    }
}
