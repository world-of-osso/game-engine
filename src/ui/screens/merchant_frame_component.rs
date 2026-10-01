//! Retail `MerchantFrame` (Blizzard_UIPanels_Game/Mainline/MerchantFrame.xml, cited
//! as MF.xml): 336×444 `ButtonFrameTemplate` window with the 10-item merchant page
//! (12 on the buyback tab), paging, repair buttons, the last-sold buyback slot, the
//! player's money and the Merchant/Buyback tabs. Positions are top-left offsets in
//! frame space converted from the XML anchors; docs/specs/merchant-frame.md lists them.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::screens::quest_art::{DynName, NORMAL_FONT_COLOR, atlas_texture, window_chrome};
use crate::ui::strata::FrameStrata;

pub const FRAME_NAME: &str = "MerchantFrame";
/// MF.xml:92 `<Size x="336" y="444"/>`.
pub const FRAME_W: f32 = 336.0;
pub const FRAME_H: f32 = 444.0;

pub const ACTION_CLOSE: &str = "merchant_close";
pub const ACTION_PAGE_PREV: &str = "merchant_page_prev";
pub const ACTION_PAGE_NEXT: &str = "merchant_page_next";
pub const ACTION_TAB_MERCHANT: &str = "merchant_tab:merchant";
pub const ACTION_TAB_BUYBACK: &str = "merchant_tab:buyback";
pub const ACTION_REPAIR_ALL: &str = "merchant_repair_all";
/// `MerchantRepairItemButton`: toggles the repair cursor (MF.xml:305-313).
pub const ACTION_REPAIR_ITEM: &str = "merchant_repair_item";
pub const ACTION_BUYBACK_LAST: &str = "merchant_buyback_last";
pub const ACTION_SELL_ALL_JUNK: &str = "merchant_sell_all_junk";
/// The frame outside its buttons: a cursor item dropped there is sold
/// (`OnMouseUp` → `PickupMerchantItem(0)`, MF.xml:620-628).
pub const ACTION_FRAME: &str = "merchant_frame";
/// `merchant_item:<index on the page>`.
pub const ACTION_ITEM_PREFIX: &str = "merchant_item:";

/// `MerchantItemTemplate` 153×44 (MF.xml:3-4).
const CELL_W: f32 = 153.0;
const CELL_H: f32 = 44.0;
/// Item1 TOPLEFT 11,-69; Item2 at Item1 TOPRIGHT +12 (MF.xml:127-186).
const CELL_LEFT: [f32; 2] = [11.0, 176.0];
const CELL_TOP: f32 = 69.0;
/// Rows step by the cell height plus 8 (merchant) or 15 (buyback, MF.lua:516).
const MERCHANT_ROW_STEP: f32 = CELL_H + 8.0;
const BUYBACK_ROW_STEP: f32 = CELL_H + 15.0;
const ITEM_BUTTON: f32 = 37.0;

const EMPTY_SLOT: u32 = 130_766; // Interface\Buttons\UI-EmptySlot
const LABEL_SLOTS: u32 = 136_423; // Interface\MerchantFrame\UI-Merchant-LabelSlots
const QUICKSLOT: u32 = 130_841; // Interface\Buttons\UI-Quickslot2
const PAGE_BUTTON_BG: u32 = 130_822; // Interface\Buttons\UI-PageButton-Background
const PREV_PAGE_UP: u32 = 130_869; // UI-SpellbookIcon-PrevPage-Up
const PREV_PAGE_DISABLED: u32 = 130_867;
const NEXT_PAGE_UP: u32 = 130_866; // UI-SpellbookIcon-NextPage-Up
const NEXT_PAGE_DISABLED: u32 = 130_864;
const INSET_BG: u32 = 374_154; // Interface\FrameGeneral\UI-Background-Marble
const MONEY_EDGE: u32 = 525_911; // Interface\Common\Moneyframe (ThinGoldEdgeTemplate)
const BUTTON_HIGHLIGHT: u32 = 130_718; // Interface\Buttons\ButtonHilight-Square

