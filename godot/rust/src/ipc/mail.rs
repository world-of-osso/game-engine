//! `mail status|send|act` in the original response text (src/ipc/mail.rs:12-52, status
//! text shared from src/ipc/mail_format.rs): send and inbox actions go to the open
//! mailbox as the MailFrame's own requests do, and answer at once.
use game_engine_network::ipc_wire::{Request, Response};
use game_engine_ui_model::mail_format::format_mail_status;
use shared::protocol::SendMail;

impl crate::GameClient {
    pub(crate) fn mail_request(&mut self, request: Request) -> Result<Response, Request> {
        let session = &mut self.mailbox.session;
        let answer = match request {
            Request::MailStatus => Ok(format_mail_status(
                session.object,
                &session.pending_senders,
                session.sent,
                session
                    .contents
                    .as_ref()
                    .map_or(&[], |contents| contents.mails.as_slice()),
            )),
            Request::MailSend { .. } | Request::MailAct { .. } if !session.is_open() => {
                Err("no mailbox is open".into())
            }
            Request::MailSend {
                recipient,
                subject,
                body,
                attachments,
                money,
                cod,
            } => {
                let mail = session.ipc_send(SendMail {
                    object: 0,
                    recipient: recipient.clone(),
                    subject,
                    body,
                    attachments,
                    money,
                    cod,
                });
                mail.map(|mail| self.account.send_mail(mail))
                    .transpose()
                    .map(|_| format!("mail send queued to {recipient}"))
                    .map_err(|error| error.to_string())
            }
            Request::MailAct { mail_id, action } => session
                .ipc_act(mail_id, action)
                .map(|request| self.account.send_mail_request(request))
                .transpose()
                .map(|_| format!("mail {mail_id} {action:?} queued"))
                .map_err(|error| error.to_string()),
            request => return Err(request),
        };
        Ok(match answer {
            Ok(text) => Response::Text(text),
            Err(error) => Response::Error(error),
        })
    }
}
