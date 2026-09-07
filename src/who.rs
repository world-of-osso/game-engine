use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::prelude::*;
use lightyear::prelude::{Message as NetworkMessage, MessageReceiver, MessageSender};
use shared::protocol::{QueryWho, WhoChannel, WhoStateUpdate};

use crate::ipc::{Request, Response};
use crate::network_events::{register_message_handler, register_outgoing_handler};
use crate::status::{WhoEntry, WhoStatusSnapshot};

#[derive(Resource, Default)]
pub struct WhoRuntimeState {
    pending_queries: VecDeque<String>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

pub struct WhoPlugin;

impl Plugin for WhoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WhoRuntimeState>();
        register_outgoing_handler(app, send_pending_queries, |world| {
            !world
                .resource::<WhoRuntimeState>()
                .pending_queries
                .is_empty()
        });
        register_message_handler::<WhoStateUpdate, _>(app, receive_who_updates, |_| true);
    }
}

pub fn queue_ipc_request(
    runtime: &mut WhoRuntimeState,
    snapshot: &WhoStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::WhoStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::WhoQuery { query } => {
            queue_query(runtime, query.clone());
            runtime.pending_replies.push_back(respond);
            true
        }
        _ => false,
    }
}

pub fn queue_query(runtime: &mut WhoRuntimeState, query: String) {
    runtime.pending_queries.push_back(query);
}

fn send_pending_queries(
    mut runtime: ResMut<WhoRuntimeState>,
    mut senders: Query<&mut MessageSender<QueryWho>>,
) {
    if runtime.pending_queries.is_empty() {
        return;
    }

    while let Some(query) = runtime.pending_queries.pop_front() {
        let sent = send_all(
            &mut senders,
            QueryWho {
                query: query.clone(),
            },
        );
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error("who is unavailable: not connected".into()));
        }
    }
}

fn send_all<T: Clone + NetworkMessage>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<WhoChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_who_updates(
    mut runtime: ResMut<WhoRuntimeState>,
    mut snapshot: ResMut<WhoStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<WhoStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_who_state_update(&mut snapshot, update);
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

pub fn apply_who_state_update(snapshot: &mut WhoStatusSnapshot, update: WhoStateUpdate) {
    if let Some(who_snapshot) = update.snapshot {
        snapshot.query = who_snapshot.query;
        snapshot.entries = who_snapshot
            .entries
            .into_iter()
            .map(|entry| WhoEntry {
                name: entry.name,
                level: entry.level,
                class_name: entry.class_name,
                area: entry.area,
            })
            .collect();
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_runtime(runtime: &mut WhoRuntimeState) {
    *runtime = WhoRuntimeState::default();
}

pub fn queued_query_count(runtime: &WhoRuntimeState) -> usize {
    runtime.pending_queries.len()
}

pub fn first_queued_query(runtime: &WhoRuntimeState) -> Option<&str> {
    runtime.pending_queries.front().map(String::as_str)
}

fn format_status(snapshot: &WhoStatusSnapshot) -> String {
    crate::ipc::format::format_who_status(snapshot)
}

#[cfg(test)]
mod tests {
    use bevy::ecs::change_detection::DetectChanges;

    use super::*;

    #[test]
    fn queued_requests_wait_for_dispatch_then_report_disconnection() {
        let mut app = App::new();
        app.add_plugins(WhoPlugin)
            .init_resource::<WhoStatusSnapshot>();
        let (reply, responses) = mpsc::channel();
        {
            let mut runtime = app.world_mut().resource_mut::<WhoRuntimeState>();
            runtime
                .pending_queries
                .extend(["Alice".into(), "Bob".into()]);
            runtime.pending_replies.extend([reply.clone(), reply]);
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<WhoRuntimeState>()
                .pending_queries
                .len(),
            2
        );
        assert!(matches!(
            responses.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        crate::network_events::dispatch_outgoing(app.world_mut());
        for _ in 0..2 {
            let Response::Error(error) = responses.try_recv().unwrap() else {
                panic!("expected disconnected error");
            };
            assert_eq!(error, "who is unavailable: not connected");
        }
        let runtime = app.world().resource::<WhoRuntimeState>();
        assert!(runtime.pending_queries.is_empty());
        assert!(runtime.pending_replies.is_empty());
    }

    #[test]
    fn idle_dispatch_leaves_waiting_replies_untouched() {
        let mut app = App::new();
        app.add_plugins(WhoPlugin);
        let (reply, responses) = mpsc::channel();
        app.world_mut()
            .resource_mut::<WhoRuntimeState>()
            .pending_replies
            .push_back(reply);
        // No inbox means no snapshot is needed by incoming handlers.
        crate::network_events::dispatch_incoming(app.world_mut());
        crate::network_events::dispatch_outgoing(app.world_mut());
        assert_eq!(
            app.world()
                .resource::<WhoRuntimeState>()
                .pending_replies
                .len(),
            1
        );
        assert!(matches!(
            responses.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn idle_who_update_does_not_advance_runtime_change_tick() {
        let mut app = crate::test_harness::headless_app();
        app.add_plugins(WhoPlugin);
        app.init_resource::<WhoStatusSnapshot>();
        crate::network_events::dispatch_outgoing(app.world_mut());
        crate::network_events::dispatch_incoming(app.world_mut());

        let before = app
            .world()
            .get_resource_ref::<WhoRuntimeState>()
            .unwrap()
            .last_changed();
        crate::network_events::dispatch_outgoing(app.world_mut());
        crate::network_events::dispatch_incoming(app.world_mut());
        let after = app
            .world()
            .get_resource_ref::<WhoRuntimeState>()
            .unwrap()
            .last_changed();

        assert_eq!(after, before);
    }

    #[test]
    fn who_state_update_populates_status_snapshot() {
        let mut snapshot = WhoStatusSnapshot::default();

        apply_who_state_update(
            &mut snapshot,
            WhoStateUpdate {
                snapshot: Some(shared::protocol::WhoSnapshot {
                    query: "ali".into(),
                    entries: vec![shared::protocol::WhoCharacterSnapshot {
                        name: "Alice".into(),
                        level: 42,
                        class_name: "Mage".into(),
                        area: "Zone 12".into(),
                    }],
                }),
                message: Some("who: 1 result(s)".into()),
                error: None,
            },
        );

        assert_eq!(snapshot.query, "ali");
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].name, "Alice");
        assert_eq!(
            snapshot.last_server_message.as_deref(),
            Some("who: 1 result(s)")
        );
    }
}
