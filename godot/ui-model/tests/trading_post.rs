use game_engine_ui_model::trading_post::{View, trading_post_screen};
use shared::protocol::*;
use std::sync::Arc;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn trading_post_refund_icon_tooltip_uses_hovered_row_time_not_selected_product() {
    let mut data = snapshot();
    let mut refundable = data.items[0].clone();
    refundable.id = 5;
    refundable.refund_seconds = 3661;
    refundable.purchased = true;
    data.items.push(refundable);
    let view = View {
        visible: true,
        selected: Some(4),
        snapshot: Some(Arc::new(data)),
        refund_elapsed: 60,
        ..Default::default()
    };
    let tooltip = view.refund_icon_tooltip(5).unwrap();
    assert_eq!(
        tooltip.content.title_text(),
        "Time remaining to refund this item: 1h"
    );
    assert_eq!(
        tooltip.content.title_suffix,
        Some(("1h".into(), [1., 1., 1., 1.]))
    );
    assert!(tooltip.content.lines.is_empty());
    assert!(tooltip.record.is_none());
    assert!(view.refund_icon_tooltip(4).is_none());
    assert!(view.refund_icon_tooltip(999).is_none());
    assert_eq!(view.selected, Some(4));
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
    assert_eq!(view.snapshot.as_ref().unwrap().frozen_item_id, Some(4));
}

#[test]
fn trading_post_refund_icon_tooltip_truncates_minutes_and_colors_time_at_one_hour() {
    for (seconds, text, color) in [
        (90061, "1d 1h", [1., 1., 1., 1.]),
        (3661, "1h 1m", [1., 1., 1., 1.]),
        (3600, "1h", [1., 0.1, 0.1, 1.]),
        (3599, "59m", [1., 0.1, 0.1, 1.]),
        (59, "0m", [1., 0.1, 0.1, 1.]),
    ] {
        let mut data = snapshot();
        data.items[0].refund_seconds = seconds;
        let view = View {
            visible: true,
            snapshot: Some(Arc::new(data)),
            ..Default::default()
        };
        let tooltip = view.refund_icon_tooltip(4).unwrap();
        assert_eq!(
            tooltip.content.title,
            "Time remaining to refund this item: "
        );
        assert_eq!(tooltip.content.title_suffix, Some((text.into(), color)));
    }
}

#[test]
fn trading_post_refund_icon_tooltip_hides_at_expiry_and_vendor_teardown() {
    let mut data = snapshot();
    data.items[0].refund_seconds = 60;
    let mut view = View {
        visible: true,
        snapshot: Some(Arc::new(data)),
        refund_elapsed: 59,
        ..Default::default()
    };
    assert!(view.refund_icon_tooltip(4).is_some());
    view.refund_elapsed = 60;
    assert!(view.refund_icon_tooltip(4).is_none());
    view.refund_elapsed = 61;
    assert!(view.refund_icon_tooltip(4).is_none());
    view.refund_elapsed = 0;
    view.visible = false;
    assert!(view.refund_icon_tooltip(4).is_none());
}

#[test]
fn trading_post_row_cart_toggle_changes_hovered_product_not_selected_preview() {
    let mut data = snapshot();
    let mut other = data.items[0].clone();
    other.id = 5;
    data.items.push(other);
    let mut view = View {
        visible: true,
        selected: Some(4),
        snapshot: Some(Arc::new(data)),
        ..Default::default()
    };
    assert!(view.click("perks:cart:toggle:5").unwrap().is_none());
    assert_eq!(view.cart, [5]);
    let tooltip = view
        .button_tooltip("PerksProgramCartToggle5", true)
        .unwrap();
    assert_eq!(tooltip.content.title, "Remove from Cart");
    assert!(tooltip.content.lines.is_empty());
    assert!(tooltip.record.is_none());
    assert_eq!(view.selected, Some(4));
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
    assert!(!view.pending);
    assert!(view.click("perks:cart:toggle:5").unwrap().is_none());
    assert!(view.cart.is_empty());
    assert_eq!(
        view.button_tooltip("PerksProgramCartToggle5", true)
            .unwrap()
            .content
            .title,
        "Add to Cart"
    );
    assert!(
        view.button_tooltip("PerksProgramCartToggle4", true)
            .is_none()
    );
    assert_eq!(view.selected, Some(4));
    assert_eq!(view.snapshot.as_ref().unwrap().frozen_item_id, Some(4));
}

#[test]
fn trading_post_row_cart_toggle_rejects_ineligible_products() {
    let mut data = snapshot();
    for id in 5..=8 {
        let mut row = data.items[0].clone();
        row.id = id;
        row.purchased = id == 5;
        row.collected = id == 6;
        row.refund_seconds = if id == 7 { 600 } else { 0 };
        row.grant_unavailable = (id == 8).then(|| "Unbound grant".into());
        data.items.push(row);
    }
    let mut view = View {
        visible: true,
        selected: Some(4),
        snapshot: Some(Arc::new(data)),
        ..Default::default()
    };
    for id in 4..=8 {
        assert!(
            view.click(&format!("perks:cart:toggle:{id}"))
                .unwrap()
                .is_none()
        );
        assert!(view.cart.is_empty());
    }
    assert_eq!(view.selected, Some(4));
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
}

#[test]
fn trading_post_row_cart_toggle_preserves_open_cart_preview_choices() {
    let mut data = snapshot();
    data.frozen_item_id = None;
    let mut sword = data.items[0].clone();
    sword.id = 5;
    sword.item_id = 25;
    sword.name = "Worn Shortsword".into();
    sword.preview_item_ids = vec![25];
    data.items.push(sword);
    let mut view = View {
        visible: true,
        selected: Some(4),
        snapshot: Some(Arc::new(data)),
        cart: vec![4],
        ..Default::default()
    };
    view.click("perks:cart:show").unwrap();
    assert_eq!(view.cart_preview_item_ids(), [189869]);
    view.click("perks:cart:item:189869").unwrap();
    assert!(view.cart_preview_item_ids().is_empty());
    view.click("perks:cart:toggle:5").unwrap();
    assert_eq!(view.cart, [4, 5]);
    assert_eq!(view.cart_preview_item_ids(), [25]);
    assert_eq!(view.selected, Some(4));
    assert!(view.cart_visible);
    view.click("perks:cart:toggle:5").unwrap();
    assert_eq!(view.cart, [4]);
    assert!(view.cart_preview_item_ids().is_empty());
}

