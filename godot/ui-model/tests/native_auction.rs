//! Native auction session, independent of Godot and Bevy rendering.
#[cfg(test)]
mod tests {
    use game_engine_ui_model::auction_house_frame_component::duration_label;
    use shared::protocol::AuctionDuration;
    #[test]
    fn native_auction_duration_contract() {
        assert_eq!(duration_label(AuctionDuration::Short), "1 Day");
        assert_eq!(duration_label(AuctionDuration::Medium), "1 Week");
        assert_eq!(duration_label(AuctionDuration::Long), "2 Weeks");
    }
}

use game_engine_ui_model::auction::{AuctionRequest, AuctionSession, view::InputTexts};
use shared::protocol::*;
#[test]
fn native_auction_interaction_gate_refresh_rejection_close() {
    let mut s = AuctionSession::default();
    assert!(s.click("auction_search", &InputTexts::new()).is_empty());
    assert!(s.net.requests.is_empty());
    s.open(99);
    assert_eq!(s.net.requests, vec![AuctionRequest::Open]);
    s.net.requests.clear();
    s.opened(AuctionHouseOpened {
        success: true,
        error: None,
    });
    assert_eq!(
        s.net.requests,
        vec![
            AuctionRequest::Inventory,
            AuctionRequest::Owned,
            AuctionRequest::Bids
        ]
    );
    s.net.requests.clear();
    s.operation(AuctionOperationResponse {
        success: false,
        message: "too far".into(),
    });
    assert_eq!(s.net.errors, vec!["too far"]);
    assert!(s.net.requests.is_empty());
    assert_eq!(s.close(), Some(99));
    assert!(!s.net.is_open);
    s.opened(AuctionHouseOpened {
        success: true,
        error: None,
    });
    assert!(!s.net.is_open);
}
#[test]
fn native_auction_all_rows_are_reachable() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut s = AuctionSession::default();
    s.open(99);
    s.opened(AuctionHouseOpened {
        success: true,
        error: None,
    });
    s.net.inventory = Some(AuctionInventorySnapshot {
        gold: 1000,
        items: (1..=43)
            .map(|i| AuctionInventoryItem {
                item_guid: i,
                item_id: i as u32,
                name: format!("item{i}"),
                quality: 1,
                required_level: 1,
                stack_count: 1,
                vendor_sell_price: 1,
            })
            .collect(),
    });
    s.click("auction_tab:sell", &InputTexts::new());
    assert_eq!(s.state(&InputTexts::new()).sell.inventory.len(), 18);
    s.click("auction_rows_next", &InputTexts::new());
    assert_eq!(s.state(&InputTexts::new()).sell.inventory[0].item_guid, 19);
    s.click("auction_rows_next", &InputTexts::new());
    assert_eq!(
        s.state(&InputTexts::new())
            .sell
            .inventory
            .last()
            .unwrap()
            .item_guid,
        43
    );
}

