use bevy::ecs::system::RunSystemOnce;
use shared::protocol::TradeSnapshot;

use super::*;

fn party(name: &str, accepted: bool) -> TradePartySnapshot {
    TradePartySnapshot {
        name: name.into(),
        accepted,
        gold: 0,
        slots: vec![None; 7],
    }
}

fn trade(phase: TradePhase, player_accepted: bool) -> TradeClientState {
    let mut player = party("Tradea", player_accepted);
    player.slots[0] = Some(TradeItemSnapshot {
        item_guid: 7,
        item_id: 2589,
        name: "Linen Cloth".into(),
        quality: 2,
        stack_count: 20,
    });
    let mut other = party("Tradeb", false);
    other.gold = 900;
    let mut state = TradeClientState::default();
    state.snapshot = Some(TradeSnapshot {
        phase,
        player,
        other,
    });
    state
}

#[test]
fn the_frame_shows_both_sides_of_an_open_trade_only() {
    let state = trade_frame_state(&trade(TradePhase::Open, false), true);
    assert!(state.visible);
    assert_eq!(state.recipient_name, "Tradeb");
    assert_eq!(state.recipient_money, 900);
    let linen = state.player_items[0].as_ref().unwrap();
    assert_eq!((linen.name.as_str(), linen.item.count), ("Linen Cloth", 20));
    assert_eq!(linen.name_color, "0.12,1.0,0.0,1.0");
    assert_eq!(state.player_items.len(), 7);
    assert_eq!(
        trade_frame_state(&trade(TradePhase::PendingIncoming, false), true),
        TradeFrameState::default()
    );
}

#[test]
fn cancel_withdraws_an_accept_first_and_offered_items_click_back() {
    let open = trade(TradePhase::Open, false);
    let accepted = trade(TradePhase::Open, true);
    assert_eq!(trade_click(ACTION_TRADE, &open), Some(TradeAction::Confirm));
    assert_eq!(trade_click(ACTION_CANCEL, &open), Some(TradeAction::Cancel));
    assert_eq!(
        trade_click(ACTION_CANCEL, &accepted),
        Some(TradeAction::CancelAccept)
    );
    assert_eq!(
        trade_click(ACTION_CLOSE, &accepted),
        Some(TradeAction::Cancel)
    );
    assert_eq!(
        trade_click("trade_player_slot:0", &open),
        Some(TradeAction::ClearItem(0))
    );
    assert_eq!(trade_click("trade_player_slot:3", &open), None);
    assert_eq!(trade_click("bank_close", &open), None);
}

#[test]
fn closing_the_window_cancels_once_and_the_ended_trade_closes_it() {
    let mut manager = WindowManager::default();
    let (window, action) = step_trade_window(TradeWindow::Closed, true, false, &mut manager);
    assert_eq!((window, action), (TradeWindow::Open, None));
    assert!(manager.is_open(WindowId::Trade) && manager.is_open(WindowId::Bag(0)));

    manager.close(WindowId::Trade);
    let (window, action) = step_trade_window(window, true, false, &mut manager);
    assert_eq!(
        (window, action),
        (TradeWindow::Cancelling, Some(TradeAction::Cancel))
    );
    // The server has not answered yet: the window stays closed, no second cancel.
    let (window, action) = step_trade_window(window, true, false, &mut manager);
    assert_eq!((window, action), (TradeWindow::Cancelling, None));
    assert!(!manager.is_open(WindowId::Trade));

    let (window, _) = step_trade_window(window, false, false, &mut manager);
    assert_eq!(window, TradeWindow::Closed);
    let (window, _) = step_trade_window(window, true, false, &mut manager);
    let (window, _) = step_trade_window(window, false, true, &mut manager);
    assert_eq!(window, TradeWindow::Closed);
    assert!(!manager.is_open(WindowId::Trade) && !manager.is_open(WindowId::Bag(0)));
}

#[test]
fn an_incoming_request_asks_trade_with_and_yes_accepts() {
    let mut app = App::new();
    app.init_resource::<PopupStack>()
        .add_message::<PopupResult>()
        .insert_resource(trade(TradePhase::PendingIncoming, false));
    app.world_mut().run_system_once(ask_trade_requests).unwrap();
    let shown = app.world().resource::<PopupStack>().visible();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].spec.text, "Trade with Tradeb?");
    assert_eq!(shown[0].spec.accept_label, "Yes");

    app.world_mut().resource_mut::<PopupStack>().accept_top();
    let results = app.world_mut().resource_mut::<PopupStack>().drain_results();
    for result in results {
        app.world_mut().write_message(result);
    }
    app.world_mut().run_system_once(answer_trade_popup).unwrap();

    let queued = app
        .world_mut()
        .resource_mut::<TradeClientState>()
        .take_queued();
    assert_eq!(queued, vec![TradeAction::Accept]);
}
