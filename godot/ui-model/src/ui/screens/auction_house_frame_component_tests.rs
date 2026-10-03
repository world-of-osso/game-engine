use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn registry(state: AuctionHouseFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(auction_house_frame_screen).sync(&shared, &mut reg);
    reg
}

fn visible(tab: AuctionHouseTab) -> AuctionHouseFrameState {
    AuctionHouseFrameState {
        visible: true,
        tab,
        search_empty: true,
        ..Default::default()
    }
}

fn shown(reg: &FrameRegistry, name: &str) -> bool {
    let mut id = reg.get_by_name(name);
    while let Some(frame) = id.and_then(|id| reg.get(id)) {
        if frame.hidden {
            return false;
        }
        id = frame.parent_id;
    }
    reg.get_by_name(name).is_some()
}

fn onclick(reg: &FrameRegistry, name: &str) -> Option<String> {
    let id = reg.get_by_name(name).expect(name);
    reg.get(id).and_then(|frame| frame.onclick.clone())
}

fn text_color(reg: &FrameRegistry, name: &str) -> [f32; 4] {
    let id = reg.get_by_name(name).expect(name);
    match reg.get(id).and_then(|frame| frame.widget_data.as_ref()) {
        Some(WidgetData::FontString(fs)) => fs.color,
        _ => panic!("{name} is not a FontString"),
    }
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|f| f.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

fn linen() -> ItemLine {
    ItemLine {
        name: "Linen Cloth".into(),
        quality: 1,
        icon_fdid: 132_889,
    }
}

fn rare_sword() -> ItemLine {
    ItemLine {
        name: "Blade of the Keeper".into(),
        quality: 3,
        icon_fdid: 135_274,
    }
}

#[test]
fn hidden_until_the_auction_house_opens() {
    let reg = registry(AuctionHouseFrameState::default());

    assert!(!shown(&reg, ROOT_FRAME));
}

#[test]
fn title_follows_the_selected_tab() {
    for (tab, title) in [
        (AuctionHouseTab::Buy, "Browse Auctions"),
        (AuctionHouseTab::Sell, "Post Auctions"),
        (AuctionHouseTab::Auctions, "Auctions"),
    ] {
        let reg = registry(visible(tab));
        assert_eq!(fontstring_text(&reg, "AuctionHouseFrameTitleText"), title);
    }
}

#[test]
fn only_the_selected_tab_content_shows() {
    let reg = registry(visible(AuctionHouseTab::Sell));

    assert!(shown(&reg, "AuctionHouseFrameSellMode"));
    assert!(!shown(&reg, "AuctionHouseFrameBuyMode"));
    assert!(!shown(&reg, "AuctionHouseFrameAuctionsFrame"));
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameTab3").as_deref(),
        Some("auction_tab:auctions")
    );
}

#[test]
fn browse_rows_show_the_lowest_price_name_quality_and_available() {
    let mut state = visible(AuctionHouseTab::Buy);
    state.browse = vec![
        BrowseRow {
            item_id: 2589,
            item: linen(),
            price: 1_234_567,
            available: 40,
        },
        BrowseRow {
            item_id: 25,
            item: rare_sword(),
            price: 305,
            available: 1,
        },
    ];

    let reg = registry(state);

    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameBrowseResultsRow1ItemName"),
        "Linen Cloth"
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameBrowseResultsRow1PriceGoldText"),
        "123"
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameBrowseResultsRow1PriceSilverText"),
        "45"
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameBrowseResultsRow1PriceCopperText"),
        "67"
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameBrowseResultsRow1Available"),
        "40"
    );
    // 3s 5c: no gold denomination.
    assert!(
        reg.get_by_name("AuctionHouseFrameBrowseResultsRow2PriceGoldText")
            .is_none()
    );
    assert_eq!(
        text_color(&reg, "AuctionHouseFrameBrowseResultsRow2ItemName"),
        [0.0, 0.44, 0.87, 1.0]
    );
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameBrowseResultsRow2").as_deref(),
        Some("auction_browse_item:25")
    );
}