#[test]
fn trading_post_row_cart_toggle_keeps_other_override_while_appending_wearables() {
    let mut data = snapshot();
    let mut other = data.items[0].clone();
    other.id = 5;
    other.category_id = 2;
    other.item_id = 117;
    other.preview_item_ids = vec![117];
    let mut sword = data.items[0].clone();
    sword.id = 6;
    sword.item_id = 25;
    sword.preview_item_ids = vec![25];
    data.items.extend([other, sword]);
    let mut view = View {
        visible: true,
        selected: Some(5),
        snapshot: Some(Arc::new(data)),
        cart: vec![5],
        ..Default::default()
    };
    view.click("perks:cart:show").unwrap();
    assert_eq!(view.preview_product().unwrap().item_id, 117);
    view.click("perks:cart:toggle:6").unwrap();
    assert_eq!(view.cart, [5, 6]);
    assert_eq!(view.preview_product().unwrap().item_id, 117);
    assert_eq!(view.cart_preview_item_ids(), [25]);
    view.click("perks:cart:item:25").unwrap();
    assert_eq!(view.preview_product().unwrap().item_id, 25);
    assert_eq!(view.cart_preview_item_ids(), [25]);
    assert_eq!(view.selected, Some(5));
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
}

fn snapshot() -> TradingPostSnapshot {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
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
            usable: Some(true),
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
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    set_thread_skin(skin);
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx.insert(view);
    let mut registry = FrameRegistry::new(1920., 1080.);
    Screen::new(trading_post_screen).sync(&ctx, &mut registry);
    game_engine_ui_model::trading_post::apply_cart_art(ctx.get::<View>().unwrap(), &mut registry);
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
fn trading_post_other_cart_preview_dims_wearables_but_keeps_remove_colored_and_clickable() {
    let mut data = snapshot();
    let mut mount = data.items[0].clone();
    mount.id = 5;
    mount.category_id = 2;
    mount.item_id = 117;
    data.items.push(mount);
    let mut view = View {
        visible: true,
        snapshot: Some(Arc::new(data)),
        cart: vec![4, 5],
        cart_visible: true,
        selected: Some(4),
        cart_selected: std::collections::BTreeMap::from([(0, 117), (16, 189869)]),
        cart_hover: Some(4),
        ..Default::default()
    };
    let registry = render(ActiveSkin::Modern, view.clone());
    let frame = registry
        .get(registry.get_by_name("PerksProgramCartItem4").unwrap())
        .unwrap();
    assert!(
        (frame.alpha - 0.7).abs() < 0.01,
        "Retail disabled-preview row alpha is0.7, not1"
    );
    let icon = registry
        .get(registry.get_by_name("PerksProgramCartItem4Icon").unwrap())
        .unwrap();
    assert!(matches!(&icon.widget_data,Some(WidgetData::Texture(texture)) if texture.desaturated));
    let remove = registry
        .get(
            registry
                .get_by_name("PerksProgramCartItem4RemoveIcon")
                .unwrap(),
        )
        .unwrap();
    assert!(
        matches!(&remove.widget_data,Some(WidgetData::Texture(texture)) if !texture.desaturated)
    );
    view.click("perks:cart:item:189869").unwrap();
    assert!(
        view.cart_selected.get(&0).is_none(),
        "Disabled wearable click must clear OTHER first"
    );
    assert_eq!(
        view.cart_preview_item_ids(),
        [189869],
        "Do not deselect the hidden wearable while clearing OTHER"
    );
}

#[test]
fn trading_post_cart_total_never_ellipsizes_four_digit_authored_prices() {
    let mut data = snapshot();
    data.items[0].price = 6546;
    data.tender = 10000;
    let view = View {
        visible: true,
        snapshot: Some(Arc::new(data)),
        cart: vec![4],
        cart_visible: true,
        selected: Some(4),
        ..Default::default()
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view.clone());
        let price = rect(&registry, "PerksProgramCartPriceText");
        let (width, _) = ui_toolkit::text_measure::measure_text(
            "6546",
            ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            20.,
        )
        .unwrap();
        assert!(
            price[2] >= width,
            "Retail right-anchored natural text must not be ellipsized: {price:?} versus{width}"
        );
        assert!(
            (price[0] + price[2] - 44.).abs() < 0.01,
            "PriceContainer44 keeps right edge, not a44px text clip"
        );
    }
}

#[test]
fn trading_post_cart_hover_shows_source_preview_status_and_dimming_mask() {
    let mut view = View {
        visible: true,
        snapshot: Some(Arc::new(snapshot())),
        selected: Some(4),
        cart: vec![4],
        cart_visible: true,
        cart_hover: Some(4),
        cart_selected: std::collections::BTreeMap::from([(16, 189869)]),
        ..Default::default()
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view.clone());
        assert!(
            registry
                .get_by_name("PerksProgramCartItem4PreviewStatus")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("PerksProgramCartItem4Vignette")
                .is_some()
        );
        view.cart_hover = None;
        let registry = render(skin, view.clone());
        assert!(
            registry
                .get_by_name("PerksProgramCartItem4PreviewStatus")
                .is_none()
        );
        assert!(
            registry
                .get_by_name("PerksProgramCartItem4Vignette")
                .is_none()
        );
        view.cart_hover = Some(4);
    }
}

