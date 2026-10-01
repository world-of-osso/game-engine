//! Portable trade contract; native receiver/UI RED is driven separately by MAIN.
use game_engine_ui_model::{
    bag_data::{InventoryState, stack_slot},
    cursor_item::CursorItem,
    popup::{PopupOutcome, PopupStack},
    trade::{NativeTradeView, TRADE_POPUP, TradeRequest, TradeSession, native_trade_screen},
};
use shared::protocol::*;
use std::time::Duration;

fn stack(guid: u64, count: u32) -> ItemStack {
    ItemStack {
        item_guid: guid,
        item_id: 2589,
        count,
        durability: None,
        soulbound: false,
    }
}
fn party(name: &str) -> TradePartySnapshot {
    TradePartySnapshot {
        name: name.into(),
        accepted: false,
        gold: 12345,
        slots: vec![None; 7],
    }
}
fn snapshot(phase: TradePhase) -> TradeSnapshot {
    TradeSnapshot {
        phase,
        player: party("Alice"),
        other: party("Bob"),
    }
}
fn item(guid: u64) -> TradeItemSnapshot {
    TradeItemSnapshot {
        item_guid: guid,
        item_id: 2589,
        name: "Linen Cloth".into(),
        quality: 2,
        stack_count: 3,
    }
}
fn update(
    trade: Option<TradeSnapshot>,
    error: Option<&str>,
    message: Option<&str>,
) -> TradeStateUpdate {
    TradeStateUpdate {
        trade,
        error: error.map(str::to_owned),
        message: message.map(str::to_owned),
    }
}
fn open() -> TradeSession {
    let mut session = TradeSession::default();
    session.receive(update(Some(snapshot(TradePhase::Open)), None, None));
    session
}
fn configure_assets() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

#[test]
fn trade_refusal_preserves_snapshot_but_authoritative_end_closes_window() {
    let mut session = open();
    let before = session.snapshot.clone();
    assert!(session.window_open());
    assert_eq!(
        session.receive(update(None, Some("ERR_TRADE_TOO_FAR"), None)),
        vec!["ERR_TRADE_TOO_FAR"]
    );
    assert_eq!(session.snapshot, before);
    assert!(session.window_open());
    assert_eq!(
        session.receive(update(None, None, Some("Trade complete."))),
        vec!["Trade complete."]
    );
    assert!(session.snapshot.is_none());
    assert!(!session.window_open());
}

#[test]
fn invitation_accept_decline_timeout_are_exact_and_once_and_stale_answers_are_ignored() {
    for (outcome, expected) in [
        (PopupOutcome::Accepted, TradeRequest::Accept),
        (PopupOutcome::Cancelled, TradeRequest::Decline),
        (PopupOutcome::TimedOut, TradeRequest::Decline),
    ] {
        let mut session = TradeSession::default();
        let mut popups = PopupStack::default();
        session.receive(update(
            Some(snapshot(TradePhase::PendingIncoming)),
            None,
            None,
        ));
        session.sync_popup(&mut popups);
        session.sync_popup(&mut popups);
        let shown = popups.visible();
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].spec.key, TRADE_POPUP);
        assert_eq!(shown[0].spec.text, "Trade with Bob?");
        assert_eq!(shown[0].spec.accept_label, "Yes");
        assert_eq!(shown[0].spec.cancel_label.as_deref(), Some("No"));
        if outcome == PopupOutcome::TimedOut {
            popups.tick(Duration::from_secs(59));
            assert!(popups.drain_results().is_empty());
            popups.tick(Duration::from_secs(1));
        } else {
            popups.resolve(shown[0].id, outcome);
        }
        let results = popups.drain_results();
        assert_eq!(session.popup_results(&results), vec![expected]);
        assert!(session.popup_results(&results).is_empty());
        session.sync_popup(&mut popups);
        assert!(!popups.contains(TRADE_POPUP));
        session.receive(update(None, None, None));
        assert!(session.popup_results(&results).is_empty());
    }
}

#[test]
fn server_cancel_and_outgoing_invitation_never_leave_a_popup_or_open_window() {
    let mut session = TradeSession::default();
    let mut popups = PopupStack::default();
    session.receive(update(
        Some(snapshot(TradePhase::PendingIncoming)),
        None,
        None,
    ));
    session.sync_popup(&mut popups);
    assert!(popups.contains(TRADE_POPUP));
    session.receive(update(None, None, Some("Trade canceled.")));
    session.sync_popup(&mut popups);
    assert!(!popups.contains(TRADE_POPUP));
    session.receive(update(
        Some(snapshot(TradePhase::PendingOutgoing)),
        None,
        None,
    ));
    session.sync_popup(&mut popups);
    assert!(!popups.is_open());
    assert!(!session.window_open());
    session.reset();
    assert!(session.snapshot.is_none());
}

#[test]
fn close_sends_cancel_once_and_stays_closed_until_authoritative_end() {
    let mut session = open();
    assert_eq!(session.close(), Some(TradeRequest::Cancel));
    assert!(session.close().is_none());
    assert!(!session.window_open());
    session.receive(update(Some(snapshot(TradePhase::Open)), None, None));
    assert!(!session.window_open());
    session.receive(update(None, None, None));
    session.receive(update(Some(snapshot(TradePhase::Open)), None, None));
    assert!(session.window_open());
}

