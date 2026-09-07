use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use shared::protocol::{
    AcceptDuel, DeclineDuel, DuelBoundarySnapshot, DuelChannel, DuelPhaseSnapshot,
    DuelResultSnapshot, DuelStateUpdate, InitiateDuel,
};

use crate::ipc::{Request, Response};
use crate::network_events::{register_message_handler, register_outgoing_handler};
use crate::status::{DuelBoundaryEntry, DuelPhaseEntry, DuelResultEntry, DuelStatusSnapshot};
use crate::targeting::CurrentTarget;

#[derive(Resource, Default)]
pub struct DuelClientState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

#[derive(Clone)]
enum Action {
    Challenge(InitiateDuel),
    Accept,
    Decline,
}

pub struct DuelPlugin;

impl Plugin for DuelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DuelClientState>();
        register_outgoing_handler(app, send_pending_actions, |world| {
            !world
                .resource::<DuelClientState>()
                .pending_actions
                .is_empty()
        });
        register_message_handler::<DuelStateUpdate, _>(app, receive_duel_updates, |_| true);
    }
}

pub fn queue_ipc_request_with_snapshot(
    state: &mut DuelClientState,
    snapshot: &DuelStatusSnapshot,
    current_target: &CurrentTarget,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    if matches!(request, Request::DuelStatus) {
        let _ = respond.send(Response::Text(format_status(snapshot)));
        return true;
    }
    let Some(action) = map_action(request, current_target) else {
        return false;
    };
    state.pending_actions.push_back(action);
    state.pending_replies.push_back(respond);
    true
}

fn map_action(request: &Request, current_target: &CurrentTarget) -> Option<Action> {
    match request {
        Request::DuelChallenge => Some(Action::Challenge(InitiateDuel {
            target_entity: current_target.0.map(|entity| entity.to_bits()),
        })),
        Request::DuelAccept => Some(Action::Accept),
        Request::DuelDecline => Some(Action::Decline),
        _ => None,
    }
}

#[derive(SystemParam)]
struct DuelSenders<'w, 's> {
    challenge: MessageSenders<'w, 's, InitiateDuel>,
    accept: MessageSenders<'w, 's, AcceptDuel>,
    decline: MessageSenders<'w, 's, DeclineDuel>,
}

fn send_pending_actions(
    mut state: ResMut<DuelClientState>,
    mut senders: DuelSenders,
    map: Res<ReplicationMirrorMap>,
) {
    while let Some(action) = state.pending_actions.pop_front() {
        if let Err(error) = send_action(action, &mut senders, &map)
            && let Some(reply) = state.pending_replies.pop_front()
        {
            let _ = reply.send(Response::Error(error.into()));
        }
    }
}

fn send_action(
    action: Action,
    senders: &mut DuelSenders,
    map: &ReplicationMirrorMap,
) -> Result<(), &'static str> {
    let sent = match action {
        Action::Challenge(message) => {
            if senders.challenge.is_empty() {
                return Err("duel is unavailable: not connected");
            }
            let message = map_challenge_target(message, map)?;
            send_all(&mut senders.challenge, message)
        }
        Action::Accept => send_all(&mut senders.accept, AcceptDuel),
        Action::Decline => send_all(&mut senders.decline, DeclineDuel),
    };
    sent.then_some(())
        .ok_or("duel is unavailable: not connected")
}

