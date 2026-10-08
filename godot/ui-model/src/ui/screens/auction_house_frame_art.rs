//! Retail auction house art: `UiTextureAtlasMember.csv` crops on their `UiTextureAtlas.csv`
//! sheets, plus shared element helpers (textures, three-slices, money display, inset border).
//! Atlas names are the ones `Blizzard_AuctionHouseUI` and `Blizzard_SharedXML` reference.

use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

pub(super) struct DynName(pub String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Pixel crop (left, right, top, bottom) on an atlas sheet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Crop {
    pub fdid: u32,
    sheet: (f32, f32),
    rect: (f32, f32, f32, f32),
}

impl Crop {
    /// `PanelTopTabButtonMixin:OnLoad` (SharedUIPanelTemplates.lua:280-286) shows the lower
    /// 75 % of the art upside down (`SetTexCoord(0, 1, 1, 0.25)`): top edge at the member's
    /// bottom row, bottom edge 25 % below its top.
    pub fn top_tab(self) -> Self {
        let (l, r, t, b) = self.rect;
        Self {
            rect: (l, r, b, t + 0.25 * (b - t)),
            ..self
        }
    }

    /// Atlas member size in pixels.
    pub fn size(self) -> (f32, f32) {
        let (l, r, t, b) = self.rect;
        ((r - l).abs(), (b - t).abs())
    }

    pub fn tex_coords(self) -> String {
        let (w, h) = self.sheet;
        let (l, r, t, b) = self.rect;
        format!("{},{},{},{}", l / w, r / w, t / h, b / h)
    }
}

/// UiTextureAtlas 1495, 1024×1024: auction house widgets.
const AH_SHEET: u32 = 3_046_538;
/// UiTextureAtlas 1499, 2048×1024: auction house panel backgrounds.
const AH_BG_SHEET: u32 = 3_054_898;
/// UiTextureAtlas 2134, 64×256: `PanelTabButtonTemplate` art.
const TAB_SHEET: u32 = 4_707_839;
/// UiTextureAtlas 948 / 949 / 950: `InsetFrameTemplate` corners, side and top/bottom tiles.
const INSET_CORNERS: u32 = 1_723_831;
const INSET_SIDES: u32 = 1_723_832;
const INSET_TOP_BOTTOM: u32 = 1_723_833;
/// UiTextureAtlas 3172, 2048×1024: `common-search-border-*` of `InputBoxVisualTemplate`.
const SEARCH_SHEET: u32 = 6_725_697;
/// `Interface\Common\Moneyframe` (`ThinGoldEdgeTemplate`), 128×64.
const MONEY_FRAME: u32 = 525_911;

const fn ah(l: f32, r: f32, t: f32, b: f32) -> Crop {
    Crop {
        fdid: AH_SHEET,
        sheet: (1024.0, 1024.0),
        rect: (l, r, t, b),
    }
}

const fn ah_bg(l: f32, r: f32, t: f32, b: f32) -> Crop {
    Crop {
        fdid: AH_BG_SHEET,
        sheet: (2048.0, 1024.0),
        rect: (l, r, t, b),
    }
}

const fn tab(l: f32, r: f32, t: f32, b: f32) -> Crop {
    Crop {
        fdid: TAB_SHEET,
        sheet: (64.0, 256.0),
        rect: (l, r, t, b),
    }
}

const fn sheet(fdid: u32, w: f32, h: f32, l: f32, r: f32, t: f32, b: f32) -> Crop {
    Crop {
        fdid,
        sheet: (w, h),
        rect: (l, r, t, b),
    }
}

// Backgrounds (member IDs in parentheses).
pub(super) const BG_INDEX: Crop = ah_bg(370.0, 965.0, 400.0, 813.0); // auctionhouse-background-index (9545) 595×413
pub(super) const BG_CATEGORIES: Crop = ah_bg(1731.0, 1869.0, 400.0, 833.0); // -categories (9544) 138×433
pub(super) const BG_SELL_LEFT: Crop = ah_bg(1.0, 358.0, 399.0, 836.0); // -sell-left (9546) 357×437
pub(super) const BG_SELL_RIGHT: Crop = ah_bg(1107.0, 1506.0, 400.0, 818.0); // -sell-right (9547) 399×418
pub(super) const BG_BUY_HEADER: Crop = ah_bg(370.0, 987.0, 317.0, 398.0); // -buy-noncommodities-header (9542) 617×81
pub(super) const BG_BUY_MARKET: Crop = ah_bg(967.0, 1562.0, 1.0, 278.0); // -buy-noncommodities-market (9543) 595×277
pub(super) const BG_SUMMARY_LIST: Crop = ah_bg(967.0, 1105.0, 400.0, 833.0); // -summarylist (9738) 138×433

// Rows and navigation.
pub(super) const NAV_BUTTON: Crop = ah(651.0, 923.0, 147.0, 211.0); // auctionhouse-nav-button (9516)
pub(super) const NAV_BUTTON_SELECT: Crop = ah(293.0, 557.0, 281.0, 323.0); // -nav-button-select (9512)
pub(super) const ROW_STRIPE: Crop = ah(1007.0, 1023.0, 125.0, 143.0); // auctionhouse-rowstripe-1 (9518)
pub(super) const ROW_SELECT: Crop = ah(827.0, 943.0, 215.0, 233.0); // auctionhouse-ui-row-select (9532)
pub(super) const ITEM_ICON_SMALL_BORDER: Crop = ah(98.0, 130.0, 991.0, 1023.0); // -itemicon-small-border (9517)
pub(super) const ITEM_HEADER_FRAME: Crop = ah(1.0, 685.0, 1.0, 145.0); // auctionhouse-itemheaderframe (9498) 684×144
pub(super) const ITEM_ICON_EMPTY: Crop = ah(139.0, 239.0, 853.0, 953.0); // auctionhouse-itemicon-empty (9530)
pub(super) const CLOCK_ICON: Crop = ah(266.0, 290.0, 991.0, 1015.0); // auctionhouse-icon-clock (10753)
pub(super) const SELL_TAB_LEFT: Crop = ah(1011.0, 1020.0, 1.0, 24.0); // auctionhouse-selltab-left (9725) 9×23
pub(super) const SELL_TAB_MIDDLE: Crop = ah(1.0, 96.0, 991.0, 1014.0); // -selltab-middle (9726)
pub(super) const SELL_TAB_RIGHT: Crop = ah(277.0, 286.0, 439.0, 462.0); // -selltab-right (9727) 9×23
pub(super) const INPUT_LEFT: Crop = ah(241.0, 257.0, 853.0, 919.0); // auctionhouse-ui-inputfield-left (9499)
pub(super) const INPUT_MIDDLE: Crop = ah(293.0, 649.0, 147.0, 213.0); // -inputfield-middle (9500)
pub(super) const INPUT_RIGHT: Crop = ah(259.0, 275.0, 853.0, 919.0); // -inputfield-right (9501)

// Coins: `auctionhouse-icon-coin-*` (9520..9522), 20×20.
pub(super) const COIN_GOLD: Crop = ah(985.0, 1005.0, 125.0, 145.0);
pub(super) const COIN_SILVER: Crop = ah(954.0, 974.0, 147.0, 167.0);
pub(super) const COIN_COPPER: Crop = ah(963.0, 983.0, 125.0, 145.0);

// `PanelTabButtonTemplate` (SharedUIPanelTemplates.xml:905): inactive caps 35/37×36, active 35/37×42.
pub(super) const TAB_LEFT: Crop = tab(1.0, 36.0, 209.0, 245.0); // uiframe-tab-left (17155)
pub(super) const TAB_MIDDLE: Crop = tab(0.0, 1.0, 45.0, 81.0); // _uiframe-tab-center (17158)
pub(super) const TAB_RIGHT: Crop = tab(1.0, 38.0, 171.0, 207.0); // uiframe-tab-right (17156)
pub(super) const TAB_ACTIVE_LEFT: Crop = tab(1.0, 36.0, 127.0, 169.0); // uiframe-activetab-left (17153)
pub(super) const TAB_ACTIVE_MIDDLE: Crop = tab(0.0, 1.0, 1.0, 43.0); // _uiframe-activetab-center (17157)
pub(super) const TAB_ACTIVE_RIGHT: Crop = tab(1.0, 38.0, 83.0, 125.0); // uiframe-activetab-right (17154)

// `InputBoxVisualTemplate` (InputBoxTemplates.xml:43) search border, 16/224/16×40 atlas pixels.
pub(super) const SEARCH_LEFT: Crop = sheet(SEARCH_SHEET, 2048.0, 1024.0, 1970.0, 1986.0, 1.0, 41.0);
pub(super) const SEARCH_MIDDLE: Crop =
    sheet(SEARCH_SHEET, 2048.0, 1024.0, 1744.0, 1968.0, 1.0, 41.0);
pub(super) const SEARCH_RIGHT: Crop =
    sheet(SEARCH_SHEET, 2048.0, 1024.0, 1615.0, 1631.0, 27.0, 67.0);
pub(super) const SEARCH_GLASS: Crop =
    sheet(SEARCH_SHEET, 2048.0, 1024.0, 1615.0, 1639.0, 1.0, 25.0);

// `ThinGoldEdgeTemplate` (UIPanelTemplates.xml:1314) tex coords on Moneyframe.
pub(super) const GOLD_EDGE_LEFT: Crop = Crop {
    fdid: MONEY_FRAME,
    sheet: (1.0, 1.0),
    rect: (0.953125, 0.9921875, 0.0, 0.296875),
};
pub(super) const GOLD_EDGE_RIGHT: Crop = Crop {
    fdid: MONEY_FRAME,
    sheet: (1.0, 1.0),
    rect: (0.0, 0.0546875, 0.0, 0.296875),
};
pub(super) const GOLD_EDGE_MIDDLE: Crop = Crop {
    fdid: MONEY_FRAME,
    sheet: (1.0, 1.0),
    rect: (0.0, 0.9921875, 0.3125, 0.609375),
};

// `InsetFrameTemplate` (NineSliceLayouts.lua:81): 6×6 corners, 3 px tiles.
const INSET_TOP_LEFT: Crop = sheet(INSET_CORNERS, 128.0, 128.0, 97.0, 103.0, 71.0, 77.0);
const INSET_TOP_RIGHT: Crop = sheet(INSET_CORNERS, 128.0, 128.0, 105.0, 111.0, 71.0, 77.0);
const INSET_BOTTOM_LEFT: Crop = sheet(INSET_CORNERS, 128.0, 128.0, 81.0, 87.0, 71.0, 77.0);
const INSET_BOTTOM_RIGHT: Crop = sheet(INSET_CORNERS, 128.0, 128.0, 89.0, 95.0, 71.0, 77.0);
const INSET_LEFT: Crop = sheet(INSET_SIDES, 64.0, 256.0, 31.0, 34.0, 0.0, 256.0);
const INSET_RIGHT: Crop = sheet(INSET_SIDES, 64.0, 256.0, 36.0, 39.0, 0.0, 256.0);
const INSET_TOP: Crop = sheet(INSET_TOP_BOTTOM, 256.0, 128.0, 0.0, 256.0, 116.0, 119.0);
const INSET_BOTTOM: Crop = sheet(INSET_TOP_BOTTOM, 256.0, 128.0, 0.0, 256.0, 111.0, 114.0);

/// `NORMAL_FONT_COLOR`.
pub(super) const NORMAL_FONT_COLOR: &str = "1.0,0.82,0.0,1.0";
/// `HIGHLIGHT_FONT_COLOR`.
pub(super) const HIGHLIGHT_FONT_COLOR: &str = "1.0,1.0,1.0,1.0";
pub(super) const SHADOW_COLOR: &str = "0.0,0.0,0.0,1.0";

/// `ITEM_QUALITY_COLORS` (ColorConstants) by quality 0 Poor .. 5 Legendary.
pub(super) fn quality_color(quality: u8) -> &'static str {
    match quality {
        0 => "0.62,0.62,0.62,1.0",
        1 => "1.0,1.0,1.0,1.0",
        2 => "0.12,1.0,0.0,1.0",
        3 => "0.0,0.44,0.87,1.0",
        4 => "0.64,0.21,0.93,1.0",
        5 => "1.0,0.5,0.0,1.0",
        _ => "0.9,0.8,0.5,1.0",
    }
}

