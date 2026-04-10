use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::screens::menu_primitives::{DropdownButton, dropdown_button};
use crate::ui::strata::FrameStrata;

#[path = "auction_house_frame_component_auctions.rs"]
mod auctions_tab;
#[path = "auction_house_frame_component_sell.rs"]
mod sell_tab;

use auctions_tab::auctions_tab_content;
use sell_tab::sell_tab_content;

#[cfg(test)]
use auctions_tab::LISTING_COLUMNS;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const FRAME_W: f32 = 608.0;
pub const FRAME_H: f32 = 486.0;
const HEADER_H: f32 = 30.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 12.0;
const CONTENT_INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";

// Browse tab layout
const SEARCH_BAR_H: f32 = 28.0;
const SEARCH_BAR_INSET: f32 = 4.0;
const SIDEBAR_W: f32 = 160.0;
const SIDEBAR_GAP: f32 = 4.0;
const RESULTS_HEADER_H: f32 = 22.0;
const RESULT_ROW_H: f32 = 24.0;
const RESULT_ROW_GAP: f32 = 1.0;

const SEARCH_BAR_BG: &str = "0.1,0.1,0.1,0.9";
const SEARCH_BAR_TEXT: &str = "0.5,0.5,0.5,0.8";
const SIDEBAR_BG: &str = "0.0,0.0,0.0,0.4";
const CAT_ROW_H: f32 = 18.0;
const CAT_ROW_GAP: f32 = 1.0;
const CAT_SELECTED_BG: &str = "0.2,0.15,0.05,0.95";
const CAT_NORMAL_BG: &str = "0.0,0.0,0.0,0.0";
const CAT_SELECTED_COLOR: &str = "1.0,0.82,0.0,1.0";
const CAT_NORMAL_COLOR: &str = "1.0,1.0,1.0,1.0";
const HEADER_BG: &str = "0.12,0.1,0.08,0.9";
const HEADER_TEXT_COLOR: &str = "0.8,0.8,0.8,1.0";
const ROW_BG_EVEN: &str = "0.04,0.04,0.04,0.6";
const ROW_BG_ODD: &str = "0.06,0.06,0.06,0.6";
const ROW_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const GOLD_COLOR: &str = "1.0,0.82,0.0,1.0";

// Sell tab layout
const SELL_INSET: f32 = 12.0;
const SELL_ITEM_SLOT_SIZE: f32 = 48.0;
const SELL_INPUT_H: f32 = 26.0;
const SELL_INPUT_W: f32 = 120.0;
const SELL_LABEL_W: f32 = 80.0;
const SELL_ROW_GAP: f32 = 8.0;
const SELL_DROPDOWN_W: f32 = 140.0;
const SELL_BUTTON_W: f32 = 100.0;
const SELL_BUTTON_H: f32 = 28.0;
const SELL_ITEM_SLOT_BG: &str = "0.08,0.07,0.06,0.88";
const SELL_INPUT_BG: &str = "0.1,0.1,0.1,0.9";
const SELL_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const SELL_INPUT_TEXT: &str = "1.0,1.0,1.0,1.0";
const SELL_BUTTON_BG: &str = "0.2,0.15,0.05,0.95";
const SELL_BUTTON_TEXT: &str = "1.0,0.82,0.0,1.0";

pub const MAX_BROWSE_CATEGORIES: usize = 12;
pub const MAX_RESULT_ROWS: usize = 8;
pub const RESULT_COLUMNS: &[(&str, f32)] = &[
    ("Name", 0.40),
    ("Level", 0.10),
    ("Time Left", 0.15),
    ("Seller", 0.15),
    ("Bid", 0.10),
    ("Buyout", 0.10),
];

#[derive(Clone, Debug, PartialEq)]
pub struct AuctionTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowseCategory {
    pub name: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowseResultRow {
    pub name: String,
    pub level: String,
    pub time_left: String,
    pub seller: String,
    pub bid: String,
    pub buyout: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SellTabState {
    /// Name of the item placed in the sell slot (empty = no item).
    pub item_name: String,
    /// Starting bid price text.
    pub bid_price: String,
    /// Buyout price text.
    pub buyout_price: String,
    /// Selected duration label (e.g. "12 Hours", "24 Hours", "48 Hours").
    pub duration: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuctionListingRow {
    pub name: String,
    pub time_left: String,
    pub bid: String,
    pub buyout: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuctionHouseFrameState {
    pub visible: bool,
    pub tabs: Vec<AuctionTab>,
    pub browse_categories: Vec<BrowseCategory>,
    pub browse_results: Vec<BrowseResultRow>,
    pub sell: SellTabState,
    pub my_auctions: Vec<AuctionListingRow>,
}

impl Default for AuctionHouseFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                AuctionTab {
                    name: "Browse".into(),
                    active: true,
                },
                AuctionTab {
                    name: "Sell".into(),
                    active: false,
                },
                AuctionTab {
                    name: "Auctions".into(),
                    active: false,
                },
            ],
            browse_categories: vec![],
            browse_results: vec![],
            sell: SellTabState {
                duration: "24 Hours".into(),
                ..Default::default()
            },
            my_auctions: vec![],
        }
    }
}