fn map_challenge_target(
    mut message: InitiateDuel,
    map: &ReplicationMirrorMap,
) -> Result<InitiateDuel, &'static str> {
    if let Some(bits) = message.target_entity {
        let server = Entity::try_from_bits(bits)
            .and_then(|main| map.main_to_server(main))
            .ok_or("duel target is no longer replicated")?;
        message.target_entity = Some(server.to_bits());
    }
    Ok(message)
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut MessageSenders<T>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<DuelChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_duel_updates(
    mut receivers: MessageReceivers<DuelStateUpdate>,
    mut state: ResMut<DuelClientState>,
    mut snapshot: ResMut<DuelStatusSnapshot>,
) {
    for receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_duel_state_update(&mut snapshot, update);
            if let Some(reply) = state.pending_replies.pop_front() {
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

pub fn apply_duel_state_update(snapshot: &mut DuelStatusSnapshot, update: DuelStateUpdate) {
    if let Some(duel) = update.snapshot {
        snapshot.phase = Some(map_phase(duel.phase));
        snapshot.opponent_name = Some(duel.opponent_name);
        snapshot.boundary = duel.boundary.map(map_boundary);
        snapshot.last_result = duel.result.map(map_result);
    } else {
        snapshot.phase = None;
        snapshot.opponent_name = None;
        snapshot.boundary = None;
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

fn map_phase(phase: DuelPhaseSnapshot) -> DuelPhaseEntry {
    match phase {
        DuelPhaseSnapshot::PendingOutgoing => DuelPhaseEntry::PendingOutgoing,
        DuelPhaseSnapshot::PendingIncoming => DuelPhaseEntry::PendingIncoming,
        DuelPhaseSnapshot::Active => DuelPhaseEntry::Active,
        DuelPhaseSnapshot::Completed => DuelPhaseEntry::Completed,
    }
}

fn map_boundary(boundary: DuelBoundarySnapshot) -> DuelBoundaryEntry {
    DuelBoundaryEntry {
        center_x: boundary.center_x,
        center_z: boundary.center_z,
        radius: boundary.radius,
    }
}

fn map_result(result: DuelResultSnapshot) -> DuelResultEntry {
    match result {
        DuelResultSnapshot::Won => DuelResultEntry::Won,
        DuelResultSnapshot::Lost => DuelResultEntry::Lost,
        DuelResultSnapshot::Declined => DuelResultEntry::Declined,
        DuelResultSnapshot::Cancelled => DuelResultEntry::Cancelled,
    }
}

pub fn reset_runtime(state: &mut DuelClientState) {
    *state = DuelClientState::default();
}

fn format_status(snapshot: &DuelStatusSnapshot) -> String {
    let phase = match snapshot.phase {
        None => "inactive",
        Some(DuelPhaseEntry::PendingOutgoing) => "pending-outgoing",
        Some(DuelPhaseEntry::PendingIncoming) => "pending-incoming",
        Some(DuelPhaseEntry::Active) => "active",
        Some(DuelPhaseEntry::Completed) => "completed",
    };
    let mut lines = vec![format!("duel: {phase}")];
    if let Some(opponent) = &snapshot.opponent_name {
        lines.push(format!("opponent: {opponent}"));
    }
    if let Some(boundary) = &snapshot.boundary {
        lines.push(format!(
            "boundary: center=({:.1},{:.1}) radius={:.1}",
            boundary.center_x, boundary.center_z, boundary.radius
        ));
    }
    if let Some(result) = &snapshot.last_result {
        lines.push(format!("result: {}", format_result(result)));
    }
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    lines.join("\n")
}

fn format_result(result: &DuelResultEntry) -> &'static str {
    match result {
        DuelResultEntry::Won => "won",
        DuelResultEntry::Lost => "lost",
        DuelResultEntry::Declined => "declined",
        DuelResultEntry::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::DuelSnapshot;

    #[test]
    fn removed_challenge_target_replies_without_sending_a_command() {
        use crate::network_runtime::messages::ConnectionSender;
        let mut app = App::new();
        app.init_resource::<ReplicationMirrorMap>();
        app.add_plugins(DuelPlugin);
        let main = app.world_mut().spawn_empty().id();
        let (commands, pending_commands) = mpsc::channel();
        app.insert_resource(ConnectionSender::new(Some(commands)));
        let (reply, responses) = mpsc::channel();
        queue_ipc_request_with_snapshot(
            &mut app.world_mut().resource_mut::<DuelClientState>(),
            &DuelStatusSnapshot::default(),
            &CurrentTarget(Some(main)),
            &Request::DuelChallenge,
            reply,
        );
        crate::network_events::dispatch_outgoing(app.world_mut());
        let Response::Error(error) = responses.try_recv().unwrap() else {
            panic!("expected unmapped target error");
        };
        assert_eq!(error, "duel target is no longer replicated");
        assert!(pending_commands.try_recv().is_err());
        assert!(
            app.world()
                .resource::<DuelClientState>()
                .pending_actions
                .is_empty()
        );
    }

    #[test]
    fn challenge_target_uses_server_identity_and_rejects_removed_mapping() {
        let mut world = World::new();
        let main = world.spawn_empty().id();
        let server = world.spawn_empty().id();
        let mut map = ReplicationMirrorMap::default();
        map.insert(server, main);
        let request = InitiateDuel {
            target_entity: Some(main.to_bits()),
        };
        assert_eq!(
            map_challenge_target(request.clone(), &map)
                .unwrap()
                .target_entity,
            Some(server.to_bits())
        );
        map.clear();
        assert_eq!(
            map_challenge_target(request, &map).unwrap_err(),
            "duel target is no longer replicated"
        );
        assert_eq!(
            map_challenge_target(
                InitiateDuel {
                    target_entity: None
                },
                &map
            )
            .unwrap()
            .target_entity,
            None
        );
    }

    #[test]
    fn format_status_reports_active_boundary() {
        let snapshot = DuelStatusSnapshot {
            phase: Some(DuelPhaseEntry::Active),
            opponent_name: Some("Alice".into()),
            boundary: Some(DuelBoundaryEntry {
                center_x: 10.0,
                center_z: 15.0,
                radius: 30.0,
            }),
            last_result: None,
            last_server_message: Some("duel started".into()),
            last_error: None,
        };

        let text = format_status(&snapshot);

        assert!(text.contains("duel: active"));
        assert!(text.contains("opponent: Alice"));
        assert!(text.contains("radius=30.0"));
    }

    #[test]
    fn apply_duel_state_update_maps_result() {
        let mut snapshot = DuelStatusSnapshot::default();

        apply_duel_state_update(
            &mut snapshot,
            DuelStateUpdate {
                snapshot: Some(DuelSnapshot {
                    phase: DuelPhaseSnapshot::PendingIncoming,
                    opponent_name: "Alice".into(),
                    boundary: None,
                    result: Some(DuelResultSnapshot::Declined),
                }),
                message: Some("duel declined".into()),
                error: None,
            },
        );

        assert_eq!(snapshot.phase, Some(DuelPhaseEntry::PendingIncoming));
        assert_eq!(snapshot.opponent_name.as_deref(), Some("Alice"));
        assert_eq!(snapshot.last_result, Some(DuelResultEntry::Declined));
    }
}