#[test]
fn trading_post_cart_ensemble_expands_header_members_and_source_list_padding() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.items[0].category_id = 8;
        data.items[0].preview_item_ids = vec![25, 876];
        let view = View {
            visible: true,
            snapshot: Some(Arc::new(data)),
            selected: Some(4),
            cart: vec![4],
            cart_visible: true,
            ..Default::default()
        };
        let registry = render(skin, view);
        assert_eq!(rect(&registry, "PerksProgramCartSet4")[3], 56.);
        assert_eq!(
            text(&registry, "PerksProgramCartSubItem4_25Name"),
            "Worn Shortsword"
        );
        assert_eq!(
            text(&registry, "PerksProgramCartSubItem4_876Name"),
            "Worn Wooden Buckler"
        );
        assert_eq!(rect(&registry, "PerksProgramCartItemList")[2], 322.);
        assert_eq!(rect(&registry, "PerksProgramCartSet4")[..2], [12., 4.]);
        assert_eq!(
            rect(&registry, "PerksProgramCartSubItem4_25")[..2],
            [12., 68.]
        );
        assert_eq!(rect(&registry, "PerksProgramCartSubItem4_876")[3], 104.);
    }
}

#[test]
fn trading_post_cart_selects_exclusive_hand_slots_and_deselects_clicked_item() {
    let mut data = snapshot();
    data.frozen_item_id = None;
    data.items.clear();
    for (index, item_id) in [25, 35, 876, 1190].into_iter().enumerate() {
        let mut row = snapshot().items[0].clone();
        row.id = index as u32 + 10;
        row.item_id = item_id;
        row.preview_item_ids = vec![item_id];
        data.items.push(row);
    }
    let mut view = View {
        visible: true,
        snapshot: Some(Arc::new(data)),
        cart: vec![10, 11, 12, 13],
        selected: Some(10),
        ..Default::default()
    };
    render(ActiveSkin::Modern, view.clone());
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    view.click("perks:cart:show").unwrap();
    assert_eq!(view.cart_preview_item_ids(), [1190, 25, 876]);
    view.click("perks:cart:item:35").unwrap();
    assert_eq!(view.cart_preview_item_ids(), [1190, 35]);
    view.click("perks:cart:item:35").unwrap();
    assert_eq!(view.cart_preview_item_ids(), [1190]);
    view.click("perks:cart:item:876").unwrap();
    assert_eq!(view.cart_preview_item_ids(), [1190, 876]);
    assert_eq!(view.cart, [10, 11, 12, 13]); // Preview never removes/charges a cart product.
}

#[test]
fn trading_post_retail_rotate_buttons_match_footer_anchors_and_hide_for_non_model_products() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = View {
            visible: true,
            snapshot: Some(Arc::new(snapshot())),
            selected: Some(4),
            ..Default::default()
        };
        let registry = render(skin, view.clone());
        assert_eq!(
            rect(&registry, "PerksProgramRotateButtonContainer"),
            [1050., 10., 100., 48.]
        );
        for (name, left, atlas) in [
            ("PerksProgramRotateLeftButton", 0., "perks-icon-rotate-left"),
            (
                "PerksProgramRotateRightButton",
                52.,
                "perks-icon-rotate-right",
            ),
        ] {
            assert_eq!(rect(&registry, name), [left, 0., 48., 48.]);
            assert_eq!(
                rect(&registry, &format!("{name}Icon")),
                [11., 11.5, 26., 25.]
            );
            let icon = registry
                .get(registry.get_by_name(&format!("{name}Icon")).unwrap())
                .unwrap();
            let Some(WidgetData::Texture(icon)) = &icon.widget_data else {
                panic!("rotate icon must be authored art")
            };
            let region =
                ui_toolkit::atlas::resolve_region(atlas, skin).expect("Retail rotate atlas");
            let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
                panic!("rotate art needs FDID")
            };
            assert_eq!(
                icon.source,
                ui_toolkit::widgets::texture::TextureSource::FileDataId(fdid)
            );
            assert_eq!(
                icon.tex_coords,
                [region.left, region.right, region.top, region.bottom]
            );
        }
        assert_eq!(view.click("perks:rotate-left").unwrap(), None);
        assert_eq!(view.click("perks:rotate-right").unwrap(), None);
        assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
        for (category, unavailable) in [(5, None), (1, Some("Source model unavailable".into()))] {
            let data = Arc::make_mut(view.snapshot.as_mut().unwrap());
            data.items[0].category_id = category;
            data.items[0].grant_unavailable = unavailable;
            assert!(
                render(skin, view.clone())
                    .get_by_name("PerksProgramRotateButtonContainer")
                    .is_none()
            );
        }
    }
}

#[test]
fn trading_post_mount_hide_character_checkbox_defaults_visible_and_retains_user_choice() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        let mut mount = data.items[0].clone();
        mount.id = 5;
        mount.category_id = 2;
        mount.item_id = 54811;
        data.items.push(mount);
        let data = Arc::new(data);
        let mut view = View {
            visible: true,
            snapshot: Some(data.clone()),
            selected: Some(5),
            ..Default::default()
        };
        let registry = render(skin, view.clone());
        assert_eq!(
            rect(&registry, "PerksProgramTogglePlayerPreview"),
            [1032., -29., 30., 29.]
        );
        assert_eq!(
            text(&registry, "PerksProgramTogglePlayerPreviewText"),
            "Hide Character"
        );
        assert!(
            registry
                .get_by_name("PerksProgramTogglePlayerPreviewChecked")
                .is_none()
        );
        assert_eq!(view.click("perks:hide-character").unwrap(), None);
        assert!(
            render(skin, view.clone())
                .get_by_name("PerksProgramTogglePlayerPreviewChecked")
                .is_some()
        );
        view.click("perks:select:4").unwrap();
        assert!(
            render(skin, view.clone())
                .get_by_name("PerksProgramTogglePlayerPreview")
                .is_none()
        );
        view.click("perks:select:5").unwrap();
        view.receive(data);
        assert!(
            render(skin, view.clone())
                .get_by_name("PerksProgramTogglePlayerPreviewChecked")
                .is_some()
        );
        view.click("perks:hide-character").unwrap();
        assert!(
            render(skin, view.clone())
                .get_by_name("PerksProgramTogglePlayerPreviewChecked")
                .is_none()
        );
        assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
    }
}

