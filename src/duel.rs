use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AcceptDuel, DeclineDuel, DuelBoundarySnapshot, DuelChannel, DuelPhaseSnapshot,
    DuelResultSnapshot, DuelStateUpdate, InitiateDuel,
};

use crate::ipc::{Request, Response};
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
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_duel_updates);
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
    challenge: Query<'w, 's, &'static mut MessageSender<InitiateDuel>>,
    accept: Query<'w, 's, &'static mut MessageSender<AcceptDuel>>,
    decline: Query<'w, 's, &'static mut MessageSender<DeclineDuel>>,
}

fn send_pending_actions(mut state: ResMut<DuelClientState>, mut senders: DuelSenders) {
    while let Some(action) = state.pending_actions.pop_front() {
        let sent = match action {
            Action::Challenge(message) => send_all(&mut senders.challenge, message),
            Action::Accept => send_all(&mut senders.accept, AcceptDuel),
            Action::Decline => send_all(&mut senders.decline, DeclineDuel),
        };
        if !sent {
            if let Some(reply) = state.pending_replies.pop_front() {
                let _ = reply.send(Response::Error("duel is unavailable: not connected".into()));
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
        sender.send::<DuelChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_duel_updates(
    mut receivers: Query<&mut MessageReceiver<DuelStateUpdate>>,
    mut state: ResMut<DuelClientState>,
    mut snapshot: ResMut<DuelStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
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
