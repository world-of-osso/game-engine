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
