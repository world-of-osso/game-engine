//! Retail `LootFrame` (Blizzard_UIPanels_Game/Mainline/LootFrame.xml, cited as LF.xml,
//! and LootFrame.lua, LF.lua): a 220-wide `ScrollingFlatPanelTemplate` titled `ITEMS`
//! with one 46-tall card per loot slot. Item cards show the icon, the name in its
//! quality colour and the quality tag; the money card shows the coins. Positions are
//! top-left offsets in frame space; docs/specs/loot-frame.md lists them.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::screens::quest_art::{DynName, atlas_texture, flat_panel_chrome};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "LootFrame";
/// LF.xml:113-117 `panelWidth` 220 (`panelMaxHeight` 290, see [`frame_height`]).
pub const FRAME_W: f32 = 220.0;

pub const ACTION_CLOSE: &str = "loot_close";
/// `loot_slot:<server slot>`.
pub const ACTION_SLOT_PREFIX: &str = "loot_slot:";

/// LF.lua:1-3 `ScrollBoxElementHeight`, `ScrollBoxPad`, `ScrollBoxSpacing`.
const ROW_H: f32 = 46.0;
const ROW_PAD: f32 = 6.0;
const ROW_SPACING: f32 = 2.0;
/// `ScrollBox` TOPLEFT 4,-22 (ScrollingFlatPanel.xml); rows fill its width
/// (`panelWidth - padding.left`) inside the padding.
const SCROLL_LEFT: f32 = 4.0;
const SCROLL_TOP: f32 = 22.0;
const ROW_W: f32 = FRAME_W - ROW_PAD - SCROLL_LEFT - 2.0 * ROW_PAD;
/// `ScrollingFlatPanelMixin:Resize` anchors 26 + extra 20.
const CHROME_H: f32 = 46.0;
const ITEM_BUTTON: f32 = 37.0;
/// Item TOPLEFT 5,-4 (LF.xml:10-14).
const ITEM_X: f32 = 5.0;
const ITEM_Y: f32 = 4.0;

const QUICKSLOT: u32 = 130_841; // Interface\Buttons\UI-Quickslot2
const WHITE: &str = "1.0,1.0,1.0,1.0";

/// UiTextureAtlas 2099 `interface/lootframe/lootframe.blp` (4700716) 512×1024.
const LOOT_ATLAS: (u32, (f32, f32)) = (4_700_716, (512.0, 1024.0));
const fn loot_art(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: LOOT_ATLAS.0,
        atlas: LOOT_ATLAS.1,
        rect,
    }
}
/// `looting_itemcard_bg` (16755), tinted by quality (LF.lua:279-282).
const CARD_BG: AtlasArt = loot_art((1.0, 299.0, 457.0, 533.0));
/// `looting_itemcard_stroke_normal` (16758).
const CARD_STROKE: AtlasArt = loot_art((1.0, 299.0, 691.0, 767.0));
/// `looting_raritytag_frame` (16759) 100×13 at TOPRIGHT.
const RARITY_TAG: AtlasArt = loot_art((301.0, 509.0, 457.0, 472.0));

/// One loot card.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LootFrameRow {
    /// Server loot slot, the click action's index.
    pub slot: u8,
    pub icon_fdid: u32,
    /// Item name, or the coin lines of the money card.
    pub name: String,
    /// `ITEM_QUALITY_COLORS` of the item (Common white for money).
    pub color: &'static str,
    /// `ITEM_QUALITY%d_DESC`; `None` for the money card.
    pub quality_text: Option<&'static str>,
    /// Shown on the icon above 1.
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LootFrameState {
    pub visible: bool,
    pub rows: Vec<LootFrameRow>,
    /// Frame top-left in UI space.
    pub left: f32,
    pub top: f32,
}

/// `ScrollingFlatPanelMixin:Resize`: rows plus chrome. Retail caps this at
/// `panelMaxHeight` and scrolls; the scroll bar is not built, so more than five
/// cards grow the frame instead.
pub fn frame_height(rows: usize) -> f32 {
    let rows = rows.max(1) as f32;
    rows * ROW_H + (rows - 1.0) * ROW_SPACING + CHROME_H
}

/// Retail `ITEM_QUALITY%d_DESC`.
pub fn quality_description(quality: u8) -> &'static str {
    match quality {
        0 => "Poor",
        2 => "Uncommon",
        3 => "Rare",
        4 => "Epic",
        5 => "Legendary",
        6 => "Artifact",
        7 => "Heirloom",
        _ => "Common",
    }
}

pub fn loot_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<LootFrameState>()
        .expect("LootFrameState must be in SharedContext");
    let hide = !state.visible;
    let height = frame_height(state.rows.len());
    let mut children = flat_panel_chrome(FRAME_NAME, (FRAME_W, height), "Items", ACTION_CLOSE);
    for (index, row) in state.rows.iter().enumerate() {
        children.extend(loot_card(index, row));
    }
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width: FRAME_W,
            height,
            strata: FrameStrata::High,
            hidden: hide,
            mouse_enabled: true,
            pos_type: "absolute",
            left: {state.left},
            top: {state.top},
            {children}
        }
    }
}

