//! Trade networking: `TradeStateUpdate` keeps the server's [`TradeSnapshot`] (the
//! viewer's side as `player`), its message and error go to the error frame, and
//! [`TradeAction`]s from the trade frame, the unit menus, the bags and IPC go out
//! on `TradeChannel`. IPC actions get a reply from the next update.

use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::ui::ui_errors::UiErrors;
use shared::protocol::{
    AcceptTrade, CancelTrade, CancelTradeAccept, ClearTradeItem, ConfirmTrade, DeclineTrade,
    InitiateTrade, SetTradeItem, SetTradeMoney, TradeChannel, TradePartySnapshot, TradePhase,
    TradeSnapshot, TradeStateUpdate,
};

use crate::ipc::{Request, Response};
use crate::network_events::{register_message_handler, register_outgoing_handler};

/// A player trade action (Retail `InitiateTrade`, `AcceptTrade`, `ClickTradeButton` …).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TradeAction {
    Initiate(String),
    /// Accept the incoming request (`BeginTrade`).
    Accept,
    /// Decline the incoming request.
    Decline,
    /// `CancelTrade` / `CloseTrade`.
    Cancel,
    SetItem(SetTradeItem),
    ClearItem(u8),
    SetMoney(u64),
    /// The Trade button (`AcceptTrade`).
    Confirm,
    /// `CancelTradeAccept`.
    CancelAccept,
}

#[derive(Resource, Default)]
pub struct TradeClientState {
    /// The open or pending trade; `None` when there is none.
    pub snapshot: Option<TradeSnapshot>,
    pub last_error: Option<String>,
    pub last_message: Option<String>,
    pending_actions: VecDeque<TradeAction>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

impl TradeClientState {
    /// Queue an action from the UI; its update only changes this state.
    pub fn queue(&mut self, action: TradeAction) {
        self.pending_actions.push_back(action);
    }

    /// Queued actions, in order (tests read what the UI asked for).
    pub fn take_queued(&mut self) -> Vec<TradeAction> {
        self.pending_actions.drain(..).collect()
    }

    pub fn phase(&self) -> Option<TradePhase> {
        self.snapshot.as_ref().map(|snapshot| snapshot.phase)
    }

    pub fn is_open(&self) -> bool {
        self.phase() == Some(TradePhase::Open)
    }

    /// The first empty traded slot of the player's side (`ClickTradeButton` on the
    /// first free button when an item is right-clicked in the bags).
    pub fn first_free_slot(&self) -> Option<u8> {
        let player = &self.snapshot.as_ref()?.player;
        (0..shared::trade::TRADE_SLOT_TRADED_COUNT)
            .find(|&slot| player.slots.get(slot).is_none_or(Option::is_none))
            .map(|slot| slot as u8)
    }