#[test]
fn trading_post_hide_armor_uses_authored_default_and_persists_toggle_between_products() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = View {
            visible: true,
            snapshot: Some(Arc::new(snapshot())),
            selected: Some(4),
            ..Default::default()
        };
        render(skin, view.clone()); // Initialize the same data root as the host.
        assert!(!view.hide_armor().unwrap()); // Scene583 flags6 includes Autodress4.
        assert!(game_engine_ui_model::trading_post::load_scene_autodress(430).unwrap());
        assert!(!game_engine_ui_model::trading_post::load_scene_autodress(418).unwrap());
        assert!(game_engine_ui_model::trading_post::load_scene_autodress(u32::MAX).is_err());
        view.click("perks:hide-armor").unwrap();
        let registry = render(skin, view.clone());
        assert_eq!(
            text(&registry, "PerksProgramToggleHideArmorText"),
            "Hide Armor"
        );
        assert!(
            registry
                .get_by_name("PerksProgramToggleHideArmorChecked")
                .is_some()
        );
        view.receive(Arc::new(snapshot()));
        assert!(
            render(skin, view.clone())
                .get_by_name("PerksProgramToggleHideArmorChecked")
                .is_some()
        );
        view.click("perks:hide-armor").unwrap();
        assert!(
            render(skin, view)
                .get_by_name("PerksProgramToggleHideArmorChecked")
                .is_none()
        );
    }
}

#[test]
fn trading_post_cart_add_remove_and_confirmed_clear_follow_retail_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.frozen_item_id = None;
        let mut wrap = data.items[0].clone();
        wrap.id = 5;
        wrap.name = "Arcanofire Wrap".into();
        wrap.price = 75;
        data.items.push(wrap);
        let mut view = View {
            visible: true,
            snapshot: Some(Arc::new(data)),
            selected: Some(4),
            ..Default::default()
        };
        assert_eq!(view.click("perks:cart:add").unwrap(), None);
        assert_eq!(view.click("perks:cart:add").unwrap(), None);
        assert_eq!(
            text(
                &render(skin, view.clone()),
                "PerksProgramViewCartItemCountText"
            ),
            "1"
        );
        view.click("perks:select:5").unwrap();
        view.click("perks:cart:add").unwrap();
        assert_eq!(
            text(
                &render(skin, view.clone()),
                "PerksProgramViewCartItemCountText"
            ),
            "2"
        );
        view.click("perks:cart:show").unwrap();
        assert!(
            render(skin, view.clone())
                .get_by_name("PerksProgramShoppingCartFrame")
                .is_some()
        );
        view.click("perks:cart:close").unwrap();
        view.click("perks:cart:remove").unwrap();
        assert_eq!(
            text(
                &render(skin, view.clone()),
                "PerksProgramViewCartItemCountText"
            ),
            "1"
        );
        view.click("perks:cart:show").unwrap();
        view.click("perks:cart:clear").unwrap();
        view.click("perks:cancel").unwrap();
        view.click("perks:cart:close").unwrap();
        assert_eq!(
            text(
                &render(skin, view.clone()),
                "PerksProgramViewCartItemCountText"
            ),
            "1"
        );
        view.click("perks:cart:show").unwrap();
        view.click("perks:cart:clear").unwrap();
        assert_eq!(view.click("perks:confirm").unwrap(), None);
        let registry = render(skin, view);
        assert!(
            registry
                .get_by_name("PerksProgramShoppingCartFrame")
                .is_none()
        );
        assert_eq!(text(&registry, "PerksProgramViewCartItemCountText"), "0");
    }
}

