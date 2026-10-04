//! Retail 12.x spellbook (`Blizzard_PlayerSpells/SpellBook`): `SpellBookFrameTemplate`
//! (1612×856 open book, evergreen art, category tabs, two 680×650 page views, paging
//! controls), `SpellBookHeaderTemplate` and `SpellBookItemTemplate`, laid out by
//! `PagedCondensedVerticalGridContentFrameTemplate` (3 columns filled column-first,
//! `viewsPerPage` 2, `spacerSize` 20, `xPadding` 15, `yPadding` 10), inside the
//! `PlayerSpellsFrame` `PortraitFrameTemplate` window (title, spec portrait, close
//! button). The frame scales down to fit the viewport.

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::screens::quest_art::window_chrome;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::GameFont;

pub const SPELLBOOK_FRAME: FrameName = FrameName("SpellBookRoot");
/// `"{ACTION_SPELLBOOK_TAB}{index}"` selects a category tab.
pub const ACTION_SPELLBOOK_TAB: &str = "spellbook_tab:";
/// `"{ACTION_SPELLBOOK_CAST}{spell_id}"`: a known active spell's icon was clicked.
pub const ACTION_SPELLBOOK_CAST: &str = "spellbook_cast:";
pub const ACTION_SPELLBOOK_PREV_PAGE: &str = "spellbook_page:prev";
pub const ACTION_SPELLBOOK_NEXT_PAGE: &str = "spellbook_page:next";
pub const ACTION_SPELLBOOK_CLOSE: &str = "spellbook_close";

/// `PlayerSpellsFrame` (`PortraitFrameTemplate`, Blizzard_PlayerSpellsFrame.xml:5-10):
/// the window holding the book, its title, portrait and close button.
pub const FRAME_W: f32 = 1618.0;
pub const FRAME_H: f32 = 883.0;
/// `SpellBookFrameTemplate` content size (`PlayerSpellsFrame` 1612×856 tab page),
/// anchored BOTTOMLEFT 0,4 (Blizzard_PlayerSpellsFrame.xml:49-57).
const BOOK_W: f32 = 1612.0;
const BOOK_H: f32 = 856.0;
const BOOK_Y: f32 = FRAME_H - 4.0 - BOOK_H;
/// `SetTitle(SPELLBOOK)` (Blizzard_PlayerSpellsFrame.lua:139), GlobalStrings `SPELLBOOK`.
const TITLE: &str = "Spellbook";
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
}

impl SpellbookFrameState {
    pub fn selected_category(&self) -> Option<&SpellbookCategory> {
        self.categories.get(self.selected)
    }

    pub fn page_count(&self) -> usize {
        self.selected_category().map_or(1, |category| {
            paginate(&category.groups).len().div_ceil(2).max(1)
        })
    }
}

/// A header or item placed in a page view, in unscaled view coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    Header {
        group: usize,
        y: f32,
    },
    Item {
        group: usize,
        item: usize,
        x: f32,
        y: f32,
    },
}

/// `PagedCondensedVerticalGridContentFrameMixin`: each group's header takes a full row,
/// its items fill `ceil(n / 3)` rows column-first; a view that runs out of height
/// continues the items (without the header) in the next view. Groups after the first in a
/// view start after a 20 px spacer.
pub fn paginate(groups: &[SpellbookGroup]) -> Vec<Vec<Placement>> {
    let row = ITEM_H + Y_PADDING;
    let mut views: Vec<Vec<Placement>> = vec![Vec::new()];
    let mut y = 0.0;
    for (group_index, group) in groups.iter().enumerate() {
        if group.items.is_empty() {
            continue;
        }
        if y > 0.0 {
            y += SPACER;
        }
        if y + HEADER_H + ITEM_H > VIEW_H {
            views.push(Vec::new());
            y = 0.0;
        }
        views.last_mut().expect("a view").push(Placement::Header {
            group: group_index,
            y,
        });
        y += HEADER_H + Y_PADDING;
        let mut placed = 0;
        while placed < group.items.len() {
            let remaining = group.items.len() - placed;
            let rows_fit = ((VIEW_H - y + Y_PADDING) / row).floor().max(0.0) as usize;
            if rows_fit == 0 {
                views.push(Vec::new());
                y = 0.0;
                continue;
            }
            let rows = remaining.div_ceil(COLUMNS).min(rows_fit);
            let count = remaining.min(rows * COLUMNS);
            let view = views.last_mut().expect("a view");
            for offset in 0..count {
                let (column, row_index) = (offset / rows, offset % rows);
                view.push(Placement::Item {
                    group: group_index,
                    item: placed + offset,
                    x: column as f32 * (ITEM_W + X_PADDING),
                    y: y + row_index as f32 * row,
                });
            }
            placed += count;
            y += rows as f32 * row;
            if placed < group.items.len() {
                views.push(Vec::new());
                y = 0.0;
            }
        }
    }
    views
}