    /// Whether a bag item is already offered by the player.
    pub fn offers(&self, item_guid: u64) -> bool {
        self.snapshot.as_ref().is_some_and(|snapshot| {
            snapshot
                .player
                .slots
                .iter()
                .flatten()
                .any(|item| item.item_guid == item_guid)
        })
    }
}

pub struct TradePlugin;

impl Plugin for TradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TradeClientState>()
            .init_resource::<UiErrors>();
        register_outgoing_handler(app, send_pending_actions, |world| {
            !world
                .resource::<TradeClientState>()
                .pending_actions
                .is_empty()
        });
        register_message_handler::<TradeStateUpdate, _>(app, receive_trade_updates, |_| true);
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

fn map_action(request: &Request) -> Option<TradeAction> {
    Some(match request {
        Request::TradeInitiate { name } => TradeAction::Initiate(name.clone()),
        Request::TradeAccept => TradeAction::Accept,
        Request::TradeDecline => TradeAction::Decline,
        Request::TradeCancel => TradeAction::Cancel,
        Request::TradeSetItem {
            slot,
            item_guid,
            stack_count,
        } => TradeAction::SetItem(SetTradeItem {
            slot: *slot,
            item_guid: *item_guid,
            stack_count: *stack_count,
        }),
        Request::TradeClearItem { slot } => TradeAction::ClearItem(*slot),
        Request::TradeSetMoney { copper } => TradeAction::SetMoney(*copper),
        Request::TradeConfirm => TradeAction::Confirm,
        Request::TradeCancelAccept => TradeAction::CancelAccept,
        _ => return None,
    })
}

#[derive(SystemParam)]
struct TradeSenders<'w, 's> {
    initiate: MessageSenders<'w, 's, InitiateTrade>,
    accept: MessageSenders<'w, 's, AcceptTrade>,
    decline: MessageSenders<'w, 's, DeclineTrade>,
    cancel: MessageSenders<'w, 's, CancelTrade>,
    set_item: MessageSenders<'w, 's, SetTradeItem>,
    clear_item: MessageSenders<'w, 's, ClearTradeItem>,
    set_money: MessageSenders<'w, 's, SetTradeMoney>,
    confirm: MessageSenders<'w, 's, ConfirmTrade>,
    cancel_accept: MessageSenders<'w, 's, CancelTradeAccept>,
}

fn send_pending_actions(mut state: ResMut<TradeClientState>, mut senders: TradeSenders) {
    while let Some(action) = state.pending_actions.pop_front() {
        let sent = match action {
            TradeAction::Initiate(target_name) => {
                send_all(&mut senders.initiate, InitiateTrade { target_name })
            }
            TradeAction::Accept => send_all(&mut senders.accept, AcceptTrade),
            TradeAction::Decline => send_all(&mut senders.decline, DeclineTrade),
            TradeAction::Cancel => send_all(&mut senders.cancel, CancelTrade),
            TradeAction::SetItem(message) => send_all(&mut senders.set_item, message),
            TradeAction::ClearItem(slot) => {
                send_all(&mut senders.clear_item, ClearTradeItem { slot })
            }
            TradeAction::SetMoney(copper) => {
                send_all(&mut senders.set_money, SetTradeMoney { copper })
            }
            TradeAction::Confirm => send_all(&mut senders.confirm, ConfirmTrade),
            TradeAction::CancelAccept => send_all(&mut senders.cancel_accept, CancelTradeAccept),
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
    senders: &mut MessageSenders<T>,
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
    mut receivers: MessageReceivers<TradeStateUpdate>,
    mut state: ResMut<TradeClientState>,
    mut errors: ResMut<UiErrors>,
) {
    for receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            for text in update.error.iter().chain(&update.message) {
                errors.add(text.clone());
            }
            apply_trade_update(&mut state, update);
        }
    }
}

/// A snapshot replaces the trade; an update without one ends it unless it only
/// carries an error (a refused request leaves the trade as it was).
fn apply_trade_update(state: &mut TradeClientState, update: TradeStateUpdate) {
    state.last_error = update.error.clone();
    state.last_message = update.message.clone();
    if let Some(snapshot) = update.trade {
        state.snapshot = Some(snapshot);
    } else if update.error.is_none() {
        state.snapshot = None;
    }
    if let Some(reply) = state.pending_replies.pop_front() {
        let response = match (update.error, update.message) {
            (Some(error), _) => Response::Error(error),
            (None, Some(message)) if state.snapshot.is_some() => {
                Response::Text(format!("{message}\n{}", format_status(state)))
            }
            (None, Some(message)) => Response::Text(message),
            (None, None) => Response::Text(format_status(state)),
        };
        let _ = reply.send(response);
    }
}

fn format_status(state: &TradeClientState) -> String {
    let phase = match state.phase() {
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
    if let Some(snapshot) = &state.snapshot {
        lines.push(format_party("you", &snapshot.player));
        lines.push(format_party("other", &snapshot.other));
    }
    lines.join("\n")
}

fn format_party(label: &str, party: &TradePartySnapshot) -> String {
    let items = party
        .slots
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| {
            slot.as_ref().map(|item| {
                format!(
                    "slot{index}={} ({}) x{}",
                    item.name, item.item_id, item.stack_count
                )
            })
        })
        .collect::<Vec<_>>();
    let items = if items.is_empty() {
        "none".into()
    } else {
        items.join(", ")
    };
    let accepted = if party.accepted { "yes" } else { "no" };
    format!(
        "{label}: {} copper={} accepted={} items={}",
        party.name, party.gold, accepted, items
    )
}

#[cfg(test)]
#[path = "trade_tests.rs"]
mod tests;
