//! Auctions mode: `AuctionsFrame` with its Auctions / Bids top tabs, summary list, the
//! all-auctions or bids list, Cancel Auction and the bid / buyout controls.

use super::*;

/// Rows that fit the list `ScrollBox` (98..508).
pub const AUCTIONS_ROWS: usize = 20;

pub(super) fn auctions_content(state: &AuctionHouseFrameState) -> Element {
    let hide = state.tab != AuctionHouseTab::Auctions;
    let view = &state.auctions;
    let mut body = top_tabs(view.tab);
    body.extend(summary_list(view.tab));
    body.extend(auctions_list(view));
    if !hide {
        // Shares the bid inputs with the item buy frame: only the visible mode builds them.
        body.extend(controls(view));
    }
    rsx! {
        r#frame {
            name: "AuctionHouseFrameAuctionsFrame",
            width: FRAME_W,
            height: FRAME_H,
            hidden: hide,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
            {body}
        }
    }
}

/// `AuctionsTab` (PanelTopTabButtonTemplate) at TOPLEFT (47,-1) of the frame's (5,42) origin,
/// `BidsTab` 3 right of it (`PanelTemplates_AnchorTabs`)
/// (Mainline/Blizzard_AuctionHouseAuctionsFrame.xml:56-62).
fn top_tabs(active: AuctionsSubTab) -> Element {
    let auctions_w = tab_width("Auctions");
    let mut out = panel_tab(
        "AuctionHouseFrameAuctionsFrameAuctionsTab",
        "Auctions",
        &format!("{ACTION_AUCTIONS_TAB_PREFIX}auctions"),
        active == AuctionsSubTab::Auctions,
        (52.0, 43.0, auctions_w),
        true,
    );
    out.extend(panel_tab(
        "AuctionHouseFrameAuctionsFrameBidsTab",
        "Bids",
        &format!("{ACTION_AUCTIONS_TAB_PREFIX}bids"),
        active == AuctionsSubTab::Bids,
        (52.0 + auctions_w + 3.0, 43.0, tab_width("Bids")),
        true,
    ));
    out
}

/// `SummaryList` 168 wide, 2 into the tab bottom down to the Cancel button, LEFT -1 →
/// (4,73)..(172,511), with its one `AUCTION_HOUSE_ALL_AUCTIONS` / `_ALL_BIDS` line selected.
fn summary_list(tab: AuctionsSubTab) -> Element {
    let (x, y, w, h) = (4.0, 73.0, 168.0, 438.0);
    let mut out = crop_texture(
        "AuctionHouseFrameAuctionsFrameSummaryListBackground".into(),
        BG_SUMMARY_LIST,
        (x + 3.0, y + 3.0, 138.0, 433.0),
    );
    out.extend(inset_border(
        "AuctionHouseFrameAuctionsFrameSummaryListNineSlice",
        (x, y, w, h),
    ));
    let text = match tab {
        AuctionsSubTab::Auctions => "All Auctions",
        AuctionsSubTab::Bids => "All Bids",
    };
    out.extend(crop_texture(
        "AuctionHouseFrameAuctionsFrameSummaryLine1Selected".into(),
        ROW_SELECT,
        (x + 3.0, y + 3.0, 148.0, 21.0),
    ));
    out.extend(label(
        "AuctionHouseFrameAuctionsFrameSummaryLine1Text",
        text,
        (x + 7.0, y + 3.0, 140.0, 21.0),
        "LEFT",
    ));
    out
}

/// `GetAllAuctionsLayout` (Blizzard_AuctionHouseTableBuilder.lua:969): Name fill, Bid 120,
/// Buyout 120, time left 50.
pub(super) fn all_auctions_columns() -> [Column; 4] {
    layout_columns(
        0.0,
        593.0,
        [
            ("Item", -1.0, 10.0, 0.0),
            ("Bid Price", 120.0, 10.0, 0.0),
            ("Buyout Price", 120.0, 10.0, 0.0),
            ("", 50.0, 0.0, 10.0),
        ],
    )
}

/// `GetBidsListLayout` (…TableBuilder.lua:983): Name fill, Bid 120, Buyout 120, band 140.
pub(super) fn bids_columns() -> [Column; 4] {
    layout_columns(
        0.0,
        593.0,
        [
            ("Item", -1.0, 10.0, 0.0),
            ("Bid Price", 120.0, 10.0, 0.0),
            ("Buyout Price", 120.0, 10.0, 0.0),
            ("", 140.0, 0.0, 10.0),
        ],
    )
}

/// `AllAuctionsList` / `BidsList` from the summary list's right to RIGHT -5 → (172,74) 623×437
/// on `auctionhouse-background-index` (Mainline/Blizzard_AuctionHouseAuctionsFrame.xml:129-153).
fn auctions_list(view: &AuctionsView) -> Element {
    let columns = match view.tab {
        AuctionsSubTab::Auctions => all_auctions_columns(),
        AuctionsSubTab::Bids => bids_columns(),
    };
    let prefix = match view.tab {
        AuctionsSubTab::Auctions => "AuctionHouseFrameAuctionsFrameAllAuctionsList",
        AuctionsSubTab::Bids => "AuctionHouseFrameAuctionsFrameBidsList",
    };
    let mut out = item_list_frame(prefix, (172.0, 74.0, 623.0, 437.0), BG_INDEX, &columns);
    out.extend(crop_texture(
        format!("{prefix}TimeLeftHeader"),
        CLOCK_ICON,
        (176.0 + columns[3].x + 10.0, 76.0, 16.0, 16.0),
    ));
    for (index, row) in view.rows.iter().take(AUCTIONS_ROWS).enumerate() {
        let name = format!("{prefix}Row{}", index + 1);
        let cells = join([
            item_cell(&format!("{name}Item"), &row.item, row.quantity, &columns[0]),
            money_cell(&format!("{name}Bid"), row.bid, &columns[1]),
            money_cell(&format!("{name}Buyout"), row.buyout, &columns[2]),
            text_cell(
                &format!("{name}TimeLeft"),
                &row.time_left,
                HIGHLIGHT_FONT_COLOR,
                (
                    columns[3].x + columns[3].pad_left,
                    columns[3].w - columns[3].pad_right,
                ),
                "LEFT",
            ),
        ]);
        let action = format!("{ACTION_SELECT_AUCTION_PREFIX}{}", row.auction_id);
        out.extend(list_row(
            &name,
            (176.0, 100.0 + index as f32 * ROW_H, 593.0, ROW_H),
            index,
            true,
            row.selected,
            &action,
            cells,
        ));
    }
    out
}

/// Auctions tab: `CancelAuctionButton` 158×22 at BOTTOMRIGHT (-3,-22) → (639,511). Bids tab:
/// `BuyoutFrame` right-aligned with it and `BidFrame` 60 left of that.
fn controls(view: &AuctionsView) -> Element {
    match view.tab {
        AuctionsSubTab::Auctions => panel_button(
            "AuctionHouseFrameAuctionsFrameCancelAuctionButton",
            "Cancel Auction",
            ACTION_CANCEL_AUCTION,
            view.can_cancel,
            (639.0, 511.0, 158.0, 22.0),
        ),
        AuctionsSubTab::Bids => {
            let mut out = buy_tab::bid_frame((387.0, 511.0), view.can_bid);
            out.extend(panel_button(
                "AuctionHouseFrameAuctionsFrameBuyoutButton",
                "Buyout",
                ACTION_BUYOUT,
                view.can_buyout,
                (687.0, 511.0, 110.0, 22.0),
            ));
            out
        }
    }
}