#[test]
fn trading_post_cart_checkout_confirms_total_then_waits_for_authority_without_local_debit() {
    use game_engine_ui_model::trading_post::Action;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.frozen_item_id = None;
        data.tender = 150;
        let mut wrap = data.items[0].clone();
        wrap.id = 5;
        wrap.name = "Arcanofire Wrap".into();
        wrap.price = 75;
        data.items.push(wrap);
        let mut view = View {
            visible: true,
            snapshot: Some(Arc::new(data.clone())),
            selected: Some(4),
            ..Default::default()
        };
        view.click("perks:cart:add").unwrap();
        view.click("perks:select:5").unwrap();
        view.click("perks:cart:add").unwrap();
        view.click("perks:cart:show").unwrap();
        assert_eq!(view.click("perks:cart:checkout").unwrap(), None);
        assert!(
            view.confirm.is_none(),
            "whole cart costs175, not just selected75"
        );
        data.tender = 500;
        view.receive(Arc::new(data.clone()));
        view.click("perks:cart:checkout").unwrap();
        let dialog = render(skin, view.clone());
        assert!(dialog.get_by_name("PerksProgramConfirmDialog").is_some());
        assert!(
            dialog.get_by_name("PerksProgramConfirmItemFrame").is_none(),
            "Retail cart confirmation has no single-item frame"
        );
        view.click("perks:cancel").unwrap();
        assert_eq!(
            text(&render(skin, view.clone()), "PerksProgramCartPriceText"),
            "175"
        );
        view.click("perks:cart:checkout").unwrap();
        assert_eq!(
            view.click("perks:confirm").unwrap(),
            Some(Action::Request(TradingPostRequest::Checkout {
                vendor_item_ids: vec![4, 5]
            }))
        );
        assert!(view.pending);
        let pending = render(skin, view.clone());
        assert!(
            pending
                .get_by_name("PerksProgramConfirmSpinnerRing")
                .is_some(),
            "Source accept keeps confirmation covered by SpinnerTemplate until reply"
        );
        let prompt: String = pending
            .frames_iter()
            .filter_map(|frame| match &frame.widget_data {
                Some(WidgetData::FontString(text))
                    if frame
                        .name
                        .as_deref()
                        .is_some_and(|name| name.starts_with("PerksProgramConfirm")) =>
                {
                    Some(text.text.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            prompt.contains("175")
                && prompt.split_whitespace().any(|word| word == "2")
                && prompt.contains("items?"),
            "Captured prompt changed after cart clear: {prompt}"
        );
        for name in [
            "PerksProgramConfirmButton",
            "PerksProgramConfirmCancelButton",
        ] {
            let frame = pending.get(pending.get_by_name(name).unwrap()).unwrap();
            assert!(
                matches!(&frame.widget_data,Some(WidgetData::Button(button)) if button.state == ui_toolkit::widgets::button::ButtonState::Disabled)
            );
        }
        assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
        assert!(
            view.snapshot
                .as_ref()
                .unwrap()
                .items
                .iter()
                .all(|row| !row.purchased)
        );
        assert!(!view.cart_visible);
        assert_eq!(
            text(
                &render(skin, view.clone()),
                "PerksProgramViewCartItemCountText"
            ),
            "0"
        );
        data.tender = 325;
        for row in &mut data.items {
            row.purchased = true;
            row.collected = true;
            row.refund_seconds = 600;
        }
        view.receive(Arc::new(data));
        assert!(!view.pending);
        assert_eq!(view.snapshot.as_ref().unwrap().tender, 325);
        assert!(view.selected_item().unwrap().purchased);
    }
}

#[test]
fn trading_post_purchase_history_shows_owned_or_refund_time_instead_of_a_purchase_button() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.items[0].purchased = true;
        data.items[0].collected = true;
        let view = View {
            visible: true,
            selected: Some(4),
            snapshot: Some(Arc::new(data.clone())),
            ..Default::default()
        };
        let owned = render(skin, view);
        assert_eq!(
            text(&owned, "PerksProgramHistoryPurchasedText"),
            "You own this item"
        );
        assert!(owned.get_by_name("PerksProgramPurchaseButton").is_none());
        assert!(
            owned
                .get_by_name("PerksProgramHistoryPurchasedIcon")
                .is_some()
        );
        data.items[0].refund_seconds = 600;
        let view = View {
            visible: true,
            selected: Some(4),
            snapshot: Some(Arc::new(data)),
            ..Default::default()
        };
        let refundable = render(skin, view);
        assert_eq!(
            text(&refundable, "PerksProgramHistoryRefundPrefix"),
            "Time remaining to refund this item:"
        );
        assert_eq!(text(&refundable, "PerksProgramHistoryRefundTime"), "10m");
        assert!(refundable.get_by_name("PerksProgramRefundButton").is_some());
        assert!(
            refundable
                .get_by_name("PerksProgramHistoryPurchasedText")
                .is_none()
        );
    }
}

#[test]
fn trading_post_refund_history_expires_without_refresh_or_local_refund() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut data = snapshot();
        data.items[0].purchased = true;
        data.items[0].collected = true;
        data.items[0].refund_seconds = 600;
        let mut view = View {
            visible: true,
            selected: Some(4),
            snapshot: Some(Arc::new(data)),
            refund_elapsed: 541,
            ..Default::default()
        };
        let almost = render(skin, view.clone());
        assert_eq!(
            text(&almost, "PerksProgramHistoryRefundTime"),
            "0m",
            "source minimum-minute formatter floors the remaining59s"
        );
        assert!(view.can_refund());
        view.refund_elapsed = 600;
        let expired = render(skin, view.clone());
        assert!(!view.can_refund());
        assert!(expired.get_by_name("PerksProgramRefundButton").is_none());
        assert_eq!(
            text(&expired, "PerksProgramHistoryPurchasedText"),
            "You own this item"
        );
        assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
        assert!(view.selected_item().unwrap().purchased && view.selected_item().unwrap().collected);
    }
}

