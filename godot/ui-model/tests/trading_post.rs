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
            icon_fdid: 135729,
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
        catalog_complete: true,
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
    assert_eq!(view.click("perks:purchase").unwrap(), None);
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
fn trading_post_virtualized_list_reaches_second_page_then_empty_category_clamps_in_both_skins() {
    // Concrete authored identities, repeated with distinct vendor IDs to exceed a 573px page.
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
            .cycle()
            .take(24)
            .enumerate()
            .map(|(index, (id, item, price, name))| {
                let mut row = data.items[0].clone();
                row.id = *id + (index / authored.len()) as u32 * 10000;
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
        assert!(registry.get_by_name("PerksProgramProduct8").is_some());
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
        assert!(registry.get_by_name("PerksProgramProduct8").is_none());
        view.click("perks:category:2").unwrap();
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
        assert!(registry.get_by_name("PerksProgramProduct5").is_none());
    }
}

#[test]
fn trading_post_drag_freeze_replace_and_drag_off_wait_for_authority() {
    use game_engine_ui_model::trading_post::Action;
    let mut data = snapshot();
    let mut replacement = data.items[0].clone();
    replacement.id = 5;
    replacement.name = "Arcanofire Wrap".into();
    data.items.push(replacement);
    let mut view = View {
        visible: true,
        snapshot: Some(Arc::new(data.clone())),
        selected: Some(4),
        ..Default::default()
    };
    assert_eq!(
        view.drop_item(5, false, false),
        None,
        "abandoned drag does nothing"
    );
    assert_eq!(
        view.drop_item(5, false, true),
        Some(Action::Request(TradingPostRequest::Freeze {
            vendor_item_id: Some(5)
        }))
    );
    assert!(view.pending);
    assert_eq!(
        view.selected,
        Some(5),
        "dragging selects the displayed product"
    );
    assert_eq!(
        view.snapshot.as_ref().unwrap().frozen_item_id,
        Some(4),
        "no optimistic frozen replacement"
    );
    assert_eq!(
        view.drop_item(4, true, false),
        None,
        "pending request blocks drag-off"
    );
    data.frozen_item_id = Some(5);
    view.receive(Arc::new(data.clone()));
    assert_eq!(
        view.drop_item(5, true, true),
        None,
        "drop on same slot retains item"
    );
    assert_eq!(
        view.drop_item(5, true, false),
        Some(Action::Request(TradingPostRequest::Freeze {
            vendor_item_id: None
        }))
    );
    assert_eq!(view.snapshot.as_ref().unwrap().frozen_item_id, Some(5));
    view.receive(Arc::new(data));
    view.click("perks:purchase").unwrap();
    assert_eq!(
        view.drop_item(5, true, false),
        None,
        "confirmation is modal"
    );
}

#[test]
fn trading_post_dropdown_selection_and_sort_headers_order_real_rows() {
    let mut data = snapshot();
    let mut row = data.items[0].clone();
    row.id = 5;
    row.name = "Arcanofire Wrap".into();
    row.price = 75;
    data.items.push(row);
    let mut view = View {
        visible: true,
        snapshot: Some(Arc::new(data)),
        selected: Some(4),
        ..Default::default()
    };
    view.click("perks:sort:price").unwrap();
    assert_eq!(
        view.rows().iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![5, 4]
    );
    view.click("perks:sort:price").unwrap();
    assert_eq!(
        view.rows().iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![4, 5]
    );
    view.click("perks:filter").unwrap();
    let registry = render(ActiveSkin::Forever, view.clone());
    assert!(registry.get_by_name("PerksProgramCategoryMenu").is_some());
    view.click("perks:category:2").unwrap();
    assert!(!view.dropdown);
    assert!(view.rows().is_empty());
    assert_eq!(view.selected, None);
}

fn rect(registry: &FrameRegistry, name: &str) -> [f32; 4] {
    use ui_toolkit::layout_values::Val;
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let (Val::Px(x), Val::Px(y)) = (frame.position.left, frame.position.top) else {
        panic!("{name}: absolute geometry required");
    };
    [x, y, frame.resolved_width(), frame.resolved_height()]
}

#[test]
fn trading_post_retail_xml_geometry_and_dropdown_in_both_skins() {
    // Blizzard_PerksProgram.xml: LEFT+46, vertical center+22; WoW Y is upwards.
    // Elements.xml base extent 176x48; list stretches width to 527-24.
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
            rect(&registry, "PerksProgramProductsScrollBoxContainer"),
            [46., 142.5, 527., 751.]
        );
        assert_eq!(
            rect(&registry, "PerksProgramScrollBox"),
            [0., 58., 503., 573.]
        );
        assert_eq!(rect(&registry, "PerksProgramProduct4"), [0., 0., 503., 48.]);
        assert_eq!(rect(&registry, "PerksProgramIcon4"), [32., 8., 32., 32.]);
        assert_eq!(
            rect(&registry, "PerksProgramFilter"),
            [55., 95.5, 122., 25.]
        );
        assert_eq!(
            rect(&registry, "PerksProgramHoldFrame"),
            [0., 631., 527., 120.]
        );
        assert_eq!(
            rect(&registry, "PerksProgramFrozenSlot"),
            [0., 18., 527., 84.]
        );
        assert_eq!(
            rect(&registry, "PerksProgramFooterFrame"),
            [0., 980., 1920., 100.]
        );
        assert_eq!(
            rect(&registry, "PerksProgramPurchaseButton"),
            [1694., 34., 180., 46.]
        );
        assert_eq!(
            rect(&registry, "PerksProgramPurchasePriceLayout"),
            [54., 13., 72., 20.]
        );
        assert!(registry.get_by_name("PerksProgramSearchBox").is_none());
        assert!(registry.get_by_name("PerksProgramFreezeButton").is_none());
        assert!(registry.get_by_name("PerksProgramUnfreezeButton").is_none());
        assert!(
            registry.get_by_name("PerksProgramCategoryMenu").is_none(),
            "closed dropdown is not a persistent category button column"
        );
        assert!(registry.get_by_name("PerksProgramCategoryAll").is_none());
        for header in ["Name", "Time", "Price"] {
            assert!(
                registry
                    .get_by_name(&format!("PerksProgram{header}SortButton"))
                    .is_some()
            );
        }
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
        assert_eq!(text(&registry, "PerksProgramCurrencyAmount"), "500");
        assert_eq!(text(&registry, "PerksProgramPrice4"), "100");
        assert_eq!(
            text(&registry, "PerksProgramFrozenLabel"),
            "Distinguished Warcloak"
        );
        assert!(
            registry.get_by_name("PerksProgramRefundButton").is_none(),
            "Retail hides Refund without a server refund window"
        );
        assert!(registry.get_by_name("PerksProgramMainModelScene").is_some());
    }
}

#[test]
fn trading_post_confirmation_uses_sourced_prompt_item_and_dialog_geometry() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(
            skin,
            View {
                visible: true,
                snapshot: Some(Arc::new(snapshot())),
                selected: Some(4),
                confirm: Some("purchase".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            text(&registry, "PerksProgramConfirmTextBefore"),
            "Exchange 100"
        );
        assert_eq!(
            text(&registry, "PerksProgramConfirmTextAfter"),
            "for the following item?"
        );
        assert_eq!(
            text(&registry, "PerksProgramConfirmItemText"),
            "Distinguished Warcloak"
        );
        assert_eq!(
            rect(&registry, "PerksProgramConfirmButton")[2..],
            [128., 21.]
        );
        assert_eq!(rect(&registry, "PerksProgramConfirmItem")[2..], [37., 37.]);
        assert_eq!(
            rect(&registry, "PerksProgramConfirmCover")[2..],
            [1920., 1080.]
        );
    }
}
