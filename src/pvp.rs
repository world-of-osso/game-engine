use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use shared::protocol::{
    DequeueFromPvp, PvpBracketSnapshot, PvpChannel, PvpQueueKindSnapshot, PvpStateUpdate,
    QueryPvpStatus, QueueForBattleground, QueueForRatedPvp,
};

use crate::ipc::{Request, Response};
use crate::network_events::{register_message_handler, register_outgoing_handler};
use crate::status::{PvpBracketEntry, PvpStatusSnapshot};

#[derive(Resource, Default)]
pub struct PvpRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
    queried_inworld: bool,
}

enum Action {
    QueueBattleground(u32),
    QueueRated(PvpBracketSnapshot),
    Dequeue,
}

pub struct PvpPlugin;

impl Plugin for PvpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PvpRuntimeState>();
        register_outgoing_handler(app, request_pvp_status_on_enter_world, pvp_query_pending);
        register_outgoing_handler(app, send_pending_actions, |world| {
            !world
                .resource::<PvpRuntimeState>()
                .pending_actions
                .is_empty()
        });
        register_message_handler::<PvpStateUpdate, _>(app, receive_pvp_updates, |_| true);
    }
}

pub fn queue_ipc_request(
    runtime: &mut PvpRuntimeState,
    snapshot: &PvpStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::PvpStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::PvpQueueBattleground { battleground_id } => {
            runtime
                .pending_actions
                .push_back(Action::QueueBattleground(*battleground_id));
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::PvpQueueRated { bracket } => {
            runtime
                .pending_actions
                .push_back(Action::QueueRated(bracket.clone()));
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::PvpDequeue => {
            runtime.pending_actions.push_back(Action::Dequeue);
            runtime.pending_replies.push_back(respond);
            true
        }
        _ => false,
    }
}

fn pvp_query_pending(world: &World) -> bool {
    if world.resource::<PvpRuntimeState>().queried_inworld {
        return false;
    }
    let snapshot = world.resource::<PvpStatusSnapshot>();
    snapshot.brackets.is_empty() && snapshot.queue.is_none()
}

fn request_pvp_status_on_enter_world(
    mut runtime: ResMut<PvpRuntimeState>,
    snapshot: Res<PvpStatusSnapshot>,
    mut senders: MessageSenders<QueryPvpStatus>,
) {
    if runtime.queried_inworld || !snapshot.brackets.is_empty() || snapshot.queue.is_some() {
        return;
    }
    if send_all(&mut senders, QueryPvpStatus) {
        runtime.queried_inworld = true;
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct PvpSenders<'w, 's> {
    battleground: MessageSenders<'w, 's, QueueForBattleground>,
    rated: MessageSenders<'w, 's, QueueForRatedPvp>,
    dequeue: MessageSenders<'w, 's, DequeueFromPvp>,
}

fn send_pending_actions(mut runtime: ResMut<PvpRuntimeState>, mut senders: PvpSenders) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::QueueBattleground(battleground_id) => send_all(
                &mut senders.battleground,
                QueueForBattleground { battleground_id },
            ),
            Action::QueueRated(bracket) => {
                send_all(&mut senders.rated, QueueForRatedPvp { bracket })
            }
            Action::Dequeue => send_all(&mut senders.dequeue, DequeueFromPvp),
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error("pvp is unavailable: not connected".into()));
        }
    }
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut MessageSenders<T>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<PvpChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_pvp_updates(
    mut runtime: ResMut<PvpRuntimeState>,
    mut snapshot: ResMut<PvpStatusSnapshot>,
    mut receivers: MessageReceivers<PvpStateUpdate>,
) {
    for receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_pvp_state_update(&mut snapshot, update);
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

pub fn apply_pvp_state_update(snapshot: &mut PvpStatusSnapshot, update: PvpStateUpdate) {
    if let Some(pvp) = update.snapshot {
        snapshot.honor = pvp.honor;
        snapshot.honor_max = pvp.honor_max;
        snapshot.conquest = pvp.conquest;
        snapshot.conquest_max = pvp.conquest_max;
        snapshot.queue = pvp.queue.map(format_queue_label);
        snapshot.brackets = pvp
            .brackets
            .into_iter()
            .map(|entry| PvpBracketEntry {
                bracket: bracket_label(&entry.bracket).into(),
                rating: entry.rating,
                season_wins: entry.season_wins,
                season_losses: entry.season_losses,
                weekly_wins: entry.weekly_wins,
                weekly_losses: entry.weekly_losses,
            })
            .collect();
        snapshot.brackets.sort_by(|a, b| a.bracket.cmp(&b.bracket));
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_runtime(runtime: &mut PvpRuntimeState) {
    *runtime = PvpRuntimeState::default();
}

fn format_queue_label(queue: shared::protocol::PvpQueueSnapshot) -> String {
    match queue.kind {
        PvpQueueKindSnapshot::Battleground {
            battleground_id,
            name,
        } => format!("{name} ({battleground_id})"),
        PvpQueueKindSnapshot::RatedBracket { bracket } => bracket_label(&bracket).into(),
    }
}

fn bracket_label(bracket: &PvpBracketSnapshot) -> &'static str {
    match bracket {
        PvpBracketSnapshot::Arena2v2 => "2v2",
        PvpBracketSnapshot::Arena3v3 => "3v3",
        PvpBracketSnapshot::RatedBattleground => "Rated BG",
        PvpBracketSnapshot::SoloShuffle => "Solo Shuffle",
    }
}

fn format_status(snapshot: &PvpStatusSnapshot) -> String {
    crate::ipc::format::format_pvp_status(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{PvpBracketStatsSnapshot, PvpQueueSnapshot, PvpSnapshot};

    #[test]
    fn query_eligibility_tracks_existing_pvp_state() {
        let mut world = World::new();
        world.init_resource::<PvpRuntimeState>();
        world.init_resource::<PvpStatusSnapshot>();
        assert!(pvp_query_pending(&world));
        world.resource_mut::<PvpStatusSnapshot>().queue = Some("Warsong Gulch".into());
        assert!(!pvp_query_pending(&world));
        world.resource_mut::<PvpStatusSnapshot>().queue = None;
        world
            .resource_mut::<PvpStatusSnapshot>()
            .brackets
            .push(PvpBracketEntry {
                bracket: "2v2".into(),
                rating: 1500,
                season_wins: 10,
                season_losses: 5,
                weekly_wins: 2,
                weekly_losses: 1,
            });
        assert!(!pvp_query_pending(&world));
        world.resource_mut::<PvpStatusSnapshot>().brackets.clear();
        assert!(pvp_query_pending(&world));
        world.resource_mut::<PvpRuntimeState>().queried_inworld = true;
        world.remove_resource::<PvpStatusSnapshot>();
        assert!(!pvp_query_pending(&world));
    }

    #[test]
    fn pvp_state_update_populates_status_snapshot() {
        let mut snapshot = PvpStatusSnapshot::default();

        apply_pvp_state_update(
            &mut snapshot,
            PvpStateUpdate {
                snapshot: Some(PvpSnapshot {
                    honor: 750,
                    honor_max: 15_000,
                    conquest: 120,
                    conquest_max: 1_800,
                    brackets: vec![PvpBracketStatsSnapshot {
                        bracket: PvpBracketSnapshot::Arena2v2,
                        rating: 1516,
                        season_wins: 1,
                        season_losses: 0,
                        weekly_wins: 1,
                        weekly_losses: 0,
                    }],
                    queue: Some(PvpQueueSnapshot {
                        kind: PvpQueueKindSnapshot::RatedBracket {
                            bracket: PvpBracketSnapshot::Arena2v2,
                        },
                    }),
                }),
                message: Some("queued for 2v2 arena".into()),
                error: None,
            },
        );

        assert_eq!(snapshot.honor, 750);
        assert_eq!(snapshot.queue.as_deref(), Some("2v2"));
        assert_eq!(snapshot.brackets.len(), 1);
        assert_eq!(snapshot.brackets[0].rating, 1516);
        assert_eq!(
            snapshot.last_server_message.as_deref(),
            Some("queued for 2v2 arena")
        );
    }
}
