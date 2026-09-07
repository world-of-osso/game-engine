use game_engine::network_runtime::messages::{MessageSenders, WorkerMessageSender};
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use shared::protocol::{
    ChatChannel, CombatChannel, EmoteIntent, GroupInviteIntent, GroupUninviteIntent,
    SpellCastIntent, StopSpellCast,
};

use super::{Command, CurrentTarget, DispatchContext, IpcSenderParams, Request, Response};

pub(super) fn dispatch_combat_request(
    cmd: &Command,
    ctx: &DispatchContext,
    sender_params: &mut IpcSenderParams,
) -> bool {
    match &cmd.request {
        Request::SpellCast { spell, target } => {
            handle_spell_cast(
                cmd,
                spell.clone(),
                target.clone(),
                ctx.current_target,
                ctx.connected,
                &sender_params.replication_map,
                &mut sender_params.spell_cast_senders,
            );
        }
        Request::SpellStop => {
            handle_spell_stop(cmd, ctx.connected, &mut sender_params.spell_stop_senders);
        }
        Request::GroupInvite { name } => {
            handle_group_invite(
                cmd,
                name.clone(),
                ctx.connected,
                &mut sender_params.group_invite_senders,
            );
        }
        Request::GroupUninvite { name } => {
            handle_group_uninvite(
                cmd,
                name.clone(),
                ctx.connected,
                &mut sender_params.group_uninvite_senders,
            );
        }
        Request::Emote { emote } => {
            handle_emote(cmd, *emote, ctx.connected, &mut sender_params.emote_senders);
        }
        _ => return false,
    }
    true
}

fn resolve_spell_cast_intent(
    cmd: &Command,
    spell: &str,
    target: Option<&str>,
    current_target: &CurrentTarget,
    map: &ReplicationMirrorMap,
) -> Option<SpellCastIntent> {
    let target_bits = match resolve_network_spell_target(target, current_target, map) {
        Ok(bits) => bits,
        Err(error) => {
            let _ = cmd.respond.send(Response::Error(error));
            return None;
        }
    };
    let (spell_id, spell_token) = match super::super::format::resolve_spell_identifier(spell) {
        Ok(value) => value,
        Err(error) => {
            let _ = cmd.respond.send(Response::Error(error));
            return None;
        }
    };
    Some(SpellCastIntent {
        spell_id,
        spell: spell_token,
        target_entity: target_bits,
    })
}

fn resolve_network_spell_target(
    selector: Option<&str>,
    current_target: &CurrentTarget,
    map: &ReplicationMirrorMap,
) -> Result<Option<u64>, String> {
    let uses_current = selector.is_none_or(|value| value.eq_ignore_ascii_case("current"));
    let server_target = if uses_current {
        current_target
            .0
            .map(|main| {
                map.main_to_server(main)
                    .ok_or_else(|| "spell target is no longer replicated".to_string())
            })
            .transpose()?
    } else {
        None
    };
    super::super::format::resolve_spell_target(selector, &CurrentTarget(server_target))
}

fn handle_spell_cast(
    cmd: &Command,
    spell: String,
    target: Option<String>,
    current_target: &CurrentTarget,
    connected: bool,
    map: &ReplicationMirrorMap,
    senders: &mut MessageSenders<SpellCastIntent>,
) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "spell cast is unavailable: not connected".into(),
        ));
        return;
    }
    let Some(intent) =
        resolve_spell_cast_intent(cmd, &spell, target.as_deref(), current_target, map)
    else {
        return;
    };
    if send_combat_message(senders, intent.clone()) {
        let target_text = intent
            .target_entity
            .map(|bits| bits.to_string())
            .unwrap_or_else(|| "-".into());
        let _ = cmd.respond.send(Response::Text(format!(
            "spell cast submitted spell={} target={target_text}",
            intent.spell
        )));
    } else {
        let _ = cmd.respond.send(Response::Error(
            "spell cast is unavailable: not connected".into(),
        ));
    }
}

fn handle_spell_stop(cmd: &Command, connected: bool, senders: &mut MessageSenders<StopSpellCast>) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "spell stop is unavailable: not connected".into(),
        ));
        return;
    }
    if send_combat_message(senders, StopSpellCast) {
        let _ = cmd
            .respond
            .send(Response::Text("spell stop submitted".into()));
    } else {
        let _ = cmd.respond.send(Response::Error(
            "spell stop is unavailable: not connected".into(),
        ));
    }
}