#[test]
fn offers_use_original_whole_stack_first_free_and_duplicate_guards_without_inventory_mutation() {
    let mut inventory = InventoryState::default();
    inventory.apply_snapshot(&InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items: vec![BagSlotItem {
                slot: 2,
                item: stack(81, 3),
            }],
        }],
    });
    let mut session = open();
    session.snapshot.as_mut().unwrap().player.slots[0] = Some(item(99));
    assert_eq!(
        session.offer("bag_slot:0:2", &inventory),
        Some(TradeRequest::SetItem(SetTradeItem {
            slot: 1,
            item_guid: 81,
            stack_count: 3
        }))
    );
    assert_eq!(inventory.slot(0, 2), Some(&stack_slot(&stack(81, 3))));
    assert!(session.snapshot.as_ref().unwrap().player.slots[1].is_none());
    session.snapshot.as_mut().unwrap().player.slots[1] = Some(item(81));
    assert!(session.offer("bag_slot:0:2", &inventory).is_none());
    assert!(session.offer("bag_slot:0:4", &inventory).is_none());
    for slot in &mut session.snapshot.as_mut().unwrap().player.slots[..6] {
        *slot = Some(item(99));
    }
    assert!(session.offer("bag_slot:0:2", &inventory).is_none()); // Seventh slot never auto-offered.
    session.reset();
    assert!(session.offer("bag_slot:0:2", &inventory).is_none());
}

#[test]
fn cursor_item_placed_in_any_slot_including_will_not_be_traded() {
    let mut inventory = InventoryState::default();
    inventory.apply_snapshot(&InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items: vec![BagSlotItem {
                slot: 2,
                item: stack(81, 5),
            }],
        }],
    });
    let from = ItemLocation::Bag { bag: 0, slot: 2 };
    let whole = CursorItem::split_from(&inventory, from, 5);
    let split = CursorItem::split_from(&inventory, from, 2);
    let session = open();
    assert_eq!(
        session.place(6, &whole, &inventory),
        Some(TradeRequest::SetItem(SetTradeItem {
            slot: 6,
            item_guid: 81,
            stack_count: 5
        }))
    );
    assert_eq!(
        session.place(0, &split, &inventory),
        Some(TradeRequest::SetItem(SetTradeItem {
            slot: 0,
            item_guid: 81,
            stack_count: 2
        }))
    );
    assert!(session.place(7, &whole, &inventory).is_none());
    assert!(session.place(0, &CursorItem::Empty, &inventory).is_none());
    assert!(
        TradeSession::default()
            .place(0, &whole, &inventory)
            .is_none()
    );
}

#[test]
fn cancel_withdraws_accept_close_cancels_and_money_and_clear_wait_for_authority() {
    let mut session = open();
    session.snapshot.as_mut().unwrap().player.slots[0] = Some(item(81));
    let before = session.snapshot.clone();
    assert_eq!(
        session.click("trade_player_slot:0"),
        Some(TradeRequest::ClearItem(0))
    );
    assert_eq!(session.money(20000), Some(TradeRequest::SetMoney(20000)));
    assert!(session.money(12345).is_none());
    assert_eq!(session.click("trade_accept"), Some(TradeRequest::Confirm));
    assert_eq!(session.snapshot, before);
    session.snapshot.as_mut().unwrap().player.accepted = true;
    assert!(session.click("trade_accept").is_none());
    assert_eq!(
        session.click("trade_cancel"),
        Some(TradeRequest::CancelAccept)
    );
    assert!(session.window_open());
    assert_eq!(session.click("trade_close"), Some(TradeRequest::Cancel));
    assert!(!session.window_open());
}

#[test]
fn authored_trade_view_has_seven_slots_quality_count_nontraded_labels_and_accept_highlights() {
    configure_assets();
    use ui_toolkit::{
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    let mut session = open();
    let s = session.snapshot.as_mut().unwrap();
    s.player.slots[0] = Some(item(81));
    s.player.accepted = true;
    s.other.gold = 67890;
    s.other.slots[6] = Some(item(92));
    let view = session.view();
    assert_eq!(view.player_items.len(), 7);
    assert_eq!(view.recipient_items.len(), 7);
    assert_eq!(view.player_items[0].as_ref().unwrap().item.count, 3);
    assert_eq!(
        view.player_items[0].as_ref().unwrap().name_color,
        "0.12,1.0,0.0,1.0"
    );
    assert_eq!(view.recipient_money, 67890);
    let mut shared = SharedContext::new();
    shared.insert(NativeTradeView { frame: view });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(native_trade_screen).sync(&shared, &mut registry);
    for side in ["Player", "Recipient"] {
        for slot in 1..=7 {
            assert!(
                registry
                    .get_by_name(&format!("Trade{side}Item{slot}ItemButton"))
                    .is_some()
            );
        }
        let label = registry
            .get(
                registry
                    .get_by_name(&format!("Trade{side}ItemEnchantText"))
                    .unwrap(),
            )
            .unwrap();
        let Some(WidgetData::FontString(text)) = &label.widget_data else {
            panic!("not a label")
        };
        assert_eq!(text.text, "Will not be traded");
    }
    assert!(registry.get_by_name("TradeHighlightPlayerTop").is_some());
    assert!(registry.get_by_name("TradeHighlightRecipientTop").is_none());
}
