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
fn native_auction_accepts_server_selected_house_but_rejects_stale_filters() {
    let mut session = AuctionSession::default();
    session.open(99);
    session.opened(AuctionHouseOpened {
        success: true,
        error: None,
    });
    let query = AuctionSearchQuery {
        text: "Linen".into(),
        item_id: Some(2589),
        ..Default::default()
    };
    session.net.request(AuctionRequest::Browse(query.clone()));
    let mut server_query = query.clone();
    server_query.faction = 1;
    session.browse_results(AuctionBrowseResults {
        query: server_query.clone(),
        total_results: 1,
        items: vec![AuctionBrowseItem {
            definition_source: shared::item_data::ItemDefinitionSource::Retail,
            item_id: 2589,
            name: "Linen Cloth".into(),
            quality: 1,
            required_level: 0,
            lowest_unit_price: 200,
            total_quantity: 3,
        }],
    });
    assert_eq!(session.net.search_revision, 1);
    assert_eq!(session.net.browse_results[0].total_quantity, 3);
    let mut stale = server_query.clone();
    stale.page = 1;
    session.browse_results(AuctionBrowseResults {
        query: stale,
        total_results: 0,
        items: vec![],
    });
    assert_eq!(session.net.search_revision, 1);
    session.net.request(AuctionRequest::Listings(query));
    session.search_results(AuctionSearchResults {
        query: server_query,
        total_results: 0,
        results: vec![],
    });
    assert_eq!(session.net.search_revision, 2);
    assert_eq!(session.net.last_query.as_ref().unwrap().faction, 1);
}

