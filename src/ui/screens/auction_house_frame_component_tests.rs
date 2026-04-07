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

// --- Additional coord validation ---

#[test]
fn coord_browse_search_bar() {
    let reg = layout_registry();
    let content_x = FRAME_X + CONTENT_INSET;
    let content_y = FRAME_Y + CONTENT_TOP;
    let r = rect(&reg, "AuctionHouseBrowseSearchBar");
    assert!((r.x - (content_x + SEARCH_BAR_INSET)).abs() < 1.0);
    assert!((r.y - (content_y + SEARCH_BAR_INSET)).abs() < 1.0);
    let expected_w = FRAME_W - 2.0 * CONTENT_INSET - 2.0 * SEARCH_BAR_INSET;
    assert!((r.width - expected_w).abs() < 1.0);
    assert!((r.height - SEARCH_BAR_H).abs() < 1.0);
}

#[test]
fn coord_browse_category_sidebar() {
    let reg = layout_registry();
    let content_x = FRAME_X + CONTENT_INSET;
    let content_y = FRAME_Y + CONTENT_TOP;
    let r = rect(&reg, "AuctionHouseBrowseCategorySidebar");
    assert!((r.x - (content_x + SEARCH_BAR_INSET)).abs() < 1.0);
    let expected_y = content_y + SEARCH_BAR_INSET + SEARCH_BAR_H + SIDEBAR_GAP;
    assert!((r.y - expected_y).abs() < 1.0);
    assert!((r.width - SIDEBAR_W).abs() < 1.0);
}

#[test]
fn coord_browse_results_panel() {
    let reg = layout_registry();
    let content_x = FRAME_X + CONTENT_INSET;
    let content_y = FRAME_Y + CONTENT_TOP;
    let r = rect(&reg, "AuctionHouseBrowseResults");
    let expected_x = content_x + SEARCH_BAR_INSET + SIDEBAR_W + SIDEBAR_GAP;
    let expected_y = content_y + SEARCH_BAR_INSET + SEARCH_BAR_H + SIDEBAR_GAP;
    assert!(
        (r.x - expected_x).abs() < 1.0,
        "x: expected {expected_x}, got {}",
        r.x
    );
    assert!(
        (r.y - expected_y).abs() < 1.0,
        "y: expected {expected_y}, got {}",
        r.y
    );
}

#[test]
fn coord_sell_item_slot() {
    let reg = layout_registry();
    let r = rect(&reg, "AuctionHouseSellItemSlot");
    assert!((r.width - SELL_ITEM_SLOT_SIZE).abs() < 1.0);
    assert!((r.height - SELL_ITEM_SLOT_SIZE).abs() < 1.0);
}

#[test]
fn coord_sell_post_button() {
    let reg = layout_registry();
    let r = rect(&reg, "AuctionHouseSellPostButton");
    assert!((r.width - SELL_BUTTON_W).abs() < 1.0);
    assert!((r.height - SELL_BUTTON_H).abs() < 1.0);
}
