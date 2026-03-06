use std::sync::mpsc;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ipc::{Request, Response};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendMail {
    pub to: String,
    pub from: String,
    pub subject: String,
    pub body: String,
    pub money: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListMailQuery {
    pub character: Option<String>,
    pub include_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadMail {
    pub mail_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteMail {
    pub mail_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimMail {
    pub mail_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MailDelivery {
    pub mail_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimedMail {
    pub mail_id: u64,
    pub money: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MailEntry {
    pub mail_id: u64,
    pub to: String,
    pub from: String,
    pub subject: String,
    pub body: String,
    pub money: u64,
    pub read: bool,
    pub claimed: bool,
    pub deleted: bool,
}

#[derive(Resource, Debug, Default, Clone, Serialize, Deserialize)]
pub struct MailState {
    next_mail_id: u64,
    entries: Vec<MailEntry>,
}

pub struct MailPlugin;

impl Plugin for MailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MailState>();
    }
}

impl MailState {
    pub fn send(&mut self, mail: SendMail) -> Result<MailDelivery, String> {
        if mail.to.trim().is_empty() || mail.from.trim().is_empty() || mail.subject.trim().is_empty()
        {
            return Err("to, from, and subject are required".into());
        }

        let mail_id = self.next_id();
        self.entries.push(MailEntry {
            mail_id,
            to: mail.to,
            from: mail.from,
            subject: mail.subject,
            body: mail.body,
            money: mail.money,
            read: false,
            claimed: false,
            deleted: false,
        });
        Ok(MailDelivery { mail_id })
    }

    pub fn list(&self, query: ListMailQuery) -> Vec<MailEntry> {
        self.entries
            .iter()
            .filter(|entry| query.include_deleted || !entry.deleted)
            .filter(|entry| {
                query
                    .character
                    .as_ref()
                    .is_none_or(|character| entry.to.eq_ignore_ascii_case(character))
            })
            .cloned()
            .collect()
    }

    pub fn read(&mut self, request: ReadMail) -> Result<MailEntry, String> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.mail_id == request.mail_id && !entry.deleted)
            .ok_or_else(|| "mail not found".to_string())?;
        entry.read = true;
        Ok(entry.clone())
    }

    pub fn claim(&mut self, mail_id: u64) -> Result<ClaimedMail, String> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.mail_id == mail_id && !entry.deleted)
            .ok_or_else(|| "mail not found".to_string())?;
        if entry.claimed {
            return Err("mail already claimed".into());
        }
        entry.claimed = true;
        Ok(ClaimedMail {
            mail_id: entry.mail_id,
            money: entry.money,
        })
    }

    pub fn delete(&mut self, request: DeleteMail) -> Result<(), String> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.mail_id == request.mail_id && !entry.deleted)
            .ok_or_else(|| "mail not found".to_string())?;
        entry.deleted = true;
        Ok(())
    }

    fn next_id(&mut self) -> u64 {
        self.next_mail_id += 1;
        self.next_mail_id
    }
}

pub fn queue_ipc_request(
    state: &mut MailState,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    let response = match request {
        Request::MailStatus => Response::Text(format_status(state)),
        Request::MailSend { mail } => match state.send(mail.clone()) {
            Ok(delivery) => Response::Text(format!(
                "mail sent: id={} to={} subject={}",
                delivery.mail_id, mail.to, mail.subject
            )),
            Err(error) => Response::Error(error),
        },
        Request::MailList { query } => Response::Text(format_mail_list(&state.list(query.clone()))),
        Request::MailRead { read } => match state.read(read.clone()) {
            Ok(entry) => Response::Text(format_mail_read(&entry)),
            Err(error) => Response::Error(error),
        },
        Request::MailClaim { claim } => match state.claim(claim.mail_id) {
            Ok(entry) => Response::Text(format!(
                "mail claimed: id={} money={}",
                entry.mail_id, entry.money
            )),
            Err(error) => Response::Error(error),
        },
        Request::MailDelete { delete } => match state.delete(delete.clone()) {
            Ok(()) => Response::Text(format!("mail deleted: id={}", delete.mail_id)),
            Err(error) => Response::Error(error),
        },
        _ => return false,
    };

    let _ = respond.send(response);
    true
}

fn format_status(state: &MailState) -> String {
    let total = state.entries.len();
    let unread = state
        .entries
        .iter()
        .filter(|entry| !entry.read && !entry.deleted)
        .count();
    let deleted = state.entries.iter().filter(|entry| entry.deleted).count();
    let claimable_money: u64 = state
        .entries
        .iter()
        .filter(|entry| !entry.deleted && !entry.claimed)
        .map(|entry| entry.money)
        .sum();
    format!(
        "mail_total: {total}\nunread: {unread}\ndeleted: {deleted}\nclaimable_money: {claimable_money}"
    )
}

fn format_mail_list(entries: &[MailEntry]) -> String {
    if entries.is_empty() {
        return "mailbox: 0\n-".into();
    }

    let lines = entries
        .iter()
        .map(|entry| {
            format!(
                "{} to={} from={} subject={} money={} read={} claimed={} deleted={}",
                entry.mail_id,
                entry.to,
                entry.from,
                entry.subject,
                entry.money,
                entry.read,
                entry.claimed,
                entry.deleted
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("mailbox: {}\n{lines}", entries.len())
}

fn format_mail_read(entry: &MailEntry) -> String {
    format!(
        "id: {}\nto: {}\nfrom: {}\nsubject: {}\nmoney: {}\nread: {}\nclaimed: {}\ndeleted: {}\nbody:\n{}",
        entry.mail_id,
        entry.to,
        entry.from,
        entry.subject,
        entry.money,
        entry.read,
        entry.claimed,
        entry.deleted,
        entry.body
    )
}
