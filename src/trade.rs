use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AcceptTrade, CancelTrade, ClearTradeItem, ConfirmTrade, DeclineTrade, InitiateTrade,
    SetTradeItem, SetTradeMoney, TradeChannel, TradePhase, TradeSnapshot, TradeStateUpdate,
};

use crate::ipc::{Request, Response};

#[derive(Resource, Default)]
pub struct TradeClientState {
    pub phase: Option<TradePhase>,
    pub trade: game_engine::trade_data::TradeState,
    pub last_error: Option<String>,
    pub last_message: Option<String>,
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

#[derive(Clone)]
enum Action {
    Initiate(InitiateTrade),
    Accept,
    Decline,
    Cancel,
    SetItem(SetTradeItem),
    ClearItem(ClearTradeItem),
    SetMoney(SetTradeMoney),
    Confirm,
}

pub struct TradePlugin;

impl Plugin for TradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TradeClientState>();
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_trade_updates);
    }
}

pub fn queue_ipc_request(
    state: &mut TradeClientState,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    if matches!(request, Request::TradeStatus) {
        let _ = respond.send(Response::Text(format_status(state)));
        return true;
    }
    let Some(action) = map_action(request) else {
        return false;
    };
    state.pending_actions.push_back(action);
    state.pending_replies.push_back(respond);
    true
}

fn map_action(request: &Request) -> Option<Action> {
    match request {
        Request::TradeInitiate { name } => Some(Action::Initiate(InitiateTrade {
            target_name: name.clone(),
        })),
        Request::TradeAccept => Some(Action::Accept),
        Request::TradeDecline => Some(Action::Decline),
        Request::TradeCancel => Some(Action::Cancel),
        Request::TradeSetItem {
            slot,
            item_guid,
            stack_count,
        } => Some(Action::SetItem(SetTradeItem {
            slot: *slot,
            item_guid: *item_guid,
            stack_count: *stack_count,
        })),
        Request::TradeClearItem { slot } => Some(Action::ClearItem(ClearTradeItem { slot: *slot })),
        Request::TradeSetMoney { copper } => {
            Some(Action::SetMoney(SetTradeMoney { copper: *copper }))
        }
        Request::TradeConfirm => Some(Action::Confirm),
        _ => None,
    }
}

#[derive(SystemParam)]
struct TradeSenders<'w, 's> {
    initiate: Query<'w, 's, &'static mut MessageSender<InitiateTrade>>,
    accept: Query<'w, 's, &'static mut MessageSender<AcceptTrade>>,
    decline: Query<'w, 's, &'static mut MessageSender<DeclineTrade>>,
    cancel: Query<'w, 's, &'static mut MessageSender<CancelTrade>>,
    set_item: Query<'w, 's, &'static mut MessageSender<SetTradeItem>>,
    clear_item: Query<'w, 's, &'static mut MessageSender<ClearTradeItem>>,
    set_money: Query<'w, 's, &'static mut MessageSender<SetTradeMoney>>,
    confirm: Query<'w, 's, &'static mut MessageSender<ConfirmTrade>>,
}

