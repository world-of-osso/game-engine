use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

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
    let bg = if tab.active {
        TAB_BG_ACTIVE
    } else {
        TAB_BG_INACTIVE
    };
    let color = if tab.active {
        TAB_TEXT_ACTIVE
    } else {
        TAB_TEXT_INACTIVE
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
            fontstring {
                name: label_id,
                width: {tab_w},
                height: {TAB_H},
                text: {tab.name.as_str()},
                font_size: 11.0,
                font_color: color,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
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
    let bg = if cat.selected {
        CAT_SELECTED_BG
    } else {
        CAT_NORMAL_BG
    };
    let color = if cat.selected {
        CAT_SELECTED_COLOR
    } else {
        CAT_NORMAL_COLOR
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
            fontstring {
                name: label_id,
                width: {SIDEBAR_W - 8.0},
                height: {CAT_ROW_H},
                text: {cat.name.as_str()},
                font_size: 9.0,
                font_color: color,
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
    let bg = if idx % 2 == 0 {
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
    for i in 0..col {
        x += RESULT_COLUMNS[i].1 * panel_w;
    }
    x
}

fn column_w(panel_w: f32, col: usize) -> f32 {
    RESULT_COLUMNS[col].1 * panel_w
}

// --- Sell tab ---

fn sell_tab_content(sell: &SellTabState) -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    rsx! {
        r#frame {
            name: "AuctionHouseSellTab",
            width: {content_w},
            height: {content_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
            {sell_item_slot(sell)}
            {sell_price_row("AuctionHouseSellBid", "Starting Bid:", &sell.bid_price, 0)}
            {sell_price_row("AuctionHouseSellBuyout", "Buyout Price:", &sell.buyout_price, 1)}
            {sell_duration_row(&sell.duration)}
            {sell_post_button()}
        }
    }
}

fn sell_item_slot(sell: &SellTabState) -> Element {
    let label = if sell.item_name.is_empty() {
        "Drop item here"
    } else {
        sell.item_name.as_str()
    };
    rsx! {
        r#frame {
            name: "AuctionHouseSellItemSlot",
            width: {SELL_ITEM_SLOT_SIZE},
            height: {SELL_ITEM_SLOT_SIZE},
            background_color: SELL_ITEM_SLOT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET},
                y: {-SELL_INSET},
            }
        }
        fontstring {
            name: "AuctionHouseSellItemName",
            width: 200.0,
            height: 16.0,
            text: label,
            font_size: 11.0,
            font_color: SELL_LABEL_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET + SELL_ITEM_SLOT_SIZE + 8.0},
                y: {-(SELL_INSET + 16.0)},
            }
        }
    }
}