use game_engine_ui_model::auction_house_frame_component::{
    BID_BOXES, QUANTITY_BOX, SEARCH_BOX, SELL_BID_BOXES, SELL_BUYOUT_BOXES,
};
fn item(guid: u64) -> AuctionInventoryItem {
    AuctionInventoryItem {
        item_guid: guid,
        item_id: 2589,
        name: "Linen Cloth".into(),
        quality: 1,
        required_level: 1,
        stack_count: 20,
        vendor_sell_price: 13,
    }
}
fn listing(id: u64) -> AuctionListingSummary {
    AuctionListingSummary {
        auction_id: id,
        item: item(id),
        owner_name: "Seller".into(),
        stack_count: 5,
        min_bid: 100,
        current_bid: None,
        min_next_bid: 100,
        buyout_price: Some(1000),
        time_left: AuctionTimeLeft::Long,
    }
}
fn open_session() -> AuctionSession {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut s = AuctionSession::default();
    s.open(99);
    s.opened(AuctionHouseOpened {
        success: true,
        error: None,
    });
    s.net.requests.clear();
    s.net.inventory = Some(AuctionInventorySnapshot {
        gold: 10_000,
        items: vec![item(17)],
    });
    s
}
#[test]
fn native_auction_invalid_bid_and_cancel_are_not_sent() {
    let mut s = open_session();
    s.net.search_results = vec![listing(5)];
    s.ui.browse_item = Some(2589);
    s.click("auction_select:5", &InputTexts::new());
    let mut texts = InputTexts::new();
    texts.insert(BID_BOXES.copper, "99".into());
    s.click("auction_bid", &texts);
    assert!(s.net.requests.is_empty());
    s.net.owned_results = vec![listing(6)];
    s.net.owned_results[0].current_bid = Some(100);
    s.click("auction_tab:auctions", &texts);
    s.click("auction_select:6", &texts);
    s.click("auction_cancel", &texts);
    assert!(s.net.requests.is_empty());
}
#[test]
fn native_auction_sell_prices_quantity_duration_deposit() {
    for (token, duration, multiplier) in [
        ("12", AuctionDuration::Short, 1),
        ("24", AuctionDuration::Medium, 2),
        ("48", AuctionDuration::Long, 4),
    ] {
        let mut s = open_session();
        let mut texts = InputTexts::new();
        s.click("auction_tab:sell", &texts);
        let edits = s.click("auction_sell_item:17", &texts);
        for (name, value) in edits {
            texts.insert(name, value);
        }
        assert!(
            matches!(&s.net.requests[0],AuctionRequest::Browse(q) if q.item_id==Some(2589) && q.text.is_empty())
        );
        s.net.requests.clear();
        texts.insert(QUANTITY_BOX, "5".into());
        texts.insert(SELL_BUYOUT_BOXES.silver, "2".into());
        s.click(&format!("auction_duration:{token}"), &texts);
        let state = s.state(&texts);
        assert_eq!(state.sell.deposit, 13 * 5 * multiplier);
        assert_eq!(state.sell.total, 1000);
        assert!(state.sell.can_post);
        s.click("auction_post", &texts);
        assert_eq!(
            s.net.requests,
            vec![AuctionRequest::Create(CreateAuction {
                item_guid: 17,
                stack_count: 5,
                min_bid: 1000,
                buyout_price: Some(1000),
                duration
            })]
        );
        assert!(s.ui.sell_item.is_none());
    }
    let mut s = open_session();
    let mut texts = InputTexts::new();
    s.click("auction_tab:sell", &texts);
    s.click("auction_sell_item:17", &texts);
    s.net.requests.clear();
    texts.insert(QUANTITY_BOX, "21".into());
    texts.insert(SELL_BUYOUT_BOXES.copper, "5".into());
    s.click("auction_post", &texts);
    assert!(s.net.requests.is_empty());
    texts.insert(QUANTITY_BOX, "1".into());
    s.click("auction_buyout_mode", &texts);
    texts.insert(SELL_BID_BOXES.copper, "6".into());
    s.click("auction_post", &texts);
    assert!(s.net.requests.is_empty());
    texts.insert(SELL_BID_BOXES.copper, "3".into());
    s.net.inventory.as_mut().unwrap().gold = 0;
    s.click("auction_post", &texts);
    assert!(s.net.requests.is_empty());
}
#[test]
fn native_auction_server_categories_item_search_and_second_page() {
    let mut s = open_session();
    let mut texts = InputTexts::new();
    texts.insert(SEARCH_BOX, "linen".into());
    s.click("auction_category:0", &texts);
    let query = match s.net.requests.pop().unwrap() {
        AuctionRequest::Browse(q) => q,
        _ => panic!(),
    };
    assert_eq!(query.class_id, Some(2));
    s.search_results(AuctionSearchResults {
        query: query.clone(),
        total_results: 103,
        results: (1..=50).map(listing).collect(),
    });
    s.click("auction_page_next", &texts);
    let q2 = match s.net.requests.pop().unwrap() {
        AuctionRequest::Browse(q) => q,
        _ => panic!(),
    };
    assert_eq!(q2.page, 1);
    assert_eq!(q2.class_id, Some(2));
    s.search_results(AuctionSearchResults {
        query: query.clone(),
        total_results: 103,
        results: vec![],
    });
    assert_eq!(s.net.search_results.len(), 50);
    s.search_results(AuctionSearchResults {
        query: q2,
        total_results: 103,
        results: (51..=100).map(listing).collect(),
    });
    s.click("auction_browse_item:2589", &texts);
    let exact = match s.net.requests.pop().unwrap() {
        AuctionRequest::Browse(q) => q,
        _ => panic!(),
    };
    assert_eq!(exact.item_id, Some(2589));
    assert!(exact.text.is_empty());
    s.search_results(AuctionSearchResults {
        query: exact,
        total_results: 103,
        results: (1..=50).map(listing).collect(),
    });
    let mut observed = Vec::new();
    for _ in 0..5 {
        observed.extend(
            s.state(&texts)
                .item_buy
                .unwrap()
                .rows
                .into_iter()
                .map(|row| row.auction_id),
        );
        s.click("auction_rows_next", &texts);
    }
    assert_eq!(observed, (1..=50).collect::<Vec<_>>());
    s.click("auction_back", &texts);
    assert!(
        matches!(s.net.requests.last(),Some(AuctionRequest::Browse(q)) if q.page==1 && q.class_id==Some(2) && q.item_id.is_none())
    );
}
#[test]
fn native_auction_bid_buyout_owned_cancel_and_refresh() {
    let mut s = open_session();
    s.net.search_results = vec![listing(5)];
    s.ui.browse_item = Some(2589);
    let mut texts = InputTexts::new();
    for (name, value) in s.click("auction_select:5", &texts) {
        texts.insert(name, value);
    }
    assert_eq!(texts[BID_BOXES.silver], "1");
    s.click("auction_bid", &texts);
    assert_eq!(
        s.net.requests,
        vec![AuctionRequest::Bid(PlaceBid {
            auction_id: 5,
            amount: 100
        })]
    );
    s.net.requests.clear();
    s.operation(AuctionOperationResponse {
        success: true,
        message: "bid accepted".into(),
    });
    assert_eq!(s.net.requests.len(), 3);
    s.net.requests.clear();
    s.net.bid_results = vec![listing(5)];
    s.click("auction_tab:auctions", &texts);
    s.click("auction_auctions_tab:bids", &texts);
    s.click("auction_select:5", &texts);
    s.click("auction_buyout", &texts);
    assert_eq!(s.state(&texts).dialog.unwrap().price, 1000);
    s.click("auction_dialog_buy", &texts);
    assert_eq!(
        s.net.requests,
        vec![AuctionRequest::Buyout(BuyoutAuction { auction_id: 5 })]
    );
    s.net.requests.clear();
    s.operation(AuctionOperationResponse {
        success: false,
        message: "Cannot buy out your own auction".into(),
    });
    assert_eq!(s.net.errors, vec!["Cannot buy out your own auction"]);
    s.net.owned_results = vec![listing(6)];
    s.click("auction_auctions_tab:auctions", &texts);
    s.click("auction_select:6", &texts);
    s.click("auction_cancel", &texts);
    assert_eq!(
        s.net.requests,
        vec![AuctionRequest::Cancel(CancelAuction { auction_id: 6 })]
    );
}

