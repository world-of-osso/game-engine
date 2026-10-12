use super::*;
use crate::bag_data::InventorySlot;
fn recipe() -> CraftingOrderRecipeSnapshot {
    CraftingOrderRecipeSnapshot {
        spell_id: 2153,
        skill_line: 2532,
        item_id: 2303,
        count: 1,
        name: "Handstitched Leather Pants".into(),
        reagents: vec![
            CraftingOrderReagent {
                item_id: 2318,
                quantity: 4,
                supplied: 0,
            },
            CraftingOrderReagent {
                item_id: 2320,
                quantity: 1,
                supplied: 0,
            },
        ],
    }
}
fn bags() -> InventoryState {
    InventoryState {
        slots: vec![vec![
            InventorySlot {
                item_id: 2318,
                count: 4,
                ..Default::default()
            },
            InventorySlot {
                item_id: 2320,
                count: 1,
                ..Default::default()
            },
        ]],
        ..Default::default()
    }
}
#[test]
fn orders_customer_service_opens_scoped_query_and_closes_only_for_its_npc() {
    let mut orders = CraftingOrders::default();
    assert_eq!(
        orders.open_customer(215258),
        CraftingOrderRequest::Query {
            skill_line: 0,
            kind: CraftingOrderKind::Public,
            mine: true
        }
    );
    assert_eq!(orders.npc, Some(215258));
    assert!(orders.customer);
    assert!(orders.creating);
    assert!(orders.pending);
    assert!(!orders.close_for(219043));
    assert_eq!(orders.npc, Some(215258));
    assert!(orders.close_for(215258));
    assert_eq!(orders.npc, None);
    assert!(!orders.customer);
    assert!(!orders.pending);
    orders.receive(CraftingOrderStateUpdate {
        snapshot: Some(CraftingOrdersSnapshot {
            skill_line: 0,
            kind: CraftingOrderKind::Public,
            mine: true,
            recipes: vec![],
            orders: vec![],
        }),
        message: None,
        error: None,
    });
    assert_eq!(orders.npc, None);
    assert!(!orders.customer);
}

#[test]
fn order_customer_places_exact_authored_reagents_without_crafting_or_mutating_bags() {
    let inventory = bags();
    let original = inventory.clone();
    let mut form = CraftingOrders {
        customer: true,
        creating: true,
        recipe: Some(2153),
        kind: CraftingOrderKind::Personal,
        recipient: "fb_crafter".into(),
        commission: "1250".into(),
        supplied: [(2318, 4), (2320, 1)].into(),
        note: "Thank you".into(),
        ..Default::default()
    };
    assert_eq!(
        form.place_request(&recipe(), &inventory).unwrap(),
        CraftingOrderRequest::Place {
            spell_id: 2153,
            kind: CraftingOrderKind::Personal,
            recipient: Some("fb_crafter".into()),
            commission: 1250,
            reagents: vec![
                CraftingOrderReagent {
                    item_id: 2318,
                    quantity: 4,
                    supplied: 4
                },
                CraftingOrderReagent {
                    item_id: 2320,
                    quantity: 1,
                    supplied: 1
                }
            ],
            duration_hours: 24,
            note: "Thank you".into()
        }
    );
    assert_eq!(inventory, original);
    assert!(form.pending);
    assert!(form.place_request(&recipe(), &inventory).is_err());
}
#[test]
fn order_customer_rejects_invalid_recipient_commission_and_supplied_counts() {
    let mut form = CraftingOrders {
        customer: true,
        recipe: Some(2153),
        kind: CraftingOrderKind::Personal,
        ..Default::default()
    };
    assert!(form.place_request(&recipe(), &bags()).is_err());
    form.recipient = "fb_crafter".into();
    form.commission = "NaN".into();
    assert!(form.place_request(&recipe(), &bags()).is_err());
    form.commission = "100".into();
    form.supplied.insert(2318, 5);
    assert!(form.place_request(&recipe(), &bags()).is_err());
    assert!(!form.pending);
}
#[test]
fn order_crafter_actions_require_server_eligibility_and_exact_selected_order() {
    let mut view = CraftingOrders {
        selected: Some(17),
        snapshot: Some(CraftingOrdersSnapshot {
            skill_line: 2532,
            kind: CraftingOrderKind::Public,
            mine: false,
            recipes: vec![recipe()],
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
                reagents: vec![],
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
    };
    assert!(view.action_request("fulfill").is_none());
    assert!(view.action_request("cancel").is_none());
    assert_eq!(
        view.action_request("claim"),
        Some(CraftingOrderRequest::Claim { order_id: 17 })
    );
    assert!(view.pending);
    assert!(view.action_request("claim").is_none());
    assert_eq!(
        view.selected_order().unwrap().status,
        CraftingOrderStatus::Open
    );
}
