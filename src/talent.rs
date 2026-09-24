//! Retail talent (trait config) client state: the last `TraitConfigSnapshot`,
//! the active spec, the pending local config and queued commit / spec requests.
//! Server rules and messages: game-server `docs/wiki/systems/talents.md`.

use std::collections::VecDeque;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use shared::protocol::{
    CommitTraitConfig, SetSpecialization, SpecializationChanged, TalentChannel, TraitCommitResult,
    TraitConfigSnapshot,
};

use crate::network_events::{register_message_handler, register_outgoing_handler};
use crate::network_runtime::messages::{MessageReceivers, MessageSenders};
use crate::talent_tree::rules::ConfigEntry;
use crate::ui::ui_errors::UiErrors;

#[derive(Resource, Default, Debug)]
pub struct TalentState {
    pub snapshot: Option<TraitConfigSnapshot>,
    /// Spec named by the last `SpecializationChanged` or snapshot.
    pub active_spec: Option<u32>,
    /// Local edits of the snapshot config; `None` when there are none.
    pub pending: Option<Vec<ConfigEntry>>,
    outgoing: VecDeque<Outgoing>,
}

#[derive(Debug, Clone, PartialEq)]
enum Outgoing {
    Commit(CommitTraitConfig),
    SetSpec(u32),
}

impl TalentState {
    pub fn queue_commit(&mut self, commit: CommitTraitConfig) {
        self.outgoing.push_back(Outgoing::Commit(commit));
    }

    pub fn queue_set_spec(&mut self, spec_id: u32) {
        self.outgoing.push_back(Outgoing::SetSpec(spec_id));
    }

    /// Commits waiting for the next outgoing dispatch.
    pub fn queued_commits(&self) -> impl Iterator<Item = &CommitTraitConfig> {
        self.outgoing.iter().filter_map(|outgoing| match outgoing {
            Outgoing::Commit(commit) => Some(commit),
            Outgoing::SetSpec(_) => None,
        })
    }

    /// Spec requests waiting for the next outgoing dispatch.
    pub fn queued_spec_requests(&self) -> impl Iterator<Item = u32> + '_ {
        self.outgoing.iter().filter_map(|outgoing| match outgoing {
            Outgoing::SetSpec(spec_id) => Some(*spec_id),
            Outgoing::Commit(_) => None,
        })
    }

    fn apply_snapshot(&mut self, snapshot: TraitConfigSnapshot) {
        self.active_spec = Some(snapshot.spec_id);
        self.snapshot = Some(snapshot);
        self.pending = None;
    }

    /// A different spec drops the old spec's snapshot until its own arrives.
    fn apply_spec_changed(&mut self, spec_id: u32) {
        self.active_spec = Some(spec_id);
        if self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.spec_id != spec_id)
        {
            self.snapshot = None;
            self.pending = None;
        }
    }
}

pub struct TalentPlugin;

impl Plugin for TalentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TalentState>();
        app.init_resource::<UiErrors>();
        register_outgoing_handler(app, send_outgoing, |world| {
            !world.resource::<TalentState>().outgoing.is_empty()
        });
        register_message_handler::<TraitConfigSnapshot, _>(app, receive_snapshots, |_| true);
        register_message_handler::<SpecializationChanged, _>(app, receive_spec_changes, |_| true);
        register_message_handler::<TraitCommitResult, _>(app, receive_commit_results, |_| true);
    }
}

pub fn reset_state(state: &mut TalentState) {
    *state = TalentState::default();
}

#[derive(SystemParam)]
struct TalentSenders<'w, 's> {
    commit: MessageSenders<'w, 's, CommitTraitConfig>,
    spec: MessageSenders<'w, 's, SetSpecialization>,
}

fn send_outgoing(mut state: ResMut<TalentState>, mut senders: TalentSenders) {
    while let Some(outgoing) = state.outgoing.pop_front() {
        let sent = match outgoing {
            Outgoing::Commit(commit) => send_all(&mut senders.commit, commit),
            Outgoing::SetSpec(spec_id) => {
                send_all(&mut senders.spec, SetSpecialization { spec_id })
            }
        };
        if !sent {
            warn!("talent request dropped: not connected");
        }
    }
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut MessageSenders<T>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<TalentChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_snapshots(
    mut state: ResMut<TalentState>,
    mut receivers: MessageReceivers<TraitConfigSnapshot>,
) {
    for receiver in receivers.iter_mut() {
        for snapshot in receiver.receive() {
            state.apply_snapshot(snapshot);
        }
    }
}

fn receive_spec_changes(
    mut state: ResMut<TalentState>,
    mut receivers: MessageReceivers<SpecializationChanged>,
) {
    for receiver in receivers.iter_mut() {
        for changed in receiver.receive() {
            state.apply_spec_changed(changed.spec_id);
        }
    }
}

/// A rejected commit keeps the pending config so the player can fix it.
fn receive_commit_results(
    mut errors: ResMut<UiErrors>,
    mut receivers: MessageReceivers<TraitCommitResult>,
) {
    for receiver in receivers.iter_mut() {
        for result in receiver.receive() {
            if !result.ok {
                let reason = result.reason.as_deref().unwrap_or("Talent change failed.");
                errors.add(reason);
            }
        }
    }
}
