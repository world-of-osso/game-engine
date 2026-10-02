//! `trade status` and the trade actions in the original response text, re-stated
//! because the originals read the Bevy `TradeClientState` resource: src/trade.rs:108
//! `queue_ipc_request`, :125 `map_action`, :220 `apply_trade_update`, :241
//! `format_status`, :262 `format_party`. An action is answered by the server's next
//! `TradeStateUpdate`.
use game_engine_network::ipc_wire::{Request, Response};
use game_engine_ui_model::trade::TradeRequest;
use shared::protocol::{SetTradeItem, TradePartySnapshot, TradePhase};

use super::Reply;

const NOT_CONNECTED: &str = "trade is unavailable: not connected";

impl crate::GameClient {
    /// A trade request, answered now (`trade status`, a send failure) or queued for the
    /// server's next update; other requests come back.
    pub(crate) fn trade_request(
        &mut self,
        request: Request,
        reply: Reply,
    ) -> Result<(), (Request, Reply)> {
        if matches!(request, Request::TradeStatus) {
            reply.send(Response::Text(self.trade_status()));
            return Ok(());
        }
        let Some(action) = trade_action(&request) else {
            return Err((request, reply));
        };
        let connected = self
            .account
            .link
            .as_ref()
            .is_some_and(|link| link.connected);
        if !connected || self.account.send_trade(action).is_err() {
            self.trade.last_error = Some(NOT_CONNECTED.into());
            reply.send(Response::Error(NOT_CONNECTED.into()));
            return Ok(());
        }
        self.trade.ipc_replies.push_back(reply);
        Ok(())
    }

    /// The oldest waiting action's answer once `receive_trade` applied an update.
    pub(crate) fn answer_trade_ipc(&mut self, error: Option<String>, message: Option<String>) {
        let Some(reply) = self.trade.ipc_replies.pop_front() else {
            return;
        };
        let response = match (error, message) {
            (Some(error), _) => Response::Error(error),
            (None, Some(message)) if self.trade.session.snapshot.is_some() => {
                Response::Text(format!("{message}\n{}", self.trade_status()))
            }
            (None, Some(message)) => Response::Text(message),
            (None, None) => Response::Text(self.trade_status()),
        };
        reply.send(response);
    }

    fn trade_status(&self) -> String {
        let snapshot = self.trade.session.snapshot.as_ref();
        let phase = match snapshot.map(|snapshot| snapshot.phase) {
            None => "inactive",
            Some(TradePhase::PendingOutgoing) => "pending-outgoing",
            Some(TradePhase::PendingIncoming) => "pending-incoming",
            Some(TradePhase::Open) => "open",
        };
        let mut lines = vec![format!("trade: {phase}")];
        if let Some(message) = &self.trade.last_message {
            lines.push(format!("message: {message}"));
        }
        if let Some(error) = &self.trade.last_error {
            lines.push(format!("error: {error}"));
        }
        if let Some(snapshot) = snapshot {
            lines.push(format_party("you", &snapshot.player));
            lines.push(format_party("other", &snapshot.other));
        }
        lines.join("\n")
    }
}

fn trade_action(request: &Request) -> Option<TradeRequest> {
    Some(match request {
        Request::TradeInitiate { name } => TradeRequest::Initiate(name.clone()),
        Request::TradeAccept => TradeRequest::Accept,
        Request::TradeDecline => TradeRequest::Decline,
        Request::TradeCancel => TradeRequest::Cancel,
        Request::TradeSetItem {
            slot,
            item_guid,
            stack_count,
        } => TradeRequest::SetItem(SetTradeItem {
            slot: *slot,
            item_guid: *item_guid,
            stack_count: *stack_count,
        }),
        Request::TradeClearItem { slot } => TradeRequest::ClearItem(*slot),
        Request::TradeSetMoney { copper } => TradeRequest::SetMoney(*copper),
        Request::TradeConfirm => TradeRequest::Confirm,
        Request::TradeCancelAccept => TradeRequest::CancelAccept,
        _ => return None,
    })
}

fn format_party(label: &str, party: &TradePartySnapshot) -> String {
    let items: Vec<String> = party
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
        .collect();
    let items = if items.is_empty() {
        "none".into()
    } else {
        items.join(", ")
    };
    let accepted = if party.accepted { "yes" } else { "no" };
    format!(
        "{label}: {} copper={} accepted={accepted} items={items}",
        party.name, party.gold
    )
}