fn handle_group_invite(
    cmd: &Command,
    name: String,
    connected: bool,
    senders: &mut MessageSenders<GroupInviteIntent>,
) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "group invite is unavailable: not connected".into(),
        ));
    } else if send_combat_message(senders, GroupInviteIntent { name: name.clone() }) {
        let _ = cmd
            .respond
            .send(Response::Text(format!("group invite submitted for {name}")));
    } else {
        let _ = cmd
            .respond
            .send(Response::Error("group invite sender unavailable".into()));
    }
}

fn handle_group_uninvite(
    cmd: &Command,
    name: String,
    connected: bool,
    senders: &mut MessageSenders<GroupUninviteIntent>,
) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "group uninvite is unavailable: not connected".into(),
        ));
    } else if send_combat_message(senders, GroupUninviteIntent { name: name.clone() }) {
        let _ = cmd.respond.send(Response::Text(format!(
            "group uninvite submitted for {name}"
        )));
    } else {
        let _ = cmd
            .respond
            .send(Response::Error("group uninvite sender unavailable".into()));
    }
}

fn handle_emote(
    cmd: &Command,
    emote: shared::protocol::EmoteKind,
    connected: bool,
    senders: &mut MessageSenders<EmoteIntent>,
) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "emote is unavailable: not connected".into(),
        ));
    } else if send_social_message(senders, EmoteIntent { emote }) {
        let _ = cmd
            .respond
            .send(Response::Text(format!("emote submitted {:?}", emote)));
    } else {
        let _ = cmd
            .respond
            .send(Response::Error("emote sender unavailable".into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::World;

    #[test]
    fn unmapped_spell_target_reports_ipc_error_instead_of_building_intent() {
        let mut world = World::new();
        let main = world.spawn_empty().id();
        let (respond, responses) = std::sync::mpsc::channel();
        let command = Command {
            request: Request::SpellCast {
                spell: "123".into(),
                target: None,
            },
            respond,
        };
        let intent = resolve_spell_cast_intent(
            &command,
            "123",
            None,
            &CurrentTarget(Some(main)),
            &ReplicationMirrorMap::default(),
        );
        assert!(intent.is_none());
        let Response::Error(error) = responses.try_recv().unwrap() else {
            panic!("expected unmapped target error");
        };
        assert_eq!(error, "spell target is no longer replicated");
    }

    #[test]
    fn spell_current_targets_map_but_explicit_server_ids_do_not() {
        let mut world = World::new();
        let main = world.spawn_empty().id();
        let server = world.spawn_empty().id();
        let mut map = ReplicationMirrorMap::default();
        map.insert(server, main);
        let target = CurrentTarget(Some(main));
        for selector in [None, Some("current"), Some("CURRENT")] {
            assert_eq!(
                resolve_network_spell_target(selector, &target, &map).unwrap(),
                Some(server.to_bits())
            );
        }
        let explicit = main.to_bits().to_string();
        assert_eq!(
            resolve_network_spell_target(Some(&explicit), &target, &map).unwrap(),
            Some(main.to_bits())
        );
        map.clear();
        assert_eq!(
            resolve_network_spell_target(Some(&explicit), &target, &map).unwrap(),
            Some(main.to_bits())
        );
        assert_eq!(
            resolve_network_spell_target(Some("none"), &target, &map).unwrap(),
            None
        );
        assert_eq!(
            resolve_network_spell_target(None, &target, &map).unwrap_err(),
            "spell target is no longer replicated"
        );
        assert_eq!(
            resolve_network_spell_target(None, &CurrentTarget(None), &map).unwrap_err(),
            "no current target selected"
        );
        assert_eq!(
            resolve_network_spell_target(Some("bogus"), &target, &map).unwrap_err(),
            "invalid target selector 'bogus'"
        );
    }
}

fn send_combat_message<T: Clone + lightyear::prelude::Message>(
    senders: &mut MessageSenders<T>,
    message: T,
) -> bool {
    send_channel_message(senders, message, |sender, message| {
        sender.send::<CombatChannel>(message);
    })
}

fn send_social_message<T: Clone + lightyear::prelude::Message>(
    senders: &mut MessageSenders<T>,
    message: T,
) -> bool {
    send_channel_message(senders, message, |sender, message| {
        sender.send::<ChatChannel>(message);
    })
}

fn send_channel_message<T: Clone + lightyear::prelude::Message>(
    senders: &mut MessageSenders<T>,
    message: T,
    send: impl Fn(&mut WorkerMessageSender<'_, T>, T),
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        send(&mut sender, message.clone());
        sent = true;
    }
    sent
}
