use game_engine_ui_model::{
    professions_frame::{ProfessionPage, ProfessionView, professions_screen},
    professions_orders::CraftingOrders,
};
use shared::protocol::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};
fn render(view: ProfessionView, skin: ActiveSkin) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(view);
    let mut r = FrameRegistry::new(1280.0, 900.0);
    Screen::new(professions_screen).sync(&shared, &mut r);
    r
}
fn view() -> ProfessionView {
    let mut v = ProfessionView {
        page: ProfessionPage::Orders,
        orders: CraftingOrders {
            selected: Some(17),
            snapshot: Some(CraftingOrdersSnapshot {
                skill_line: 2532,
                kind: CraftingOrderKind::Public,
                mine: false,
                recipes: vec![],
                orders: vec![CraftingOrderSnapshot {
                    id: 17,
                    spell_id: 2153,
                    item_id: 2303,
                    count: 1,
                    kind: CraftingOrderKind::Public,
                    customer: "Customer".into(),
                    recipient: None,
                    crafter: None,
                    commission: 1250,
                    reagents: vec![CraftingOrderReagent {
                        item_id: 2318,
                        quantity: 4,
                        supplied: 2,
                    }],
                    expires_at: 1791763200,
                    note: "Thank you".into(),
                    status: CraftingOrderStatus::Open,
                    can_claim: true,
                    can_fulfill: false,
                    can_release: false,
                    can_cancel: false,
                }],
            }),
            ..Default::default()
        },
        items: [
            (
                2303,
                game_engine_ui_model::professions_frame::ItemDisplay {
                    name: "Handstitched Leather Pants".into(),
                    icon_fdid: 134585,
                    quality: 1,
                },
            ),
            (
                2318,
                game_engine_ui_model::professions_frame::ItemDisplay {
                    name: "Light Leather".into(),
                    icon_fdid: 134252,
                    quality: 1,
                },
            ),
        ]
        .into(),
        ..Default::default()
    };
    v.book.visible = true;
    v
}
fn frame<'a>(r: &'a FrameRegistry, n: &str) -> &'a Frame {
    r.get(r.get_by_name(n).expect(n)).unwrap()
}
fn text(r: &FrameRegistry, n: &str) -> String {
    match frame(r, n).widget_data.as_ref().unwrap() {
        WidgetData::FontString(f) => f.text.clone(),
        _ => panic!("not text"),
    }
}
#[test]
fn orders_scroll_reaches_second_page_with_forty_concrete_rows() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(ActiveSkin::Modern);
    let mut v = view();
    v.orders.selected = None;
    let template = v.orders.snapshot.as_ref().unwrap().orders[0].clone();
    v.orders.snapshot.as_mut().unwrap().orders = (1..=40)
        .map(|id| {
            let mut order = template.clone();
            order.id = id;
            order.customer = format!("Customer{id}");
            order
        })
        .collect();
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(v);
    let mut screen = Screen::new(professions_screen);
    let mut registry = FrameRegistry::new(1280.0, 900.0);
    screen.sync(&shared, &mut registry);
    assert_eq!(text(&registry, "CraftingOrder1Customer"), "Customer1");
    assert!(registry.scroll_lists.scroll_to("CraftingOrderList", 10_000));
    screen.sync(&shared, &mut registry);
    let offset = registry
        .scroll_lists
        .get("CraftingOrderList")
        .unwrap()
        .first_row;
    assert!(offset > 300);
    assert_eq!(
        frame(&registry, "CraftingOrderListChild").position.top,
        ui_toolkit::layout_values::Val::Px(-(offset as f32))
    );
    assert_eq!(text(&registry, "CraftingOrder40Customer"), "Customer40");
    let ui_toolkit::layout_values::Val::Px(row_top) =
        frame(&registry, "CraftingOrderRow40").position.top
    else {
        panic!("Expected pixel row position");
    };
    assert!(row_top - (offset as f32) < 469.0);
}

