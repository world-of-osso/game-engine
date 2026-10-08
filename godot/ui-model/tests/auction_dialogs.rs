//! Retail AH confirmation boundaries and commodity dialog art, in both skins.
use game_engine_ui_model::{
    auction::{AuctionSession, native_auction_screen, preview::preview_view, view::InputTexts},
    auction_house_frame_component::BID_BOXES,
};
use shared::protocol::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn session() -> (AuctionSession, InputTexts) {
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
        gold: 10_000,
        items: vec![],
    });
    s.net.search_results = vec![AuctionListingSummary {
        auction_id: 42,
        item: AuctionInventoryItem {
            definition_source: shared::item_data::ItemDefinitionSource::Retail,
            item_guid: 7,
            item_id: 25,
            name: "Worn Shortsword".into(),
            quality: 1,
            required_level: 1,
            stack_count: 1,
            vendor_sell_price: 10,
        },
        owner_name: "Seller".into(),
        stack_count: 1,
        min_bid: 100,
        current_bid: None,
        min_next_bid: 100,
        buyout_price: Some(1234),
        time_left: AuctionTimeLeft::Long,
    }];
    s.ui.browse_item = Some(25);
    s.net.requests.clear();
    let mut texts = InputTexts::new();
    for (name, text) in s.click("auction_select:42", &texts) {
        texts.insert(name, text);
    }
    texts.insert(BID_BOXES.silver, "2".into());
    (s, texts)
}

#[test]
fn item_buyout_does_not_open_commodity_buy_dialog() {
    let (mut s, texts) = session();
    s.click("auction_buyout", &texts);
    assert!(
        s.state(&texts).dialog.is_none(),
        "stack buyout must use BUYOUT_AUCTION, not BuyDialog"
    );
    assert!(s.net.requests.is_empty());
}

#[test]
fn bid_waits_for_accept_instead_of_submitting_directly() {
    let (mut s, texts) = session();
    s.click("auction_bid", &texts);
    assert!(
        s.net.requests.is_empty(),
        "BID_AUCTION must wait for Accept"
    );
}

#[test]
fn commodity_buy_dialog_retains_size_and_uses_dark_texture_inset_seven() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut ctx = SharedContext::new();
        ctx.insert(preview_view("dialog").unwrap());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(native_auction_screen).sync(&ctx, &mut registry);
        let root = registry
            .get(registry.get_by_name("AuctionHouseFrameBuyDialog").unwrap())
            .unwrap();
        assert_eq!((root.width.value(), root.height.value()), (420.0, 100.0));
        let bg = registry
            .get(
                registry
                    .get_by_name("AuctionHouseFrameBuyDialogBg")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!((bg.width.value(), bg.height.value()), (406.0, 86.0));
        assert_eq!(
            (bg.position.left, bg.position.top),
            (Val::Px(7.0), Val::Px(7.0))
        );
        assert!(
            matches!(bg.widget_data, Some(WidgetData::Texture(_))),
            "dark textured inset, not flat black"
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}