#[test]
fn trading_post_renders_player_faction_gradients_in_both_skins() {
    use ui_toolkit::atlas::{AtlasSource, resolve_region};
    use ui_toolkit::widgets::texture::TextureSource;
    // PerksProgramProducts.lua:23-30, UnitFactionGroup("player").
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (horde, city) in [(false, "stormwind"), (true, "orgrimmar")] {
            let registry = render(
                skin,
                View {
                    visible: true,
                    horde,
                    ..Default::default()
                },
            );
            for (name, side) in [
                ("PerksProgramLeftGradient", "left"),
                ("PerksProgramRightGradient", "right"),
            ] {
                let region =
                    resolve_region(&format!("perks-gradient-{city}-{side}"), skin).unwrap();
                let AtlasSource::FileDataId(fdid) = region.source else {
                    panic!("source atlas");
                };
                let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
                let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                    panic!("gradient");
                };
                assert_eq!(
                    texture.source,
                    TextureSource::FileDataId(fdid),
                    "{skin:?} {city} {side}"
                );
                assert_eq!(
                    texture.tex_coords,
                    [region.left, region.right, region.top, region.bottom],
                    "{skin:?} {city} {side}"
                );
            }
        }
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
    view.click("perks:type:2").unwrap();
    assert!(
        view.filters.excluded_types.is_empty(),
        "modal blocks underlying filter clicks"
    );
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
        view.click("perks:type:1").unwrap();
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
        None,
        "occupied slot waits for confirmation"
    );
    view.update_freeze_confirmation(5.0);
    assert_eq!(
        view.click("perks:confirm").unwrap(),
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
    assert!(registry.get_by_name("PerksProgramFilterMenu").is_some());
    view.click("perks:type:1").unwrap();
    assert!(view.dropdown, "checkbox refresh keeps the menu open");
    assert!(view.rows().is_empty());
    assert_eq!(
        view.selected,
        Some(4),
        "frozen selection remains independent of list filters"
    );
    view.click("perks:filter").unwrap();
    assert!(!view.dropdown);
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
            [55., 95.5, 145., 25.]
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
            registry.get_by_name("PerksProgramFilterMenu").is_none(),
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

#[test]
fn trading_post_header_labels_center_price_and_follow_hover_colors() {
    let mut registry = render(
        ActiveSkin::Modern,
        View {
            visible: true,
            snapshot: Some(Arc::new(snapshot())),
            selected: Some(4),
            ..Default::default()
        },
    );
    assert_eq!(text(&registry, "PerksProgramNameSortLabel"), "Name");
    let time = rect(&registry, "PerksProgramTimeSortButton");
    let price = rect(&registry, "PerksProgramPriceSortButton");
    assert_eq!(time[1] + time[3] / 2., price[1] + price[3] / 2.);
    let name = registry.get_by_name("PerksProgramNameSortButton").unwrap();
    let label = registry.get_by_name("PerksProgramNameSortLabel").unwrap();
    for (hovered, expected) in [
        (false, [1., 0.82, 0., 1.]),
        (true, [1.; 4]),
        (false, [1., 0.82, 0., 1.]),
    ] {
        let Some(WidgetData::Button(button)) = &mut registry.get_mut(name).unwrap().widget_data
        else {
            panic!("button");
        };
        button.hovered = hovered;
        game_engine_ui_model::menu_template_art::sync_template_art(&mut registry);
        let Some(WidgetData::FontString(font)) = &registry.get(label).unwrap().widget_data else {
            panic!("font");
        };
        assert_eq!(font.color, expected);
    }
    let Some(WidgetData::Texture(icon)) = &registry
        .get(registry.get_by_name("PerksProgramTimeSortIcon").unwrap())
        .unwrap()
        .widget_data
    else {
        panic!("icon");
    };
    assert_eq!(icon.vertex_color, [1., 0.82, 0., 1.]);
}

#[test]
fn trading_post_empty_freeze_slot_and_leave_use_sourced_text_and_arrow() {
    let mut source = snapshot();
    source.frozen_item_id = None;
    let registry = render(
        ActiveSkin::Modern,
        View {
            visible: true,
            snapshot: Some(Arc::new(source)),
            selected: Some(4),
            ..Default::default()
        },
    );
    assert_eq!(
        text(&registry, "PerksProgramFrozenLabel"),
        "Drag an item here to freeze it and prevent it from expiring"
    );
    assert_eq!(rect(&registry, "PerksProgramLeaveArrow")[2..], [8., 13.]);
}

#[test]
fn trading_post_retail_filter_checks_ownership_usability_and_multiple_types() {
    let mut source = snapshot();
    let mut owned = source.items[0].clone();
    owned.id = 5;
    owned.purchased = true;
    owned.collected = true;
    owned.category_id = 2;
    owned.usable = Some(false);
    let mut mount = source.items[0].clone();
    mount.id = 6;
    mount.category_id = 2;
    let mut unknown = source.items[0].clone();
    unknown.id = 7;
    unknown.usable = None;
    source.items.extend([owned, mount, unknown]);
    let mut view = View {
        visible: true,
        snapshot: Some(Arc::new(source)),
        selected: Some(4),
        ..Default::default()
    };
    assert_eq!(view.rows().len(), 4);
    view.click("perks:filter:collected").unwrap();
    assert!(!view.rows().iter().any(|row| row.id == 5));
    view.click("perks:filter:uncollected").unwrap();
    assert!(view.rows().is_empty());
    view.click("perks:filters:reset").unwrap();
    assert_eq!(view.rows().len(), 4);
    view.click("perks:filter:usable").unwrap();
    let mut ids: Vec<_> = view.rows().iter().map(|row| row.id).collect();
    ids.sort();
    assert_eq!(ids, vec![4, 6]);
    view.click("perks:type:1").unwrap();
    assert_eq!(
        view.rows().iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![6]
    );
    view.click("perks:type:2").unwrap();
    assert!(view.rows().is_empty());
    view.click("perks:filters:reset").unwrap();
    view.click("perks:filter").unwrap();
    let registry = render(ActiveSkin::Modern, view);
    assert_eq!(text(&registry, "PerksProgramFilterText"), "Filter");
    assert_eq!(rect(&registry, "PerksProgramFilter")[2..], [145., 25.]);
    for name in [
        "PerksProgramFilterCollected",
        "PerksProgramFilterUncollected",
        "PerksProgramFilterUsable",
        "PerksProgramFilterTypes",
        "PerksProgramFilterSort",
    ] {
        assert!(registry.get_by_name(name).is_some(), "{name}");
    }
    assert!(registry.get_by_name("PerksProgramCategoryAll").is_none());
}

fn freeze_view() -> View {
    let mut data = snapshot();
    let mut candidate = data.items[0].clone();
    candidate.id = 5;
    candidate.name = "Arcanofire Wrap".into();
    let mut other = data.items[0].clone();
    other.id = 6;
    other.name = "Azure Warcloak".into();
    data.items.extend([candidate, other]);
    View {
        visible: true,
        snapshot: Some(Arc::new(data)),
        selected: Some(4),
        cart: vec![5, 6],
        ..Default::default()
    }
}

#[test]
fn trading_post_freeze_replacement_previews_candidate_and_cancel_preserves_authority_and_cart() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = freeze_view();
        assert_eq!(
            view.drop_item(5, false, true),
            None,
            "replacement waits for consent"
        );
        assert!(!view.pending);
        assert_eq!(view.snapshot.as_ref().unwrap().frozen_item_id, Some(4));
        assert_eq!(view.cart, vec![5, 6]);
        let dialog = render(skin, view.clone());
        assert!(dialog.get_by_name("PerksProgramConfirmDialog").is_some());
        assert_eq!(
            text(&dialog, "PerksProgramConfirmItemText"),
            "Distinguished Warcloak"
        );
        assert_eq!(text(&dialog, "PerksProgramFrozenLabel"), "Arcanofire Wrap");
        assert_eq!(view.click("perks:cancel").unwrap(), None);
        assert_eq!(view.snapshot.as_ref().unwrap().frozen_item_id, Some(4));
        assert_eq!(view.cart, vec![5, 6]);
        let cancelled = render(skin, view);
        assert!(cancelled.get_by_name("PerksProgramConfirmDialog").is_none());
        assert_eq!(
            text(&cancelled, "PerksProgramFrozenLabel"),
            "Distinguished Warcloak"
        );
    }
}