#[test]
fn item_buy_frame_lists_each_auction_with_bid_buyout_and_time_left() {
    let mut state = visible(AuctionHouseTab::Buy);
    state.item_buy = Some(ItemBuyView {
        item: linen(),
        rows: vec![ListingRow {
            auction_id: 42,
            item: linen(),
            quantity: 20,
            bid: Some(200),
            buyout: Some(10_000),
            time_left: "Very Long".into(),
            selected: true,
        }],
        can_bid: true,
        can_buyout: false,
    });

    let reg = registry(state);

    assert!(shown(&reg, "AuctionHouseFrameItemBuyFrameBackButton"));
    assert!(!shown(&reg, "AuctionHouseFrameBrowseResultsRow1"));
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameItemBuyFrameRow1Quantity"),
        "20"
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameItemBuyFrameRow1TimeLeft"),
        "Very Long"
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameItemBuyFrameRow1BuyoutGoldText"),
        "1"
    );
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameItemBuyFrameRow1").as_deref(),
        Some("auction_select:42")
    );
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameBidButton").as_deref(),
        Some("auction_bid")
    );
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameItemBuyFrameBuyoutButton").as_deref(),
        Some("")
    );
}

#[test]
fn money_frame_shows_the_player_gold() {
    let mut state = visible(AuctionHouseTab::Buy);
    state.money = 100_000;

    let reg = registry(state);

    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameMoneyFrameGoldText"),
        "10"
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameMoneyFrameSilverText"),
        "0"
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameMoneyFrameCopperText"),
        "0"
    );
}

#[test]
fn sell_frame_lists_sellable_items_until_one_is_chosen() {
    let mut state = visible(AuctionHouseTab::Sell);
    state.sell.inventory = vec![SellInventoryRow {
        item_guid: 77,
        item: linen(),
        count: 20,
        selected: false,
    }];

    let reg = registry(state.clone());
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameItemSellListItem1").as_deref(),
        Some("auction_sell_item:77")
    );
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameItemSellFramePostButton").as_deref(),
        Some("")
    );

    state.sell.item = Some(SellItemView {
        item: linen(),
        count: 20,
    });
    state.sell.deposit = 520;
    state.sell.can_post = true;
    let reg = registry(state);
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameItemSellFrameItemDisplayName"),
        "Linen Cloth"
    );
    assert_eq!(
        fontstring_text(
            &reg,
            "AuctionHouseFrameItemSellFrameDepositMoneyDisplayFrameSilverText"
        ),
        "5"
    );
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameItemSellFramePostButton").as_deref(),
        Some("auction_post")
    );
    assert!(
        reg.get_by_name("AuctionHouseFrameItemSellListItem1")
            .is_none()
    );
}

#[test]
fn unchecking_buyout_mode_adds_the_bid_price_input() {
    let mut state = visible(AuctionHouseTab::Sell);
    assert!(
        registry(state.clone())
            .get_by_name(SELL_BID_BOXES.gold)
            .is_none()
    );

    state.sell.buyout_mode = false;
    let reg = registry(state);

    assert!(shown(&reg, SELL_BID_BOXES.gold));
    assert_eq!(
        fontstring_text(
            &reg,
            "AuctionHouseFrameItemSellFrameSecondaryPriceInputLabel"
        ),
        "Bid Price"
    );
}

#[test]
fn auctions_tab_lists_owned_auctions_with_cancel() {
    let mut state = visible(AuctionHouseTab::Auctions);
    state.auctions.rows = vec![ListingRow {
        auction_id: 9,
        item: linen(),
        quantity: 5,
        bid: Some(500),
        buyout: Some(1_000),
        time_left: "24h".into(),
        selected: true,
    }];
    state.auctions.can_cancel = true;

    let reg = registry(state);

    assert_eq!(
        fontstring_text(
            &reg,
            "AuctionHouseFrameAuctionsFrameAllAuctionsListRow1ItemName"
        ),
        "Linen Cloth x5"
    );
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameAuctionsFrameCancelAuctionButton").as_deref(),
        Some("auction_cancel")
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameAuctionsFrameSummaryLine1Text"),
        "All Auctions"
    );
}

