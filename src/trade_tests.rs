use bevy::ecs::system::RunSystemOnce;
use shared::protocol::TradeItemSnapshot;

use super::*;

#[test]
fn dispatcher_reports_disconnected_action_then_leaves_idle_state_unchanged() {
    let mut app = App::new();
    app.init_resource::<game_engine::network_runtime::messages::ConnectionSender>();
    app.add_plugins(TradePlugin);
    let (reply, responses) = mpsc::channel();
    {
        let mut state = app.world_mut().resource_mut::<TradeClientState>();
        assert!(queue_ipc_request(&mut state, &Request::TradeCancel, reply));
    }
    crate::network_events::dispatch_outgoing(app.world_mut());
    let Response::Error(error) = responses.try_recv().unwrap() else {
        panic!("expected disconnected trade error");
    };
    assert_eq!(error, "trade is unavailable: not connected");
    assert!(
        app.world()
            .resource::<TradeClientState>()
            .pending_actions
            .is_empty()
    );
}

fn item(item_guid: u64, item_id: u32) -> Option<TradeItemSnapshot> {
    Some(TradeItemSnapshot {
        item_guid,
        item_id,
        name: "Linen Cloth".into(),
        quality: 1,
        stack_count: 20,
    })
}

fn open_snapshot() -> TradeSnapshot {
    let mut slots = vec![None; 7];
    slots[0] = item(7, 2589);
    slots[1] = item(8, 2589);
    slots[6] = item(9, 2770);
    TradeSnapshot {
        phase: TradePhase::Open,
        player: TradePartySnapshot {
            name: "Tradea".into(),
            accepted: true,
            gold: 15_000,
            slots,
        },
        other: TradePartySnapshot {
            name: "Tradeb".into(),
            accepted: false,
            gold: 250,
            slots: vec![None; 7],
        },
    }
}

#[test]
fn status_names_the_phase_and_both_offers() {
    let mut state = TradeClientState::default();
    let (tx, rx) = mpsc::channel();
    assert!(queue_ipc_request(&mut state, &Request::TradeStatus, tx));
    assert!(matches!(rx.recv().unwrap(), Response::Text(text) if text == "trade: inactive"));

    apply_trade_update(
        &mut state,
        TradeStateUpdate {
            trade: Some(open_snapshot()),
            message: None,
            error: None,
        },
    );

    let status = format_status(&state);
    assert!(status.contains("trade: open"));
    assert!(
        status.contains("you: Tradea copper=15000 accepted=yes items=slot0=Linen Cloth (2589) x20")
    );
    assert!(status.contains("other: Tradeb copper=250 accepted=no items=none"));
}

#[test]
fn an_error_keeps_the_trade_and_an_update_without_snapshot_ends_it() {
    let mut state = TradeClientState::default();
    apply_trade_update(
        &mut state,
        TradeStateUpdate {
            trade: Some(open_snapshot()),
            message: None,
            error: None,
        },
    );
    apply_trade_update(
        &mut state,
        TradeStateUpdate {
            trade: None,
            message: None,
            error: Some("You can't trade a soulbound item.".into()),
        },
    );
    assert!(state.is_open());

    apply_trade_update(
        &mut state,
        TradeStateUpdate {
            trade: None,
            message: Some("Trade complete.".into()),
            error: None,
        },
    );
    assert_eq!(state.snapshot, None);
    assert_eq!(state.last_message.as_deref(), Some("Trade complete."));
}

#[test]
fn the_first_free_slot_skips_offered_ones_and_never_picks_the_seventh() {
    let mut state = TradeClientState {
        snapshot: Some(open_snapshot()),
        ..Default::default()
    };
    assert_eq!(state.first_free_slot(), Some(2));
    assert!(state.offers(8) && !state.offers(10));
    let player = &mut state.snapshot.as_mut().unwrap().player;
    for slot in 0..6 {
        player.slots[slot] = item(20 + slot as u64, 2589);
    }
    assert_eq!(state.first_free_slot(), None);
}

#[test]
fn updates_show_their_text_in_the_error_frame() {
    let mut app = App::new();
    app.add_plugins(TradePlugin);
    app.insert_resource(game_engine::network_runtime::messages::Inbox::new(vec![
        TradeStateUpdate {
            trade: None,
            message: Some("Trade complete.".into()),
            error: None,
        },
    ]));
    app.world_mut()
        .run_system_once(receive_trade_updates)
        .unwrap();
    assert_eq!(
        app.world().resource::<UiErrors>().lines[0].text,
        "Trade complete."
    );
}
