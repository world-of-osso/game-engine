//! Native global bounds, derived from effective PerksProgram XML/Lua anchors.
use super::compute_layout_with_intrinsics;
use game_engine_ui_model::trading_post::{FilterSubmenu, View, trading_post_screen};
use image::GenericImageView;
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
fn trading_post_rotation_controls_project_source_bounds_and_visible_icons_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let (registry, bounds) = render(View {
            visible: true,
            selected: Some(4),
            snapshot: Some(Arc::new(vendor())),
            ..Default::default()
        });
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramRotateButtonContainer",
            [1050., 990., 100., 48.],
        );
        for (direction, left) in [("Left", 1050.), ("Right", 1102.)] {
            let name = format!("PerksProgramRotate{direction}Button");
            assert_rect(&registry, &bounds, &name, [left, 990., 48., 48.]);
            let icon_name = format!("{name}Icon");
            assert_rect(
                &registry,
                &bounds,
                &icon_name,
                [left + 11., 1001.5, 26., 25.],
            );
            let frame = registry
                .get(registry.get_by_name(&icon_name).unwrap())
                .unwrap();
            let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("Retail rotate icon texture")
            };
            let ui_toolkit::widgets::texture::TextureSource::FileDataId(fdid) = texture.source
            else {
                panic!("Retail rotate icon FDID")
            };
            let bytes = std::fs::read(game_engine_ui_model::paths::resolve_data_path(format!(
                "textures/{fdid}.blp"
            )))
            .unwrap();
            let decoded = game_engine_core::blp::decode_rgba(&bytes).unwrap();
            let image =
                image::RgbaImage::from_raw(decoded.width, decoded.height, decoded.pixels).unwrap();
            let [left, right, top, bottom] = texture.tex_coords;
            let region = image::imageops::crop_imm(
                &image,
                (left * image.width() as f32).round() as u32,
                (top * image.height() as f32).round() as u32,
                ((right - left) * image.width() as f32).round() as u32,
                ((bottom - top) * image.height() as f32).round() as u32,
            );
            assert!(
                region.pixels().any(|(_, _, pixel)| pixel[3] > 0),
                "Retail rotate icon contains no visible pixels"
            );
        }
    }
}

#[test]
fn trading_post_rotate_pressed_icon_survives_registry_layout_then_restores_on_release() {
    use ui_toolkit::{frame::WidgetData, widgets::button::ButtonState};
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let (mut registry, _) = render(View {
            visible: true,
            selected: Some(4),
            snapshot: Some(Arc::new(vendor())),
            ..Default::default()
        });
        for (name, left) in [
            ("PerksProgramRotateLeftButton", 1050.),
            ("PerksProgramRotateRightButton", 1102.),
        ] {
            let button = registry.get_by_name(name).unwrap();
            for (state, offset) in [(ButtonState::Pushed, 1.), (ButtonState::Normal, 0.)] {
                let Some(WidgetData::Button(data)) =
                    &mut registry.get_mut(button).unwrap().widget_data
                else {
                    panic!("rotation button")
                };
                data.state = state;
                game_engine_ui_model::trading_post::apply_rotation_icon_positions(&mut registry);
                let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
                assert_rect(
                    &registry,
                    &bounds,
                    &format!("{name}Icon"),
                    [left + 11. + offset, 1001.5 + offset, 26., 25.],
                );
            }
        }
    }
}

#[test]
fn trading_post_pending_checkout_keeps_source_overlay_and_centred_spinner_bounds() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = View {
            visible: true,
            selected: Some(4),
            cart: vec![4],
            snapshot: Some(Arc::new(vendor())),
            ..Default::default()
        };
        view.click("perks:cart:checkout").unwrap();
        view.click("perks:confirm").unwrap();
        let (registry, bounds) = render(view);
        let dialog = rect(&registry, &bounds, "PerksProgramConfirmDialog");
        assert_rect(&registry, &bounds, "PerksProgramConfirmDarkOverlay", dialog);
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramConfirmSpinner",
            [
                dialog[0] + (dialog[2] - 60.) / 2.,
                dialog[1] + (dialog[3] - 60.) / 2.,
                60.,
                60.,
            ],
        );
        for name in [
            "PerksProgramConfirmSpinnerRing",
            "PerksProgramConfirmSpinnerSparks",
        ] {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("source spinner texture");
            };
            let ui_toolkit::widgets::texture::TextureSource::FileDataId(fdid) = texture.source
            else {
                panic!("source atlas FDID");
            };
            let bytes = std::fs::read(game_engine_ui_model::paths::resolve_data_path(format!(
                "textures/{fdid}.blp"
            )))
            .unwrap();
            let decoded = game_engine_core::blp::decode_rgba(&bytes).unwrap();
            let [left, right, top, bottom] = texture.tex_coords;
            let image =
                image::RgbaImage::from_raw(decoded.width, decoded.height, decoded.pixels).unwrap();
            let region = image::imageops::crop_imm(
                &image,
                (left * image.width() as f32).round() as u32,
                (top * image.height() as f32).round() as u32,
                ((right - left) * image.width() as f32).round() as u32,
                ((bottom - top) * image.height() as f32).round() as u32,
            )
            .to_image();
            assert!(
                region.pixels().any(|pixel| pixel[3] > 0),
                "Source spinner region must contain visible pixels"
            );
        }
    }
}

#[test]
fn trading_post_cart_projected_padding_rows_and_second_page_follow_retail() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut data = vendor();
        data.items = (4..14)
            .map(|id| {
                let mut row = data.items[0].clone();
                row.id = id;
                row
            })
            .collect();
        let view = View {
            visible: true,
            selected: Some(4),
            cart: (4..14).collect(),
            cart_visible: true,
            snapshot: Some(Arc::new(data)),
            ..Default::default()
        };
        let mut shared = SharedContext::new();
        shared.insert(view);
        shared.insert(skin);
        let mut registry = FrameRegistry::new(1920., 1080.);
        let mut screen = Screen::new(trading_post_screen);
        screen.sync(&shared, &mut registry);
        let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        let panel = rect(&registry, &bounds, "PerksProgramShoppingCartFrame");
        assert_eq!(panel, [1474., 115., 400., 900.]);
        let list = rect(&registry, &bounds, "PerksProgramCartItemList");
        assert!((list[0] - panel[0] - 42.).abs() <= 0.51);
        assert_eq!(list[2], 322.);
        let scroll = rect(&registry, &bounds, "PerksProgramCartScrollBox");
        assert_eq!(scroll[2], 314.);
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramCartItem4",
            [scroll[0] + 12., scroll[1] + 4., 290., 84.],
        );
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramCartItem5",
            [scroll[0] + 12., scroll[1] + 4. + 92., 290., 84.],
        );
        assert_rect(
            &registry,
            &bounds,
            "PerksProgramCartItem4Icon",
            [scroll[0] + 12. + 16., scroll[1] + 4. + 14., 56., 56.],
        );
        registry
            .scroll_lists
            .scroll_to("PerksProgramCartScrollBox", usize::MAX);
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("PerksProgramCartItem13").is_some());
        assert!(registry.get_by_name("PerksProgramCartItem4").is_none());
        assert!(
            registry
                .scroll_lists
                .get("PerksProgramCartScrollBox")
                .unwrap()
                .first_row
                > 0
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