pub fn auction_house_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<AuctionHouseFrameState>()
        .expect("AuctionHouseFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "AuctionHouseFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "100",
                y: "-80",
            }
            {title_bar()}
            {tab_row(&state.tabs)}
            {browse_tab_content(&state.browse_categories, &state.browse_results)}
            {sell_tab_content(&state.sell)}
            {auctions_tab_content(&state.my_auctions)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "AuctionHouseFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Auction House",
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: "0",
            }
        }
    }
}

fn tab_row(tabs: &[AuctionTab]) -> Element {
    let count = tabs.len().max(1) as f32;
    let tab_w = (FRAME_W - 2.0 * TAB_INSET - (count - 1.0) * TAB_GAP) / count;
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let x = TAB_INSET + i as f32 * (tab_w + TAB_GAP);
            let y = -(HEADER_H + TAB_GAP);
            tab_button(i, tab, tab_w, x, y)
        })
        .collect()
}

fn tab_button(i: usize, tab: &AuctionTab, tab_w: f32, x: f32, y: f32) -> Element {
    let tab_id = DynName(format!("AuctionHouseTab{i}"));
    let label_id = DynName(format!("AuctionHouseTab{i}Label"));
    let (bg, color) = if tab.active {
        (TAB_BG_ACTIVE, TAB_TEXT_ACTIVE)
    } else {
        (TAB_BG_INACTIVE, TAB_TEXT_INACTIVE)
    };
    rsx! {
        r#frame {
            name: tab_id,
            width: {tab_w},
            height: {TAB_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {auction_tab_label(label_id, &tab.name, tab_w, color)}
        }
    }
}

fn auction_tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {TAB_H},
            text: text,
            font_size: 11.0,
            font_color: color,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn browse_tab_content(categories: &[BrowseCategory], results: &[BrowseResultRow]) -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    rsx! {
        r#frame {
            name: "AuctionHouseContentArea",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
            {browse_search_bar(content_w)}
            {browse_category_sidebar(categories)}
            {browse_results_panel(results, content_w)}
        }
    }
}

fn browse_search_bar(parent_w: f32) -> Element {
    let bar_w = parent_w - 2.0 * SEARCH_BAR_INSET;
    rsx! {
        r#frame {
            name: "AuctionHouseBrowseSearchBar",
            width: {bar_w},
            height: {SEARCH_BAR_H},
            background_color: SEARCH_BAR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SEARCH_BAR_INSET},
                y: {-SEARCH_BAR_INSET},
            }
            fontstring {
                name: "AuctionHouseBrowseSearchText",
                width: {bar_w - 8.0},
                height: {SEARCH_BAR_H},
                text: "Search...",
                font_size: 10.0,
                font_color: SEARCH_BAR_TEXT,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
        }
    }
}

fn browse_category_sidebar(categories: &[BrowseCategory]) -> Element {
    let top_y = -(SEARCH_BAR_INSET + SEARCH_BAR_H + SIDEBAR_GAP);
    let sidebar_h = FRAME_H
        - CONTENT_TOP
        - CONTENT_INSET
        - SEARCH_BAR_INSET
        - SEARCH_BAR_H
        - SIDEBAR_GAP
        - SEARCH_BAR_INSET;
    let rows: Element = categories
        .iter()
        .enumerate()
        .take(MAX_BROWSE_CATEGORIES)
        .flat_map(|(i, cat)| browse_category_row(i, cat))
        .collect();
    rsx! {
        r#frame {
            name: "AuctionHouseBrowseCategorySidebar",
            width: {SIDEBAR_W},
            height: {sidebar_h},
            background_color: SIDEBAR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SEARCH_BAR_INSET},
                y: {top_y},
            }
            {rows}
        }
    }
}

fn browse_category_row(idx: usize, cat: &BrowseCategory) -> Element {
    let row_id = DynName(format!("AuctionHouseBrowseCat{idx}"));
    let label_id = DynName(format!("AuctionHouseBrowseCat{idx}Label"));
    let (bg, color) = if cat.selected {
        (CAT_SELECTED_BG, CAT_SELECTED_COLOR)
    } else {
        (CAT_NORMAL_BG, CAT_NORMAL_COLOR)
    };
    let y = -(idx as f32 * (CAT_ROW_H + CAT_ROW_GAP));
    rsx! {
        r#frame {
            name: row_id,
            width: {SIDEBAR_W},
            height: {CAT_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            {browse_cat_label(label_id, &cat.name, color)}
        }
    }
}

