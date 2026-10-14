//! Native global bounds, derived from effective PerksProgram XML/Lua anchors.
use super::compute_layout_with_intrinsics;
use game_engine_ui_model::trading_post::{FilterSubmenu, View, trading_post_screen};
use shared::protocol::{TradingPostCategory, TradingPostItem, TradingPostSnapshot};
use std::{collections::HashMap, sync::Arc};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn vendor() -> TradingPostSnapshot {
    TradingPostSnapshot {
        npc_net_id: Some(185468),
        tender: 500,
        points: 0,
        now_unix: 1790812900,
        month_end_unix: 1793491200,
        frozen_item_id: None,
        categories: vec![TradingPostCategory {
            id: 1,
            name: "Transmog".into(),
            model_scene_id: 430,
        }],
        items: vec![TradingPostItem {
            id: 4,
            category_id: 1,
            item_id: 189869,
            name: "Distinguished Warcloak".into(),
            description: "A red cloak".into(),
            icon_fdid: 135729,
            quality: 1,
            usable: Some(true),
            price: 100,
            purchased: false,
            collected: false,
            refund_seconds: 0,
            ui_group_name: String::new(),
            ui_group_priority: 0,
            model_scene_id: 583,
            creature_display_id: 0,
            preview_item_ids: vec![189869],
            grant_unavailable: None,
        }],
        thresholds: vec![],
        source_gaps: vec![],
        error: None,
        catalog_complete: true,
    }
}

fn render(view: View) -> (FrameRegistry, HashMap<u64, ui_toolkit::layout::LayoutRect>) {
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920., 1080.);
    Screen::new(trading_post_screen).sync(&shared, &mut registry);
    let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
    (registry, bounds)
}

fn rect(
    registry: &FrameRegistry,
    bounds: &HashMap<u64, ui_toolkit::layout::LayoutRect>,
    name: &str,
) -> [f32; 4] {
    let id = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("{name}"));
    let r = &bounds[&id];
    [r.x, r.y, r.width, r.height]
}
fn assert_rect(
    registry: &FrameRegistry,
    bounds: &HashMap<u64, ui_toolkit::layout::LayoutRect>,
    name: &str,
    expected: [f32; 4],
) {
    let actual = rect(registry, bounds, name);
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= 0.51,
            "{name}: {actual} != {expected} (native pixel rounding)"
        );
    }
}

#[test]
fn trading_post_projected_global_rects_follow_effective_retail_anchors_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let view = View {
            visible: true,
            snapshot: Some(Arc::new(vendor())),
            selected: Some(4),
            ..Default::default()
        };
        let (registry, bounds) = render(view.clone());
        for (name, expected) in [
            (
                "PerksProgramProductsScrollBoxContainer",
                [46., (1080. - 751.) / 2. - 22., 527., 751.],
            ),
            (
                "PerksProgramScrollBox",
                [46., 142.5 + 58., 527. - 24., 751. - 58. - 120.],
            ),
            ("PerksProgramIcon4", [46. + 32., 142.5 + 58. + 8., 32., 32.]),
            (
                "PerksProgramHoldFrame",
                [46., 142.5 + 58. + 573., 527., 120.],
            ),
            (
                "PerksProgramFrozenSlot",
                [46., 142.5 + 58. + 573. + (120. - 84.) / 2., 527., 84.],
            ),
            (
                "PerksProgramFilter",
                [46. + 9., 142.5 - 22. - 25., 145., 25.],
            ),
            ("PerksProgramFooterFrame", [0., 1080. - 100., 1920., 100.]),
            (
                "PerksProgramLeaveButton",
                [46., 1080. - 20. - 46., 180., 46.],
            ),
            (
                "PerksProgramPurchaseButton",
                [1920. - 46. - 180., 1080. - 20. - 46., 180., 46.],
            ),
            ("PerksProgramLeftGradient", [0., 0., 950., 1080.]),
            ("PerksProgramRightGradient", [1920. - 600., 0., 600., 1080.]),
        ] {
            assert_rect(&registry, &bounds, name, expected);
        }
        let details = rect(
            &registry,
            &bounds,
            "PerksProgramProductDetailsContainerFrame",
        );
        assert!((details[0] - (1920. - 46. - 360.)).abs() <= 0.51);
        assert!((details[1] + details[3] / 2. - 1080. / 2.).abs() <= 0.51);
        let time = rect(&registry, &bounds, "PerksProgramTimeSortButton");
        let price = rect(&registry, &bounds, "PerksProgramPriceSortButton");
        assert!((price[0] + price[2] + 48. - time[0]).abs() <= 0.51);
        assert!((price[1] + price[3] / 2. - time[1] - time[3] / 2.).abs() <= 0.51);
        let name = rect(&registry, &bounds, "PerksProgramNameSortButton");
        let label = rect(&registry, &bounds, "PerksProgramNameSortLabel");
        assert_eq!(label[0], name[0]);
        let (registry, bounds) = render(View {
            dropdown: true,
            submenu: Some(FilterSubmenu::Types),
            ..view.clone()
        });
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramFilterMenu",
            [55., 95.5 + 25., 145., 8. + 3. * 20. + 10. + 2. * 20. + 15.],
        );
        let owner = rect(&registry, &bounds, "PerksProgramFilterTypes");
        let child = rect(&registry, &bounds, "PerksProgramTypeMenu");
        assert!((child[0] - owner[0] - owner[2]).abs() <= 0.51);
        assert!((child[1] - owner[1]).abs() <= 0.51);
        let (registry, bounds) = render(View {
            confirm: Some("purchase".into()),
            ..view
        });
        let text_height = ui_toolkit::text_measure::measure_text(
            "Exchange 100",
            ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            12.,
        )
        .unwrap()
        .1;
        let item_y = 135. + 16. + text_height + 5.;
        let buttons_y = item_y + 38. + 18.;
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramConfirmCover",
            [0., 0., 1920., 1080.],
        );
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramConfirmDialog",
            [
                800.,
                135.,
                320.,
                16. + text_height + 5. + 38. + 18. + 21. + 16.,
            ],
        );
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramConfirmItem",
            [800. + (320. - 146.) / 2. - 4., item_y + 0.5, 37., 37.],
        );
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramConfirmItemText",
            [800. + (320. - 146.) / 2. + 43., item_y, 103., 38.],
        );
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramConfirmButton",
            [800. + (320. - 2. * 128. - 10.) / 2., buttons_y, 128., 21.],
        );
    }
}