fn sell_price_row(prefix: &str, label: &str, value: &str, row_index: usize) -> Element {
    let row_name = DynName(format!("{prefix}Row"));
    let label_name = DynName(format!("{prefix}Label"));
    let input_name = DynName(format!("{prefix}Input"));
    let base_y = SELL_INSET + SELL_ITEM_SLOT_SIZE + SELL_ROW_GAP;
    let y = -(base_y + row_index as f32 * (SELL_INPUT_H + SELL_ROW_GAP));
    rsx! {
        r#frame {
            name: row_name,
            width: {SELL_LABEL_W + SELL_INPUT_W},
            height: {SELL_INPUT_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET},
                y: {y},
            }
            fontstring {
                name: label_name,
                width: {SELL_LABEL_W},
                height: {SELL_INPUT_H},
                text: label,
                font_size: 10.0,
                font_color: SELL_LABEL_COLOR,
                justify_h: "RIGHT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
            r#frame {
                name: input_name,
                width: {SELL_INPUT_W},
                height: {SELL_INPUT_H},
                background_color: SELL_INPUT_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {SELL_LABEL_W},
                    y: "0",
                }
                fontstring {
                    name: DynName(format!("{prefix}Value")),
                    width: {SELL_INPUT_W - 8.0},
                    height: {SELL_INPUT_H},
                    text: value,
                    font_size: 10.0,
                    font_color: SELL_INPUT_TEXT,
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
}

fn sell_duration_row(duration: &str) -> Element {
    let base_y = SELL_INSET + SELL_ITEM_SLOT_SIZE + SELL_ROW_GAP;
    let y = -(base_y + 2.0 * (SELL_INPUT_H + SELL_ROW_GAP));
    rsx! {
        r#frame {
            name: "AuctionHouseSellDurationRow",
            width: {SELL_LABEL_W + SELL_DROPDOWN_W},
            height: {SELL_INPUT_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET},
                y: {y},
            }
            fontstring {
                name: "AuctionHouseSellDurationLabel",
                width: {SELL_LABEL_W},
                height: {SELL_INPUT_H},
                text: "Duration:",
                font_size: 10.0,
                font_color: SELL_LABEL_COLOR,
                justify_h: "RIGHT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
            r#frame {
                name: "AuctionHouseSellDurationDropdown",
                width: {SELL_DROPDOWN_W},
                height: {SELL_INPUT_H},
                background_color: SELL_INPUT_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {SELL_LABEL_W},
                    y: "0",
                }
                fontstring {
                    name: "AuctionHouseSellDurationValue",
                    width: {SELL_DROPDOWN_W - 8.0},
                    height: {SELL_INPUT_H},
                    text: duration,
                    font_size: 10.0,
                    font_color: SELL_INPUT_TEXT,
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
}

fn sell_post_button() -> Element {
    let base_y = SELL_INSET + SELL_ITEM_SLOT_SIZE + SELL_ROW_GAP;
    let y = -(base_y + 3.0 * (SELL_INPUT_H + SELL_ROW_GAP));
    rsx! {
        r#frame {
            name: "AuctionHouseSellPostButton",
            width: {SELL_BUTTON_W},
            height: {SELL_BUTTON_H},
            background_color: SELL_BUTTON_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SELL_INSET + SELL_LABEL_W},
                y: {y},
            }
            fontstring {
                name: "AuctionHouseSellPostButtonText",
                width: {SELL_BUTTON_W},
                height: {SELL_BUTTON_H},
                text: "Create Auction",
                font_size: 11.0,
                font_color: SELL_BUTTON_TEXT,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

// --- Auctions tab ---

const AUCTION_INSET: f32 = 4.0;
const LISTING_ROW_H: f32 = 24.0;
const LISTING_ROW_GAP: f32 = 1.0;
const LISTING_HEADER_H: f32 = 22.0;
const CANCEL_BUTTON_W: f32 = 80.0;
const CANCEL_BUTTON_H: f32 = 24.0;
const CANCEL_BUTTON_BG: &str = "0.25,0.08,0.08,0.95";
const CANCEL_BUTTON_TEXT_COLOR: &str = "1.0,0.4,0.4,1.0";

pub const MAX_LISTING_ROWS: usize = 10;
pub const LISTING_COLUMNS: &[(&str, f32)] = &[
    ("Item", 0.30),
    ("Time Left", 0.15),
    ("Bid", 0.15),
    ("Buyout", 0.15),
    ("Status", 0.15),
    ("", 0.10),
];

fn auctions_tab_content(listings: &[AuctionListingRow]) -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    let panel_w = content_w - 2.0 * AUCTION_INSET;
    let header = listing_header(panel_w);
    let rows: Element = listings
        .iter()
        .enumerate()
        .take(MAX_LISTING_ROWS)
        .flat_map(|(i, row)| listing_row(i, row, panel_w))
        .collect();
    rsx! {
        r#frame {
            name: "AuctionHouseAuctionsTab",
            width: {content_w},
            height: {content_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
            {header}
            {rows}
        }
    }
}

fn listing_header(panel_w: f32) -> Element {
    let cols: Element = LISTING_COLUMNS
        .iter()
        .enumerate()
        .flat_map(|(i, (name, _))| {
            let x = listing_col_x(panel_w, i);
            let w = listing_col_w(panel_w, i);
            listing_header_cell(i, name, x, w)
        })
        .collect();
    rsx! {
        r#frame {
            name: "AuctionHouseListingHeader",
            width: {panel_w},
            height: {LISTING_HEADER_H},
            background_color: HEADER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {AUCTION_INSET},
                y: {-AUCTION_INSET},
            }
            {cols}
        }
    }
}

