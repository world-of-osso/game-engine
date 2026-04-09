use bevy::prelude::*;
use lightyear::prelude::MessageSender;
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
) -> Option<SpellCastIntent> {
    let target_bits = match super::super::format::resolve_spell_target(target, current_target) {
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

fn handle_spell_cast(
    cmd: &Command,
    spell: String,
    target: Option<String>,
    current_target: &CurrentTarget,
    connected: bool,
    senders: &mut Query<&mut MessageSender<SpellCastIntent>>,
) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "spell cast is unavailable: not connected".into(),
        ));
        return;
    }
    let Some(intent) = resolve_spell_cast_intent(cmd, &spell, target.as_deref(), current_target)
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

fn handle_spell_stop(
    cmd: &Command,
    connected: bool,
    senders: &mut Query<&mut MessageSender<StopSpellCast>>,
) {
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
    senders: &mut Query<&mut MessageSender<GroupInviteIntent>>,
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
    senders: &mut Query<&mut MessageSender<GroupUninviteIntent>>,
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
    senders: &mut Query<&mut MessageSender<EmoteIntent>>,
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

fn send_combat_message<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    send_channel_message(senders, message, |sender, message| {
        sender.send::<CombatChannel>(message);
    })
}

fn send_social_message<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    send_channel_message(senders, message, |sender, message| {
        sender.send::<ChatChannel>(message);
    })
}

fn send_channel_message<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
    send: impl Fn(&mut MessageSender<T>, T),
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        send(&mut sender, message.clone());
        sent = true;
    }
    sent
}