fn browse_cat_label(id: DynName, text: &str, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {SIDEBAR_W - 8.0},
            height: {CAT_ROW_H},
            text: text,
            font_size: 9.0,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
        }
    }
}

fn browse_results_panel(results: &[BrowseResultRow], parent_w: f32) -> Element {
    let panel_x = SEARCH_BAR_INSET + SIDEBAR_W + SIDEBAR_GAP;
    let panel_y = -(SEARCH_BAR_INSET + SEARCH_BAR_H + SIDEBAR_GAP);
    let panel_w = parent_w - panel_x - SEARCH_BAR_INSET;
    let panel_h = FRAME_H
        - CONTENT_TOP
        - CONTENT_INSET
        - SEARCH_BAR_INSET
        - SEARCH_BAR_H
        - SIDEBAR_GAP
        - SEARCH_BAR_INSET;
    let header = results_header(panel_w);
    let rows: Element = results
        .iter()
        .enumerate()
        .take(MAX_RESULT_ROWS)
        .flat_map(|(i, row)| result_row(i, row, panel_w))
        .collect();
    rsx! {
        r#frame {
            name: "AuctionHouseBrowseResults",
            width: {panel_w},
            height: {panel_h},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {panel_x},
                y: {panel_y},
            }
            {header}
            {rows}
        }
    }
}

fn results_header(panel_w: f32) -> Element {
    let cols: Element = RESULT_COLUMNS
        .iter()
        .enumerate()
        .flat_map(|(i, (name, _))| {
            let x = column_x(panel_w, i);
            let w = column_w(panel_w, i);
            results_header_cell(i, name, x, w)
        })
        .collect();
    rsx! {
        r#frame {
            name: "AuctionHouseBrowseResultsHeader",
            width: {panel_w},
            height: {RESULTS_HEADER_H},
            background_color: HEADER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
            }
            {cols}
        }
    }
}

fn results_header_cell(idx: usize, text: &str, x: f32, w: f32) -> Element {
    let cell_id = DynName(format!("AuctionHouseResultsCol{idx}"));
    rsx! {
        fontstring {
            name: cell_id,
            width: {w},
            height: {RESULTS_HEADER_H},
            text,
            font_size: 9.0,
            font_color: HEADER_TEXT_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "0",
            }
        }
    }
}

fn result_row(idx: usize, row: &BrowseResultRow, panel_w: f32) -> Element {
    let row_id = DynName(format!("AuctionHouseResult{idx}"));
    let y = -(RESULTS_HEADER_H + idx as f32 * (RESULT_ROW_H + RESULT_ROW_GAP));
    let bg = if idx.is_multiple_of(2) {
        ROW_BG_EVEN
    } else {
        ROW_BG_ODD
    };
    let cells = result_row_cells(idx, row, panel_w);
    rsx! {
        r#frame {
            name: row_id,
            width: {panel_w},
            height: {RESULT_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            {cells}
        }
    }
}

fn result_row_cells(idx: usize, row: &BrowseResultRow, panel_w: f32) -> Element {
    let values = [
        &row.name,
        &row.level,
        &row.time_left,
        &row.seller,
        &row.bid,
        &row.buyout,
    ];
    values
        .iter()
        .enumerate()
        .flat_map(|(col, text)| {
            let cell_id = DynName(format!("AuctionHouseResult{idx}Col{col}"));
            let x = column_x(panel_w, col);
            let w = column_w(panel_w, col);
            let color = if col >= 4 { GOLD_COLOR } else { ROW_TEXT_COLOR };
            result_cell(cell_id, text, x, w, color)
        })
        .collect()
}

fn result_cell(name: DynName, text: &str, x: f32, w: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name,
            width: {w},
            height: {RESULT_ROW_H},
            text,
            font_size: 9.0,
            font_color: color,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "0",
            }
        }
    }
}

fn column_x(panel_w: f32, col: usize) -> f32 {
    let mut x = 4.0;
    for (_, width_frac) in RESULT_COLUMNS.iter().take(col) {
        x += width_frac * panel_w;
    }
    x
}

fn column_w(panel_w: f32, col: usize) -> f32 {
    RESULT_COLUMNS[col].1 * panel_w
}

#[cfg(test)]
#[path = "auction_house_frame_component_tests.rs"]
mod tests;