#[test]
fn buy_dialog_shows_the_buyout_price() {
    let mut state = visible(AuctionHouseTab::Buy);
    state.dialog = Some(BuyDialogView {
        item_text: "Linen Cloth  x20".into(),
        price: 10_000,
    });

    let reg = registry(state);

    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameBuyDialogItemText"),
        "Linen Cloth  x20"
    );
    assert_eq!(
        fontstring_text(&reg, "AuctionHouseFrameBuyDialogPriceGoldText"),
        "1"
    );
    assert_eq!(
        onclick(&reg, "AuctionHouseFrameBuyDialogBuyNowButton").as_deref(),
        Some("auction_dialog_buy")
    );
}

#[test]
fn retail_layout_places_the_frame_search_bar_and_lists() {
    let mut state = visible(AuctionHouseTab::Buy);
    state.categories = vec![CategoryRow {
        name: "Weapons".into(),
        selected: true,
    }];
    let mut reg = registry(state);
    compute_layout(&mut reg);

    let frame = rect(&reg, ROOT_FRAME);
    assert_eq!((frame.width, frame.height), (800.0, 538.0));
    let at = |name: &str| {
        let r = rect(&reg, name);
        (r.x - frame.x, r.y - frame.y, r.width, r.height)
    };
    assert_eq!(at(SEARCH_BOX), (211.0, 38.0, 241.0, 22.0));
    assert_eq!(
        at("AuctionHouseFrameSearchButton"),
        (656.0, 38.0, 132.0, 22.0)
    );
    assert_eq!(
        at("AuctionHouseFrameCategoriesListButton1"),
        (7.0, 79.0, 132.0, 21.0)
    );
    assert_eq!(at("AuctionHouseFrameTab1"), (20.0, 534.0, 70.0, 32.0));
}

/// `PanelTemplates_SelectTab` / `DeselectTab` (SharedUIPanelTemplates.lua:518-548): the label
/// is CENTER-offset 2 up when deselected and 3 down when selected; top tabs use
/// `-offset - 6` / `-offset - 7` (8 and 4 down).
#[test]
fn tab_labels_follow_retail_selected_and_deselected_offsets() {
    let label_top =
        |reg: &FrameRegistry, tab: &str| rect(reg, &format!("{tab}Text")).y - rect(reg, tab).y;

    let mut reg = registry(visible(AuctionHouseTab::Auctions));
    compute_layout(&mut reg);
    assert_eq!(label_top(&reg, "AuctionHouseFrameTab3"), 3.0);
    assert_eq!(label_top(&reg, "AuctionHouseFrameTab1"), -2.0);
    assert_eq!(
        label_top(&reg, "AuctionHouseFrameAuctionsFrameAuctionsTab"),
        4.0
    );
    assert_eq!(
        label_top(&reg, "AuctionHouseFrameAuctionsFrameBidsTab"),
        8.0
    );
}

/// `PanelTopTabButtonMixin` draws the lower 75 % of the tab art upside down
/// (`SetTexCoord(0, 1, 1, 0.25)`): the selected Auctions top tab's left cap
/// (`uiframe-activetab-left`, rows 127..169 of 256) runs from row 169 up to 137.5.
#[test]
fn top_tab_art_is_the_upside_down_lower_three_quarters() {
    let reg = registry(visible(AuctionHouseTab::Auctions));
    let id = reg
        .get_by_name("AuctionHouseFrameAuctionsFrameAuctionsTabLeft")
        .expect("top tab left cap");
    let Some(WidgetData::Texture(texture)) = reg.get(id).and_then(|f| f.widget_data.as_ref())
    else {
        panic!("left cap is not a texture");
    };
    assert_eq!(texture.tex_coords[2], 169.0 / 256.0);
    assert_eq!(texture.tex_coords[3], 137.5 / 256.0);
}

/// `PanelTemplates_SetNumTabs` → `PanelTemplates_AnchorTabs` (SharedUIPanelTemplates.lua:460-470)
/// re-anchors each tab TOPLEFT to the previous TOPRIGHT + 3, replacing the XML's LEFT -15.
#[test]
fn bottom_tabs_sit_three_apart() {
    let mut reg = registry(visible(AuctionHouseTab::Buy));
    compute_layout(&mut reg);
    let tabs: Vec<LayoutRect> = (1..=3)
        .map(|i| rect(&reg, &format!("AuctionHouseFrameTab{i}")))
        .collect();
    for pair in tabs.windows(2) {
        assert_eq!(pair[1].x, pair[0].x + pair[0].width + 3.0);
        assert_eq!(pair[1].y, pair[0].y);
    }
}
