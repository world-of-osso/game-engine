//! Concrete Retail geometry/art on production auction snapshots in both skins.
use game_engine_ui_model::auction::{native_auction_screen, preview::preview_view};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn render(skin: ActiveSkin, view: &str) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(preview_view(view).unwrap());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(native_auction_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

#[test]
fn auction_duration_uses_retail_dropdown_geometry() {
    // Blizzard_Menu/Mainline/MenuTemplates.xml:3-28; AH SellFrame.xml:124-130.
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, "duration");
        let dropdown = frame(&registry, "AuctionHouseFrameItemSellFrameDurationDropdown");
        let rect = dropdown.layout_rect.as_ref().unwrap();
        assert_eq!((rect.width, rect.height), (120.0, 25.0));
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn auction_buy_dialog_has_retail_dark_background() {
    // DialogTemplates.xml:83-96, UI-DialogBox-Background-Dark file.
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, "dialog");
        let background = frame(&registry, "AuctionHouseFrameBuyDialogBg");
        assert!(
            matches!(background.widget_data, Some(WidgetData::Texture(_))),
            "Dialog background must display supplied art, not a flat colour"
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn auction_snapshots_preserve_root_search_and_band_geometry() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let browse = render(skin, "browse");
        let root = frame(&browse, "AuctionHouseFrame")
            .layout_rect
            .as_ref()
            .unwrap();
        assert_eq!((root.width, root.height), (800.0, 538.0));
        let search = frame(&browse, "AuctionHouseFrameSearchBox")
            .layout_rect
            .as_ref()
            .unwrap();
        assert_eq!((search.width, search.height), (241.0, 22.0));
        let item = render(skin, "item");
        let band = frame(&item, "AuctionHouseFrameItemBuyFrameRow1TimeLeft")
            .layout_rect
            .as_ref()
            .unwrap();
        assert_eq!(band.width, 120.0); // 140 minus 10px on either side.
        let bids = render(skin, "bids");
        let band = frame(&bids, "AuctionHouseFrameAuctionsFrameBidsListRow1TimeLeft")
            .layout_rect
            .as_ref()
            .unwrap();
        assert_eq!(band.width, 130.0); // 140 minus right padding.
    }
    set_thread_skin(ActiveSkin::Modern);
}