fn loot_card(index: usize, row: &LootFrameRow) -> Element {
    let prefix = format!("LootFrameElement{}", index + 1);
    let x = SCROLL_LEFT + ROW_PAD;
    let y = SCROLL_TOP + ROW_PAD + index as f32 * (ROW_H + ROW_SPACING);
    let full = (0.0, 0.0, ROW_W, ROW_H);
    let mut children = tinted(format!("{prefix}NameFrame"), &CARD_BG, full, row.color);
    if row.quality_text.is_some() {
        children.extend(atlas_texture(
            format!("{prefix}QualityStripe"),
            &RARITY_TAG,
            (ROW_W - 100.0, 0.0, 100.0, 13.0),
        ));
    }
    children.extend(atlas_texture(
        format!("{prefix}BorderFrame"),
        &CARD_STROKE,
        full,
    ));
    children.extend(item_button(&prefix, row));
    children.extend(card_texts(&prefix, row));
    let action = format!("{ACTION_SLOT_PREFIX}{}", row.slot);
    rsx! {
        r#frame {
            name: {DynName(prefix)},
            width: ROW_W,
            height: ROW_H,
            onclick: {action.as_str()},
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

fn tinted(name: String, art: &AtlasArt, rect: (f32, f32, f32, f32), color: &str) -> Element {
    let coords = art.tex_coords(1.0);
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            vertex_color: color,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn texture(name: String, fdid: u32, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: fdid,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

struct Text<'a> {
    name: String,
    text: &'a str,
    rect: (f32, f32, f32, f32),
    size: f32,
    color: &'a str,
    justify: &'a str,
}

fn text(t: Text) -> Element {
    let (x, y, width, height) = t.rect;
    rsx! {
        fontstring {
            name: {DynName(t.name)},
            width,
            height,
            text: t.text,
            font: GameFont::FrizQuadrata,
            font_size: t.size,
            font_color: t.color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: t.justify,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// `ItemButton` 37×37: icon, UI-Quickslot2 normal texture, `Count` BOTTOMRIGHT -5,2.
fn item_button(prefix: &str, row: &LootFrameRow) -> Element {
    let button = format!("{prefix}Item");
    let mut children = texture(
        format!("{button}Icon"),
        row.icon_fdid,
        (ITEM_X, ITEM_Y, ITEM_BUTTON, ITEM_BUTTON),
    );
    let quick = 64.0;
    let inset = (ITEM_BUTTON - quick) / 2.0;
    children.extend(texture(
        format!("{button}NormalTexture"),
        QUICKSLOT,
        (ITEM_X + inset, ITEM_Y + inset + 1.0, quick, quick),
    ));
    if row.count > 1 {
        let count = row.count.to_string();
        children.extend(rsx! {
            fontstring {
                name: {DynName(format!("{button}Count"))},
                width: {ITEM_BUTTON - 5.0},
                height: 14.0,
                text: {count.as_str()},
                font: GameFont::ArialNarrow,
                font_size: 14.0,
                font_color: WHITE,
                shadow_color: "0.0,0.0,0.0,1.0",
                shadow_offset: "1,-1",
                justify_h: "RIGHT",
                pos_type: "absolute",
                left: ITEM_X,
                top: {ITEM_Y + ITEM_BUTTON - 16.0},
            }
        });
    }
    children
}

/// Item card: `Text` 150×30 at the Item's TOPRIGHT +8,-8 and `QualityText`
/// (GameFontWhiteTiny2) TOPRIGHT -4,-2 (LF.xml:79-92). Money card: `Text` 93×38
/// LEFT of the Item's RIGHT +8 (LF.xml:103-108).
fn card_texts(prefix: &str, row: &LootFrameRow) -> Element {
    let text_x = ITEM_X + ITEM_BUTTON + 8.0;
    let Some(quality) = row.quality_text else {
        return text(Text {
            name: format!("{prefix}Text"),
            text: &row.name,
            rect: (text_x, ITEM_Y + ITEM_BUTTON / 2.0 - 19.0, 93.0, 38.0),
            size: 12.0,
            color: row.color,
            justify: "LEFT",
        });
    };
    let mut children = text(Text {
        name: format!("{prefix}Text"),
        text: &row.name,
        rect: (text_x, ITEM_Y + 8.0, 150.0, 30.0),
        size: 12.0,
        color: row.color,
        justify: "LEFT",
    });
    children.extend(text(Text {
        name: format!("{prefix}QualityText"),
        text: quality,
        rect: (ROW_W - 4.0 - 96.0, 2.0, 96.0, 10.0),
        size: 8.0,
        color: WHITE,
        justify: "RIGHT",
    }));
    children
}
