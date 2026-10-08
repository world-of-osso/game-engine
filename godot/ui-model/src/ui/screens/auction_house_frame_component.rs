//! Retail `AuctionHouseFrame` (Blizzard_AuctionHouseUI, see `docs/specs/auction-house-ui.md`):
//! Buy / Sell / Auctions tabs, search bar, category list, browse and item buy lists,
//! buy confirmation dialog, item sell frame, auctions and bids lists, money display.
//! Every position is the absolute result of the Retail anchors cited next to it.

use shared::protocol::AuctionDuration;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::quest_art::window_chrome;
use crate::ui::strata::FrameStrata;

#[path = "auction_house_frame_art.rs"]
mod art;
/// `InsetFrameTemplate` border, shared with the trade and mail frames.
pub(crate) use art::inset_border;
#[path = "auction_house_frame_component_auctions.rs"]
mod auctions_tab;
#[path = "auction_house_frame_component_buy.rs"]
mod buy_tab;
#[path = "auction_house_frame_component_sell.rs"]
mod sell_tab;

pub use art::money_parts;
use art::*;

/// `AuctionHouseFrame` size (Blizzard_AuctionHouseFrame.xml:5).
pub const FRAME_W: f32 = 800.0;
pub const FRAME_H: f32 = 538.0;
pub const ROOT_FRAME: &str = "AuctionHouseFrame";

pub const ACTION_CLOSE: &str = "auction_close";
pub const ACTION_TAB_PREFIX: &str = "auction_tab:";
pub const ACTION_SEARCH: &str = "auction_search";
pub const ACTION_SORT_PREFIX: &str = "auction_sort:";
pub const ACTION_CATEGORY_PREFIX: &str = "auction_category:";
pub const ACTION_BROWSE_ITEM_PREFIX: &str = "auction_browse_item:";
pub const ACTION_BACK: &str = "auction_back";
pub const ACTION_SELECT_AUCTION_PREFIX: &str = "auction_select:";
pub const ACTION_BID: &str = "auction_bid";
pub const ACTION_BUYOUT: &str = "auction_buyout";
pub const ACTION_DIALOG_BUY: &str = "auction_dialog_buy";
pub const ACTION_DIALOG_CANCEL: &str = "auction_dialog_cancel";
pub const ACTION_SELL_ITEM_PREFIX: &str = "auction_sell_item:";
pub const ACTION_SELL_CLEAR: &str = "auction_sell_clear";
pub const ACTION_MAX_QUANTITY: &str = "auction_max_quantity";
pub const ACTION_BUYOUT_MODE: &str = "auction_buyout_mode";
pub const ACTION_DURATION_MENU: &str = "auction_duration_menu";
pub const ACTION_DURATION_PREFIX: &str = "auction_duration:";
pub const ACTION_POST: &str = "auction_post";
pub const ACTION_AUCTIONS_TAB_PREFIX: &str = "auction_auctions_tab:";
pub const ACTION_CANCEL_AUCTION: &str = "auction_cancel";

/// Edit boxes; the frame registry owns their text.
pub const SEARCH_BOX: &str = "AuctionHouseFrameSearchBox";
pub const QUANTITY_BOX: &str = "AuctionHouseFrameItemSellFrameQuantityInputBox";
pub const SELL_BUYOUT_BOXES: MoneyBoxes = MoneyBoxes {
    gold: "AuctionHouseFrameItemSellFramePriceInputGold",
    silver: "AuctionHouseFrameItemSellFramePriceInputSilver",
    copper: "AuctionHouseFrameItemSellFramePriceInputCopper",
};
pub const SELL_BID_BOXES: MoneyBoxes = MoneyBoxes {
    gold: "AuctionHouseFrameItemSellFrameSecondaryPriceInputGold",
    silver: "AuctionHouseFrameItemSellFrameSecondaryPriceInputSilver",
    copper: "AuctionHouseFrameItemSellFrameSecondaryPriceInputCopper",
};
pub const BID_BOXES: MoneyBoxes = MoneyBoxes {
    gold: "AuctionHouseFrameBidAmountGold",
    silver: "AuctionHouseFrameBidAmountSilver",
    copper: "AuctionHouseFrameBidAmountCopper",
};

