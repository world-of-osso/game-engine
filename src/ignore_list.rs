use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::{Message as NetworkMessage, MessageReceiver, MessageSender};
use shared::protocol::{AddIgnore, IgnoreChannel, IgnoreListStateUpdate, RemoveIgnore};

use crate::ipc::{Request, Response};
use crate::status::IgnoreListStatusSnapshot;

#[derive(Resource, Default)]
pub struct IgnoreListRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

enum Action {
    Add { name: String },
    Remove { name: String },
}

pub struct IgnoreListPlugin;

impl Plugin for IgnoreListPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IgnoreListRuntimeState>();
        app.add_systems(Update, (send_pending_actions, receive_ignore_updates));
    }
}

pub fn queue_ipc_request(
    runtime: &mut IgnoreListRuntimeState,
    snapshot: &IgnoreListStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    let action = match request {
        Request::IgnoreStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            return true;
        }
        Request::IgnoreAdd { name } => Action::Add { name: name.clone() },
        Request::IgnoreRemove { name } => Action::Remove { name: name.clone() },
        _ => return false,
    };
    runtime.pending_actions.push_back(action);
    runtime.pending_replies.push_back(respond);
    true
}

fn format_status(snapshot: &IgnoreListStatusSnapshot) -> String {
    crate::ipc::format::format_ignore_list_status(snapshot)
}

#[derive(SystemParam)]
struct IgnoreListSenders<'w, 's> {
    add: Query<'w, 's, &'static mut MessageSender<AddIgnore>>,
    remove: Query<'w, 's, &'static mut MessageSender<RemoveIgnore>>,
}

fn send_pending_actions(
    mut runtime: ResMut<IgnoreListRuntimeState>,
    mut senders: IgnoreListSenders,
) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::Add { name } => send_all(&mut senders.add, AddIgnore { name }),
            Action::Remove { name } => send_all(&mut senders.remove, RemoveIgnore { name }),
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(
                "ignore list is unavailable: not connected".into(),
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
        sender.send::<IgnoreChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_ignore_updates(
    mut runtime: ResMut<IgnoreListRuntimeState>,
    mut snapshot: ResMut<IgnoreListStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<IgnoreListStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_ignore_list_state_update(&mut snapshot, update);
            if let Some(reply) = runtime.pending_replies.pop_front() {
                let response = if let Some(error) = &snapshot.last_error {
                    Response::Error(error.clone())
                } else {
                    Response::Text(
                        snapshot
                            .last_server_message
                            .clone()
                            .unwrap_or_else(|| "ignore list updated".into()),
                    )
                };
                let _ = reply.send(response);
            }
        }
    }
}

pub fn apply_ignore_list_state_update(
    snapshot: &mut IgnoreListStatusSnapshot,
    update: IgnoreListStateUpdate,
) {
    if let Some(ignore_list) = update.snapshot {
        snapshot.names = ignore_list.names;
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_runtime(runtime: &mut IgnoreListRuntimeState) {
    *runtime = IgnoreListRuntimeState::default();
}

pub fn is_ignored(snapshot: &IgnoreListStatusSnapshot, name: &str) -> bool {
    snapshot
        .names
        .iter()
        .any(|ignored| ignored.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_list_state_update_populates_status_snapshot() {
        let mut snapshot = IgnoreListStatusSnapshot::default();

        apply_ignore_list_state_update(
            &mut snapshot,
            IgnoreListStateUpdate {
                snapshot: Some(shared::protocol::IgnoreListSnapshot {
                    names: vec!["Alice".into(), "Bob".into()],
                }),
                message: Some("ignored: Alice".into()),
                error: None,
            },
        );

        assert_eq!(snapshot.names, vec!["Alice", "Bob"]);
        assert_eq!(
            snapshot.last_server_message.as_deref(),
            Some("ignored: Alice")
        );
    }

    #[test]
    fn is_ignored_matches_case_insensitively() {
        let snapshot = IgnoreListStatusSnapshot {
            names: vec!["Alice".into()],
            ..Default::default()
        };

        assert!(is_ignored(&snapshot, "alice"));
        assert!(!is_ignored(&snapshot, "bob"));
    }
}
