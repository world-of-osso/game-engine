use game_engine_ui_model::trading_post::{View, trading_post_screen};
use shared::protocol::*;
use std::sync::Arc;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn snapshot() -> TradingPostSnapshot {
    TradingPostSnapshot {
        npc_net_id: Some(185468),
        tender: 500,
        points: 1000,
        now_unix: 1790812900,
        month_end_unix: 1793491200,
        frozen_item_id: Some(4),
        categories: vec![
            TradingPostCategory {
                id: 1,
                name: "Transmog".into(),
                model_scene_id: 430,
            },
            TradingPostCategory {
                id: 2,
                name: "Mounts".into(),
                model_scene_id: 418,
            },
        ],
        items: vec![TradingPostItem {
            id: 4,
            category_id: 1,
            item_id: 189869,
            name: "Distinguished Warcloak".into(),
            description: String::new(),
            icon_fdid: 0,
            quality: 1,
            price: 100,
            purchased: false,
            collected: false,
            refund_seconds: 0,
            ui_group_name: "New Arrivals".into(),
            ui_group_priority: 1000,
            model_scene_id: 583,
            creature_display_id: 0,
            preview_item_ids: vec![189869],
            grant_unavailable: None,
        }],
        thresholds: (1..=5)
            .map(|id| TradingPostThreshold {
                id,
                points: id * 200,
                tender: 100,
                claimed: true,
            })
            .collect(),
        source_gaps: vec!["Refund duration unsourced: disabled.".into()],
        error: None,
    }
}
fn render(skin: ActiveSkin, view: View) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx.insert(view);
    let mut registry = FrameRegistry::new(1920., 1080.);
    Screen::new(trading_post_screen).sync(&ctx, &mut registry);
    registry
}
fn text(registry: &FrameRegistry, name: &str) -> String {
    let id = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("Missing {name}"));
    match &registry.get(id).unwrap().widget_data {
        Some(WidgetData::FontString(font)) => font.text.clone(),
        other => panic!("Expected text, got {other:?}"),
    }
}
#[test]
fn trading_post_controls_wait_for_server_and_confirm_purchase_without_local_debit() {
    use game_engine_ui_model::trading_post::Action;
    let mut view = View {
        visible: true,
        snapshot: Some(Arc::new(snapshot())),
        selected: Some(4),
        ..Default::default()
    };
    assert!(view.can_purchase());
    assert_eq!(view.click("perks:purchase").unwrap(), None);
    assert_eq!(view.confirm.as_deref(), Some("purchase"));
    view.click("perks:category:2").unwrap();
    assert_eq!(view.category, 0, "modal blocks underlying category clicks");
    assert_eq!(
        view.click("perks:confirm").unwrap(),
        Some(Action::Request(TradingPostRequest::Purchase {
            vendor_item_id: 4
        }))
    );
    assert!(view.pending);
    assert_eq!(
        view.snapshot.as_ref().unwrap().tender,
        500,
        "no client debit"
    );
    assert_eq!(view.click("perks:freeze").unwrap(), None);
    let mut paid = snapshot();
    paid.tender = 400;
    paid.items[0].purchased = true;
    paid.items[0].collected = true;
    view.receive(Arc::new(paid));
    assert!(!view.pending);
    assert!(!view.can_purchase());
    assert!(!view.can_refund());
    let mut refundable = snapshot();
    refundable.items[0].purchased = true;
    refundable.items[0].refund_seconds = 60;
    view.receive(Arc::new(refundable));
    view.click("perks:refund").unwrap();
    assert_eq!(
        view.click("perks:confirm").unwrap(),
        Some(Action::Request(TradingPostRequest::Refund {
            vendor_item_id: 4
        }))
    );
}

#[test]
fn trading_post_virtualized_list_reaches_second_page_then_search_clamps_in_both_skins() {
    // Twelve distinct local PerksVendorItem/ItemSparse identities; optional group fields omitted.
    let authored = [
        (4, 189869, 100, "Distinguished Warcloak"),
        (5, 189870, 75, "Arcanofire Wrap"),
        (6, 189871, 100, "Squire's Warhammer"),
        (8, 189898, 750, "Fury of the Firelord"),
        (9, 189897, 250, "Infiltrator's Bandolier"),
        (10, 189896, 100, "Forsaken Cresset"),
        (11, 189895, 100, "Watchman's Flare"),
        (12, 189894, 123, "Fel-Automaton Stabilizers"),
        (13, 189893, 123, "Fel-Automaton Pauldrons"),
        (14, 189892, 123, "Fel-Automaton Mobilizers"),
        (15, 189891, 123, "Fel-Automaton Cranioplate"),
        (16, 189890, 123, "Fel-Automaton Manipulators"),
    ];
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        set_thread_skin(skin);
        let mut data = snapshot();
        data.items = authored
            .iter()
            .map(|(id, item, price, name)| {
                let mut row = data.items[0].clone();
                row.id = *id;
                row.item_id = *item;
                row.price = *price;
                row.name = (*name).into();
                row.ui_group_name.clear();
                row.ui_group_priority = 0;
                row.preview_item_ids = vec![*item];
                row
            })
            .collect();
        let mut view = View {
            visible: true,
            snapshot: Some(Arc::new(data)),
            selected: Some(4),
            ..Default::default()
        };
        let mut ctx = SharedContext::new();
        ctx.insert(skin);
        ctx.insert(view.clone());
        let mut registry = FrameRegistry::new(1920., 1080.);
        let mut screen = Screen::new(trading_post_screen);
        screen.sync(&ctx, &mut registry);
        assert!(registry.get_by_name("PerksProgramProduct5").is_some());
        assert!(
            registry
                .scroll_lists
                .get("PerksProgramScrollBox")
                .unwrap()
                .geometry
                .max_first_row()
                > 0
        );
        registry
            .scroll_lists
            .scroll_to("PerksProgramScrollBox", usize::MAX);
        screen.sync(&ctx, &mut registry);
        assert!(registry.get_by_name("PerksProgramProduct11").is_some());
        assert!(registry.get_by_name("PerksProgramProduct5").is_none());
        view.search = "Arcanofire".into();
        view.reselect();
        ctx.insert(view);
        screen.sync(&ctx, &mut registry);
        assert_eq!(
            registry
                .scroll_lists
                .get("PerksProgramScrollBox")
                .unwrap()
                .first_row,
            0
        );
        assert!(registry.get_by_name("PerksProgramProduct5").is_some());
    }
}

#[test]
fn trading_post_both_skins_show_real_tender_price_frozen_item_and_disabled_refund() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(
            skin,
            View {
                visible: true,
                snapshot: Some(Arc::new(snapshot())),
                selected: Some(4),
                ..Default::default()
            },
        );
        assert_eq!(
            text(&registry, "PerksProgramCurrencyAmount"),
            "500 Trader's Tender"
        );
        assert_eq!(text(&registry, "PerksProgramPrice4"), "100");
        assert_eq!(
            text(&registry, "PerksProgramFrozenLabel"),
            "Distinguished Warcloak"
        );
        let id = registry.get_by_name("PerksProgramRefundButton").unwrap();
        assert!(
            registry
                .get(id)
                .unwrap()
                .onclick
                .as_deref()
                .unwrap_or_default()
                .is_empty(),
            "zero server refundable seconds cannot issue a refund"
        );
        assert!(registry.get_by_name("PerksProgramMainModelScene").is_some());
    }
}