/// Gold / silver / copper edit box names of one money input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoneyBoxes {
    pub gold: &'static str,
    pub silver: &'static str,
    pub copper: &'static str,
}

impl MoneyBoxes {
    pub fn names(self) -> [&'static str; 3] {
        [self.gold, self.silver, self.copper]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AuctionHouseTab {
    #[default]
    Buy,
    Sell,
    Auctions,
}

impl AuctionHouseTab {
    pub const ALL: [Self; 3] = [Self::Buy, Self::Sell, Self::Auctions];

    pub fn token(self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::Auctions => "auctions",
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.token() == token)
    }

    /// `AUCTION_HOUSE_BUY_TAB`, `AUCTION_HOUSE_SELL_TAB`, `AUCTION_HOUSE_AUCTIONS_SUB_TAB`.
    fn label(self) -> &'static str {
        match self {
            Self::Buy => "Buy",
            Self::Sell => "Sell",
            Self::Auctions => "Auctions",
        }
    }

    /// `AuctionHouseFrameMixin:UpdateTitle` (Blizzard_AuctionHouseFrame.lua:640).
    pub fn title(self) -> &'static str {
        match self {
            Self::Buy => "Browse Auctions",
            Self::Sell => "Post Auctions",
            Self::Auctions => "Auctions",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AuctionsSubTab {
    #[default]
    Auctions,
    Bids,
}

impl AuctionsSubTab {
    pub fn token(self) -> &'static str {
        match self {
            Self::Auctions => "auctions",
            Self::Bids => "bids",
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        [Self::Auctions, Self::Bids]
            .into_iter()
            .find(|tab| tab.token() == token)
    }
}

/// Name, quality and icon of an item as the lists show it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ItemLine {
    pub name: String,
    pub quality: u8,
    pub icon_fdid: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryRow {
    pub name: String,
    pub selected: bool,
}

/// One item of the browse list (`GetBrowseListLayout`): lowest price and total available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowseRow {
    pub item_id: u32,
    pub item: ItemLine,
    pub price: u64,
    pub available: u64,
}

/// One auction: item buy list, all auctions and bids lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListingRow {
    pub auction_id: u64,
    pub item: ItemLine,
    pub quantity: u32,
    pub bid: Option<u64>,
    pub buyout: Option<u64>,
    pub time_left: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemBuyView {
    pub item: ItemLine,
    pub rows: Vec<ListingRow>,
    pub can_bid: bool,
    pub can_buyout: bool,
}

/// `AuctionHouseBuyDialog`: confirm a buyout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuyDialogView {
    pub item_text: String,
    pub price: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SellItemView {
    pub item: ItemLine,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SellInventoryRow {
    pub item_guid: u64,
    pub item: ItemLine,
    pub count: u32,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SellView {
    pub item: Option<SellItemView>,
    pub inventory: Vec<SellInventoryRow>,
    /// Current auctions of the selected item (right-hand list).
    pub listings: Vec<ListingRow>,
    pub buyout_mode: bool,
    pub duration: AuctionDuration,
    pub duration_menu_open: bool,
    pub deposit: u64,
    pub total: u64,
    pub can_post: bool,
}

impl Default for SellView {
    fn default() -> Self {
        Self {
            item: None,
            inventory: Vec::new(),
            listings: Vec::new(),
            buyout_mode: true,
            duration: AuctionDuration::Medium,
            duration_menu_open: false,
            deposit: 0,
            total: 0,
            can_post: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AuctionsView {
    pub tab: AuctionsSubTab,
    pub rows: Vec<ListingRow>,
    pub can_cancel: bool,
    pub can_bid: bool,
    pub can_buyout: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AuctionHouseFrameState {
    pub visible: bool,
    pub tab: AuctionHouseTab,
    pub money: u64,
    /// The search box holds no text: its "Search" instructions show.
    pub search_empty: bool,
    pub categories: Vec<CategoryRow>,
    pub browse: Vec<BrowseRow>,
    /// `BROWSE_NO_RESULTS` after a search that found nothing.
    pub browse_empty_text: Option<String>,
    pub item_buy: Option<ItemBuyView>,
    pub dialog: Option<BuyDialogView>,
    pub sell: SellView,
    pub auctions: AuctionsView,
}

/// Retail Blizzard_AuctionHouseFrame.lua:392 `SetPortraitToUnit("npc")`.
pub const PORTRAIT: crate::inworld_unit_frames_component::PortraitSlot =
    crate::quest_art::window_portrait_slot("AuctionHouseFramePortrait");

pub fn auction_house_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<AuctionHouseFrameState>()
        .expect("AuctionHouseFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: {DynName(ROOT_FRAME.to_string())},
            width: FRAME_W,
            height: FRAME_H,
            // Below `UIErrorsFrame` (`frameStrata="DIALOG"`, Blizzard_UIErrorsFrame/Mainline/UIErrorsFrame.xml) like every Retail
            // UIParent panel, so server errors show over the frame.
            strata: FrameStrata::High,
            hidden: hide,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
            {window_chrome(ROOT_FRAME, (FRAME_W, FRAME_H), state.tab.title(), ACTION_CLOSE)}
            {crate::quest_art::window_portrait(&PORTRAIT)}
            {money_frame(state.money)}
            {tabs(state.tab)}
            {buy_tab::buy_content(state)}
            {sell_tab::sell_content(state)}
            {auctions_tab::auctions_content(state)}
            {buy_tab::buy_dialog(state.dialog.as_ref())}
        }
    }
}

/// `MoneyFrameInset` (2,-27..167,-3 from the bottom) and `MoneyFrameBorder` 158×19 at
/// BOTTOMLEFT (5,6) holding the player's money right-aligned 6 px in
/// (Blizzard_AuctionHouseFrame.xml:11-40).
fn money_frame(money: u64) -> Element {
    let mut out = inset_border(
        "AuctionHouseFrameMoneyFrameInset",
        (2.0, 511.0, 165.0, 24.0),
    );
    out.extend(three_slice(
        "AuctionHouseFrameMoneyFrameBorder",
        [GOLD_EDGE_LEFT, GOLD_EDGE_MIDDLE, GOLD_EDGE_RIGHT],
        7.0,
        (5.0, 513.0, 158.0, 19.0),
    ));
    out.extend(money_display(
        "AuctionHouseFrameMoneyFrame",
        money,
        157.0,
        522.5,
    ));
    out
}

/// `AuctionHouseFrameDisplayModeTabTemplate` tabs: Buy at BOTTOMLEFT (20,-28), 32 tall
/// (Blizzard_AuctionHouseFrame.xml:44-67). The XML's LEFT -15 chaining is replaced at load by
/// `PanelTemplates_SetNumTabs` → `PanelTemplates_AnchorTabs`: each next tab TOPLEFT at the
/// previous TOPRIGHT + 3 (SharedUIPanelTemplates.lua:460-470).
fn tabs(active: AuctionHouseTab) -> Element {
    let mut x = 20.0;
    let mut out = Vec::new();
    for (index, tab) in AuctionHouseTab::ALL.into_iter().enumerate() {
        let width = tab_width(tab.label());
        let action = format!("{ACTION_TAB_PREFIX}{}", tab.token());
        out.extend(panel_tab(
            &format!("AuctionHouseFrameTab{}", index + 1),
            tab.label(),
            &action,
            tab == active,
            (x, 534.0, width),
            false,
        ));
        x += width + 3.0;
    }
    out
}

/// `PanelTemplates_TabResize(tab, TAB_PADDING = 20, nil, MIN_TAB_WIDTH = 70)`
/// (Blizzard_AuctionHouseTab.lua:2-11): text width + `TAB_SIDES_PADDING` (20) + 20.
fn tab_width(label: &str) -> f32 {
    let text = measure_text(label, GameFont::FrizQuadrata, 10.0).map_or(0.0, |(w, _)| w);
    (text + 40.0).max(70.0)
}

/// `PanelTabButtonTemplate` (SharedUIPanelTemplates.xml:905): left cap at x -3 (active -1),
/// right cap ending 7 (active 8) past the tab, 36 (active 42) tall; the label
/// `GameFontNormalSmall`, white while selected. `top` draws the
/// `PanelTopTabButtonTemplate` art: upside down, 75 % tall, anchored to the tab bottom.
fn panel_tab(
    name: &str,
    label: &str,
    action: &str,
    active: bool,
    (x, y, width): (f32, f32, f32),
    top: bool,
) -> Element {
    let (caps, left_x, right_x, art_h) = if active {
        (
            [TAB_ACTIVE_LEFT, TAB_ACTIVE_MIDDLE, TAB_ACTIVE_RIGHT],
            -1.0,
            8.0,
            42.0,
        )
    } else {
        ([TAB_LEFT, TAB_MIDDLE, TAB_RIGHT], -3.0, 7.0, 36.0)
    };
    let [left, middle, right] = caps.map(|crop| if top { crop.top_tab() } else { crop });
    let art_h = if top { art_h * 0.75 } else { art_h };
    let art_y = if top { 32.0 - art_h } else { 0.0 };
    let left_rect = (left_x, art_y, 35.0, art_h);
    let right_rect = (width + right_x - 37.0, art_y, 37.0, art_h);
    let middle_rect = (left_x + 35.0, art_y, right_rect.0 - left_x - 35.0, art_h);
    let text_color = if active {
        HIGHLIGHT_FONT_COLOR
    } else {
        NORMAL_FONT_COLOR
    };
    // `PanelTemplates_SelectTab` / `DeselectTab`: CENTER y -3 selected, +2 deselected (up is
    // positive); top tabs `-y - 7` / `-y - 6`. The label spans the tab, so top = -y.
    let label_y = match (top, active) {
        (false, true) => 3.0,
        (false, false) => -2.0,
        (true, true) => 4.0,
        (true, false) => 8.0,
    };
    let mut art = crop_texture(format!("{name}Left"), left, left_rect);
    art.extend(crop_texture(format!("{name}Middle"), middle, middle_rect));
    art.extend(crop_texture(format!("{name}Right"), right, right_rect));
    rsx! {
        button {
            name: {DynName(name.to_string())},
            width,
            height: 32.0,
            onclick: action,
            button_default_skin: false,
            pos_type: "absolute",
            left: x,
            top: y,
            {art}
            fontstring {
                name: {DynName(format!("{name}Text"))},
                width,
                height: 32.0,
                text: label,
                font: GameFont::FrizQuadrata,
                font_size: 10.0,
                font_color: text_color,
                shadow_color: SHADOW_COLOR,
                shadow_offset: "1,-1",
                justify_h: "CENTER",
                pos_type: "absolute",
                left: 0.0,
                top: label_y,
            }
        }
    }
}

/// Concatenates cell elements.
fn join<const N: usize>(parts: [Element; N]) -> Element {
    parts.into_iter().flatten().collect()
}

/// One list column: header label, left edge, width, left and right cell padding.
#[derive(Clone, Copy)]
struct Column {
    pub label: &'static str,
    pub x: f32,
    pub w: f32,
    pub pad_left: f32,
    pub pad_right: f32,
}

/// Lays out `AuctionHouseTableBuilder` columns (relative to the header container) across
/// `width` from `x`: fixed widths as
/// given, the single fill column (`w < 0`) takes the rest.
fn layout_columns<const N: usize>(
    x: f32,
    width: f32,
    specs: [(&'static str, f32, f32, f32); N],
) -> [Column; N] {
    let fixed: f32 = specs.iter().map(|spec| spec.1.max(0.0)).sum();
    let mut left = x;
    specs.map(|(label, w, pad_left, pad_right)| {
        let w = if w < 0.0 { width - fixed } else { w };
        let column = Column {
            label,
            x: left,
            w,
            pad_left,
            pad_right,
        };
        left += w;
        column
    })
}

/// `AuctionHouseItemListTemplate` (Blizzard_AuctionHouseItemList.xml:58): the inset
/// NineSlice and background start `backgroundYOffset` (19) below the list top, the header
/// container sits at (4,-1)..(-26,-1), 19 tall.
fn item_list_frame(
    prefix: &str,
    rect: (f32, f32, f32, f32),
    background: Crop,
    columns: &[Column],
) -> Element {
    let (x, y, w, h) = rect;
    let (bg_w, bg_h) = background.size();
    let mut out = crop_texture(
        format!("{prefix}Background"),
        background,
        (x + 3.0, y + 22.0, bg_w.min(w - 6.0), bg_h.min(h - 25.0)),
    );
    out.extend(inset_border(
        &format!("{prefix}NineSlice"),
        (x, y + 19.0, w, h - 19.0),
    ));
    for (index, column) in columns.iter().enumerate() {
        if column.label.is_empty() {
            continue;
        }
        out.extend(list_header(
            &format!("{prefix}Header{index}"),
            column,
            (x + 4.0, y + 1.0),
        ));
    }
    out
}

/// `AuctionHouseTableHeaderStringTemplate` label (`GameFontHighlightSmall`); columns are
/// relative to the header container at `origin`.
fn list_header(name: &str, column: &Column, (x, y): (f32, f32)) -> Element {
    let position = (x + column.x + column.pad_left, y);
    let Some(token) = buy_header_sort_token(name, column.label) else {
        return list_header_text(name, column, position);
    };
    let action = format!("{ACTION_SORT_PREFIX}{token}");
    let label = list_header_text(&format!("{name}Text"), column, (0.0, 0.0));
    rsx! {
        button {
            name: {DynName(name.to_string())},
            width: {column.w - column.pad_left - column.pad_right},
            height: 19.0,
            onclick: {action.as_str()},
            button_default_skin: false,
            pos_type: "absolute",
            left: {position.0},
            top: {position.1},
            {label}
        }
    }
}

fn buy_header_sort_token(name: &str, label: &str) -> Option<&'static str> {
    let is_buy_list = name.starts_with("AuctionHouseFrameBrowseResultsFrameItemList")
        || name.starts_with("AuctionHouseFrameItemBuyFrame");
    if !is_buy_list {
        return None;
    }
    match label {
        "Name" => Some("name"),
        "Price" | "Buyout Price" => Some("price"),
        "Current Bid" => Some("bid"),
        _ => None,
    }
}

fn list_header_text(name: &str, column: &Column, (x, y): (f32, f32)) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width: {column.w - column.pad_left - column.pad_right},
            height: 19.0,
            text: {column.label},
            font: GameFont::FrizQuadrata,
            font_size: 10.0,
            font_color: HIGHLIGHT_FONT_COLOR,
            shadow_color: SHADOW_COLOR,
            shadow_offset: "1,-1",
            justify_h: "LEFT",
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// Rows start 6 below the header container (`ScrollBox` TOPLEFT (0,-6)); 20 tall
/// (`AuctionHouseItemListLineTemplate`).
const ROW_H: f32 = 20.0;

/// A clickable list line: row stripe on odd rows (`auctionhouse-rowstripe-1`), the
/// `auctionhouse-ui-row-select` highlight when selected.
fn list_row(
    name: &str,
    rect: (f32, f32, f32, f32),
    index: usize,
    stripes: bool,
    selected: bool,
    action: &str,
    cells: Element,
) -> Element {
    let (x, y, w, h) = rect;
    // Stripe and selection are siblings drawn before the row: children of the row share
    // the cells' frame level and a later-added selection would cover the text.
    let mut out = Vec::new();
    if stripes && index % 2 == 1 {
        out.extend(crop_texture(format!("{name}Stripe"), ROW_STRIPE, rect));
    }
    if selected {
        out.extend(crop_texture(format!("{name}Selected"), ROW_SELECT, rect));
    }
    out.extend(rsx! {
        button {
            name: {DynName(name.to_string())},
            width: w,
            height: h,
            onclick: action,
            button_default_skin: false,
            pos_type: "absolute",
            left: x,
            top: y,
            {cells}
        }
    });
    out
}

/// `AuctionHouseTableCellItemDisplayTemplate`: 14×14 icon with the 16×16
/// `auctionhouse-itemicon-small-border`, name in the item's quality colour, `xN` stacks.
fn item_cell(name: &str, item: &ItemLine, quantity: u32, column: &Column) -> Element {
    let x = column.x + column.pad_left;
    let mut out = icon_texture(
        format!("{name}Icon"),
        item.icon_fdid,
        (x + 1.0, 3.0, 14.0, 14.0),
    );
    out.extend(crop_texture(
        format!("{name}IconBorder"),
        ITEM_ICON_SMALL_BORDER,
        (x, 2.0, 16.0, 16.0),
    ));
    let text = if quantity > 1 {
        format!("{} x{quantity}", item.name)
    } else {
        item.name.clone()
    };
    out.extend(text_cell(
        &format!("{name}Name"),
        &text,
        quality_color(item.quality),
        (
            x + 20.0,
            column.w - column.pad_left - column.pad_right - 20.0,
        ),
        "LEFT",
    ));
    out
}

/// `AuctionHouseTableCellTextTemplate` (`Number14FontWhite`), 16 tall, row-centred.
fn text_cell(name: &str, text: &str, color: &str, (x, w): (f32, f32), justify: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width: {w.max(1.0)},
            height: 16.0,
            text,
            font: GameFont::ArialNarrow,
            font_size: 14.0,
            font_color: color,
            shadow_color: SHADOW_COLOR,
            shadow_offset: "1,-1",
            justify_h: justify,
            pos_type: "absolute",
            left: x,
            top: 2.0,
        }
    }
}

/// A money cell right-aligned inside `column` (`AuctionHouseTableMoneyDisplayTemplate`).
fn money_cell(name: &str, copper: Option<u64>, column: &Column) -> Element {
    let Some(copper) = copper else {
        return Vec::new();
    };
    money_display(name, copper, column.x + column.w - column.pad_right, 10.0)
}

/// `UIPanelButtonTemplate` (`defaultbutton-nineslice-*`); disabled buttons carry no action.
fn panel_button(
    name: &str,
    text: &str,
    action: &str,
    enabled: bool,
    rect: (f32, f32, f32, f32),
) -> Element {
    crate::ui::screens::quest_art::panel_button(name.to_string(), text, action, enabled, rect)
}

/// A `GameFontNormal` label (gold, shadowed).
fn label(name: &str, text: &str, rect: (f32, f32, f32, f32), justify: &str) -> Element {
    let (x, y, w, h) = rect;
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width: w,
            height: h,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: NORMAL_FONT_COLOR,
            shadow_color: SHADOW_COLOR,
            shadow_offset: "1,-1",
            justify_h: justify,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// An edit box without text: the registry keeps what the player typed across rebuilds.
fn edit_box(name: &'static str, rect: (f32, f32, f32, f32), insets: &str) -> Element {
    let (x, y, w, h) = rect;
    rsx! {
        editbox {
            name: {DynName(name.to_string())},
            width: w,
            height: h,
            font: GameFont::ArialNarrow,
            font_size: 14.0,
            font_color: HIGHLIGHT_FONT_COLOR,
            text_insets: insets,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// `LargeMoneyInputFrameTemplate` 190×33 (Blizzard_MoneyFrame/Shared/MoneyInputFrame.xml:33):
/// copper 50 wide at the right, silver 50 six to its left, gold filling the rest; each a
/// `LargeInputBoxTemplate` with its coin 12×14 at RIGHT (-10, 2).
fn large_money_input(boxes: MoneyBoxes, (x, y): (f32, f32)) -> Element {
    let parts = [
        (boxes.gold, x, 78.0, COIN_GOLD),
        (boxes.silver, x + 84.0, 50.0, COIN_SILVER),
        (boxes.copper, x + 140.0, 50.0, COIN_COPPER),
    ];
    parts
        .into_iter()
        .flat_map(|(name, bx, w, coin)| {
            let mut out = large_input_art(name, (bx, y, w, 33.0));
            out.extend(edit_box(name, (bx, y, w - 22.0, 33.0), "10,0,0,5"));
            out.extend(crop_texture(
                format!("{name}Icon"),
                coin,
                (bx + w - 22.0, y + 7.5, 12.0, 14.0),
            ));
            out
        })
        .collect()
}

/// `LargeInputBoxTemplate` art: `auctionhouse-ui-inputfield-*` caps (8 wide at this
/// 33 px height) and middle (InputBoxTemplates.xml:13).
fn large_input_art(name: &str, rect: (f32, f32, f32, f32)) -> Element {
    three_slice(
        &format!("{name}Art"),
        [INPUT_LEFT, INPUT_MIDDLE, INPUT_RIGHT],
        8.0,
        rect,
    )
}

/// `MoneyInputFrameTemplate` 176×18 (Blizzard_MoneyFrame/Mainline/MoneyInputFrame.xml:72):
/// gold 70, silver and copper 48 wide, 10 apart; `InputBoxVisualTemplate` borders with the
/// coin at the right.
fn small_money_input(boxes: MoneyBoxes, (x, y): (f32, f32)) -> Element {
    let parts = [
        (boxes.gold, x, 70.0, COIN_GOLD),
        (boxes.silver, x + 80.0, 48.0, COIN_SILVER),
        (boxes.copper, x + 138.0, 48.0, COIN_COPPER),
    ];
    parts
        .into_iter()
        .flat_map(|(name, bx, w, coin)| {
            let mut out = search_border(name, (bx, y, w, 20.0));
            out.extend(edit_box(name, (bx, y, w - 14.0, 20.0), "0,0,0,0"));
            out.extend(crop_texture(
                format!("{name}Icon"),
                coin,
                (bx + w - 13.0, y + 3.0, 12.0, 14.0),
            ));
            out
        })
        .collect()
}

/// `InputBoxVisualTemplate` (InputBoxTemplates.xml:43): 8×20 caps, left one 5 px outside.
fn search_border(name: &str, (x, y, w, h): (f32, f32, f32, f32)) -> Element {
    three_slice(
        &format!("{name}Border"),
        [SEARCH_LEFT, SEARCH_MIDDLE, SEARCH_RIGHT],
        8.0,
        (x - 5.0, y + (h - 20.0) / 2.0, w + 5.0, 20.0),
    )
}

/// Retail's AuctionHouse `C_AuctionHouse` duration labels (`AUCTION_DURATION_ONE..THREE`).
pub fn duration_label(duration: AuctionDuration) -> &'static str {
    match duration {
        AuctionDuration::Short => "1 Day",
        AuctionDuration::Medium => "1 Week",
        AuctionDuration::Long => "2 Weeks",
    }
}

pub fn duration_token(duration: AuctionDuration) -> &'static str {
    match duration {
        AuctionDuration::Short => "12",
        AuctionDuration::Medium => "24",
        AuctionDuration::Long => "48",
    }
}
