use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    DequeueFromLfg, GroupRoleSnapshot, LfgChannel, LfgMatchFoundSnapshot, LfgRoleCheckSnapshot,
    LfgStateUpdate, QueryLfgStatus, QueueForLfg, RespondToLfgRoleCheck,
};

use crate::ipc::{Request, Response};
use crate::status::{
    GroupRole, LfgMatchFoundEntry, LfgMatchMemberEntry, LfgRoleCheckEntry, LfgStatusSnapshot,
};

#[derive(Resource, Default)]
pub struct LfgRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
    queried_inworld: bool,
}

enum Action {
    Queue {
        role: GroupRole,
        dungeon_ids: Vec<u32>,
    },
    Dequeue,
    Respond {
        accepted: bool,
    },
}

pub struct LfgPlugin;

impl Plugin for LfgPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LfgRuntimeState>();
        app.add_systems(Update, request_lfg_status_on_enter_world);
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_lfg_updates);
    }
}

pub fn queue_ipc_request(
    runtime: &mut LfgRuntimeState,
    snapshot: &LfgStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::LfgStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::LfgQueue { role, dungeon_ids } => {
            runtime.pending_actions.push_back(Action::Queue {
                role: role.clone(),
                dungeon_ids: dungeon_ids.clone(),
            });
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::LfgDequeue => {
            runtime.pending_actions.push_back(Action::Dequeue);
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::LfgAccept => {
            runtime
                .pending_actions
                .push_back(Action::Respond { accepted: true });
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::LfgDecline => {
            runtime
                .pending_actions
                .push_back(Action::Respond { accepted: false });
            runtime.pending_replies.push_back(respond);
            true
        }
        _ => false,
    }
}

fn request_lfg_status_on_enter_world(
    mut runtime: ResMut<LfgRuntimeState>,
    snapshot: Res<LfgStatusSnapshot>,
    mut senders: Query<&mut MessageSender<QueryLfgStatus>>,
) {
    if runtime.queried_inworld
        || snapshot.queued
        || snapshot.selected_role.is_some()
        || snapshot.role_check.is_some()
        || snapshot.match_found.is_some()
    {
        return;
    }
    if send_all(&mut senders, QueryLfgStatus) {
        runtime.queried_inworld = true;
    }
}

#[derive(SystemParam)]
struct LfgSenders<'w, 's> {
    queue: Query<'w, 's, &'static mut MessageSender<QueueForLfg>>,
    dequeue: Query<'w, 's, &'static mut MessageSender<DequeueFromLfg>>,
    respond: Query<'w, 's, &'static mut MessageSender<RespondToLfgRoleCheck>>,
}

fn send_pending_actions(mut runtime: ResMut<LfgRuntimeState>, mut senders: LfgSenders) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::Queue { role, dungeon_ids } => send_all(
                &mut senders.queue,
                QueueForLfg {
                    role: map_role_to_snapshot(role),
                    dungeon_ids,
                },
            ),
            Action::Dequeue => send_all(&mut senders.dequeue, DequeueFromLfg),
            Action::Respond { accepted } => {
                send_all(&mut senders.respond, RespondToLfgRoleCheck { accepted })
            }
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error("lfg is unavailable: not connected".into()));
        }
    }
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<LfgChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_lfg_updates(
    mut runtime: ResMut<LfgRuntimeState>,
    mut snapshot: ResMut<LfgStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<LfgStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_lfg_state_update(&mut snapshot, update);
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

pub fn apply_lfg_state_update(snapshot: &mut LfgStatusSnapshot, update: LfgStateUpdate) {
    if let Some(lfg) = update.snapshot {
        snapshot.queued = lfg.queued;
        snapshot.selected_role = lfg.selected_role.map(map_role_from_snapshot);
        snapshot.dungeon_ids = lfg.dungeon_ids;
        snapshot.queue_size = lfg.queue_size;
        snapshot.average_wait_secs = lfg.average_wait_secs;
        snapshot.in_demand_roles = lfg
            .in_demand_roles
            .into_iter()
            .map(map_role_from_snapshot)
            .collect();
        snapshot.role_check = lfg.role_check.map(map_role_check);
        snapshot.match_found = lfg.match_found.map(map_match_found);
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

fn map_role_check(role_check: LfgRoleCheckSnapshot) -> LfgRoleCheckEntry {
    LfgRoleCheckEntry {
        dungeon_id: role_check.dungeon_id,
        dungeon_name: role_check.dungeon_name,
        assigned_role: map_role_from_snapshot(role_check.assigned_role),
        accepted_count: role_check.accepted_count,
        total_count: role_check.total_count,
    }
}

fn map_match_found(match_found: LfgMatchFoundSnapshot) -> LfgMatchFoundEntry {
    LfgMatchFoundEntry {
        dungeon_id: match_found.dungeon_id,
        dungeon_name: match_found.dungeon_name,
        assigned_role: map_role_from_snapshot(match_found.assigned_role),
        members: match_found
            .members
            .into_iter()
            .map(|member| LfgMatchMemberEntry {
                name: member.name,
                role: map_role_from_snapshot(member.role),
            })
            .collect(),
    }
}

fn map_role_from_snapshot(role: GroupRoleSnapshot) -> GroupRole {
    match role {
        GroupRoleSnapshot::Tank => GroupRole::Tank,
        GroupRoleSnapshot::Healer => GroupRole::Healer,
        GroupRoleSnapshot::Damage => GroupRole::Damage,
        GroupRoleSnapshot::None => GroupRole::None,
    }
}

fn map_role_to_snapshot(role: GroupRole) -> GroupRoleSnapshot {
    match role {
        GroupRole::Tank => GroupRoleSnapshot::Tank,
        GroupRole::Healer => GroupRoleSnapshot::Healer,
        GroupRole::Damage => GroupRoleSnapshot::Damage,
        GroupRole::None => GroupRoleSnapshot::None,
    }
}

pub fn reset_runtime(runtime: &mut LfgRuntimeState) {
    *runtime = LfgRuntimeState::default();
}

fn format_status(snapshot: &LfgStatusSnapshot) -> String {
    crate::ipc::format::format_lfg_status(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::LfgSnapshot;

    #[test]
    fn lfg_state_update_populates_status_snapshot() {
        let mut snapshot = LfgStatusSnapshot::default();

        apply_lfg_state_update(
            &mut snapshot,
            LfgStateUpdate {
                snapshot: Some(LfgSnapshot {
                    queued: true,
                    selected_role: Some(GroupRoleSnapshot::Healer),
                    dungeon_ids: vec![33, 44],
                    queue_size: 9,
                    average_wait_secs: 120,
                    in_demand_roles: vec![GroupRoleSnapshot::Tank],
                    role_check: Some(LfgRoleCheckSnapshot {
                        dungeon_id: 33,
                        dungeon_name: "Shadowfang Keep".into(),
                        assigned_role: GroupRoleSnapshot::Healer,
                        accepted_count: 3,
                        total_count: 5,
                    }),
                    match_found: None,
                }),
                message: Some("role check started".into()),
                error: None,
            },
        );

        assert!(snapshot.queued);
        assert_eq!(snapshot.selected_role, Some(GroupRole::Healer));
        assert_eq!(snapshot.dungeon_ids, vec![33, 44]);
        assert_eq!(snapshot.in_demand_roles, vec![GroupRole::Tank]);
        assert_eq!(
            snapshot
                .role_check
                .as_ref()
                .map(|entry| entry.dungeon_name.as_str()),
            Some("Shadowfang Keep")
        );
        assert_eq!(
            snapshot.last_server_message.as_deref(),
            Some("role check started")
        );
    }

    #[test]
    fn format_status_reports_role_check_and_match() {
        let snapshot = LfgStatusSnapshot {
            queued: false,
            selected_role: Some(GroupRole::Tank),
            dungeon_ids: vec![33],
            queue_size: 4,
            average_wait_secs: 75,
            in_demand_roles: vec![GroupRole::Healer],
            role_check: Some(LfgRoleCheckEntry {
                dungeon_id: 33,
                dungeon_name: "Shadowfang Keep".into(),
                assigned_role: GroupRole::Tank,
                accepted_count: 4,
                total_count: 5,
            }),
            match_found: Some(LfgMatchFoundEntry {
                dungeon_id: 33,
                dungeon_name: "Shadowfang Keep".into(),
                assigned_role: GroupRole::Tank,
                members: vec![
                    LfgMatchMemberEntry {
                        name: "Alice".into(),
                        role: GroupRole::Healer,
                    },
                    LfgMatchMemberEntry {
                        name: "Bob".into(),
                        role: GroupRole::Damage,
                    },
                ],
            }),
            last_server_message: Some("match found".into()),
            last_error: None,
        };

        let text = format_status(&snapshot);

        assert!(text.contains("lfg: queued=false role=tank"));
        assert!(text.contains("role_check: Shadowfang Keep role=tank accepted=4/5"));
        assert!(text.contains("match_found: Shadowfang Keep role=tank members=2"));
        assert!(text.contains("member: Alice role=healer"));
    }
}
