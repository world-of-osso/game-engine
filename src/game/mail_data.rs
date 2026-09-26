//! Client mail state: the open mailbox's `MailboxContents`, the tab, inbox page and
//! open mail the `MailFrame` shows, the Send Mail attachments and money mode, and
//! the unread senders of `PendingMail` for the minimap indicator. Frame and IPC
//! actions become [`MailRequest`]s the network layer sends to the open mailbox.

use bevy::prelude::*;
use shared::protocol::{MAX_MAIL_ATTACHMENTS, MailAction, MailHeader, MailboxContents};

/// Retail `INBOXITEMS_TO_DISPLAY` (MailFrame.lua:1).
pub const INBOX_PAGE_SIZE: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MailTab {
    #[default]
    Inbox,
    Send,
}

/// `SendMailSendMoneyButton` / `SendMailCODButton`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SendMoneyMode {
    #[default]
    Money,
    Cod,
}

/// What the Send Mail tab posts (Retail `SendMail(recipient, subject, body)`).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MailDraft {
    pub recipient: String,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<u64>,
    pub money: u64,
    pub cod: u64,
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub enum MailRequest {
    Send(MailDraft),
    Act { mail_id: u64, action: MailAction },
}

#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct MailState {
    /// Server entity bits of the open mailbox; `None` = closed.
    pub object: Option<u64>,
    pub contents: Option<MailboxContents>,
    pub tab: MailTab,
    /// Inbox page, 0-based.
    pub page: usize,
    /// The mail shown in `OpenMailFrame`.
    pub open_mail: Option<u64>,
    /// Bag item guids on the Send Mail attachment buttons, in button order.
    pub attachments: Vec<u64>,
    pub money_mode: SendMoneyMode,
    /// `PendingMail` senders: the minimap mail indicator shows while any.
    pub pending_senders: Vec<String>,
    /// Mails sent since the mailbox opened (`MAIL_SEND_SUCCESS`).
    pub sent: u32,
}

impl MailState {
    pub fn is_open(&self) -> bool {
        self.object.is_some()
    }

    /// The mailbox role opened. The inbox may already be here (it can arrive in the
    /// same network batch as `InteractionOpened`), so it is kept.
    pub fn open(&mut self, object: u64) {
        let contents = self
            .contents
            .take()
            .filter(|contents| contents.object == object);
        *self = Self {
            object: Some(object),
            contents,
            pending_senders: std::mem::take(&mut self.pending_senders),
            ..Self::default()
        };
    }

    pub fn close(&mut self) {
        *self = Self {
            pending_senders: std::mem::take(&mut self.pending_senders),
            ..Self::default()
        };
    }

    pub fn apply(&mut self, contents: MailboxContents) {
        let count = contents.mails.len();
        if self
            .open_mail
            .is_some_and(|id| !contents.mails.iter().any(|mail| mail.mail_id == id))
        {
            self.open_mail = None;
        }
        self.contents = Some(contents);
        self.page = self.page.min(page_count(count).saturating_sub(1));
    }

    pub fn mails(&self) -> &[MailHeader] {
        self.contents
            .as_ref()
            .map_or(&[], |contents| contents.mails.as_slice())
    }

    pub fn page_count(&self) -> usize {
        page_count(self.mails().len())
    }

    /// The inbox rows of the current page.
    pub fn page_mails(&self) -> &[MailHeader] {
        let mails = self.mails();
        let start = (self.page * INBOX_PAGE_SIZE).min(mails.len());
        let end = (start + INBOX_PAGE_SIZE).min(mails.len());
        &mails[start..end]
    }

    pub fn find(&self, mail_id: u64) -> Option<&MailHeader> {
        self.mails().iter().find(|mail| mail.mail_id == mail_id)
    }

    pub fn opened(&self) -> Option<&MailHeader> {
        self.find(self.open_mail?)
    }

    pub fn next_page(&mut self) {
        if self.page + 1 < self.page_count() {
            self.page += 1;
        }
    }

    pub fn prev_page(&mut self) {
        self.page = self.page.saturating_sub(1);
    }

    /// A bag item right-clicked while Send Mail shows: the next free button.
    /// Returns false when it is already attached or all 12 are used.
    pub fn attach(&mut self, item_guid: u64) -> bool {
        if self.attachments.contains(&item_guid) || self.attachments.len() >= MAX_MAIL_ATTACHMENTS {
            return false;
        }
        self.attachments.push(item_guid);
        true
    }

    pub fn detach(&mut self, index: usize) {
        if index < self.attachments.len() {
            self.attachments.remove(index);
        }
    }

    /// `GetSendMailPrice`.
    pub fn postage(&self) -> u64 {
        shared::mail::postage(self.attachments.len())
    }

    /// `MAIL_SEND_SUCCESS` clears the Send Mail form's items and money mode.
    pub fn mail_sent(&mut self) {
        self.attachments.clear();
        self.money_mode = SendMoneyMode::Money;
        self.sent += 1;
    }
}

fn page_count(mails: usize) -> usize {
    mails.div_ceil(INBOX_PAGE_SIZE).max(1)
}

#[cfg(test)]
#[path = "mail_data_tests.rs"]
mod tests;