#[test]
fn native_auction_sort_preserves_filters_and_resets_pages() {
    let mut session = AuctionSession::default();
    session.open(99);
    session.opened(AuctionHouseOpened {
        success: true,
        error: None,
    });
    let mut query = AuctionSearchQuery {
        text: "cloth".into(),
        class_id: Some(7),
        page: 3,
        ..Default::default()
    };
    session.net.request(AuctionRequest::Browse(query.clone()));
    session.ui.row_page = 2;
    session.net.requests.clear();

    session.click("auction_sort:price", &InputTexts::new());
    query.sort_field = AuctionSortField::Buyout;
    query.page = 0;
    assert_eq!(
        session.net.requests,
        vec![AuctionRequest::Browse(query.clone())]
    );
    assert_eq!(session.ui.row_page, 0);

    session.net.requests.clear();
    session.click("auction_sort:price", &InputTexts::new());
    query.sort_dir = AuctionSortDir::Desc;
    assert_eq!(
        session.net.requests,
        vec![AuctionRequest::Browse(query.clone())]
    );

    query.item_id = Some(2589);
    query.class_id = None;
    query.text.clear();
    session.ui.browse_item = Some(2589);
    session.net.request(AuctionRequest::Listings(query.clone()));
    session.net.requests.clear();
    session.click("auction_sort:price", &InputTexts::new());
    query.sort_field = AuctionSortField::Buyout;
    query.sort_dir = AuctionSortDir::Asc;
    assert_eq!(session.net.requests, vec![AuctionRequest::Listings(query)]);
}

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
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
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
                definition_source: shared::item_data::ItemDefinitionSource::Retail,
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
        definition_source: shared::item_data::ItemDefinitionSource::Retail,
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
fn browse_item(id: u32) -> AuctionBrowseItem {
    AuctionBrowseItem {
        definition_source: shared::item_data::ItemDefinitionSource::Retail,
        item_id: id,
        name: format!("item{id}"),
        quality: 2,
        required_level: 10,
        lowest_unit_price: 17,
        total_quantity: 5_000_000_001,
    }
}
fn open_session() -> AuctionSession {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
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
fn native_auction_live_refund_updates_money_and_bid_affordability() {
    let mut session = open_session();
    session.net.inventory.as_mut().unwrap().gold = 998_800;
    session.sync_replicated_money(998_800);
    let mut auction = listing(102);
    auction.current_bid = Some(951_381);
    auction.min_next_bid = 998_950;
    auction.buyout_price = Some(1_000_000);
    session.net.search_results = vec![auction];
    session.ui.browse_item = Some(2589);
    let mut texts = InputTexts::new();
    for (name, value) in session.click("auction_select:102", &texts) {
        texts.insert(name, value);
    }
    session.click("auction_bid", &texts);
    assert!(session.net.requests.is_empty());

    // Outbid refund arrives through entity Gold without an auction inventory reply.
    session.sync_replicated_money(999_000);
    assert_eq!(session.state(&texts).money, 999_000);
    session.click("auction_bid", &texts);
    assert_eq!(
        session.net.requests,
        vec![AuctionRequest::Bid(PlaceBid {
            auction_id: 102,
            amount: 998_950,
        })]
    );

    // A newer query reply must not be overwritten by the same old entity snapshot.
    session.net.inventory.as_mut().unwrap().gold = 999_100;
    session.sync_replicated_money(999_000);
    assert_eq!(session.state(&texts).money, 999_100);
    session.close();
    session.sync_replicated_money(5);
    assert!(session.net.inventory.is_none());
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
            matches!(&s.net.requests[0],AuctionRequest::Listings(q) if q.item_id==Some(2589) && q.text.is_empty())
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
    s.browse_results(AuctionBrowseResults {
        query: query.clone(),
        total_results: 103,
        items: (1..=50).map(browse_item).collect(),
    });
    // Flat data must never replace authoritative global price/stock.
    s.net.search_results = vec![listing(1)];
    let row = &s.state(&texts).browse[0];
    assert_eq!(row.item_id, 1);
    assert_eq!(row.price, 17);
    assert_eq!(row.available, 5_000_000_001);
    s.click("auction_page_next", &texts);
    let q2 = match s.net.requests.pop().unwrap() {
        AuctionRequest::Browse(q) => q,
        _ => panic!(),
    };
    assert_eq!(q2.page, 1);
    assert_eq!(q2.class_id, Some(2));
    s.browse_results(AuctionBrowseResults {
        query: query.clone(),
        total_results: 103,
        items: vec![],
    });
    assert_eq!(s.net.browse_results.len(), 50);
    s.browse_results(AuctionBrowseResults {
        query: q2,
        total_results: 103,
        items: (51..=100).map(browse_item).collect(),
    });
    assert_eq!(s.state(&texts).browse[0].item_id, 51);
    assert_eq!(s.native_view(&texts).search_pages, 3);
    s.click("auction_browse_item:2589", &texts);
    let exact = match s.net.requests.pop().unwrap() {
        AuctionRequest::Listings(q) => q,
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
    s.click("auction_select:37", &texts);
    let selected = s.state(&texts).item_buy.unwrap();
    assert!(selected.can_buyout);
    s.click("auction_buyout", &texts);
    assert_eq!(s.state(&texts).dialog.unwrap().price, 1000);
    s.click("auction_dialog_buy", &texts);
    assert_eq!(
        s.net.requests.pop(),
        Some(AuctionRequest::Buyout(BuyoutAuction { auction_id: 37 }))
    );
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
            "buy" => s.net.browse_results = (1..=50).map(browse_item).collect(),
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

#[test]
fn ahsort_subcategory_click_sends_chest_and_robe_alternatives_and_toggles() {
    let mut s = open_session();
    let texts = InputTexts::from([(SEARCH_BOX, "robe".into())]);
    s.click("auction_category:1", &texts); // Armor
    s.net.requests.clear();
    s.click("auction_category:1/3", &texts); // Cloth
    let cloth = AuctionSearchQuery {
        text: "robe".into(),
        class_id: Some(4),
        subcategory_filters: vec![AuctionItemFilter {
            class_id: 4,
            subclass_id: Some(1),
            inventory_type: None,
        }],
        ..Default::default()
    };
    assert_eq!(s.net.requests, vec![AuctionRequest::Browse(cloth.clone())]);
    s.net.requests.clear();
    s.ui.row_page = 2;
    s.ui.selected_auction = Some(77);
    s.click("auction_category:1/3/2", &texts); // Chest (includes robes)
    let mut chest = cloth.clone();
    chest.subcategory_filters = vec![
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
    ];
    assert_eq!(s.net.requests, vec![AuctionRequest::Browse(chest)]);
    assert_eq!(s.ui.row_page, 0);
    assert_eq!(s.ui.selected_auction, None);
    let view = s.state(&texts);
    assert!(
        view.categories
            .iter()
            .any(|row| row.name == "Chest" && row.selected)
    );
    s.net.requests.clear();
    s.click("auction_category:1/3/2", &texts);
    assert_eq!(s.net.requests, vec![AuctionRequest::Browse(cloth)]);
    s.net.requests.clear();
    s.click("auction_category:1", &texts);
    assert_eq!(
        s.net.requests,
        vec![AuctionRequest::Browse(AuctionSearchQuery {
            text: "robe".into(),
            ..Default::default()
        })]
    );
}

#[test]
fn ahsort_available_and_current_bid_headers_reverse_and_preserve_filters() {
    let mut s = open_session();
    let query = AuctionSearchQuery {
        text: "robe".into(),
        class_id: Some(4),
        page: 2,
        subcategory_filters: vec![AuctionItemFilter {
            class_id: 4,
            subclass_id: Some(1),
            inventory_type: Some(20),
        }],
        ..Default::default()
    };
    for (token, field, listing) in [
        ("available", AuctionSortField::Quantity, false),
        ("bid", AuctionSortField::Bid, true),
        ("available", AuctionSortField::Quantity, true),
    ] {
        let mut expected = query.clone();
        expected.item_id = listing.then_some(123);
        s.ui.browse_item = expected.item_id;
        s.net.request(if listing {
            AuctionRequest::Listings(expected.clone())
        } else {
            AuctionRequest::Browse(expected.clone())
        });
        for direction in [
            AuctionSortDir::Asc,
            AuctionSortDir::Desc,
            AuctionSortDir::Asc,
        ] {
            s.net.requests.clear();
            s.ui.row_page = 2;
            s.ui.selected_auction = Some(77);
            s.click(&format!("auction_sort:{token}"), &InputTexts::new());
            expected.page = 0;
            expected.sort_field = field;
            expected.sort_dir = direction;
            assert_eq!(
                s.net.requests,
                vec![if listing {
                    AuctionRequest::Listings(expected.clone())
                } else {
                    AuctionRequest::Browse(expected.clone())
                }]
            );
            assert_eq!(s.ui.row_page, 0);
            assert_eq!(s.ui.selected_auction, None);
        }
    }
}

#[test]
fn ahsort_named_weapon_group_and_profession_slot_send_retail_filters() {
    let mut s = open_session();
    s.click("auction_category:0", &InputTexts::new());
    s.net.requests.clear();
    s.click("auction_category:0/0", &InputTexts::new());
    let expected = [0, 4, 7, 9, 15, 13, 19]
        .into_iter()
        .map(|subclass_id| AuctionItemFilter {
            class_id: 2,
            subclass_id: Some(subclass_id),
            inventory_type: None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        s.net.requests,
        vec![AuctionRequest::Browse(AuctionSearchQuery {
            class_id: Some(2),
            subcategory_filters: expected,
            ..Default::default()
        })]
    );
    s.net.requests.clear();
    s.click("auction_category:0/0/0", &InputTexts::new());
    assert_eq!(
        s.net.requests,
        vec![AuctionRequest::Browse(AuctionSearchQuery {
            class_id: Some(2),
            subcategory_filters: vec![AuctionItemFilter {
                class_id: 2,
                subclass_id: Some(0),
                inventory_type: None
            }],
            ..Default::default()
        })]
    );
    s.click("auction_category:9", &InputTexts::new());
    s.click("auction_category:9/0", &InputTexts::new());
    s.net.requests.clear();
    s.click("auction_category:9/0/0", &InputTexts::new());
    assert_eq!(
        s.net.requests,
        vec![AuctionRequest::Browse(AuctionSearchQuery {
            class_id: Some(19),
            subcategory_filters: vec![AuctionItemFilter {
                class_id: 19,
                subclass_id: Some(12),
                inventory_type: Some(29)
            }],
            ..Default::default()
        })]
    );
}