/// Frame scale and top-left origin for a viewport.
pub fn frame_layout(viewport: [f32; 2]) -> (f32, [f32; 2]) {
    let [width, height] = viewport;
    let fit_w = (width - 2.0 * SCREEN_MARGIN) / FRAME_W;
    let fit_h = (height - 2.0 * SCREEN_MARGIN) / FRAME_H;
    let scale = fit_w.min(fit_h).clamp(0.1, 1.0);
    let origin = [
        ((width - FRAME_W * scale) / 2.0).round(),
        ((height - FRAME_H * scale) / 2.0).round(),
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

fn background(s: f32) -> Element {
    [
        art(
            "SpellBookBookBGLeft".into(),
            &BOOK_LEFT,
            [0.0, BOOK_TOP, BOOK_W / 2.0, BOOK_H - BOOK_TOP],
            s,
        ),
        art(
            "SpellBookBookBGRight".into(),
            &BOOK_RIGHT,
            [BOOK_W / 2.0, BOOK_TOP, BOOK_W / 2.0, BOOK_H - BOOK_TOP],
            s,
        ),
        art(
            "SpellBookTopBar".into(),
            &TOP_BAR,
            [0.0, 0.0, BOOK_W - 2.0, TOP_BAR_H],
            s,
        ),
        art(
            "SpellBookBookmark".into(),
            &BOOKMARK,
            [BOOK_W / 2.0 + 62.0 - 102.0, BOOK_TOP, 102.0, 557.0],
            s,
        ),
        art(
            "SpellBookCorner".into(),
            &CORNER,
            [BOOK_W - 15.0 - 150.0, BOOK_H - 6.0 - 155.0, 150.0, 155.0],
            s,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn tab_width(name: &str) -> f32 {
    (name.chars().count() as f32 * TAB_GLYPH_W + 40.0).clamp(TAB_MIN_W, TAB_MAX_W)
}

/// `TabSystemButtonArtTemplate` with `isTabOnTop`: the tab art is flipped vertically.
fn category_tab(index: usize, name: &str, left: f32, selected: bool, s: f32) -> Element {
    let width = tab_width(name);
    let frame = category_tab_name(index);
    let (left_art, middle_art, right_art, height) = if selected {
        (ACTIVE_TAB_LEFT, ACTIVE_TAB_MIDDLE, ACTIVE_TAB_RIGHT, 42.0)
    } else {
        (TAB_LEFT, TAB_MIDDLE, TAB_RIGHT, 36.0)
    };
    let white = "1.0,1.0,1.0,1.0";
    let pieces: Element = [
        art_colored(
            format!("{frame}Left"),
            &left_art,
            [0.0, 0.0, 35.0, height],
            s,
            white,
            true,
        ),
        art_colored(
            format!("{frame}Middle"),
            &middle_art,
            [35.0, 0.0, width - 35.0 - 31.0, height],
            s,
            white,
            true,
        ),
        art_colored(
            format!("{frame}Right"),
            &right_art,
            [width - 31.0, 0.0, 37.0, height],
            s,
            white,
            true,
        ),
        label(
            Label {
                name: format!("{frame}Text"),
                text: name,
                rect: [0.0, 11.0, width, 14.0],
                size: TAB_TEXT_SIZE,
                color: if selected {
                    TAB_TEXT_SELECTED
                } else {
                    TAB_TEXT
                },
                justify: "CENTER",
            },
            s,
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        r#frame {
            name: {DynName(frame)},
            width: {width * s},
            height: {TAB_H * s},
            onclick: {format!("{ACTION_SPELLBOOK_TAB}{index}")},
            pos_type: "absolute",
            pos_x: {left * s},
            pos_y: {TABS_TOP * s},
            {pieces}
        }
    }
}

fn category_tabs(state: &SpellbookFrameState, s: f32) -> Element {
    let mut left = TABS_LEFT;
    let mut tabs = Vec::new();
    for (index, category) in state.categories.iter().enumerate() {
        tabs.extend(category_tab(
            index,
            &category.name,
            left,
            index == state.selected,
            s,
        ));
        left += tab_width(&category.name) + TAB_SPACING;
    }
    tabs
}

fn header(group: &SpellbookGroup, rect: [f32; 2], s: f32) -> Element {
    let [x, y] = rect;
    let name = format!("SpellBookHeader{}", group.name.replace(' ', ""));
    let children: Element = [
        art_colored(
            format!("{name}Backplate"),
            &HEADER_BACKPLATE,
            [-85.0, (HEADER_H - 106.0) / 2.0 - 10.0, 416.0, 106.0],
            s,
            "1.0,1.0,1.0,0.65",
            false,
        ),
        label(
            Label {
                name: format!("{name}Text"),
                text: &group.name,
                rect: [-8.0, 6.0, VIEW_W - 60.0 + 8.0, 30.0],
                size: HEADER_SIZE,
                color: FONT_COLOR,
                justify: "LEFT",
            },
            s,
        ),
        art(
            format!("{name}Border"),
            &DIVIDER,
            [-32.0, HEADER_H - 11.0, VIEW_W - 60.0 + 32.0, 11.0],
            s,
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: {VIEW_W * s},
            height: {HEADER_H * s},
            pos_type: "absolute",
            pos_x: {x * s},
            pos_y: {y * s},
            {children}
        }
    }
}

/// Button border per `SpellBookItemMixin.ArtSet`: Square (-11,1 / 1,-7 active;
/// -10,1 / 2,-5 inactive) or Circle for passives (all points).
fn item_border(name: &str, item: &SpellbookItemView, s: f32) -> Element {
    let top = (ITEM_H - BUTTON_SIZE) / 2.0;
    let (border, rect) = match (item.passive, item.available_at.is_some()) {
        (false, false) => (
            ICON_FRAME,
            [-11.0, top - 1.0, BUTTON_SIZE + 12.0, BUTTON_SIZE + 8.0],
        ),
        (false, true) => (
            ICON_FRAME_INACTIVE,
            [-10.0, top - 1.0, BUTTON_SIZE + 12.0, BUTTON_SIZE + 6.0],
        ),
        (true, false) => (PASSIVE_FRAME, [0.0, top, BUTTON_SIZE, BUTTON_SIZE]),
        (true, true) => (PASSIVE_FRAME_INACTIVE, [0.0, top, BUTTON_SIZE, BUTTON_SIZE]),
    };
    art(format!("{name}Border"), &border, rect, s)
}

fn item_texts(name: &str, item: &SpellbookItemView, s: f32) -> Element {
    let color = if item.available_at.is_some() {
        UNLEARNED_FONT_COLOR
    } else {
        FONT_COLOR
    };
    let level = item
        .available_at
        .map(|level| format!("{AVAILABLE_AT}{level}"));
    let lines: Vec<(&str, &str, f32)> = [
        Some(("Name", item.name.as_str(), NAME_SIZE)),
        (!item.subtext.is_empty()).then_some(("SubName", item.subtext.as_str(), SUBTEXT_SIZE)),
        level
            .as_deref()
            .map(|text| ("RequiredLevel", text, SUBTEXT_SIZE)),
    ]
    .into_iter()
    .flatten()
    .collect();
    let line_h = |size: f32| size + 3.0;
    let block: f32 = lines.iter().map(|&(_, _, size)| line_h(size)).sum::<f32>()
        + 2.0 * (lines.len() as f32 - 1.0);
    let mut y = (ITEM_H - block) / 2.0 - 1.0;
    let mut out = Vec::new();
    for (part, text, size) in lines {
        out.extend(label(
            Label {
                name: format!("{name}{part}"),
                text,
                rect: [TEXT_LEFT, y, ITEM_W - TEXT_LEFT, line_h(size)],
                size,
                color,
                justify: "LEFT",
            },
            s,
        ));
        y += line_h(size) + 2.0;
    }
    out
}

fn item(item: &SpellbookItemView, rect: [f32; 2], s: f32) -> Element {
    let [x, y] = rect;
    let name = spell_item_name(item.spell_id);
    let icon_top = (ITEM_H - ICON_SIZE) / 2.0;
    let icon_left = (BUTTON_SIZE - ICON_SIZE) / 2.0;
    let tint = if item.available_at.is_some() {
        UNLEARNED_TINT
    } else {
        "1.0,1.0,1.0,1.0"
    };
    let castable = !item.passive && item.available_at.is_none();
    let onclick = castable.then(|| format!("{ACTION_SPELLBOOK_CAST}{}", item.spell_id));
    let icon_name = format!("{name}Icon");
    let icon = rsx! {
        texture {
            name: {DynName(icon_name)},
            width: {ICON_SIZE * s},
            height: {ICON_SIZE * s},
            hidden: {item.icon_fdid == 0},
            texture_fdid: {item.icon_fdid},
            vertex_color: tint,
            pos_type: "absolute",
            pos_x: {icon_left * s},
            pos_y: {icon_top * s},
        }
    };
    let children: Element = [
        art_colored(
            format!("{name}Backplate"),
            &ITEM_BACKPLATE,
            [
                (ITEM_W - 256.0) / 2.0 + 5.0,
                (ITEM_H - 64.0) / 2.0 + 5.0,
                256.0,
                64.0,
            ],
            s,
            "1.0,1.0,1.0,0.25",
            false,
        ),
        icon,
        item_border(&name, item, s),
        item_texts(&name, item, s),
    ]
    .into_iter()
    .flatten()
    .collect();
    let button_name = format!("{name}Button");
    let button_y = (ITEM_H - BUTTON_SIZE) / 2.0 * s;
    let button = match onclick {
        Some(onclick) => rsx! {
            button {
                name: {DynName(button_name)},
                width: {BUTTON_SIZE * s},
                height: {BUTTON_SIZE * s},
                onclick,
                button_default_skin: false,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: button_y,
            }
        },
        None => rsx! {
            button {
                name: {DynName(button_name)},
                width: {BUTTON_SIZE * s},
                height: {BUTTON_SIZE * s},
                button_default_skin: false,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: button_y,
            }
        },
    };
    rsx! {
        r#frame {
            name: {DynName(name)},
            width: {ITEM_W * s},
            height: {ITEM_H * s},
            pos_type: "absolute",
            pos_x: {x * s},
            pos_y: {y * s},
            {children}
            {button}
        }
    }
}

fn view(
    state: &SpellbookFrameState,
    placements: Option<&Vec<Placement>>,
    index: usize,
    s: f32,
) -> Element {
    let groups = state
        .selected_category()
        .map_or(&[][..], |category| category.groups.as_slice());
    let children: Element = placements
        .into_iter()
        .flatten()
        .flat_map(|placement| match *placement {
            Placement::Header { group, y } => header(&groups[group], [0.0, y], s),
            Placement::Item {
                group,
                item: index,
                x,
                y,
            } => item(&groups[group].items[index], [x, y], s),
        })
        .collect();
    let left = if index == 0 { VIEW1_LEFT } else { VIEW2_LEFT };
    rsx! {
        r#frame {
            name: {DynName(format!("SpellBookView{}", index + 1))},
            width: {VIEW_W * s},
            height: {VIEW_H * s},
            pos_type: "absolute",
            pos_x: {left * s},
            pos_y: {VIEW_TOP * s},
            {children}
        }
    }
}

fn paging(state: &SpellbookFrameState, s: f32) -> Element {
    let pages = state.page_count();
    let page = state.page.min(pages - 1);
    let text = format!("Page {}/{}", page + 1, pages);
    let top = PAGING_BOTTOM - PAGE_BUTTON;
    let next_left = PAGING_RIGHT - PAGE_BUTTON;
    let prev_left = next_left - PAGING_SPACING - PAGE_BUTTON;
    let text_left = prev_left - PAGING_SPACING - PAGE_TEXT_W;
    let prev_art = if page == 0 {
        PREV_PAGE_DISABLED
    } else {
        PREV_PAGE_UP
    };
    let next_art = if page + 1 >= pages {
        NEXT_PAGE_DISABLED
    } else {
        NEXT_PAGE_UP
    };
    let prev_icon = file_texture(
        "SpellBookPrevPageButtonIcon".into(),
        prev_art,
        [0.0, 0.0, PAGE_BUTTON, PAGE_BUTTON],
        s,
    );
    let next_icon = file_texture(
        "SpellBookNextPageButtonIcon".into(),
        next_art,
        [0.0, 0.0, PAGE_BUTTON, PAGE_BUTTON],
        s,
    );
    let mut out = label(
        Label {
            name: "SpellBookPageText".into(),
            text: &text,
            rect: [text_left, top + 8.0, PAGE_TEXT_W, 18.0],
            size: PAGE_TEXT_SIZE,
            color: FONT_COLOR,
            justify: "RIGHT",
        },
        s,
    );
    out.extend(rsx! {
        r#frame {
            name: "SpellBookPrevPageButton",
            width: {PAGE_BUTTON * s},
            height: {PAGE_BUTTON * s},
            onclick: ACTION_SPELLBOOK_PREV_PAGE,
            pos_type: "absolute",
            pos_x: {prev_left * s},
            pos_y: {top * s},
            {prev_icon}
        }
        r#frame {
            name: "SpellBookNextPageButton",
            width: {PAGE_BUTTON * s},
            height: {PAGE_BUTTON * s},
            onclick: ACTION_SPELLBOOK_NEXT_PAGE,
            pos_type: "absolute",
            pos_x: {next_left * s},
            pos_y: {top * s},
            {next_icon}
        }
    });
    out
}

pub fn spellbook_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<SpellbookFrameState>()
        .expect("SpellbookFrameState must be in SharedContext");
    let (s, [x, y]) = frame_layout(state.viewport);
    let views = state
        .selected_category()
        .map(|category| paginate(&category.groups))
        .unwrap_or_default();
    let page = state.page.min(state.page_count() - 1);
    let chrome = window_chrome(
        "SpellBook",
        (FRAME_W * s, FRAME_H * s),
        TITLE,
        ACTION_SPELLBOOK_CLOSE,
    );
    let [portrait_x, portrait_y, portrait_w, portrait_h] = PORTRAIT;
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
            r#frame {
                name: "SpellBookFrame",
                width: {BOOK_W * s},
                height: {BOOK_H * s},
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: {BOOK_Y * s},
                {background(s)}
                {category_tabs(state, s)}
                {view(state, views.get(page * 2), 0, s)}
                {view(state, views.get(page * 2 + 1), 1, s)}
                {paging(state, s)}
            }
            texture {
                name: "SpellBookPortrait",
                width: {portrait_w * s},
                height: {portrait_h * s},
                texture_fdid: {state.portrait_fdid},
                pos_type: "absolute",
                pos_x: {portrait_x * s},
                pos_y: {portrait_y * s},
            }
        }
    }
}

/// Retail desaturates the icons of spells not learned yet (`SetDesaturated`).
pub fn apply_spellbook_postsetup(state: &SpellbookFrameState, registry: &mut FrameRegistry) {
    let Some(category) = state.selected_category() else {
        return;
    };
    for item in category.groups.iter().flat_map(|group| &group.items) {
        let name = format!("{}Icon", spell_item_name(item.spell_id));
        if let Some(id) = registry.get_by_name(&name)
            && let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
        {
            texture.desaturated = item.available_at.is_some();
        }
    }
}