/// UiTextureAtlas `interface/merchantframe/merchant.blp` 512×256.
const MERCHANT_ATLAS: (u32, (f32, f32)) = (5_222_222, (512.0, 256.0));
const fn merchant_art(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: MERCHANT_ATLAS.0,
        atlas: MERCHANT_ATLAS.1,
        rect,
    }
}
/// `UI-Merchant-BotFrame` 332×61.
const BOT_FRAME: AtlasArt = merchant_art((1.0, 333.0, 1.0, 62.0));
/// `SpellIcon-256x256-RepairAll`.
const REPAIR_ALL_ICON: AtlasArt = merchant_art((1.0, 73.0, 138.0, 210.0));
/// `SpellIcon-256x256-Repair`.
const REPAIR_ICON: AtlasArt = merchant_art((75.0, 147.0, 64.0, 136.0));
/// `SpellIcon-256x256-SellJunk`.
const SELL_JUNK_ICON: AtlasArt = merchant_art((1.0, 73.0, 64.0, 136.0));
/// `common-icon-undo`, atlas 3487944 2048×1024.
const UNDO_ICON: AtlasArt = AtlasArt {
    fdid: 3_487_944,
    atlas: (2048.0, 1024.0),
    rect: (775.0, 1031.0, 259.0, 515.0),
};
/// `coin-gold/silver/copper` (members `coin-*-20x20`), atlas 1667824 256×128.
const COIN_ATLAS: (u32, (f32, f32)) = (1_667_824, (256.0, 128.0));
const fn coin(left: f32) -> AtlasArt {
    AtlasArt {
        fdid: COIN_ATLAS.0,
        atlas: COIN_ATLAS.1,
        rect: (left, left + 20.0, 1.0, 21.0),
    }
}
const COIN_GOLD: AtlasArt = coin(197.0);
const COIN_SILVER: AtlasArt = coin(219.0);
const COIN_COPPER: AtlasArt = coin(175.0);

/// PanelTabButtonTemplate atlas 4707839 64×256 (SharedUIPanelTemplates.xml:905-977).
const TAB_ATLAS: (u32, (f32, f32)) = (4_707_839, (64.0, 256.0));
const fn tab_art(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: TAB_ATLAS.0,
        atlas: TAB_ATLAS.1,
        rect,
    }
}
const TAB_ACTIVE: [AtlasArt; 3] = [
    tab_art((1.0, 36.0, 127.0, 169.0)),
    tab_art((0.0, 1.0, 1.0, 43.0)),
    tab_art((1.0, 38.0, 83.0, 125.0)),
];
const TAB_INACTIVE: [AtlasArt; 3] = [
    tab_art((1.0, 36.0, 209.0, 245.0)),
    tab_art((0.0, 1.0, 45.0, 81.0)),
    tab_art((1.0, 38.0, 171.0, 207.0)),
];

const HIGHLIGHT_FONT_COLOR: &str = "1.0,1.0,1.0,1.0";
const GRAY_FONT_COLOR: &str = "0.5,0.5,0.5,1.0";
const WHITE: &str = "1.0,1.0,1.0,1.0";

