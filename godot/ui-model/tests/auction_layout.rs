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

#[test]
fn ui_template_behavior_services_auction_rows_declare_live_item_owners() {
    use game_engine_ui_model::auction_house_frame_component::{
        ACTION_BROWSE_ITEM_PREFIX, ACTION_SELECT_AUCTION_PREFIX, ACTION_SELL_ITEM_PREFIX,
    };
    use ui_toolkit::tooltip::{TooltipAnchor, tooltip_owner};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for view in ["browse", "item", "owned", "bids", "sell"] {
            let registry = render(skin, view);
            let hits: Vec<_> = registry
                .frames_iter()
                .filter(|frame| {
                    frame.onclick.as_deref().is_some_and(|action| {
                        [
                            ACTION_BROWSE_ITEM_PREFIX,
                            ACTION_SELECT_AUCTION_PREFIX,
                            ACTION_SELL_ITEM_PREFIX,
                        ]
                        .iter()
                        .any(|prefix| action.starts_with(prefix))
                    })
                })
                .map(|frame| frame.id)
                .collect();
            assert!(!hits.is_empty(), "{skin:?} {view} supplies real item rows");
            for hit in hits {
                let (owner, binding) = tooltip_owner(&registry, hit).unwrap_or_else(|| {
                    panic!(
                        "{skin:?} {view} auction live owner: {:?}",
                        registry
                            .get(hit)
                            .map(|frame| (&frame.name, frame.visible, &frame.tooltip))
                    )
                });
                assert_eq!(owner.id, hit);
                assert_eq!(binding.provider.as_deref(), Some("auction"));
                assert_eq!(binding.anchor, TooltipAnchor::Right);
            }
        }
    }
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

