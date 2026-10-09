//! Offline snapshots of the production auction screen; no session, requests or network.
use super::NativeAuctionView;
pub use super::confirmation::confirmation_spec;
use crate::auction_house_frame_component::*;

pub fn preview_view(view: &str) -> Result<NativeAuctionView, String> {
    let mut frame = browse_frame();
    match view {
        "browse" => {}
        "item" | "dialog" | "bid-popup" | "buyout-popup" => populate_item_buy(&mut frame, view),
        "inventory" | "sell" | "duration" => populate_sell(&mut frame, view),
        "owned" | "bids" => populate_auctions(&mut frame, view),
        _ => return Err(format!("Unknown offline auction view: {view}")),
    }
    Ok(NativeAuctionView {
        frame,
        row_page: 0,
        row_pages: 1,
        search_page: 0,
        search_pages: 1,
        search_paging: matches!(view, "browse" | "item" | "dialog" | "sell" | "duration"),
    })
}

fn browse_frame() -> AuctionHouseFrameState {
    AuctionHouseFrameState {
        visible: true,
        money: 12_345_678,
        search_empty: true,
        categories: super::view::CATEGORIES
            .iter()
            .map(|(name, _)| CategoryRow {
                name: (*name).into(),
                selected: false,
            })
            .collect(),
        browse: preview_items()
            .into_iter()
            .enumerate()
            .map(|(index, item)| BrowseRow {
                item_id: index as u32 + 1,
                item,
                price: 12_345 + index as u64 * 10_000,
                available: 42 + index as u64,
            })
            .collect(),
        ..Default::default()
    }
}

fn populate_item_buy(frame: &mut AuctionHouseFrameState, view: &str) {
    let item = preview_items()[0].clone();
    if view == "dialog" {
        frame.dialog = Some(BuyDialogView {
            item_text: format!("{}  x3", item.name),
            price: 12_345,
        });
    }
    frame.item_buy = Some(ItemBuyView {
        item,
        rows: preview_listings(),
        can_bid: true,
        can_buyout: true,
    });
}

fn populate_sell(frame: &mut AuctionHouseFrameState, view: &str) {
    frame.tab = AuctionHouseTab::Sell;
    frame.sell.inventory = preview_items()
        .into_iter()
        .enumerate()
        .map(|(index, item)| SellInventoryRow {
            item_guid: index as u64 + 1,
            item,
            count: 20,
            selected: false,
        })
        .collect();
    if view == "inventory" {
        return;
    }
    frame.sell.item = Some(SellItemView {
        item: preview_items()[0].clone(),
        count: 20,
    });
    frame.sell.listings = preview_listings();
    frame.sell.deposit = 12_345;
    frame.sell.total = 37_035;
    frame.sell.can_post = true;
    frame.sell.duration_menu_open = view == "duration";
}

fn populate_auctions(frame: &mut AuctionHouseFrameState, view: &str) {
    frame.tab = AuctionHouseTab::Auctions;
    frame.auctions = AuctionsView {
        tab: if view == "owned" {
            AuctionsSubTab::Auctions
        } else {
            AuctionsSubTab::Bids
        },
        rows: preview_listings(),
        can_cancel: true,
        can_bid: true,
        can_buyout: true,
    };
}

fn preview_items() -> Vec<ItemLine> {
    vec![
        ItemLine {
            name: "Peerless Robe of the Auctioneer's Extraordinary Fortune".into(),
            quality: 4,
            icon_fdid: 132662,
        },
        ItemLine {
            name: "Bolt of Linen Cloth".into(),
            quality: 2,
            icon_fdid: 132890,
        },
        ItemLine {
            name: "Uncatalogued Sapphire with a Deliberately Blank Icon".into(),
            quality: 3,
            icon_fdid: 0,
        },
    ]
}

fn preview_listings() -> Vec<ListingRow> {
    preview_items()
        .into_iter()
        .enumerate()
        .map(|(index, item)| ListingRow {
            auction_id: index as u64 + 1,
            item,
            quantity: 3 + index as u32,
            bid: Some(12_345 + index as u64 * 10_000),
            buyout: Some(23_456 + index as u64 * 10_000),
            time_left: ["Very Long", "Long", "Short"][index].into(),
            selected: index == 0,
        })
        .collect()
}