/// How an item cell is tinted (MerchantFrame.lua:362-390).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CellTint {
    #[default]
    Normal,
    /// Not usable by the player: red slot and icon.
    Unusable,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MerchantCell {
    pub name: String,
    pub name_color: &'static str,
    pub icon_fdid: u32,
    /// Stack count shown on the button when above 1.
    pub count: u32,
    /// Limited stock, shown as `(%d)`.
    pub stock: Option<u32>,
    /// Copper.
    pub price: u64,
    /// `canAfford == false` greys the price (MF.lua:316).
    pub price_gray: bool,
    pub tint: CellTint,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MerchantFrameState {
    pub visible: bool,
    pub title: String,
    pub buyback_tab: bool,
    pub cells: Vec<MerchantCell>,
    /// `MERCHANT_PAGE_NUMBER` text; `None` hides paging (10 items or fewer).
    pub page_text: Option<String>,
    pub prev_enabled: bool,
    pub next_enabled: bool,
    /// Repair buttons: `None` when the NPC can't repair; `Some(enabled)`.
    pub repair: Option<bool>,
    /// `InRepairMode()`: `MerchantRepairItemButton` keeps its highlight locked (MF.lua:136-142).
    pub repair_mode: bool,
    /// Most recent sale for `MerchantBuyBackItem` on the merchant tab.
    pub last_buyback: Option<MerchantCell>,
    pub money: u64,
    /// `MerchantSellAllJunkButton` enabled: a junk item is in the bags
    /// (`C_MerchantFrame.GetNumJunkItems() > 0`, MF.lua:196-198).
    pub has_junk: bool,
}

pub fn merchant_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<MerchantFrameState>()
        .expect("MerchantFrameState must be in SharedContext");
    let hide = !state.visible;
    let mut children = window_chrome(FRAME_NAME, (FRAME_W, FRAME_H), &state.title, ACTION_CLOSE);
    children.extend(inset());
    children.extend(cells(state));
    if !state.buyback_tab {
        children.extend(merchant_tab_extras(state));
    }
    children.extend(money_bar(state.money));
    children.extend(tabs(state.buyback_tab));
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            hidden: hide,
            mouse_enabled: true,
            onclick: ACTION_FRAME,
            pos_type: "absolute",
            left: 16.0,
            top: 104.0,
            {children}
        }
    }
}

fn texture(name: String, fdid: u32, rect: (f32, f32, f32, f32), color: &str) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: fdid,
            vertex_color: color,
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
    font: GameFont,
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
            font: t.font,
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

/// `Inset` (InsetFrameTemplate) TOPLEFT 4,-60 / BOTTOMRIGHT -6,26.
fn inset() -> Element {
    texture(
        format!("{FRAME_NAME}InsetBg"),
        INSET_BG,
        (4.0, 60.0, FRAME_W - 10.0, FRAME_H - 86.0),
        WHITE,
    )
}

fn cells(state: &MerchantFrameState) -> Element {
    let step = if state.buyback_tab {
        BUYBACK_ROW_STEP
    } else {
        MERCHANT_ROW_STEP
    };
    let slots = if state.buyback_tab { 12 } else { 10 };
    (0..slots)
        .flat_map(|index| {
            let origin = (CELL_LEFT[index % 2], CELL_TOP + (index / 2) as f32 * step);
            item_cell(index, origin, state.cells.get(index))
        })
        .collect()
}