#[test]
fn auction_search_uses_inherited_small_font_and_instruction_colour() {
    // InputBoxTemplates.xml:177-203 overrides ChatFontNormal with GameFontHighlightSmall.
    // FontStyles.xml:56,92 and Fonts.xml:39-45 resolve that to FRIZQT__ 10.
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, "browse");
        let Some(WidgetData::EditBox(edit)) =
            &frame(&registry, "AuctionHouseFrameSearchBox").widget_data
        else {
            panic!("search input missing")
        };
        assert_eq!(
            edit.font,
            ui_toolkit::widgets::font_string::GameFont::FrizQuadrata
        );
        assert_eq!(edit.font_size, 10.0);
        let Some(WidgetData::FontString(instructions)) =
            &frame(&registry, "AuctionHouseFrameSearchBoxInstructions").widget_data
        else {
            panic!("instructions missing")
        };
        assert_eq!(instructions.font_size, 10.0);
        assert_eq!(instructions.color, [0.35, 0.35, 0.35, 1.0]);
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn auction_duration_uses_retail_dropdown_geometry() {
    // SellFrame.lua:159-170 overrides the shared template's width/font/alignment.
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, "duration");
        let dropdown = frame(&registry, "AuctionHouseFrameItemSellFrameDurationDropdown");
        assert_eq!(
            (dropdown.width.value(), dropdown.height.value()),
            (115.0, 25.0)
        );
        let caption = frame(
            &registry,
            "AuctionHouseFrameItemSellFrameDurationDropdownText",
        );
        assert_eq!(
            caption.position.top,
            ui_toolkit::layout_values::Val::Px(
                (dropdown.height.value() - caption.height.value()) / 2.0
            ),
            "Duration caption must be vertically centered"
        );
        let Some(WidgetData::FontString(text)) = &caption.widget_data else {
            panic!("duration text missing")
        };
        assert_eq!(
            text.font,
            ui_toolkit::widgets::font_string::GameFont::ArialNarrow
        );
        assert_eq!(text.font_size, 12.0);
        assert_eq!(
            text.justify_h,
            ui_toolkit::widgets::font_string::JustifyH::Right
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn auction_duration_radios_follow_selection_and_preserve_actions() {
    use game_engine_ui_model::auction::{AuctionSession, view::InputTexts};
    use shared::protocol::AuctionDuration;
    use ui_toolkit::{
        layout_values::Val,
        widgets::font_string::{GameFont, JustifyH},
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let _ = render(skin, "sell");
        for buyout in [false, true] {
            for (selected, duration) in [
                AuctionDuration::Short,
                AuctionDuration::Medium,
                AuctionDuration::Long,
            ]
            .into_iter()
            .enumerate()
            {
                let mut view = preview_view("duration").unwrap();
                view.frame.sell.duration = duration;
                view.frame.sell.buyout_mode = buyout;
                let mut shared = SharedContext::new();
                shared.insert(view);
                let mut registry = FrameRegistry::new(1920.0, 1080.0);
                Screen::new(native_auction_screen).sync(&shared, &mut registry);
                let menu = frame(&registry, "AuctionHouseFrameDurationMenu");
                let owner = frame(&registry, "AuctionHouseFrameItemSellFrameDurationDropdown");
                assert_eq!(menu.width.value(), 115.0);
                assert_eq!(menu.height.value(), 83.0);
                assert_eq!(menu.position.left, owner.position.left);
                let Val::Px(top) = owner.position.top else {
                    panic!("owner position")
                };
                assert_eq!(menu.position.top, Val::Px(top + 25.0));
                for (index, token) in ["12", "24", "48"].into_iter().enumerate() {
                    let name = format!("AuctionHouseFrameDurationMenuOption{}", index + 1);
                    let row = frame(&registry, &name);
                    assert_eq!(row.position.top, Val::Px(8.0 + index as f32 * 20.0));
                    assert_eq!(row.height.value(), 20.0);
                    assert!(
                        registry.get_by_name(&format!("{name}Box")).is_some(),
                        "radio outline {name}"
                    );
                    assert_eq!(
                        registry.get_by_name(&format!("{name}Tick")).is_some(),
                        index == selected
                    );
                    let Some(WidgetData::FontString(text)) =
                        &frame(&registry, &format!("{name}Text")).widget_data
                    else {
                        panic!("radio text")
                    };
                    assert_eq!(
                        (text.font, text.font_size, text.justify_h),
                        (GameFont::ArialNarrow, 12.0, JustifyH::Left)
                    );
                    let action = row.onclick.as_deref().expect("duration action");
                    assert_eq!(action, format!("auction_duration:{token}"));
                    let mut session = AuctionSession::default();
                    session.open(1);
                    session.opened(shared::protocol::AuctionHouseOpened {
                        success: true,
                        error: None,
                    });
                    session.ui.duration_menu_open = true;
                    session.click(action, &InputTexts::new());
                    assert!(!session.ui.duration_menu_open);
                    assert_eq!(
                        session.ui.duration,
                        [
                            AuctionDuration::Short,
                            AuctionDuration::Medium,
                            AuctionDuration::Long
                        ][index]
                    );
                }
            }
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn auction_item_header_icon_uses_retail_crop_and_size() {
    // Blizzard_ItemButton/Mainline/ItemButtonTemplate.xml:23-41.
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, "item");
        let icon = frame(
            &registry,
            "AuctionHouseFrameItemBuyFrameItemDisplayItemButtonIcon",
        );
        assert_eq!((icon.width.value(), icon.height.value()), (46.0, 46.0));
        let Some(WidgetData::Texture(texture)) = &icon.widget_data else {
            panic!("item icon missing")
        };
        assert_eq!(texture.tex_coords, [0.078125, 0.921875, 0.078125, 0.921875]);
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn auction_sell_bid_column_uses_retail_fixed_width() {
    // TableBuilder.lua:1124 uses120, with10px cell padding, in every item variant.
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, "sell");
        assert_eq!(
            frame(&registry, "AuctionHouseFrameItemSellListHeader0")
                .width
                .value(),
            110.0
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn auction_bid_button_does_not_cover_copper_input() {
    use ui_toolkit::layout_values::Val;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, "item");
        let copper = frame(&registry, "AuctionHouseFrameBidAmountCopper");
        let button = frame(&registry, "AuctionHouseFrameBidButton");
        let Val::Px(left) = copper.position.left else {
            panic!("copper not positioned")
        };
        let Val::Px(button_left) = button.position.left else {
            panic!("bid not positioned")
        };
        assert!(
            left + copper.width.value() <= button_left,
            "Bid button covers copper input: {} > {button_left}",
            left + copper.width.value()
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn auction_list_and_bid_geometry_matches_retail_anchors() {
    use ui_toolkit::layout_values::Val;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let owned = render(skin, "owned");
        let row = frame(&owned, "AuctionHouseFrameAuctionsFrameAllAuctionsListRow1");
        assert_eq!(row.position.top, Val::Px(100.0));
        let item = render(skin, "item");
        assert_eq!(
            frame(&item, "AuctionHouseFrameBidButton").position.left,
            Val::Px(575.0)
        );
        assert_eq!(
            frame(&item, "AuctionHouseFrameBidAmountGold").width.value(),
            70.0
        );
        assert_eq!(
            frame(&item, "AuctionHouseFrameBidAmountSilver")
                .width
                .value(),
            48.0
        );
        let browse = render(skin, "browse");
        assert_eq!(
            frame(
                &browse,
                "AuctionHouseFrameBrowseResultsFrameItemListBackground"
            )
            .height
            .value(),
            413.0
        );
        assert_eq!(
            frame(&item, "AuctionHouseFrameItemBuyFrameItemListBackground")
                .height
                .value(),
            277.0
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn auction_snapshots_preserve_root_search_and_band_geometry() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let browse = render(skin, "browse");
        let root = frame(&browse, "AuctionHouseFrame");
        assert_eq!((root.width.value(), root.height.value()), (800.0, 538.0));
        let search = frame(&browse, "AuctionHouseFrameSearchBox");
        assert_eq!((search.width.value(), search.height.value()), (241.0, 22.0));
        let item = render(skin, "item");
        let band = frame(&item, "AuctionHouseFrameItemBuyFrameRow1TimeLeft");
        assert_eq!(band.width.value(), 120.0); // 140 minus 10px on either side.
        let bids = render(skin, "bids");
        let band = frame(&bids, "AuctionHouseFrameAuctionsFrameBidsListRow1TimeLeft");
        assert_eq!(band.width.value(), 130.0); // 140 minus right padding.
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn ahsort_displayed_buy_headers_dispatch_bid_and_available() {
    use game_engine_ui_model::auction::{AuctionRequest, AuctionSession, view::InputTexts};
    use shared::protocol::{AuctionHouseOpened, AuctionSortField};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (view, header, expected) in [
            (
                "browse",
                "AuctionHouseFrameBrowseResultsFrameItemListHeader2",
                AuctionSortField::Quantity,
            ),
            (
                "item",
                "AuctionHouseFrameItemBuyFrameItemListHeader0",
                AuctionSortField::Bid,
            ),
            (
                "item",
                "AuctionHouseFrameItemBuyFrameItemListHeader2",
                AuctionSortField::Quantity,
            ),
        ] {
            let registry = render(skin, view);
            let action = frame(&registry, header)
                .onclick
                .as_deref()
                .expect("sortable visible header");
            let mut session = AuctionSession::default();
            session.open(1);
            session.opened(AuctionHouseOpened {
                success: true,
                error: None,
            });
            if view == "item" {
                session.ui.browse_item = Some(123);
            }
            session.net.requests.clear();
            session.click(action, &InputTexts::new());
            let query = match session.net.requests.last().unwrap() {
                AuctionRequest::Browse(query) | AuctionRequest::Listings(query) => query,
                other => panic!("unexpected header request {other:?}"),
            };
            assert_eq!(query.sort_field, expected);
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn ahsort_expanded_category_tree_scroll_reaches_last_category() {
    use game_engine_ui_model::auction::{AuctionSession, view::InputTexts};
    use shared::protocol::AuctionHouseOpened;
    let _ = render(ActiveSkin::Modern, "browse");
    let mut session = AuctionSession::default();
    session.open(1);
    session.opened(AuctionHouseOpened {
        success: true,
        error: None,
    });
    let texts = InputTexts::new();
    for action in [
        "auction_category:1",
        "auction_category:1/3",
        "auction_category:1/3/2",
    ] {
        session.click(action, &texts);
    }
    let state = session.native_view(&texts);
    assert!(state.frame.categories.len() > 20);
    let last = state.frame.categories.len();
    let mut shared = SharedContext::new();
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut screen = Screen::new(native_auction_screen);
    screen.sync(&shared, &mut registry);
    assert!(
        registry
            .scroll_lists
            .scroll_to("AuctionHouseFrameCategoriesScrollList", usize::MAX)
    );
    screen.sync(&shared, &mut registry);
    let last_row = frame(
        &registry,
        &format!("AuctionHouseFrameCategoriesListButton{last}"),
    );
    assert_eq!(last_row.onclick.as_deref(), Some("auction_category:13"));
}
