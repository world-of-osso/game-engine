//! IPC mail commands: `mail status` reads [`MailState`]; `mail send` and the inbox
//! actions write the same [`MailRequest`]s the MailFrame sends to the open mailbox.

use std::sync::mpsc;

use bevy::prelude::*;

use crate::ipc::{Request, Response};
use crate::mail_data::{MailDraft, MailRequest, MailState};

#[path = "mail_format.rs"]
mod mail_format;

pub(crate) fn queue_mail_ipc_request(
    state: &MailState,
    requests: &mut MessageWriter<MailRequest>,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    let response = match request {
        Request::MailStatus => Response::Text(format_mail_status(state)),
        Request::MailSend { .. } | Request::MailAct { .. } if !state.is_open() => {
            Response::Error("no mailbox is open".into())
        }
        Request::MailSend {
            recipient,
            subject,
            body,
            attachments,
            money,
            cod,
        } => {
            requests.write(MailRequest::Send(MailDraft {
                recipient: recipient.clone(),
                subject: subject.clone(),
                body: body.clone(),
                attachments: attachments.clone(),
                money: *money,
                cod: *cod,
            }));
            Response::Text(format!("mail send queued to {recipient}"))
        }
        Request::MailAct { mail_id, action } => {
            requests.write(MailRequest::Act {
                mail_id: *mail_id,
                action: *action,
            });
            Response::Text(format!("mail {mail_id} {action:?} queued"))
        }
        _ => return false,
    };
    let _ = respond.send(response);
    true
}

pub(crate) fn format_mail_status(state: &MailState) -> String {
    mail_format::format_mail_status(
        state.object,
        &state.pending_senders,
        state.sent,
        state.mails(),
    )
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use shared::protocol::{ItemStack, MailAction, MailAttachment, MailHeader, MailboxContents};

    use super::*;

    fn opened() -> MailState {
        let mut state = MailState::default();
        state.open(9);
        state.pending_senders = vec!["Tradea".into()];
        state.apply(MailboxContents {
            object: 9,
            mails: vec![MailHeader {
                mail_id: 4,
                sender: "Tradea".into(),
                subject: "Linen".into(),
                body: String::new(),
                money: 5_000,
                cod: 0,
                attachments: vec![MailAttachment {
                    slot: 0,
                    item: ItemStack {
                        item_guid: 90,
                        item_id: 2589,
                        count: 20,
                        durability: None,
                        soulbound: false,
                    },
                    name: "Linen Cloth".into(),
                    quality: 1,
                }],
                expires_at: 1_800_000_000,
                read: false,
                returned: false,
                from_player: true,
                returnable: true,
            }],
            now: 0,
        });
        state
    }

    fn handle(app: &mut App, request: Request) -> Response {
        let (respond, replies) = mpsc::channel();
        app.world_mut()
            .run_system_once(
                move |state: Res<MailState>, mut requests: MessageWriter<MailRequest>| {
                    assert!(queue_mail_ipc_request(
                        &state,
                        &mut requests,
                        &request,
                        respond.clone()
                    ));
                },
            )
            .unwrap();
        replies.try_recv().unwrap()
    }

    #[test]
    fn status_lists_the_inbox_and_actions_become_mail_requests() {
        let mut app = App::new();
        app.add_message::<MailRequest>().insert_resource(opened());

        let Response::Text(status) = handle(&mut app, Request::MailStatus) else {
            panic!("status is text");
        };
        assert!(status.contains("pending_mail: Tradea"));
        assert!(status.contains("4 from=Tradea subject=Linen money=5000 cod=0 items=[0:2589x20]"));

        handle(
            &mut app,
            Request::MailAct {
                mail_id: 4,
                action: MailAction::TakeAttachment { slot: 0 },
            },
        );
        let requests: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<MailRequest>>()
            .drain()
            .collect();
        assert_eq!(
            requests,
            vec![MailRequest::Act {
                mail_id: 4,
                action: MailAction::TakeAttachment { slot: 0 }
            }]
        );
    }

    #[test]
    fn actions_need_an_open_mailbox() {
        let mut app = App::new();
        app.add_message::<MailRequest>()
            .insert_resource(MailState::default());
        let response = handle(
            &mut app,
            Request::MailAct {
                mail_id: 4,
                action: MailAction::TakeMoney,
            },
        );
        assert!(matches!(response, Response::Error(text) if text == "no mailbox is open"));
    }
}
