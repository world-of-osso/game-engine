use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AcceptSpiritHealerResurrection, DeathChannel, DeathPositionSnapshot, DeathStateSnapshot,
    DeathStateUpdate, QueryDeathStatus, ReleaseSpirit, ResurrectAtCorpse,
};

use crate::ipc::{Request, Response};
use crate::status::{
    DeathPositionEntry, DeathStateEntry, DeathStatusSnapshot, MapStatusSnapshot, Waypoint,
};

#[derive(Resource, Default)]
pub struct DeathRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
    queried_inworld: bool,
}

enum Action {
    ReleaseSpirit,
    ResurrectAtCorpse,
    AcceptSpiritHealer,
}

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeathRuntimeState>();
        app.init_resource::<DeathStatusSnapshot>();
        app.add_systems(Update, request_death_status_on_enter_world);
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_death_updates);
    }
}

pub fn queue_ipc_request(
    runtime: &mut DeathRuntimeState,
    snapshot: &DeathStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::DeathStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::DeathReleaseSpirit => {
            runtime.pending_actions.push_back(Action::ReleaseSpirit);
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::DeathResurrectAtCorpse => {
            runtime.pending_actions.push_back(Action::ResurrectAtCorpse);
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::DeathAcceptSpiritHealer => {
            runtime
                .pending_actions
                .push_back(Action::AcceptSpiritHealer);
            runtime.pending_replies.push_back(respond);
            true
        }
        _ => false,
    }
}

fn request_death_status_on_enter_world(
    mut runtime: ResMut<DeathRuntimeState>,
    snapshot: Res<DeathStatusSnapshot>,
    mut senders: Query<&mut MessageSender<QueryDeathStatus>>,
) {
    if runtime.queried_inworld || snapshot.state.is_some() {
        return;
    }
    if send_all(&mut senders, QueryDeathStatus) {
        runtime.queried_inworld = true;
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct DeathSenders<'w, 's> {
    release: Query<'w, 's, &'static mut MessageSender<ReleaseSpirit>>,
    resurrect: Query<'w, 's, &'static mut MessageSender<ResurrectAtCorpse>>,
    spirit_healer: Query<'w, 's, &'static mut MessageSender<AcceptSpiritHealerResurrection>>,
}

fn send_pending_actions(mut runtime: ResMut<DeathRuntimeState>, mut senders: DeathSenders) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::ReleaseSpirit => send_all(&mut senders.release, ReleaseSpirit),
            Action::ResurrectAtCorpse => send_all(&mut senders.resurrect, ResurrectAtCorpse),
            Action::AcceptSpiritHealer => {
                send_all(&mut senders.spirit_healer, AcceptSpiritHealerResurrection)
            }
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(
                "death flow is unavailable: not connected".into(),
            ));
        }
    }
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<DeathChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_death_updates(
    mut runtime: ResMut<DeathRuntimeState>,
    mut snapshot: ResMut<DeathStatusSnapshot>,
    mut map_status: ResMut<MapStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<DeathStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_death_state_update(&mut snapshot, &mut map_status, update);
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

pub fn apply_death_state_update(
    snapshot: &mut DeathStatusSnapshot,
    map_status: &mut MapStatusSnapshot,
    update: DeathStateUpdate,
) {
    if let Some(death) = update.snapshot {
        snapshot.state = Some(map_state(death.state));
        snapshot.corpse = death.corpse.map(map_position);
        snapshot.graveyard = death.graveyard.map(map_position);
        snapshot.can_resurrect_at_corpse = death.can_resurrect_at_corpse;
        snapshot.spirit_healer_available = death.spirit_healer_available;
        map_status.graveyard_marker = snapshot.graveyard.as_ref().map(|graveyard| Waypoint {
            x: graveyard.x,
            y: graveyard.z,
        });
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

fn map_state(state: DeathStateSnapshot) -> DeathStateEntry {
    match state {
        DeathStateSnapshot::Alive => DeathStateEntry::Alive,
        DeathStateSnapshot::Dead => DeathStateEntry::Dead,
        DeathStateSnapshot::Ghost => DeathStateEntry::Ghost,
        DeathStateSnapshot::Resurrecting => DeathStateEntry::Resurrecting,
    }
}

fn map_position(position: DeathPositionSnapshot) -> DeathPositionEntry {
    DeathPositionEntry {
        map_id: position.map_id,
        x: position.x,
        y: position.y,
        z: position.z,
    }
}

pub fn reset_runtime(runtime: &mut DeathRuntimeState, snapshot: &mut DeathStatusSnapshot) {
    *runtime = DeathRuntimeState::default();
    *snapshot = DeathStatusSnapshot::default();
}

fn format_status(snapshot: &DeathStatusSnapshot) -> String {
    crate::ipc::format::format_death_status(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::DeathSnapshot;

    #[test]
    fn death_state_update_populates_status_snapshot() {
        let mut snapshot = DeathStatusSnapshot::default();
        let mut map_status = MapStatusSnapshot::default();

        apply_death_state_update(
            &mut snapshot,
            &mut map_status,
            DeathStateUpdate {
                snapshot: Some(DeathSnapshot {
                    state: DeathStateSnapshot::Ghost,
                    corpse: Some(DeathPositionSnapshot {
                        map_id: 0,
                        x: 1.0,
                        y: 2.0,
                        z: 3.0,
                    }),
                    graveyard: Some(DeathPositionSnapshot {
                        map_id: 0,
                        x: 10.0,
                        y: 20.0,
                        z: 30.0,
                    }),
                    can_resurrect_at_corpse: false,
                    spirit_healer_available: true,
                }),
                message: Some("released spirit".into()),
                error: None,
            },
        );

        assert_eq!(snapshot.state, Some(DeathStateEntry::Ghost));
        assert!(snapshot.spirit_healer_available);
        assert_eq!(
            map_status.graveyard_marker,
            Some(Waypoint { x: 10.0, y: 30.0 })
        );
        assert_eq!(
            snapshot.last_server_message.as_deref(),
            Some("released spirit")
        );
    }
}
