//! Offline snapshots of the production auction screen; no session, requests or network.
use super::NativeAuctionView;
use crate::auction_house_frame_component::*;

pub fn preview_view(view: &str) -> Result<NativeAuctionView, String> {
    if view == "subcategory_sorted" {
        return subcategory_sorted_browse();
    }
    let mut frame = browse_frame();
    match view {
        "browse" => {}
        "item" | "dialog" => populate_item_buy(&mut frame, view),
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

/// Offline response fixture after real production category and sort actions.
/// Item.csv / ItemSparse.csv 12.1.0.69933: cloth Chest/Robe, no live client/server.
fn subcategory_sorted_browse() -> Result<NativeAuctionView, String> {
    use super::{AuctionSession, view::InputTexts};
    use shared::{item_data::ItemDefinitionSource::Retail, protocol::*};
    crate::item_catalog::wait_for_item_catalog();
    let mut session = AuctionSession::default();
    session.open(1);
    session.opened(AuctionHouseOpened {
        success: true,
        error: None,
    });
    session.net.inventory = Some(AuctionInventorySnapshot {
        gold: 12_345_678,
        items: Vec::new(),
    });
    let texts = InputTexts::from([(SEARCH_BOX, "linen".into())]);
    for action in [
        "auction_category:1",
        "auction_category:1/3",
        "auction_category:1/3/2",
        "auction_sort:available",
        "auction_sort:available",
    ] {
        session.click(action, &texts);
    }
    let query = session
        .net
        .last_query
        .clone()
        .ok_or("Missing sorted preview query")?;
    if query.sort_field != AuctionSortField::Quantity
        || query.sort_dir != AuctionSortDir::Desc
        || query.subcategory_filters
            != vec![
                AuctionItemFilter {
                    class_id: 4,
                    subclass_id: Some(1),
                    inventory_type: Some(5),
                },
                AuctionItemFilter {
                    class_id: 4,
                    subclass_id: Some(1),
                    inventory_type: Some(20),
                },
            ]
    {
        return Err(format!("Incorrect sorted preview query: {query:?}"));
    }
    eprintln!("AUCTION_SORTED_PREVIEW query={query:?}");
    let items = [
        (6238, "Brown Linen Robe", 37, 12345),
        (6241, "White Linen Robe", 12, 23456),
        (6240, "Blue Linen Vest", 8, 34567),
    ]
    .into_iter()
    .map(
        |(item_id, name, total_quantity, lowest_unit_price)| AuctionBrowseItem {
            item_id,
            definition_source: Retail,
            name: name.into(),
            quality: 2,
            required_level: 5,
            lowest_unit_price,
            total_quantity,
        },
    )
    .collect();
    session.browse_results(AuctionBrowseResults {
        query,
        total_results: 3,
        items,
    });
    Ok(session.native_view(&texts))
}

fn browse_frame() -> AuctionHouseFrameState {
    AuctionHouseFrameState {
        visible: true,
        money: 12_345_678,
        search_empty: true,
        categories: super::categories::rows(&[]),
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