#[test]
fn trading_post_freeze_empty_slot_is_immediate_and_removes_only_frozen_cart_member() {
    use game_engine_ui_model::trading_post::Action;
    let mut view = freeze_view();
    let mut data = view.snapshot.as_ref().unwrap().as_ref().clone();
    data.frozen_item_id = None;
    view.receive(Arc::new(data));
    assert_eq!(
        view.drop_item(5, false, true),
        Some(Action::Request(TradingPostRequest::Freeze {
            vendor_item_id: Some(5)
        }))
    );
    assert_eq!(view.cart, vec![6]);
    assert!(view.pending);
    assert!(view.confirm.is_none());
    assert_eq!(view.snapshot.as_ref().unwrap().frozen_item_id, None);
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
}

#[test]
fn trading_post_freeze_replacement_accepts_after_five_seconds_and_waits_for_authority() {
    use game_engine_ui_model::trading_post::Action;
    let mut view = freeze_view();
    assert_eq!(view.drop_item(5, false, true), None);
    view.update_freeze_confirmation(4.999);
    assert_eq!(view.click("perks:confirm").unwrap(), None);
    assert_eq!(view.cart, vec![5, 6]);
    view.update_freeze_confirmation(5.0);
    assert_eq!(
        view.click("perks:confirm").unwrap(),
        Some(Action::Request(TradingPostRequest::Freeze {
            vendor_item_id: Some(5)
        }))
    );
    assert!(view.pending);
    assert!(view.confirm.is_none());
    assert_eq!(view.cart, vec![6]);
    assert_eq!(view.snapshot.as_ref().unwrap().frozen_item_id, Some(4));
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
    assert_eq!(
        view.click("perks:confirm").unwrap(),
        None,
        "accept cannot be replayed"
    );
    let mut reply = view.snapshot.as_ref().unwrap().as_ref().clone();
    reply.frozen_item_id = Some(5);
    view.receive(Arc::new(reply));
    assert!(!view.pending);
    assert_eq!(
        text(&render(ActiveSkin::Modern, view), "PerksProgramFrozenLabel"),
        "Arcanofire Wrap"
    );
}

#[test]
fn trading_post_freeze_replacement_initial_accept_delay_disables_only_confirm() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = freeze_view();
        assert_eq!(view.drop_item(5, false, true), None);
        assert_eq!(
            view.click("perks:confirm").unwrap(),
            None,
            "early click cannot freeze"
        );
        assert_eq!(view.cart, vec![5, 6]);
        assert_eq!(view.snapshot.as_ref().unwrap().frozen_item_id, Some(4));
        let dialog = render(skin, view);
        for (name, disabled) in [
            ("PerksProgramConfirmButton", true),
            ("PerksProgramConfirmCancelButton", false),
        ] {
            let frame = dialog.get(dialog.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::Button(button)) = &frame.widget_data else {
                panic!("{name} button missing");
            };
            assert_eq!(
                button.state == ui_toolkit::widgets::button::ButtonState::Disabled,
                disabled,
                "{name}"
            );
        }
        assert_eq!(
            text(&dialog, "PerksProgramConfirmButtonDialogButtonText"),
            "5"
        );
    }
}

#[test]
fn trading_post_button_tooltips_follow_source_enabled_and_affordability_branches() {
    use game_engine_ui_model::tooltip_presentation::{TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_WHITE};
    let mut data = snapshot();
    data.tender = 99;
    let view = View {
        visible: true,
        selected: Some(4),
        cart: vec![4],
        snapshot: Some(Arc::new(data)),
        ..Default::default()
    };
    for name in [
        "PerksProgramPurchaseButton",
        "PerksProgramCartPurchaseButton",
    ] {
        let tooltip = view.button_tooltip(name, false).unwrap();
        assert_eq!(
            tooltip.content.title,
            "You don't have enough currency for this purchase"
        );
        assert_eq!(tooltip.content.title_color, TOOLTIP_DESCRIPTION_COLOR);
    }
    let mut affordable = view.clone();
    Arc::make_mut(affordable.snapshot.as_mut().unwrap()).tender = 100;
    for (name, title) in [
        ("PerksProgramPurchaseButton", "Purchase"),
        ("PerksProgramCartPurchaseButton", "Purchase Cart"),
    ] {
        let tooltip = affordable.button_tooltip(name, true).unwrap();
        assert_eq!(tooltip.content.title, title);
        assert_eq!(tooltip.content.title_color, TOOLTIP_WHITE);
        assert!(tooltip.record.is_none());
        assert!(tooltip.content.lines.is_empty());
        assert_eq!(
            affordable
                .button_tooltip(name, false)
                .unwrap()
                .content
                .title,
            "Purchasing Unavailable"
        );
    }
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 99);
    assert_eq!(view.selected, Some(4));
    assert_eq!(view.cart, vec![4]);
}

#[test]
fn trading_post_button_tooltips_keep_disabled_view_cart_but_hide_disabled_clear_and_enabled_refund()
{
    let view = View::default();
    for enabled in [true, false] {
        assert_eq!(
            view.button_tooltip("PerksProgramViewCartButton", enabled)
                .unwrap()
                .content
                .title,
            "View Cart"
        );
    }
    assert_eq!(
        view.button_tooltip("PerksProgramCartClearButton", true)
            .unwrap()
            .content
            .title,
        "Clear Cart"
    );
    assert!(
        view.button_tooltip("PerksProgramCartClearButton", false)
            .is_none()
    );
    assert_eq!(
        view.button_tooltip("PerksProgramRefundButton", false)
            .unwrap()
            .content
            .title,
        "Refund Unavailable"
    );
    assert!(
        view.button_tooltip("PerksProgramRefundButton", true)
            .is_none()
    );
    assert!(
        view.button_tooltip("PerksProgramLeaveButton", true)
            .is_none()
    );
}