#[test]
fn native_auction_browse_owned_and_bids_visit_every_fetched_listing() {
    for mode in ["buy", "owned", "bids"] {
        let mut s = open_session();
        let texts = InputTexts::new();
        let rows: Vec<_> = (1..=50)
            .map(|id| {
                let mut row = listing(id);
                row.item.item_id = id as u32;
                row
            })
            .collect();
        match mode {
            "buy" => s.net.search_results = rows,
            "owned" => {
                s.net.owned_results = rows;
                s.click("auction_tab:auctions", &texts);
            }
            "bids" => {
                s.net.bid_results = rows;
                s.click("auction_tab:auctions", &texts);
                s.click("auction_auctions_tab:bids", &texts);
            }
            _ => unreachable!(),
        }
        let mut seen = Vec::new();
        for _ in 0..3 {
            let state = s.state(&texts);
            if mode == "buy" {
                seen.extend(state.browse.iter().map(|row| u64::from(row.item_id)));
            } else {
                seen.extend(state.auctions.rows.iter().map(|row| row.auction_id));
            }
            s.click("auction_rows_next", &texts);
        }
        assert_eq!(seen, (1..=50).collect::<Vec<_>>(), "mode {mode}");
        s.click("auction_rows_prev", &texts);
        assert_eq!(s.ui.row_page, 1);
    }
}
