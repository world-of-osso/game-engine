use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::{Message as NetworkMessage, MessageReceiver, MessageSender};
use shared::components::PresenceStatus;
use shared::protocol::{
    AddFriend, FriendsChannel, FriendsStateUpdate, RemoveFriend, SetPresenceStatus,
};

use crate::ipc::{Request, Response};
use crate::status::{FriendEntry, FriendsStatusSnapshot, PresenceStateEntry};

#[derive(Resource, Default)]
pub struct FriendsRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

enum Action {
    Add { name: String },
    Remove { name: String },
    SetPresence { presence: PresenceStatus },
}

pub struct FriendsPlugin;

impl Plugin for FriendsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FriendsRuntimeState>();
        app.add_systems(Update, (send_pending_actions, receive_friends_updates));
    }
}

pub fn queue_ipc_request(
    runtime: &mut FriendsRuntimeState,
    snapshot: &FriendsStatusSnapshot,
    character_stats: &crate::status::CharacterStatsSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    let action = match request {
        Request::FriendsStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            return true;
        }
        Request::FriendAdd { name } => Action::Add { name: name.clone() },
        Request::FriendRemove { name } => Action::Remove { name: name.clone() },
        Request::PresenceAfk => Action::SetPresence {
            presence: PresenceStatus::Afk,
        },
        Request::PresenceDnd => Action::SetPresence {
            presence: PresenceStatus::Dnd,
        },
        Request::PresenceOnline => Action::SetPresence {
            presence: PresenceStatus::Online,
        },
        Request::PresenceStatus => {
            let _ = respond.send(Response::Text(format_presence_status(character_stats)));
            return true;
        }
        _ => return false,
    };
    runtime.pending_actions.push_back(action);
    runtime.pending_replies.push_back(respond);
    true
}

fn format_status(snapshot: &FriendsStatusSnapshot) -> String {
    crate::ipc::format::format_friends_status(snapshot)
}

#[derive(SystemParam)]
struct FriendsSenders<'w, 's> {
    add: Query<'w, 's, &'static mut MessageSender<AddFriend>>,
    remove: Query<'w, 's, &'static mut MessageSender<RemoveFriend>>,
    presence: Query<'w, 's, &'static mut MessageSender<SetPresenceStatus>>,
}

fn send_pending_actions(mut runtime: ResMut<FriendsRuntimeState>, mut senders: FriendsSenders) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::Add { name } => send_all(&mut senders.add, AddFriend { name }),
            Action::Remove { name } => send_all(&mut senders.remove, RemoveFriend { name }),
            Action::SetPresence { presence } => send_all(
                &mut senders.presence,
                SetPresenceStatus { status: presence },
            ),
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(
                "friends are unavailable: not connected".into(),
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
        sender.send::<FriendsChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_friends_updates(
    mut runtime: ResMut<FriendsRuntimeState>,
    mut snapshot: ResMut<FriendsStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<FriendsStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_friends_state_update(&mut snapshot, update);
            if let Some(reply) = runtime.pending_replies.pop_front() {
                let response = if let Some(error) = &snapshot.last_error {
                    Response::Error(error.clone())
                } else {
                    Response::Text(
                        snapshot
                            .last_server_message
                            .clone()
                            .unwrap_or_else(|| "friends updated".into()),
                    )
                };
                let _ = reply.send(response);
            }
        }
    }
}

pub fn apply_friends_state_update(
    snapshot: &mut FriendsStatusSnapshot,
    update: FriendsStateUpdate,
) {
    if let Some(friends_snapshot) = update.snapshot {
        snapshot.entries = friends_snapshot
            .entries
            .into_iter()
            .map(|entry| FriendEntry {
                name: entry.name,
                level: entry.level,
                class_name: entry.class_name,
                area: entry.area,
                online: !matches!(entry.presence, PresenceStatus::Offline),
                presence: map_presence(entry.presence),
                note: entry.note,
            })
            .collect();
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_runtime(runtime: &mut FriendsRuntimeState) {
    *runtime = FriendsRuntimeState::default();
}

fn map_presence(value: PresenceStatus) -> PresenceStateEntry {
    match value {
        PresenceStatus::Online => PresenceStateEntry::Online,
        PresenceStatus::Afk => PresenceStateEntry::Afk,
        PresenceStatus::Dnd => PresenceStateEntry::Dnd,
        PresenceStatus::Offline => PresenceStateEntry::Offline,
    }
}

fn format_presence_status(snapshot: &crate::status::CharacterStatsSnapshot) -> String {
    let presence = match snapshot.presence.as_ref() {
        Some(PresenceStateEntry::Online) => "online",
        Some(PresenceStateEntry::Afk) => "afk",
        Some(PresenceStateEntry::Dnd) => "dnd",
        Some(PresenceStateEntry::Offline) => "offline",
        None => "-",
    };
    format!("presence: {presence}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friends_state_update_populates_status_snapshot() {
        let mut snapshot = FriendsStatusSnapshot::default();

        apply_friends_state_update(
            &mut snapshot,
            FriendsStateUpdate {
                snapshot: Some(shared::protocol::FriendsSnapshot {
                    entries: vec![shared::protocol::FriendCharacterSnapshot {
                        name: "Alice".into(),
                        level: 42,
                        class_name: "Mage".into(),
                        area: "Zone 12".into(),
                        presence: shared::components::PresenceStatus::Online,
                        note: String::new(),
                    }],
                }),
                message: Some("friend added: Alice".into()),
                error: None,
            },
        );

        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].name, "Alice");
        assert!(snapshot.entries[0].online);
        assert_eq!(snapshot.entries[0].presence, PresenceStateEntry::Online);
        assert_eq!(
            snapshot.last_server_message.as_deref(),
            Some("friend added: Alice")
        );
    }
}
