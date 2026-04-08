use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::{Message as NetworkMessage, MessageReceiver, MessageSender};
use shared::protocol::{CurrencyChannel, CurrencyStateUpdate, EarnCurrency, SpendCurrency};

use crate::ipc::{Request, Response};
use crate::status::{CurrenciesStatusSnapshot, CurrencyEntry};

#[derive(Resource, Default)]
pub struct CurrencyRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

enum Action {
    Earn { currency_id: u32, amount: u32 },
    Spend { currency_id: u32, amount: u32 },
}

pub struct CurrencyPlugin;

impl Plugin for CurrencyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrencyRuntimeState>();
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_currency_updates);
    }
}

pub fn queue_ipc_request(
    runtime: &mut CurrencyRuntimeState,
    snapshot: &CurrenciesStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::CurrenciesStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::CurrencyEarn {
            currency_id,
            amount,
        } => {
            runtime.pending_actions.push_back(Action::Earn {
                currency_id: *currency_id,
                amount: *amount,
            });
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::CurrencySpend {
            currency_id,
            amount,
        } => {
            runtime.pending_actions.push_back(Action::Spend {
                currency_id: *currency_id,
                amount: *amount,
            });
            runtime.pending_replies.push_back(respond);
            true
        }
        _ => false,
    }
}

#[derive(SystemParam)]
struct CurrencySenders<'w, 's> {
    earn: Query<'w, 's, &'static mut MessageSender<EarnCurrency>>,
    spend: Query<'w, 's, &'static mut MessageSender<SpendCurrency>>,
}

fn send_pending_actions(mut runtime: ResMut<CurrencyRuntimeState>, mut senders: CurrencySenders) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::Earn {
                currency_id,
                amount,
            } => send_all(
                &mut senders.earn,
                EarnCurrency {
                    currency_id,
                    amount,
                },
            ),
            Action::Spend {
                currency_id,
                amount,
            } => send_all(
                &mut senders.spend,
                SpendCurrency {
                    currency_id,
                    amount,
                },
            ),
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(
                "currencies are unavailable: not connected".into(),
            ));
        }
    }
}

fn send_all<T: Clone + NetworkMessage>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<CurrencyChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_currency_updates(
    mut runtime: ResMut<CurrencyRuntimeState>,
    mut snapshot: ResMut<CurrenciesStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<CurrencyStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_currency_state_update(&mut snapshot, update);
            if let Some(reply) = runtime.pending_replies.pop_front() {
                let response = if let Some(error) = &snapshot.last_error {
                    Response::Error(error.clone())
                } else {
                    Response::Text(format_status(&snapshot))
                };
                let _ = reply.send(response);
            }
        }
    }
}

pub fn apply_currency_state_update(
    snapshot: &mut CurrenciesStatusSnapshot,
    update: CurrencyStateUpdate,
) {
    if let Some(currency_snapshot) = update.snapshot {
        snapshot.entries = currency_snapshot
            .entries
            .into_iter()
            .map(|entry| CurrencyEntry {
                id: entry.id,
                name: entry.name,
                amount: entry.amount,
            })
            .collect();
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_runtime(runtime: &mut CurrencyRuntimeState) {
    *runtime = CurrencyRuntimeState::default();
}

fn format_status(snapshot: &CurrenciesStatusSnapshot) -> String {
    let mut lines = vec![format!("currencies: {}", snapshot.entries.len())];
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if snapshot.entries.is_empty() {
        lines.push("-".into());
        return lines.join("\n");
    }
    lines.extend(
        snapshot
            .entries
            .iter()
            .map(|entry| format!("{} {} amount={}", entry.id, entry.name, entry.amount)),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_status_includes_server_message() {
        let snapshot = CurrenciesStatusSnapshot {
            entries: vec![CurrencyEntry {
                id: 1,
                name: "Honor".into(),
                amount: 125,
            }],
            last_server_message: Some("earned 125 Honor".into()),
            last_error: None,
        };

        let text = format_status(&snapshot);

        assert!(text.contains("currencies: 1"));
        assert!(text.contains("message: earned 125 Honor"));
        assert!(text.contains("1 Honor amount=125"));
    }
}