/// An absolutely positioned texture showing `crop` in `rect` (x, y, w, h).
pub(super) fn crop_texture(name: String, crop: Crop, rect: (f32, f32, f32, f32)) -> Element {
    let coords = crop.tex_coords();
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: {crop.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// An icon texture by FileDataID; `0` (unknown item) draws nothing.
pub(super) fn icon_texture(name: String, fdid: u32, rect: (f32, f32, f32, f32)) -> Element {
    if fdid == 0 {
        return Vec::new();
    }
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

/// Left cap, stretched middle, right cap across `rect`.
pub(super) fn three_slice(
    prefix: &str,
    [left, middle, right]: [Crop; 3],
    cap_w: f32,
    rect: (f32, f32, f32, f32),
) -> Element {
    let (x, y, w, h) = rect;
    let mut out = crop_texture(format!("{prefix}Left"), left, (x, y, cap_w, h));
    out.extend(crop_texture(
        format!("{prefix}Middle"),
        middle,
        (x + cap_w, y, (w - 2.0 * cap_w).max(0.0), h),
    ));
    out.extend(crop_texture(
        format!("{prefix}Right"),
        right,
        (x + w - cap_w, y, cap_w, h),
    ));
    out
}

/// `InsetFrameTemplate` NineSlice border around `rect`.
pub(crate) fn inset_border(prefix: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, w, h) = rect;
    let parts = [
        ("TopEdge", INSET_TOP, (x + 6.0, y, w - 12.0, 3.0)),
        (
            "BottomEdge",
            INSET_BOTTOM,
            (x + 6.0, y + h - 3.0, w - 12.0, 3.0),
        ),
        ("LeftEdge", INSET_LEFT, (x, y + 6.0, 3.0, h - 12.0)),
        (
            "RightEdge",
            INSET_RIGHT,
            (x + w - 3.0, y + 6.0, 3.0, h - 12.0),
        ),
        ("TopLeft", INSET_TOP_LEFT, (x, y, 6.0, 6.0)),
        ("TopRight", INSET_TOP_RIGHT, (x + w - 6.0, y, 6.0, 6.0)),
        // BottomLeft/RightCorner carry `y = -1`.
        ("BottomLeft", INSET_BOTTOM_LEFT, (x, y + h - 5.0, 6.0, 6.0)),
        (
            "BottomRight",
            INSET_BOTTOM_RIGHT,
            (x + w - 6.0, y + h - 5.0, 6.0, 6.0),
        ),
    ];
    parts
        .into_iter()
        .flat_map(|(part, crop, rect)| crop_texture(format!("{prefix}{part}"), crop, rect))
        .collect()
}

/// `PriceFontWhite` (ArialNarrow 14) used by `MoneyDenominationDisplayTemplate`.
pub(super) const PRICE_FONT_SIZE: f32 = 14.0;
/// `DENOMINATION_DISPLAY_WIDTH` (MoneyFrame.lua:472): space for two digits and the icon.
const DENOMINATION_W: f32 = 36.0;
/// Coin icon size for atlas display types (MoneyFrame.lua:388).
const COIN_W: f32 = 12.0;
const COIN_H: f32 = 14.0;

/// Gold / silver / copper text for a copper amount: `MoneyDisplayFrameMixin` always shows
/// silver and copper, gold only when non-zero, gold with `BreakUpLargeNumbers` separators.
pub fn money_parts(copper: u64) -> (Option<String>, String, String) {
    let gold = copper / 10_000;
    let silver = (copper / 100) % 100;
    let copper = copper % 100;
    let gold = (gold > 0).then(|| break_up_large_number(gold));
    (gold, silver.to_string(), copper.to_string())
}

fn break_up_large_number(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Width of the money display for `copper` (`MoneyDisplayFrameMixin:UpdateWidth`).
pub(super) fn money_width(copper: u64) -> f32 {
    let (gold, _, _) = money_parts(copper);
    match gold {
        Some(gold) => denomination_w(&gold) + 2.0 * DENOMINATION_W,
        None => denomination_w("00") + DENOMINATION_W,
    }
}

fn denomination_w(text: &str) -> f32 {
    measure_text(text, GameFont::ArialNarrow, PRICE_FONT_SIZE).map_or(0.0, |(w, _)| w)
        + COIN_W
        + 1.0
}

/// A right-aligned `MoneyDisplayFrameTemplate` (AH icons) ending at `right`, centred on `mid_y`.
pub(super) fn money_display(prefix: &str, copper: u64, right: f32, mid_y: f32) -> Element {
    let (gold, silver, copper_text) = money_parts(copper);
    let mut out = denomination(
        &format!("{prefix}Copper"),
        &copper_text,
        COIN_COPPER,
        right,
        mid_y,
    );
    let silver_right = right - DENOMINATION_W;
    out.extend(denomination(
        &format!("{prefix}Silver"),
        &silver,
        COIN_SILVER,
        silver_right,
        mid_y,
    ));
    if let Some(gold) = gold {
        out.extend(denomination(
            &format!("{prefix}Gold"),
            &gold,
            COIN_GOLD,
            silver_right - DENOMINATION_W,
            mid_y,
        ));
    }
    out
}

/// `MoneyDenominationDisplayTemplate`: icon at the right, text right-aligned 13 px left of it.
fn denomination(name: &str, text: &str, coin: Crop, right: f32, mid_y: f32) -> Element {
    let text_w = 80.0;
    let mut out = rsx! {
        fontstring {
            name: {DynName(format!("{name}Text"))},
            width: text_w,
            height: 16.0,
            text,
            font: GameFont::ArialNarrow,
            font_size: PRICE_FONT_SIZE,
            font_color: HIGHLIGHT_FONT_COLOR,
            shadow_color: SHADOW_COLOR,
            shadow_offset: "1,-1",
            justify_h: "RIGHT",
            pos_type: "absolute",
            left: {right - 13.0 - text_w},
            top: {mid_y - 8.0},
        }
    };
    out.extend(crop_texture(
        format!("{name}Icon"),
        coin,
        (right - COIN_W, mid_y - COIN_H / 2.0, COIN_W, COIN_H),
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_parts_hide_zero_gold_and_keep_zero_silver_and_copper() {
        assert_eq!(money_parts(305), (None, "3".into(), "5".into()));
        assert_eq!(money_parts(0), (None, "0".into(), "0".into()));
        assert_eq!(
            money_parts(12_340_506),
            (Some("1,234".into()), "5".into(), "6".into())
        );
    }
}
