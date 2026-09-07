use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::prelude::*;
use lightyear::prelude::{Message as NetworkMessage, MessageReceiver, MessageSender};
use shared::protocol::{
    GuildChannel, GuildStateUpdate, QueryGuild, SetGuildInfo, SetGuildMotd, SetGuildOfficerNote,
};

use crate::ipc::{Request, Response};
use crate::network_events::{register_message_handler, register_outgoing_handler};
use crate::status::{GuildMemberEntry, GuildStatusSnapshot};

#[derive(Resource, Default)]
pub struct GuildRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

enum Action {
    Query,
    SetMotd { text: String },
    SetInfo { text: String },
    SetOfficerNote { name: String, note: String },
}

pub struct GuildPlugin;

impl Plugin for GuildPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildRuntimeState>();
        app.init_resource::<GuildStatusSnapshot>();
        register_outgoing_handler(app, send_pending_actions, |world| {
            !world
                .resource::<GuildRuntimeState>()
                .pending_actions
                .is_empty()
        });
        register_message_handler::<GuildStateUpdate, _>(app, receive_guild_updates, |_| true);
    }
}

pub fn queue_ipc_request(
    runtime: &mut GuildRuntimeState,
    snapshot: &GuildStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    let action = match request {
        Request::GuildStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            return true;
        }
        Request::GuildQuery => Action::Query,
        Request::GuildSetMotd { text } => Action::SetMotd { text: text.clone() },
        Request::GuildSetInfo { text } => Action::SetInfo { text: text.clone() },
        Request::GuildSetOfficerNote { name, note } => Action::SetOfficerNote {
            name: name.clone(),
            note: note.clone(),
        },
        _ => return false,
    };
    runtime.pending_actions.push_back(action);
    runtime.pending_replies.push_back(respond);
    true
}

pub fn queue_query(runtime: &mut GuildRuntimeState) {
    runtime.pending_actions.push_back(Action::Query);
}

fn send_pending_actions(
    mut runtime: ResMut<GuildRuntimeState>,
    mut query_senders: Query<&mut MessageSender<QueryGuild>>,
    mut motd_senders: Query<&mut MessageSender<SetGuildMotd>>,
    mut info_senders: Query<&mut MessageSender<SetGuildInfo>>,
    mut note_senders: Query<&mut MessageSender<SetGuildOfficerNote>>,
) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::Query => send_all(&mut query_senders, QueryGuild),
            Action::SetMotd { text } => send_all(&mut motd_senders, SetGuildMotd { text }),
            Action::SetInfo { text } => send_all(&mut info_senders, SetGuildInfo { text }),
            Action::SetOfficerNote { name, note } => send_all(
                &mut note_senders,
                SetGuildOfficerNote {
                    character_name: name,
                    note,
                },
            ),
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(
                "guild is unavailable: not connected".into(),
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
        sender.send::<GuildChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_guild_updates(
    mut runtime: ResMut<GuildRuntimeState>,
    mut snapshot: ResMut<GuildStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<GuildStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_guild_state_update(&mut snapshot, update);
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

pub fn apply_guild_state_update(snapshot: &mut GuildStatusSnapshot, update: GuildStateUpdate) {
    if let Some(guild) = update.guild {
        snapshot.guild_id = Some(guild.guild_id);
        snapshot.guild_name = guild.guild_name;
        snapshot.motd = guild.motd;
        snapshot.info_text = guild.info_text;
        snapshot.entries = guild
            .members
            .into_iter()
            .map(|member| GuildMemberEntry {
                character_name: member.character_name,
                level: member.level,
                class_name: member.class_name,
                rank_name: member.rank_name,
                online: member.is_online,
                officer_note: member.officer_note,
                last_online: member.last_online,
            })
            .collect();
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_runtime(runtime: &mut GuildRuntimeState) {
    *runtime = GuildRuntimeState::default();
}

fn format_status(snapshot: &GuildStatusSnapshot) -> String {
    crate::ipc::format::format_guild_status(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_requests_wait_for_dispatch_then_report_disconnection() {
        let mut app = App::new();
        app.add_plugins(GuildPlugin)
            .init_resource::<GuildStatusSnapshot>();
        let (reply, responses) = mpsc::channel();
        {
            let mut runtime = app.world_mut().resource_mut::<GuildRuntimeState>();
            runtime.pending_actions.extend([
                Action::Query,
                Action::SetMotd {
                    text: "Raid tonight".into(),
                },
            ]);
            runtime.pending_replies.extend([reply.clone(), reply]);
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<GuildRuntimeState>()
                .pending_actions
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
            assert_eq!(error, "guild is unavailable: not connected");
        }
        let runtime = app.world().resource::<GuildRuntimeState>();
        assert!(runtime.pending_actions.is_empty());
        assert!(runtime.pending_replies.is_empty());
    }

    #[test]
    fn idle_dispatch_leaves_waiting_replies_untouched() {
        let mut app = App::new();
        app.add_plugins(GuildPlugin);
        let (reply, responses) = mpsc::channel();
        app.world_mut()
            .resource_mut::<GuildRuntimeState>()
            .pending_replies
            .push_back(reply);
        // No inbox means no snapshot is needed by incoming handlers.
        crate::network_events::dispatch_incoming(app.world_mut());
        crate::network_events::dispatch_outgoing(app.world_mut());
        assert_eq!(
            app.world()
                .resource::<GuildRuntimeState>()
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
    fn apply_guild_state_update_maps_members() {
        let mut snapshot = GuildStatusSnapshot::default();
        apply_guild_state_update(
            &mut snapshot,
            GuildStateUpdate {
                guild: Some(shared::protocol::GuildSnapshot {
                    guild_id: 7,
                    guild_name: "Raid Team".into(),
                    motd: "Bring flasks".into(),
                    info_text: "Wed/Sun raids".into(),
                    members: vec![shared::protocol::GuildMemberSnapshot {
                        character_name: "Alice".into(),
                        level: 60,
                        class_name: "Priest".into(),
                        rank_name: "Member".into(),
                        is_online: true,
                        officer_note: "Reliable healer".into(),
                        last_online: "Online".into(),
                    }],
                }),
                message: Some("guild loaded".into()),
                error: None,
            },
        );

        assert_eq!(snapshot.guild_id, Some(7));
        assert_eq!(snapshot.guild_name, "Raid Team");
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].officer_note, "Reliable healer");
        assert_eq!(
            snapshot.last_server_message.as_deref(),
            Some("guild loaded")
        );
    }
}