/// One `MerchantItemTemplate`; an empty cell keeps its dimmed slot art (MF.lua:399).
fn item_cell(index: usize, (x, y): (f32, f32), cell: Option<&MerchantCell>) -> Element {
    let prefix = format!("MerchantItem{}", index + 1);
    let (slot_color, label_color) = match cell.map(|cell| cell.tint) {
        None => ("0.4,0.4,0.4,1.0", "0.5,0.5,0.5,1.0"),
        Some(CellTint::Unusable) => ("1.0,0.0,0.0,1.0", "1.0,0.0,0.0,1.0"),
        Some(CellTint::Normal) => (WHITE, "0.5,0.5,0.5,1.0"),
    };
    let mut children = texture(
        format!("{prefix}SlotTexture"),
        EMPTY_SLOT,
        (-13.0, -13.0, 64.0, 64.0),
        slot_color,
    );
    children.extend(texture(
        format!("{prefix}NameFrame"),
        LABEL_SLOTS,
        (42.0, -2.0, 128.0, 78.0),
        label_color,
    ));
    if let Some(cell) = cell {
        children.extend(item_contents(&prefix, cell));
    }
    let action = cell.map_or("", |cell| cell.action.as_str());
    rsx! {
        r#frame {
            name: {DynName(prefix)},
            width: CELL_W,
            height: CELL_H,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

fn item_contents(prefix: &str, cell: &MerchantCell) -> Element {
    let icon_color = match cell.tint {
        CellTint::Unusable => "0.9,0.0,0.0,1.0",
        CellTint::Normal => WHITE,
    };
    let mut children = item_button(&format!("{prefix}ItemButton"), cell, 1.0, icon_color);
    // `Name` 100×30, LEFT to the slot's RIGHT -5,7 (MF.xml:19-23).
    children.extend(text(Text {
        name: format!("{prefix}Name"),
        text: &cell.name,
        rect: (46.0, -3.0, 100.0, 30.0),
        font: GameFont::FrizQuadrata,
        size: 10.0,
        color: cell.name_color,
        justify: "LEFT",
    }));
    // `$parentMoneyFrame` BOTTOMLEFT to NameFrame BOTTOMLEFT 2,31 (MF.xml:72-81).
    children.extend(money(
        &format!("{prefix}MoneyFrame"),
        cell.price,
        (44.0, 45.0),
        MoneyAlign::Left,
        cell.price_gray,
    ));
    children
}

/// Intrinsic `ItemButton` 37×37 scaled by `scale`: icon, UI-Quickslot2 normal
/// texture, `Count` BOTTOMRIGHT -5,2 and `Stock` TOPLEFT 0,-2.
fn item_button(prefix: &str, cell: &MerchantCell, scale: f32, icon_color: &str) -> Element {
    let size = ITEM_BUTTON * scale;
    let mut children = texture(
        format!("{prefix}Icon"),
        cell.icon_fdid,
        (0.0, 0.0, size, size),
        icon_color,
    );
    let quick = 64.0 * scale;
    children.extend(texture(
        format!("{prefix}NormalTexture"),
        QUICKSLOT,
        (
            (size - quick) / 2.0,
            (size - quick) / 2.0 + scale,
            quick,
            quick,
        ),
        icon_color,
    ));
    if cell.count > 1 {
        children.extend(text(Text {
            name: format!("{prefix}Count"),
            text: &cell.count.to_string(),
            rect: (0.0, size - 16.0, size - 5.0, 14.0),
            font: GameFont::ArialNarrow,
            size: 14.0,
            color: WHITE,
            justify: "RIGHT",
        }));
    }
    if let Some(stock) = cell.stock {
        children.extend(text(Text {
            name: format!("{prefix}Stock"),
            text: &format!("({stock})"),
            rect: (0.0, 2.0, size, 14.0),
            font: GameFont::ArialNarrow,
            size: 14.0,
            color: NORMAL_FONT_COLOR,
            justify: "LEFT",
        }));
    }
    children
}

/// Paging, repair buttons, the last-sold buyback slot and the bottom border.
fn merchant_tab_extras(state: &MerchantFrameState) -> Element {
    // `MerchantFrameBottomLeftBorder` 334×61 at BOTTOMLEFT 1,26 (MF.xml:118-122).
    let mut children = atlas_texture(
        format!("{FRAME_NAME}BottomLeftBorder"),
        &BOT_FRAME,
        (1.0, FRAME_H - 26.0 - 61.0, 334.0, 61.0),
    );
    if let Some(page_text) = &state.page_text {
        children.extend(paging(page_text, state.prev_enabled, state.next_enabled));
    }
    if let Some(enabled) = state.repair {
        children.extend(repair_buttons(enabled, state.repair_mode));
    }
    children.extend(sell_all_junk_button(state.repair.is_some(), state.has_junk));
    children.extend(buyback_slot(state.last_buyback.as_ref()));
    children
}

/// Prev/Next 32×32 centred at BOTTOMLEFT 25,96 and 310,96; `MerchantPageText`
/// BOTTOM 0,86 (MF.xml:110-114, 520-574).
fn paging(page_text: &str, prev: bool, next: bool) -> Element {
    let y = FRAME_H - 96.0 - 16.0;
    let mut children = page_button(
        "MerchantPrevPageButton",
        9.0,
        y,
        prev,
        PREV_PAGE_UP,
        PREV_PAGE_DISABLED,
        ACTION_PAGE_PREV,
    );
    children.extend(page_button(
        "MerchantNextPageButton",
        294.0,
        y,
        next,
        NEXT_PAGE_UP,
        NEXT_PAGE_DISABLED,
        ACTION_PAGE_NEXT,
    ));
    let label = |name: &str, x: f32, justify: &str, value: &str| {
        text(Text {
            name: name.into(),
            text: value,
            rect: (x, y + 9.0, 40.0, 14.0),
            font: GameFont::FrizQuadrata,
            size: 10.0,
            color: NORMAL_FONT_COLOR,
            justify,
        })
    };
    children.extend(label("MerchantPrevPageButtonText", 41.0, "LEFT", "Prev"));
    children.extend(label("MerchantNextPageButtonText", 251.0, "RIGHT", "Next"));
    children.extend(text(Text {
        name: "MerchantPageText".into(),
        text: page_text,
        rect: (FRAME_W / 2.0 - 52.0, FRAME_H - 86.0 - 14.0, 104.0, 14.0),
        font: GameFont::FrizQuadrata,
        size: 12.0,
        color: NORMAL_FONT_COLOR,
        justify: "CENTER",
    }));
    children
}

fn page_button(
    name: &str,
    x: f32,
    y: f32,
    enabled: bool,
    up: u32,
    disabled: u32,
    action: &str,
) -> Element {
    let action = if enabled { action } else { "" };
    let art = if enabled { up } else { disabled };
    let mut children = texture(
        format!("{name}Background"),
        PAGE_BUTTON_BG,
        (0.0, -1.0, 32.0, 32.0),
        WHITE,
    );
    children.extend(texture(
        format!("{name}Normal"),
        art,
        (0.0, 0.0, 32.0, 32.0),
        WHITE,
    ));
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width: 32.0,
            height: 32.0,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

/// The 36×36 service buttons along the bottom edge (BOTTOMRIGHT at BOTTOMLEFT …,33).
const SERVICE_Y: f32 = FRAME_H - 33.0 - 36.0;
const REPAIR_ALL_X: f32 = 118.0 - 36.0;

struct ServiceButton<'a> {
    name: &'a str,
    x: f32,
    icon: &'a AtlasArt,
    enabled: bool,
    action: &'a str,
    /// `LockHighlight` (`Some(true)`) for a button with a `ButtonHilight-Square`.
    highlight: Option<bool>,
}

/// `MerchantRepairAllButton` 36×36, BOTTOMRIGHT at BOTTOMLEFT 118,33, and
/// `MerchantRepairItemButton` RIGHT at its LEFT −8 (MF.lua:948-960, no guild bank).
/// The item button is always enabled; its highlight stays lit in repair mode.
fn repair_buttons(enabled: bool, repair_mode: bool) -> Element {
    let mut children = service_button(ServiceButton {
        name: "MerchantRepairAllButton",
        x: REPAIR_ALL_X,
        icon: &REPAIR_ALL_ICON,
        enabled,
        action: ACTION_REPAIR_ALL,
        highlight: None,
    });
    children.extend(service_button(ServiceButton {
        name: "MerchantRepairItemButton",
        x: REPAIR_ALL_X - 8.0 - 36.0,
        icon: &REPAIR_ICON,
        enabled: true,
        action: ACTION_REPAIR_ITEM,
        highlight: Some(repair_mode),
    }));
    children
}

/// `MerchantSellAllJunkButton` 36×36: RIGHT at RepairAll LEFT +80 with a repairer
/// (MF.lua:943), else BOTTOMRIGHT at BOTTOMRIGHT -148,33 (MF.lua:954).
fn sell_all_junk_button(repairer: bool, enabled: bool) -> Element {
    let right = if repairer {
        REPAIR_ALL_X + 80.0
    } else {
        FRAME_W - 148.0
    };
    service_button(ServiceButton {
        name: "MerchantSellAllJunkButton",
        x: right - 36.0,
        icon: &SELL_JUNK_ICON,
        enabled,
        action: ACTION_SELL_ALL_JUNK,
        highlight: None,
    })
}

fn service_button(button: ServiceButton) -> Element {
    let name = button.name;
    let action = if button.enabled { button.action } else { "" };
    // Disabled buttons desaturate their icon (MF.lua:909-931).
    let icon_color = if button.enabled {
        WHITE
    } else {
        "0.4,0.4,0.4,1.0"
    };
    let mut children = texture(
        format!("{name}Slot"),
        EMPTY_SLOT,
        (-13.0, -14.0, 64.0, 64.0),
        WHITE,
    );
    let icon = button.icon;
    let coords = icon.tex_coords(1.0);
    children.extend(rsx! {
        texture {
            name: {DynName(format!("{name}Icon"))},
            width: 36.0,
            height: 36.0,
            texture_fdid: {icon.fdid},
            tex_coords: {coords.as_str()},
            vertex_color: icon_color,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    });
    if let Some(lit) = button.highlight {
        let unlit = !lit;
        children.extend(rsx! {
            texture {
                name: {DynName(format!("{name}Highlight"))},
                width: 36.0,
                height: 36.0,
                texture_fdid: BUTTON_HIGHLIGHT,
                hidden: unlit,
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
        });
    }
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width: 36.0,
            height: 36.0,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: {button.x},
            top: SERVICE_Y,
            {children}
        }
    }
}

/// `MerchantBuyBackItem` 115×37 at MerchantItem10 BOTTOMLEFT 30,-53 (MF.xml:402-486):
/// the most recent sale on a 0.65-scale button with the undo arrow.
fn buyback_slot(last: Option<&MerchantCell>) -> Element {
    let (x, y) = (
        CELL_LEFT[1] + 30.0,
        CELL_TOP + 4.0 * MERCHANT_ROW_STEP + CELL_H + 53.0,
    );
    let name = "MerchantBuyBackItem";
    let mut children = texture(
        format!("{name}SlotTexture"),
        EMPTY_SLOT,
        (-13.0, -13.0, 64.0, 64.0),
        WHITE,
    );
    let button = ITEM_BUTTON * 0.65;
    match last {
        Some(cell) => {
            children.extend(item_button(&format!("{name}ItemButton"), cell, 0.65, WHITE));
            children.extend(text(Text {
                name: format!("{name}Name"),
                text: &cell.name,
                rect: (46.0, 1.0, 70.0, 35.0),
                font: GameFont::FrizQuadrata,
                size: 10.0,
                color: cell.name_color,
                justify: "LEFT",
            }));
            children.extend(money(
                &format!("{name}MoneyFrame"),
                cell.price,
                (42.0, 36.0),
                MoneyAlign::Left,
                cell.price_gray,
            ));
        }
        None => {
            // `UndoFrame.Arrow` desaturated while nothing was sold.
            let coords = UNDO_ICON.tex_coords(1.0);
            children.extend(rsx! {
                texture {
                    name: {DynName(format!("{name}UndoArrow"))},
                    width: 20.0,
                    height: 20.0,
                    texture_fdid: {UNDO_ICON.fdid},
                    tex_coords: {coords.as_str()},
                    vertex_color: "0.5,0.5,0.5,1.0",
                    pos_type: "absolute",
                    left: {(button - 20.0) / 2.0},
                    top: {(button - 20.0) / 2.0 + 1.0},
                }
            });
        }
    }
    let action = if last.is_some() {
        ACTION_BUYBACK_LAST
    } else {
        ""
    };
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width: 115.0,
            height: 37.0,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

/// `MerchantMoneyBg` (ThinGoldEdgeTemplate, UIPanelTemplates.xml:1314: 7 px caps and a
/// stretched middle cut from Interface\Common\Moneyframe) at BOTTOMRIGHT −7,6 ..
/// −166,25, and `MerchantMoneyFrame` at BOTTOMRIGHT −4,8 when the vendor sells for no
/// currency.
fn money_bar(copper: u64) -> Element {
    let (x, y, width, height) = (FRAME_W - 166.0, FRAME_H - 25.0, 159.0, 19.0);
    let piece = |name: &str, coords: &str, rect: (f32, f32, f32, f32)| {
        let (px, py, pw, ph) = rect;
        rsx! {
            texture {
                name: {DynName(format!("MerchantMoneyBg{name}"))},
                width: pw,
                height: ph,
                texture_fdid: MONEY_EDGE,
                tex_coords: coords,
                pos_type: "absolute",
                left: px,
                top: py,
            }
        }
    };
    let mut children = piece(
        "Left",
        "0.953125,0.9921875,0.0,0.296875",
        (x, y, 7.0, height),
    );
    children.extend(piece(
        "Middle",
        "0.0,0.9921875,0.3125,0.609375",
        (x + 7.0, y, width - 14.0, height),
    ));
    children.extend(piece(
        "Right",
        "0.0,0.0546875,0.0,0.296875",
        (x + width - 7.0, y, 7.0, height),
    ));
    children.extend(money(
        "MerchantMoneyFrame",
        copper,
        (FRAME_W - 4.0 - 6.0, FRAME_H - 8.0),
        MoneyAlign::Right,
        false,
    ));
    children
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MoneyAlign {
    Left,
    Right,
}

const COIN_SIZE: f32 = 13.0;
const MONEY_FONT_SIZE: f32 = 12.0;
const MONEY_H: f32 = 14.0;

/// Denominations a `SmallMoneyFrameTemplate` shows: every non-zero one, or 0 copper.
fn coins(copper: u64) -> Vec<(u64, &'static AtlasArt)> {
    let parts = [
        (copper / 10_000, &COIN_GOLD),
        (copper / 100 % 100, &COIN_SILVER),
        (copper % 100, &COIN_COPPER),
    ];
    let shown: Vec<_> = parts
        .into_iter()
        .filter(|(amount, _)| *amount > 0)
        .collect();
    if shown.is_empty() {
        vec![(0, &COIN_COPPER)]
    } else {
        shown
    }
}

/// Amount + coin pairs, 4 px apart; `anchor` is the bottom-left (Left) or
/// bottom-right (Right) corner in parent space.
pub fn money(
    prefix: &str,
    copper: u64,
    anchor: (f32, f32),
    align: MoneyAlign,
    gray: bool,
) -> Element {
    let color = if gray {
        GRAY_FONT_COLOR
    } else {
        HIGHLIGHT_FONT_COLOR
    };
    money_colored(prefix, copper, anchor, align, color)
}

/// Each shown denomination's digits, their width and its coin.
fn money_parts(copper: u64) -> Vec<(String, f32, &'static AtlasArt)> {
    coins(copper)
        .into_iter()
        .map(|(amount, art)| {
            let digits = amount.to_string();
            let width = measure_text(&digits, GameFont::ArialNarrow, MONEY_FONT_SIZE)
                .map_or(7.0 * digits.len() as f32, |(w, _)| w.ceil());
            (digits, width, art)
        })
        .collect()
}

fn parts_width(parts: &[(String, f32, &AtlasArt)]) -> f32 {
    parts
        .iter()
        .map(|(_, w, _)| w + 1.0 + COIN_SIZE)
        .sum::<f32>()
        + 4.0 * (parts.len() as f32 - 1.0)
}

/// Width of [`money`]'s coins for `copper`.
pub fn money_width(copper: u64) -> f32 {
    parts_width(&money_parts(copper))
}

/// [`money`] with the amounts in `color` (the trainer's red unaffordable cost).
pub(crate) fn money_colored(
    prefix: &str,
    copper: u64,
    anchor: (f32, f32),
    align: MoneyAlign,
    color: &str,
) -> Element {
    let parts = money_parts(copper);
    let total = parts_width(&parts);
    let mut x = match align {
        MoneyAlign::Left => anchor.0,
        MoneyAlign::Right => anchor.0 - total,
    };
    let top = anchor.1 - MONEY_H;
    let mut children = Element::default();
    for (index, (digits, width, art)) in parts.iter().enumerate() {
        children.extend(text(Text {
            name: format!("{prefix}Amount{index}"),
            text: digits,
            rect: (x, top, *width + 1.0, MONEY_H),
            font: GameFont::ArialNarrow,
            size: MONEY_FONT_SIZE,
            color,
            justify: "RIGHT",
        }));
        x += width + 1.0;
        children.extend(atlas_texture(
            format!("{prefix}Coin{index}"),
            art,
            (x, top + (MONEY_H - COIN_SIZE) / 2.0, COIN_SIZE, COIN_SIZE),
        ));
        x += COIN_SIZE + 4.0;
    }
    children
}

/// `MerchantFrameTab1` CENTER at BOTTOMLEFT 50,-15 and Tab2 at its RIGHT -16,0
/// (MF.xml:576-594); PanelTabButtonTemplate 32 high, width = text + 20 but at
/// least both caps (SharedUIPanelTemplates.lua:393-395).
fn tabs(buyback_tab: bool) -> Element {
    let tab1_w = tab_width("Merchant");
    let tab2_w = tab_width("Buyback");
    let top = FRAME_H + 15.0 - 16.0;
    let tab1_x = 50.0 - tab1_w / 2.0;
    let merchant = tab(
        "MerchantFrameTab1",
        "Merchant",
        (tab1_x, top, tab1_w),
        !buyback_tab,
        ACTION_TAB_MERCHANT,
    );
    let buyback = tab(
        "MerchantFrameTab2",
        "Buyback",
        (tab1_x + tab1_w - 16.0, top, tab2_w),
        buyback_tab,
        ACTION_TAB_BUYBACK,
    );
    // The tabs overlap by 16 px; the selected one is drawn over the other.
    if buyback_tab {
        merchant.into_iter().chain(buyback).collect()
    } else {
        buyback.into_iter().chain(merchant).collect()
    }
}

pub(crate) fn tab_width(label: &str) -> f32 {
    let text_w = measure_text(label, GameFont::FrizQuadrata, 10.0).map_or(50.0, |(w, _)| w);
    (text_w + 20.0)
        .max(TAB_INACTIVE[0].size().0 + TAB_INACTIVE[2].size().0)
        .ceil()
}

pub(crate) fn tab(
    name: &str,
    label: &str,
    (x, y, width): (f32, f32, f32),
    selected: bool,
    action: &str,
) -> Element {
    let art = if selected { &TAB_ACTIVE } else { &TAB_INACTIVE };
    let height = art[0].size().1;
    let (left_w, right_w) = (art[0].size().0, art[2].size().0);
    // Left at TOPLEFT x -1 (active) / -3, Right at TOPRIGHT +8 / +7 (xml:909-935).
    let (left_x, right_x) = if selected { (-1.0, 8.0) } else { (-3.0, 7.0) };
    let mut children = atlas_texture(
        format!("{name}Left"),
        &art[0],
        (left_x, 0.0, left_w, height),
    );
    children.extend(atlas_texture(
        format!("{name}Middle"),
        &art[1],
        (
            left_x + left_w,
            0.0,
            (width + right_x - right_w) - (left_x + left_w),
            height,
        ),
    ));
    children.extend(atlas_texture(
        format!("{name}Right"),
        &art[2],
        (width + right_x - right_w, 0.0, right_w, height),
    ));
    // Text CENTER (0, 2) deselected, (0, -3) selected (SharedUIPanelTemplates.lua:524-542).
    let text_y = if selected {
        16.0 - 5.0 + 3.0
    } else {
        16.0 - 5.0 - 2.0
    };
    let color = if selected {
        HIGHLIGHT_FONT_COLOR
    } else {
        NORMAL_FONT_COLOR
    };
    children.extend(text(Text {
        name: format!("{name}Text"),
        text: label,
        rect: (0.0, text_y, width, 10.0),
        font: GameFont::FrizQuadrata,
        size: 10.0,
        color,
        justify: "CENTER",
    }));
    let action = if selected { "" } else { action };
    rsx! {
        r#frame {
            name: {DynName(name.into())},
            width,
            height: 32.0,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

#[cfg(all(test, feature = "dev"))]
#[path = "merchant_frame_component_tests.rs"]
mod tests;