#[test]
fn trading_post_cart_tooltip_resolves_regular_and_ensemble_members_in_retail_namespace() {
    use game_engine_ui_model::game_tooltip::TooltipRecord;
    let mut data = snapshot();
    data.items[0].category_id = 8;
    data.items[0].preview_item_ids = vec![25, 876];
    let mut regular = data.items[0].clone();
    regular.id = 5;
    regular.category_id = 1;
    regular.item_id = 25;
    regular.name = "Worn Shortsword".into();
    regular.quality = 1;
    regular.preview_item_ids = vec![25];
    data.items.push(regular);
    let view = View {
        visible: true,
        selected: Some(4),
        cart: vec![5, 4],
        cart_visible: true,
        snapshot: Some(Arc::new(data)),
        ..Default::default()
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for (owner, id, title) in [
            ("PerksProgramCartItem5", 25, "Worn Shortsword"),
            ("PerksProgramCartSubItem4_876", 876, "Worn Wooden Buckler"),
        ] {
            let tooltip = view.cart_item_tooltip(owner, Some(80)).unwrap().unwrap();
            assert_eq!(tooltip.record, Some(TooltipRecord::Item(id)));
            assert_eq!(tooltip.content.title, title);
            assert!(
                tooltip
                    .content
                    .lines
                    .iter()
                    .all(|line| line.money.is_none())
            );
            assert!(
                tooltip.content.lines.len() >= 2,
                "Real catalog item details remain present"
            );
        }
    }
    assert_eq!(view.cart, vec![5, 4]);
    assert_eq!(view.selected, Some(4));
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
}

#[test]
fn trading_post_cart_tooltip_rejects_headers_other_vendor_members_and_removed_owners() {
    let mut data = snapshot();
    data.items[0].category_id = 8;
    data.items[0].preview_item_ids = vec![25, 876];
    let mut view = View {
        visible: true,
        selected: Some(4),
        cart: vec![4],
        cart_visible: true,
        snapshot: Some(Arc::new(data)),
        ..Default::default()
    };
    for owner in [
        "PerksProgramCartSet4",
        "PerksProgramCartSubItem5_876",
        "PerksProgramCartSubItem4_12345",
        "PerksProgramProduct4",
    ] {
        assert!(view.cart_item_tooltip(owner, Some(80)).unwrap().is_none());
    }
    assert!(
        view.cart_item_tooltip("PerksProgramCartSubItem4_876", Some(80))
            .unwrap()
            .is_some()
    );
    view.click("perks:cart:remove:4").unwrap();
    assert!(
        view.cart_item_tooltip("PerksProgramCartSubItem4_876", Some(80))
            .unwrap()
            .is_none()
    );
}

#[test]
fn trading_post_pending_dialog_purchase_waits_for_reply_and_prunes_only_accepted_cart_item() {
    let mut data = snapshot();
    let mut other = data.items[0].clone();
    other.id = 5;
    other.item_id = 190000;
    data.items.push(other);
    let mut view = View {
        visible: true,
        selected: Some(4),
        cart: vec![4, 5],
        snapshot: Some(Arc::new(data.clone())),
        ..Default::default()
    };
    view.click("perks:purchase").unwrap();
    assert_eq!(
        view.click("perks:confirm").unwrap(),
        Some(game_engine_ui_model::trading_post::Action::Request(
            TradingPostRequest::Purchase { vendor_item_id: 4 }
        ))
    );
    assert_eq!(
        view.confirm.as_deref(),
        Some("purchase"),
        "Pending purchase keeps the confirmation owner alive"
    );
    assert!(view.pending);
    assert_eq!(view.cart, vec![5]);
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
    assert!(!view.selected_item().unwrap().purchased);
    assert_eq!(view.click("perks:confirm").unwrap(), None);
    assert_eq!(view.click("perks:cancel").unwrap(), None);
    assert_eq!(view.confirm.as_deref(), Some("purchase"));
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, view.clone());
        assert!(registry.get_by_name("PerksProgramConfirmSpinner").is_some());
        assert!(
            registry
                .get_by_name("PerksProgramConfirmDarkOverlay")
                .is_some()
        );
        for name in [
            "PerksProgramConfirmButton",
            "PerksProgramConfirmCancelButton",
        ] {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::Button(button)) = &frame.widget_data else {
                panic!("Confirmation button")
            };
            assert_eq!(
                button.state,
                ui_toolkit::widgets::button::ButtonState::Disabled
            );
        }
    }
    data.tender = 400;
    data.items[0].purchased = true;
    view.receive(Arc::new(data));
    assert!(!view.pending);
    assert!(view.confirm.is_none());
    assert!(view.selected_item().unwrap().purchased);
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 400);
    assert_eq!(view.cart, vec![5]);
}

#[test]
fn trading_post_pending_dialog_refund_keeps_owner_until_authoritative_reply() {
    let mut data = snapshot();
    data.tender = 400;
    data.items[0].purchased = true;
    data.items[0].refund_seconds = 3600;
    let mut view = View {
        visible: true,
        selected: Some(4),
        snapshot: Some(Arc::new(data.clone())),
        ..Default::default()
    };
    view.click("perks:refund").unwrap();
    assert_eq!(
        view.click("perks:confirm").unwrap(),
        Some(game_engine_ui_model::trading_post::Action::Request(
            TradingPostRequest::Refund { vendor_item_id: 4 }
        ))
    );
    assert_eq!(
        view.confirm.as_deref(),
        Some("refund"),
        "Pending refund keeps the confirmation owner alive"
    );
    assert!(view.pending);
    assert!(view.selected_item().unwrap().purchased);
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 400);
    assert_eq!(view.click("perks:cancel").unwrap(), None);
    assert_eq!(view.click("perks:confirm").unwrap(), None);
    data.tender = 500;
    data.items[0].purchased = false;
    data.items[0].refund_seconds = 0;
    view.receive(Arc::new(data));
    assert!(!view.pending);
    assert!(view.confirm.is_none());
    assert!(!view.selected_item().unwrap().purchased);
    assert_eq!(view.snapshot.as_ref().unwrap().tender, 500);
}
