use super::*;

const AUCTION_INSET: f32 = 4.0;
const LISTING_ROW_H: f32 = 24.0;
const LISTING_ROW_GAP: f32 = 1.0;
const LISTING_HEADER_H: f32 = 22.0;
const CANCEL_BUTTON_W: f32 = 80.0;
const CANCEL_BUTTON_H: f32 = 24.0;
const CANCEL_BUTTON_BG: &str = "0.25,0.08,0.08,0.95";
const CANCEL_BUTTON_TEXT_COLOR: &str = "1.0,0.4,0.4,1.0";

pub(super) const MAX_LISTING_ROWS: usize = 10;
pub(super) const LISTING_COLUMNS: &[(&str, f32)] = &[
    ("Item", 0.30),
    ("Time Left", 0.15),
    ("Bid", 0.15),
    ("Buyout", 0.15),
    ("Status", 0.15),
    ("", 0.10),
];

pub(super) fn auctions_tab_content(listings: &[AuctionListingRow]) -> Element {
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
    let bg = if idx.is_multiple_of(2) {
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
            let color = if (2..=3).contains(&col) {
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
    for (_, width_frac) in LISTING_COLUMNS.iter().take(col) {
        x += width_frac * panel_w;
    }
    x
}

fn listing_col_w(panel_w: f32, col: usize) -> f32 {
    LISTING_COLUMNS[col].1 * panel_w
}