fn send_pending_actions(mut state: ResMut<TradeClientState>, mut senders: TradeSenders) {
    while let Some(action) = state.pending_actions.pop_front() {
        let sent = match action {
            Action::Initiate(message) => send_all(&mut senders.initiate, message),
            Action::Accept => send_all(&mut senders.accept, AcceptTrade),
            Action::Decline => send_all(&mut senders.decline, DeclineTrade),
            Action::Cancel => send_all(&mut senders.cancel, CancelTrade),
            Action::SetItem(message) => send_all(&mut senders.set_item, message),
            Action::ClearItem(message) => send_all(&mut senders.clear_item, message),
            Action::SetMoney(message) => send_all(&mut senders.set_money, message),
            Action::Confirm => send_all(&mut senders.confirm, ConfirmTrade),
        };
        if !sent {
            state.last_error = Some("trade is unavailable: not connected".into());
            if let Some(reply) = state.pending_replies.pop_front() {
                let _ = reply.send(Response::Error(
                    "trade is unavailable: not connected".into(),
                ));
            }
        }
    }
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<TradeChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_trade_updates(
    mut receivers: Query<&mut MessageReceiver<TradeStateUpdate>>,
    mut state: ResMut<TradeClientState>,
) {
    for mut receiver in &mut receivers {
        for update in receiver.receive() {
            apply_trade_update(&mut state, update);
        }
    }
}

fn apply_trade_update(state: &mut TradeClientState, update: TradeStateUpdate) {
    state.last_error = update.error.clone();
    state.last_message = update.message.clone();
    if let Some(snapshot) = update.trade {
        state.phase = Some(snapshot.phase);
        state.trade = map_trade_snapshot(&snapshot);
    } else if update.error.is_none() {
        state.phase = None;
        state.trade = game_engine::trade_data::TradeState::default();
    }
    if let Some(reply) = state.pending_replies.pop_front() {
        let response = if let Some(error) = update.error {
            Response::Error(error)
        } else if let Some(message) = update.message {
            if state.trade.active {
                Response::Text(format!("{message}\n{}", format_status(state)))
            } else {
                Response::Text(message)
            }
        } else {
            Response::Text(format_status(state))
        };
        let _ = reply.send(response);
    }
}

fn map_trade_snapshot(snapshot: &TradeSnapshot) -> game_engine::trade_data::TradeState {
    let mut state = game_engine::trade_data::TradeState {
        active: true,
        player: map_trade_party(&snapshot.player),
        other: map_trade_party(&snapshot.other),
    };
    state.player.slots[6] = None;
    state.other.slots[6] = None;
    state
}

fn map_trade_party(
    party: &shared::protocol::TradePartySnapshot,
) -> game_engine::trade_data::TradePlayerData {
    let mut slots: [Option<game_engine::trade_data::TradeSlot>; 7] = Default::default();
    for (index, slot) in party.slots.iter().take(6).enumerate() {
        slots[index] = slot
            .as_ref()
            .map(|item| game_engine::trade_data::TradeSlot {
                item_name: item.name.clone(),
                icon_fdid: 0,
                quantity: item.stack_count,
                item_quality: map_quality(item.quality),
            });
    }
    game_engine::trade_data::TradePlayerData {
        name: party.name.clone(),
        slots,
        money: game_engine::trade_data::Money { copper: party.gold },
        accept: if party.accepted {
            game_engine::trade_data::AcceptState::Accepted
        } else {
            game_engine::trade_data::AcceptState::Pending
        },
    }
}

fn map_quality(quality: u8) -> game_engine::trade_data::ItemQuality {
    match quality {
        0 => game_engine::trade_data::ItemQuality::Poor,
        1 => game_engine::trade_data::ItemQuality::Common,
        2 => game_engine::trade_data::ItemQuality::Uncommon,
        3 => game_engine::trade_data::ItemQuality::Rare,
        4 => game_engine::trade_data::ItemQuality::Epic,
        5 => game_engine::trade_data::ItemQuality::Legendary,
        _ => game_engine::trade_data::ItemQuality::Common,
    }
}

fn format_status(state: &TradeClientState) -> String {
    let phase = match state.phase {
        None => "inactive",
        Some(TradePhase::PendingOutgoing) => "pending-outgoing",
        Some(TradePhase::PendingIncoming) => "pending-incoming",
        Some(TradePhase::Open) => "open",
    };
    let mut lines = vec![format!("trade: {phase}")];
    if let Some(message) = &state.last_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &state.last_error {
        lines.push(format!("error: {error}"));
    }
    if state.trade.active {
        lines.push(format_party("you", &state.trade.player));
        lines.push(format_party("other", &state.trade.other));
    }
    lines.join("\n")
}

fn format_party(label: &str, party: &game_engine::trade_data::TradePlayerData) -> String {
    let items = party
        .slots
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| {
            slot.as_ref()
                .map(|item| format!("slot{index}={} x{}", item.item_name, item.quantity))
        })
        .collect::<Vec<_>>();
    let items = if items.is_empty() {
        "none".into()
    } else {
        items.join(", ")
    };
    let accepted = if party.is_accepted() { "yes" } else { "no" };
    format!(
        "{label}: {} gold={} accepted={} items={}",
        party.name,
        party.money.display(),
        accepted,
        items
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot(phase: TradePhase) -> TradeSnapshot {
        TradeSnapshot {
            phase,
            player: shared::protocol::TradePartySnapshot {
                name: "Theron".into(),
                accepted: true,
                gold: 15_000,
                slots: vec![Some(shared::protocol::TradeItemSnapshot {
                    item_guid: 7,
                    item_id: 11,
                    name: "Bronze Sword".into(),
                    quality: 2,
                    stack_count: 1,
                })],
            },
            other: shared::protocol::TradePartySnapshot {
                name: "Alice".into(),
                accepted: false,
                gold: 250,
                slots: vec![None],
            },
        }
    }

    #[test]
    fn queue_trade_status_request_returns_formatted_status() {
        let mut state = TradeClientState::default();
        let (tx, rx) = mpsc::channel();

        let handled = queue_ipc_request(&mut state, &Request::TradeStatus, tx);

        assert!(handled);
        let response = rx.recv().expect("response");
        match response {
            Response::Text(text) => assert!(text.contains("trade: inactive")),
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn apply_trade_update_maps_runtime_state() {
        let mut state = TradeClientState::default();

        apply_trade_update(
            &mut state,
            TradeStateUpdate {
                trade: Some(sample_snapshot(TradePhase::Open)),
                message: Some("trade opened".into()),
                error: None,
            },
        );

        assert_eq!(state.phase, Some(TradePhase::Open));
        assert!(state.trade.active);
        assert_eq!(state.trade.player.name, "Theron");
        assert_eq!(state.trade.other.name, "Alice");
        assert_eq!(
            state.trade.player.slots[0]
                .as_ref()
                .expect("slot")
                .item_name,
            "Bronze Sword"
        );
        assert_eq!(state.trade.player.money.copper, 15_000);
        assert!(state.trade.player.is_accepted());
        assert!(!state.trade.other.is_accepted());
    }

    #[test]
    fn closing_trade_resets_runtime_state() {
        let mut state = TradeClientState {
            phase: Some(TradePhase::Open),
            trade: map_trade_snapshot(&sample_snapshot(TradePhase::Open)),
            ..Default::default()
        };

        apply_trade_update(
            &mut state,
            TradeStateUpdate {
                trade: None,
                message: Some("trade completed".into()),
                error: None,
            },
        );

        assert_eq!(state.phase, None);
        assert!(!state.trade.active);
        assert_eq!(state.last_message.as_deref(), Some("trade completed"));
    }

    #[test]
    fn trade_error_does_not_clear_existing_trade_state() {
        let mut state = TradeClientState {
            phase: Some(TradePhase::Open),
            trade: map_trade_snapshot(&sample_snapshot(TradePhase::Open)),
            ..Default::default()
        };

        apply_trade_update(
            &mut state,
            TradeStateUpdate {
                trade: None,
                message: None,
                error: Some("not enough gold".into()),
            },
        );

        assert_eq!(state.phase, Some(TradePhase::Open));
        assert!(state.trade.active);
        assert_eq!(state.last_error.as_deref(), Some("not enough gold"));
    }
}