#[test]
fn orders_customer_form_offers_real_recipient_and_duration_menu_choices() {
    use game_engine_ui_model::professions_orders::OrderMenu;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.orders.customer = true;
        v.orders.creating = true;
        v.orders.recipe = Some(2153);
        v.orders.snapshot.as_mut().unwrap().recipes = vec![CraftingOrderRecipeSnapshot {
            spell_id: 2153,
            skill_line: 2532,
            item_id: 2303,
            count: 1,
            name: "Handstitched Leather Pants".into(),
            reagents: vec![CraftingOrderReagent {
                item_id: 2318,
                quantity: 4,
                supplied: 0,
            }],
        }];
        v.orders.menu = Some(OrderMenu::Recipient);
        let r = render(v.clone(), skin);
        assert_eq!(
            frame(&r, "CraftingOrderRecipientDropdown")
                .onclick
                .as_deref(),
            Some("profession:order:recipient-menu")
        );
        assert_eq!(
            frame(&r, "CraftingOrderMenuPersonal").onclick.as_deref(),
            Some("profession:order:recipient:personal")
        );
        // Form.lua:556-560 radio menu: the current Public type is ticked, entries are
        // menu rows (not push buttons) under the Retail dropdown background.
        for (key, chosen) in [("Public", true), ("Guild", false), ("Personal", false)] {
            let row = format!("CraftingOrderMenu{key}");
            assert!(matches!(&frame(&r, &row).widget_data,
                Some(WidgetData::Button(button)) if !button.use_default_skin));
            assert_eq!(frame(&r, &format!("{row}Tick")).visible, chosen, "{row}");
            assert!(frame(&r, &format!("{row}Box")).visible);
            assert_eq!(
                frame(&r, &format!("{row}RowHighlight")).draw_layer,
                ui_toolkit::strata::DrawLayer::Highlight
            );
        }
        assert!(frame(&r, "CraftingOrderMenuBackground").visible);
        // Form.xml TOPRIGHT(-10,-11); Form.lua:530 sets recipient width136.
        let recipient = frame(&r, "CraftingOrderRecipientDropdown");
        assert_eq!(
            (recipient.width.value(), recipient.height.value()),
            (136.0, 25.0)
        );
        assert_eq!(
            recipient.position.left,
            ui_toolkit::layout_values::Val::Px(674.0)
        );
        assert_eq!(
            recipient.position.top,
            ui_toolkit::layout_values::Val::Px(71.0)
        );
        assert_eq!(
            frame(&r, "CraftingOrderDuration").position.left,
            ui_toolkit::layout_values::Val::Px(620.0)
        );
        assert_eq!(
            frame(&r, "CraftingOrderDuration").position.top,
            ui_toolkit::layout_values::Val::Px(352.5)
        );
        assert_eq!(frame(&r, "CraftingOrderDuration").width.value(), 143.0);
        assert_eq!(text(&r, "CraftingOrdersBrowseTabText"), "Place Order");
        assert_eq!(frame(&r, "CraftingOrdersBrowseTab").height.value(), 32.0);
        assert_eq!(
            frame(&r, "CraftingOrdersBrowseTab").position.top,
            ui_toolkit::layout_values::Val::Px(564.0)
        );
        assert!(r.get_by_name("CraftingOrderRecipes").is_none());
        assert!(r.get_by_name("CraftingOrderGold").is_some());
        v.orders.menu = Some(OrderMenu::Duration);
        v.orders.duration_hours = 48;
        let r = render(v, skin);
        for (hours, chosen) in [(12, false), (24, false), (48, true)] {
            assert_eq!(
                frame(&r, &format!("CraftingOrderMenu{hours}Tick")).visible,
                chosen
            );
            assert_eq!(
                frame(&r, &format!("CraftingOrderMenu{hours}"))
                    .onclick
                    .as_deref(),
                Some(format!("profession:order:duration:{hours}").as_str())
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn orders_customer_labels_align_with_source_payment_and_recipient_fields() {
    use ui_toolkit::layout_values::Val;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.orders.customer = true;
        v.orders.creating = true;
        v.orders.kind = CraftingOrderKind::Personal;
        v.orders.recipe = Some(2153);
        v.orders.snapshot.as_mut().unwrap().recipes = vec![CraftingOrderRecipeSnapshot {
            spell_id: 2153,
            skill_line: 2532,
            item_id: 2303,
            count: 1,
            name: "Handstitched Leather Pants".into(),
            reagents: vec![],
        }];
        let r = render(v, skin);
        assert_eq!(text(&r, "CraftingOrderRecipientLabel"), "Order To:");
        for name in [
            "CraftingOrderCommissionLabel",
            "CraftingOrderDurationLabel",
            "CraftingOrderRecipientLabel",
        ] {
            let Some(WidgetData::FontString(label)) = &frame(&r, name).widget_data else {
                panic!("Expected label {name}");
            };
            assert_eq!(
                label.justify_h,
                ui_toolkit::widgets::font_string::JustifyH::Right,
                "{name}"
            );
        }
        let recipient_label = frame(&r, "CraftingOrderRecipientLabel");
        let Val::Px(left) = recipient_label.position.left else {
            panic!("pixel label");
        };
        assert_eq!(left + recipient_label.width.value(), 653.0);
        for name in ["CraftingOrderCommissionLabel", "CraftingOrderDurationLabel"] {
            let label = frame(&r, name);
            let Val::Px(left) = label.position.left else {
                panic!("pixel label");
            };
            assert!(left >= 525.0, "{name} outside right pane");
            assert_eq!(left + label.width.value(), 620.0);
        }
        assert_eq!(
            frame(&r, "CraftingOrderRecipientLabel").position.top,
            Val::Px(97.0)
        );
        assert_eq!(frame(&r, "CraftingOrderGold").position.top, Val::Px(316.0));
        for (label_name, input_name) in [
            ("CraftingOrderCommissionLabel", "CraftingOrderGold"),
            ("CraftingOrderDurationLabel", "CraftingOrderDuration"),
            ("CraftingOrderRecipientLabel", "CraftingOrderRecipient"),
        ] {
            let label = frame(&r, label_name);
            let input = frame(&r, input_name);
            let Val::Px(label_top) = label.position.top else {
                panic!("pixel label");
            };
            let Val::Px(input_top) = input.position.top else {
                panic!("pixel input");
            };
            assert_eq!(
                label_top + label.height.value() / 2.0,
                input_top + input.height.value() / 2.0,
                "{label_name} centerline"
            );
        }
        let portrait = frame(&r, "ProfessionsCustomerOrdersPortrait");
        assert_eq!(
            (portrait.width.value(), portrait.height.value()),
            (62.0, 62.0)
        );
        let back = frame(&r, "CraftingOrderFormBack");
        assert_eq!(back.onclick.as_deref(), Some("profession:order:browse"));
        let Some(WidgetData::Button(button)) = &back.widget_data else {
            panic!("Retail UIPanelButton");
        };
        if skin == ActiveSkin::Modern {
            assert_eq!(
                button.normal_texture,
                Some(ui_toolkit::widgets::texture::TextureSource::FileDataId(
                    130828
                ))
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn orders_native_crafter_detail_uses_server_eligibility_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let v = view();
        let r = render(v, skin);
        assert_eq!(text(&r, "CraftingOrderCustomer"), "Customer");
        assert_eq!(
            text(&r, "CraftingOrderOutputName"),
            "Handstitched Leather Pants"
        );
        assert_eq!(
            frame(&r, "CraftingOrderClaim").onclick.as_deref(),
            Some("profession:order:claim")
        );
        assert!(
            frame(&r, "CraftingOrderFulfill")
                .onclick
                .as_deref()
                .is_none_or(str::is_empty)
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}
#[test]
fn orders_native_customer_frame_is_separate_and_can_cancel_only_own_orders() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.orders.customer = true;
        v.orders.snapshot.as_mut().unwrap().mine = true;
        v.orders.snapshot.as_mut().unwrap().orders[0].can_cancel = true;
        let r = render(v, skin);
        assert!(r.get_by_name("ProfessionsCustomerOrdersFrame").is_some());
        assert!(r.get_by_name("ProfessionsFrame").is_none());
        assert_eq!(
            frame(&r, "CraftingOrderCancel").onclick.as_deref(),
            Some("profession:order:cancel")
        );
        assert!(r.get_by_name("CraftingOrderClaim").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}
#[test]
fn orders_rows_keep_auction_row_hover_without_generic_search_art() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.orders.selected = None;
        let r = render(v, skin);
        let hover = frame(&r, "CraftingOrderRow17RowHighlight");
        assert_eq!(hover.draw_layer, ui_toolkit::strata::DrawLayer::Highlight);
        assert!(
            matches!(&hover.widget_data, Some(WidgetData::Texture(t))
            if t.source == ui_toolkit::widgets::texture::TextureSource::Atlas(
                "auctionhouse-ui-row-highlight".into())),
            "CrafterOrderPage.xml:36 row highlight"
        );
        assert!(
            r.get_by_name("CraftingOrderRow17RowSelected").is_none(),
            "crafter order rows have no persistent selected texture"
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}
#[test]
fn orders_customer_committed_form_fits_its_retail_portrait_window() {
    let mut v = view();
    v.orders.customer = true;
    let registry = render(v, ActiveSkin::Modern);
    let background = frame(&registry, "ProfessionsSchematicBackground");
    let ui_toolkit::layout_values::Val::Px(left) = background.position.left else {
        panic!("Expected pixels");
    };
    let ui_toolkit::frame::Dimension::Fixed(width) = background.width else {
        panic!("Expected fixed width");
    };
    assert!(
        left + width <= 825.0,
        "Customer form background exceeds its Retail window: {}",
        left + width
    );
}

#[test]
fn recraft_native_slot_explicitly_refuses_missing_evaluator_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut v = view();
        v.page = ProfessionPage::Recraft;
        let r = render(v, skin);
        assert!(r.get_by_name("ProfessionsRecraftInputSlot").is_some());
        assert!(text(&r, "ProfessionsRecraftStatus").contains("lineage"));
        assert!(
            frame(&r, "ProfessionsRecraftSubmit")
                .onclick
                .as_deref()
                .is_none_or(str::is_empty)
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}
