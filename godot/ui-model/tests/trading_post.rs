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
