//! Mail networking: the mailbox role (`NpcFrameEvent::Opened` / `Mailbox`) opens
//! [`MailState`] and `MailboxContents` fill it; `MailSent` clears the Send Mail form
//! (`ERR_MAIL_SENT`), refusals show their Retail error text, `PendingMail` feeds the
//! minimap indicator, the end of the interaction closes the frame, and frame and IPC
//! actions ([`MailRequest`]) go to the open mailbox.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::mail_data::{MailRequest, MailState};
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::quest_runtime::NpcFrameEvent;
use game_engine::ui::ui_errors::UiErrors;
use shared::protocol::{
    MailChannel, MailFailed, MailRequest as MailRequestMessage, MailSent, MailboxContents, NpcRole,
    PendingMail, SendMail,
};

use crate::game_state::GameState;

/// Retail `ERR_MAIL_SENT`.
const ERR_MAIL_SENT: &str = "Mail sent.";

pub struct MailNetworkPlugin;

impl Plugin for MailNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<MailState>()
            .init_resource::<UiErrors>()
            .add_message::<MailRequest>()
            .add_message::<NpcFrameEvent>();
        let handler = register_message_handler::<MailboxContents, _>(app, receive_mail, in_world);
        add_message_route::<MailSent>(app, handler);
        add_message_route::<MailFailed>(app, handler);
        add_message_route::<PendingMail>(app, handler);
        app.add_systems(
            Update,
            (follow_interactions, send_mail_requests)
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(OnExit(GameState::InWorld), reset_mail);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

#[derive(SystemParam)]
struct MailReceivers<'w, 's> {
    contents: MessageReceivers<'w, 's, MailboxContents>,
    sent: MessageReceivers<'w, 's, MailSent>,
    failures: MessageReceivers<'w, 's, MailFailed>,
    pending: MessageReceivers<'w, 's, PendingMail>,
}

fn receive_mail(
    mut receivers: MailReceivers,
    mut mail: ResMut<MailState>,
    mut errors: ResMut<UiErrors>,
) {
    for inbox in receivers.pending.iter_mut() {
        for pending in inbox.receive() {
            mail.pending_senders = pending.senders;
        }
    }
    for inbox in receivers.contents.iter_mut() {
        for contents in inbox.receive() {
            mail.apply(contents);
        }
    }
    for inbox in receivers.sent.iter_mut() {
        for _ in inbox.receive() {
            mail.mail_sent();
            errors.add(ERR_MAIL_SENT);
        }
    }
    for inbox in receivers.failures.iter_mut() {
        for failed in inbox.receive() {
            errors.add(failed.error.message());
        }
    }
}

/// The mailbox role opens the frame; the end of the interaction closes it.
fn follow_interactions(mut events: MessageReader<NpcFrameEvent>, mut mail: ResMut<MailState>) {
    for event in events.read() {
        match *event {
            NpcFrameEvent::Opened {
                npc,
                role: NpcRole::Mailbox,
            } => mail.open(npc),
            NpcFrameEvent::Closed { npc } if mail.object == Some(npc) => mail.close(),
            _ => {}
        }
    }
}

#[derive(SystemParam)]
struct MailSenders<'w, 's> {
    send: MessageSenders<'w, 's, SendMail>,
    act: MessageSenders<'w, 's, MailRequestMessage>,
}

fn send_mail_requests(
    mut requests: MessageReader<MailRequest>,
    mail: Res<MailState>,
    mut senders: MailSenders,
) {
    let Some(object) = mail.object else {
        requests.clear();
        return;
    };
    for request in requests.read() {
        match request.clone() {
            MailRequest::Send(draft) => {
                let message = SendMail {
                    object,
                    recipient: draft.recipient,
                    subject: draft.subject,
                    body: draft.body,
                    attachments: draft.attachments,
                    money: draft.money,
                    cod: draft.cod,
                };
                for mut sender in senders.send.iter_mut() {
                    sender.send::<MailChannel>(message.clone());
                }
            }
            MailRequest::Act { mail_id, action } => {
                let message = MailRequestMessage {
                    object,
                    mail_id,
                    action,
                };
                for mut sender in senders.act.iter_mut() {
                    sender.send::<MailChannel>(message);
                }
            }
        }
    }
}

fn reset_mail(mut mail: ResMut<MailState>) {
    *mail = MailState::default();
}

#[cfg(test)]
#[path = "mail_tests.rs"]
mod tests;