fn listing_header_cell(idx: usize, text: &str, x: f32, w: f32) -> Element {
    let cell_id = DynName(format!("AuctionHouseListingCol{idx}"));
    rsx! {
        fontstring {
            name: cell_id,
            width: {w},
            height: {LISTING_HEADER_H},
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

fn listing_row(idx: usize, row: &AuctionListingRow, panel_w: f32) -> Element {
    let row_id = DynName(format!("AuctionHouseListing{idx}"));
    let header_offset = AUCTION_INSET + LISTING_HEADER_H;
    let y = -(header_offset + idx as f32 * (LISTING_ROW_H + LISTING_ROW_GAP));
    let bg = if idx % 2 == 0 {
        ROW_BG_EVEN
    } else {
        ROW_BG_ODD
    };
    let cells = listing_row_cells(idx, row, panel_w);
    let cancel = listing_cancel_button(idx, panel_w);
    rsx! {
        r#frame {
            name: row_id,
            width: {panel_w},
            height: {LISTING_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {AUCTION_INSET},
                y: {y},
            }
            {cells}
            {cancel}
        }
    }
}

fn listing_row_cells(idx: usize, row: &AuctionListingRow, panel_w: f32) -> Element {
    let values = [
        &row.name,
        &row.time_left,
        &row.bid,
        &row.buyout,
        &row.status,
    ];
    values
        .iter()
        .enumerate()
        .flat_map(|(col, text)| {
            let cell_id = DynName(format!("AuctionHouseListing{idx}Col{col}"));
            let x = listing_col_x(panel_w, col);
            let w = listing_col_w(panel_w, col);
            let color = if col >= 2 && col <= 3 {
                GOLD_COLOR
            } else {
                ROW_TEXT_COLOR
            };
            result_cell(cell_id, text, x, w, color)
        })
        .collect()
}

fn listing_cancel_button(idx: usize, panel_w: f32) -> Element {
    let btn_id = DynName(format!("AuctionHouseListing{idx}Cancel"));
    let txt_id = DynName(format!("AuctionHouseListing{idx}CancelText"));
    let x = panel_w - CANCEL_BUTTON_W - 4.0;
    rsx! {
        r#frame {
            name: btn_id,
            width: {CANCEL_BUTTON_W},
            height: {CANCEL_BUTTON_H},
            background_color: CANCEL_BUTTON_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "0",
            }
            fontstring {
                name: txt_id,
                width: {CANCEL_BUTTON_W},
                height: {CANCEL_BUTTON_H},
                text: "Cancel",
                font_size: 9.0,
                font_color: CANCEL_BUTTON_TEXT_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

fn listing_col_x(panel_w: f32, col: usize) -> f32 {
    let mut x = 4.0;
    for i in 0..col {
        x += LISTING_COLUMNS[i].1 * panel_w;
    }
    x
}

fn listing_col_w(panel_w: f32, col: usize) -> f32 {
    LISTING_COLUMNS[col].1 * panel_w
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> AuctionHouseFrameState {
        AuctionHouseFrameState {
            visible: true,
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(auction_house_frame_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_registry() -> FrameRegistry {
        let mut reg = build_registry();
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    fn make_browse_state() -> AuctionHouseFrameState {
        let mut state = make_test_state();
        state.browse_categories = vec![
            BrowseCategory {
                name: "Weapons".into(),
                selected: true,
            },
            BrowseCategory {
                name: "Armor".into(),
                selected: false,
            },
            BrowseCategory {
                name: "Consumables".into(),
                selected: false,
            },
        ];
        state.browse_results = vec![
            BrowseResultRow {
                name: "Arcanite Reaper".into(),
                level: "58".into(),
                time_left: "Long".into(),
                seller: "Arthas".into(),
                bid: "50g".into(),
                buyout: "80g".into(),
            },
            BrowseResultRow {
                name: "Thunderfury".into(),
                level: "60".into(),
                time_left: "Medium".into(),
                seller: "Illidan".into(),
                bid: "500g".into(),
                buyout: "1000g".into(),
            },
        ];
        state
    }

    fn browse_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_browse_state());
        Screen::new(auction_house_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn builds_expected_frames() {
        let reg = build_registry();
        assert!(reg.get_by_name("AuctionHouseFrame").is_some());
        assert!(reg.get_by_name("AuctionHouseFrameTitle").is_some());
        assert!(reg.get_by_name("AuctionHouseContentArea").is_some());
    }

    #[test]
    fn browse_tab_builds_search_bar() {
        let reg = build_registry();
        assert!(reg.get_by_name("AuctionHouseBrowseSearchBar").is_some());
        assert!(reg.get_by_name("AuctionHouseBrowseSearchText").is_some());
    }

    #[test]
    fn browse_tab_builds_category_sidebar() {
        let reg = browse_registry();
        assert!(
            reg.get_by_name("AuctionHouseBrowseCategorySidebar")
                .is_some()
        );
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("AuctionHouseBrowseCat{i}"))
                    .is_some(),
                "AuctionHouseBrowseCat{i} missing"
            );
        }
    }

    #[test]
    fn browse_tab_builds_results_header() {
        let reg = browse_registry();
        assert!(reg.get_by_name("AuctionHouseBrowseResults").is_some());
        assert!(reg.get_by_name("AuctionHouseBrowseResultsHeader").is_some());
        for i in 0..RESULT_COLUMNS.len() {
            assert!(
                reg.get_by_name(&format!("AuctionHouseResultsCol{i}"))
                    .is_some(),
                "AuctionHouseResultsCol{i} missing"
            );
        }
    }

    #[test]
    fn browse_tab_builds_result_rows() {
        let reg = browse_registry();
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("AuctionHouseResult{i}")).is_some(),
                "AuctionHouseResult{i} missing"
            );
            for col in 0..RESULT_COLUMNS.len() {
                assert!(
                    reg.get_by_name(&format!("AuctionHouseResult{i}Col{col}"))
                        .is_some(),
                    "AuctionHouseResult{i}Col{col} missing"
                );
            }
        }
    }

    #[test]
    fn builds_three_tabs() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("AuctionHouseTab{i}")).is_some(),
                "AuctionHouseTab{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("AuctionHouseTab{i}Label"))
                    .is_some(),
                "AuctionHouseTab{i}Label missing"
            );
        }
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.visible = false;
        shared.insert(state);
        Screen::new(auction_house_frame_screen).sync(&shared, &mut reg);

        let id = reg.get_by_name("AuctionHouseFrame").expect("frame");
        let frame = reg.get(id).expect("data");
        assert!(frame.hidden);
    }

    // --- Coord validation ---

    const FRAME_X: f32 = 100.0;
    const FRAME_Y: f32 = 80.0;

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        let r = rect(&reg, "AuctionHouseFrame");
        assert!((r.x - FRAME_X).abs() < 1.0);
        assert!((r.y - FRAME_Y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_tabs() {
        let reg = layout_registry();
        let tab_count = 3.0_f32;
        let tab_w = (FRAME_W - 2.0 * TAB_INSET - (tab_count - 1.0) * TAB_GAP) / tab_count;
        let tab_y = FRAME_Y + HEADER_H + TAB_GAP;
        let tab0 = rect(&reg, "AuctionHouseTab0");
        assert!((tab0.x - (FRAME_X + TAB_INSET)).abs() < 1.0);
        assert!((tab0.y - tab_y).abs() < 1.0);
        assert!((tab0.width - tab_w).abs() < 1.0);
        let tab2 = rect(&reg, "AuctionHouseTab2");
        let expected_x2 = FRAME_X + TAB_INSET + 2.0 * (tab_w + TAB_GAP);
        assert!((tab2.x - expected_x2).abs() < 1.0);
    }

    #[test]
    fn coord_content_area() {
        let reg = layout_registry();
        let r = rect(&reg, "AuctionHouseContentArea");
        assert!((r.x - (FRAME_X + CONTENT_INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + CONTENT_TOP)).abs() < 1.0);
        let expected_w = FRAME_W - 2.0 * CONTENT_INSET;
        let expected_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
        assert!((r.width - expected_w).abs() < 1.0);
        assert!((r.height - expected_h).abs() < 1.0);
    }

    // --- Sell tab tests ---

    #[test]
    fn sell_tab_builds_item_slot_and_name() {
        let reg = build_registry();
        assert!(reg.get_by_name("AuctionHouseSellTab").is_some());
        assert!(reg.get_by_name("AuctionHouseSellItemSlot").is_some());
        assert!(reg.get_by_name("AuctionHouseSellItemName").is_some());
    }

    #[test]
    fn sell_tab_builds_price_inputs() {
        let reg = build_registry();
        assert!(reg.get_by_name("AuctionHouseSellBidRow").is_some());
        assert!(reg.get_by_name("AuctionHouseSellBidInput").is_some());
        assert!(reg.get_by_name("AuctionHouseSellBuyoutRow").is_some());
        assert!(reg.get_by_name("AuctionHouseSellBuyoutInput").is_some());
    }

    #[test]
    fn sell_tab_builds_duration_dropdown() {
        let reg = build_registry();
        assert!(reg.get_by_name("AuctionHouseSellDurationRow").is_some());
        assert!(
            reg.get_by_name("AuctionHouseSellDurationDropdown")
                .is_some()
        );
        assert!(reg.get_by_name("AuctionHouseSellDurationValue").is_some());
    }

    #[test]
    fn sell_tab_builds_post_button() {
        let reg = build_registry();
        assert!(reg.get_by_name("AuctionHouseSellPostButton").is_some());
        assert!(reg.get_by_name("AuctionHouseSellPostButtonText").is_some());
    }

    // --- Auctions tab tests ---

    fn make_auctions_state() -> AuctionHouseFrameState {
        let mut state = make_test_state();
        state.my_auctions = vec![
            AuctionListingRow {
                name: "Arcanite Reaper".into(),
                time_left: "Long".into(),
                bid: "50g".into(),
                buyout: "80g".into(),
                status: "Active".into(),
            },
            AuctionListingRow {
                name: "Lionheart Helm".into(),
                time_left: "Short".into(),
                bid: "200g".into(),
                buyout: "350g".into(),
                status: "Sold".into(),
            },
        ];
        state
    }

    fn auctions_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_auctions_state());
        Screen::new(auction_house_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn auctions_tab_builds_root_and_header() {
        let reg = auctions_registry();
        assert!(reg.get_by_name("AuctionHouseAuctionsTab").is_some());
        assert!(reg.get_by_name("AuctionHouseListingHeader").is_some());
        for i in 0..LISTING_COLUMNS.len() {
            assert!(
                reg.get_by_name(&format!("AuctionHouseListingCol{i}"))
                    .is_some(),
                "AuctionHouseListingCol{i} missing"
            );
        }
    }

    #[test]
    fn auctions_tab_builds_listing_rows() {
        let reg = auctions_registry();
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("AuctionHouseListing{i}"))
                    .is_some(),
                "AuctionHouseListing{i} missing"
            );
            for col in 0..5 {
                assert!(
                    reg.get_by_name(&format!("AuctionHouseListing{i}Col{col}"))
                        .is_some(),
                    "AuctionHouseListing{i}Col{col} missing"
                );
            }
        }
    }

    #[test]
    fn auctions_tab_builds_cancel_buttons() {
        let reg = auctions_registry();
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("AuctionHouseListing{i}Cancel"))
                    .is_some(),
                "AuctionHouseListing{i}Cancel missing"
            );
            assert!(
                reg.get_by_name(&format!("AuctionHouseListing{i}CancelText"))
                    .is_some(),
                "AuctionHouseListing{i}CancelText missing"
            );
        }
    }
}
