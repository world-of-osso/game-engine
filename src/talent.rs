use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    ApplyTalentChoice, QueryTalents, ResetTalents, TalentChannel, TalentStateUpdate,
};

use crate::ipc::{Request, Response};
use crate::status::{TalentNodeEntry, TalentSpecTabEntry, TalentStatusSnapshot};

#[derive(Resource, Default)]
pub struct TalentRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
    queried_inworld: bool,
}

enum Action {
    Apply(u32),
    Reset,
}

pub struct TalentPlugin;

impl Plugin for TalentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TalentRuntimeState>();
        app.add_systems(Update, request_talents_on_enter_world);
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_talent_updates);
    }
}

pub fn queue_ipc_request(
    runtime: &mut TalentRuntimeState,
    snapshot: &TalentStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::TalentStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::TalentApply { talent_id } => {
            runtime.pending_actions.push_back(Action::Apply(*talent_id));
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::TalentReset => {
            runtime.pending_actions.push_back(Action::Reset);
            runtime.pending_replies.push_back(respond);
            true
        }
        _ => false,
    }
}

fn request_talents_on_enter_world(
    mut runtime: ResMut<TalentRuntimeState>,
    snapshot: Res<TalentStatusSnapshot>,
    mut senders: Query<&mut MessageSender<QueryTalents>>,
) {
    if runtime.queried_inworld || !snapshot.talents.is_empty() {
        return;
    }
    if send_all(&mut senders, QueryTalents) {
        runtime.queried_inworld = true;
    }
}

#[derive(SystemParam)]
struct TalentSenders<'w, 's> {
    apply: Query<'w, 's, &'static mut MessageSender<ApplyTalentChoice>>,
    reset: Query<'w, 's, &'static mut MessageSender<ResetTalents>>,
}

fn send_pending_actions(mut runtime: ResMut<TalentRuntimeState>, mut senders: TalentSenders) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::Apply(talent_id) => {
                send_all(&mut senders.apply, ApplyTalentChoice { talent_id })
            }
            Action::Reset => send_all(&mut senders.reset, ResetTalents),
        };
        if !sent {
            if let Some(reply) = runtime.pending_replies.pop_front() {
                let _ = reply.send(Response::Error(
                    "talents are unavailable: not connected".into(),
                ));
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
        sender.send::<TalentChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_talent_updates(
    mut runtime: ResMut<TalentRuntimeState>,
    mut snapshot: ResMut<TalentStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<TalentStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_talent_state_update(&mut snapshot, update);
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

pub fn reset_runtime(runtime: &mut TalentRuntimeState) {
    *runtime = TalentRuntimeState::default();
}

fn apply_talent_state_update(snapshot: &mut TalentStatusSnapshot, update: TalentStateUpdate) {
    if let Some(talent_snapshot) = update.snapshot {
        snapshot.spec_tabs = talent_snapshot
            .spec_tabs
            .into_iter()
            .map(|tab| TalentSpecTabEntry {
                name: tab.name,
                active: tab.active,
            })
            .collect();
        snapshot.talents = talent_snapshot
            .talents
            .into_iter()
            .map(|talent| TalentNodeEntry {
                talent_id: talent.talent_id,
                name: talent.name,
                points_spent: talent.points_spent,
                max_points: talent.max_points,
                active: talent.active,
            })
            .collect();
        snapshot.points_remaining = talent_snapshot.points_remaining;
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

fn format_status(snapshot: &TalentStatusSnapshot) -> String {
    let mut lines = Vec::new();
    let active_tabs = snapshot
        .spec_tabs
        .iter()
        .filter(|tab| tab.active)
        .map(|tab| tab.name.as_str())
        .collect::<Vec<_>>();
    lines.push(format!(
        "talents: tabs={} selected={} points_remaining={}",
        if active_tabs.is_empty() {
            "none".into()
        } else {
            active_tabs.join(",")
        },
        snapshot
            .talents
            .iter()
            .filter(|talent| talent.active)
            .count(),
        snapshot.points_remaining
    ));
    if let Some(message) = &snapshot.last_server_message {
        lines.push(format!("message: {message}"));
    }
    if let Some(error) = &snapshot.last_error {
        lines.push(format!("error: {error}"));
    }
    if !snapshot.talents.is_empty() {
        let selected = snapshot
            .talents
            .iter()
            .filter(|talent| talent.active)
            .map(|talent| talent.name.as_str())
            .collect::<Vec<_>>();
        lines.push(format!(
            "selected: {}",
            if selected.is_empty() {
                "none".into()
            } else {
                selected.join(", ")
            }
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_status_reports_selected_talents() {
        let snapshot = TalentStatusSnapshot {
            spec_tabs: vec![crate::status::TalentSpecTabEntry {
                name: "Protection".into(),
                active: true,
            }],
            talents: vec![crate::status::TalentNodeEntry {
                talent_id: 101,
                name: "Divine Strength".into(),
                points_spent: 1,
                max_points: 1,
                active: true,
            }],
            points_remaining: 50,
            last_server_message: Some("talent applied".into()),
            last_error: None,
        };

        let text = format_status(&snapshot);

        assert!(text.contains("tabs=Protection"));
        assert!(text.contains("selected=1"));
        assert!(text.contains("Divine Strength"));
    }
}
