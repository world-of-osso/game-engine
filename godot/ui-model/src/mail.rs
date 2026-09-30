//! Receiving-only mailbox session. Claims wait for authoritative mailbox/inventory/Gold updates.
use crate::mail_frame_component::{
    self as frame, InboxRow, MailFrameState, OpenAttachment, OpenMailView, ReceivingMailState,
};
use crate::{bag_data::stack_slot, bank_art::SlotItem, merchant_data::quality_color};
use shared::protocol::{
    GAMEOBJECT_TYPE_MAILBOX, GameObjectInfo, MailAction, MailFailed, MailHeader, MailRequest,
    MailboxContents,
};

#[derive(Clone, Debug, PartialEq)]
pub struct NativeMailView {
    pub inbox: ReceivingMailState,
    pub bags: crate::bag_frame_component::BagFrameState,
}

pub fn native_mail_screen(
    ctx: &ui_toolkit::screen::SharedContext,
) -> ui_toolkit::widget_def::Element {
    let view = ctx
        .get::<NativeMailView>()
        .expect("NativeMailView must be in SharedContext");
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(view.inbox.clone());
    shared.insert(view.bags.clone());
    let mut elements = frame::receiving_mail_screen(&shared);
    elements.extend(crate::bag_frame_component::bag_frame_screen(&shared));
    elements
}

const PAGE_SIZE: usize = 7;

pub fn can_use_mailbox(info: &GameObjectInfo, distance: f32) -> bool {
    info.go_type == GAMEOBJECT_TYPE_MAILBOX && distance <= 5.0
}

#[derive(Default)]
pub struct MailSession {
    pub object: Option<u64>,
    expected: Option<u64>,
    pub contents: Option<MailboxContents>,
    pub page: usize,
    pub selected: Option<u64>,
    pending: Option<MailRequest>,
    pub pending_senders: Vec<String>,
}

impl MailSession {
    pub fn is_open(&self) -> bool {
        self.object.is_some()
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Only the real object's UseGameObject request authorizes a later role/contents pair.
    pub fn expect_open(&mut self, object: u64) {
        self.close();
        self.expected = Some(object);
    }
    pub fn open(&mut self, object: u64) {
        if self.expected != Some(object) {
            return;
        }
        self.object = Some(object);
    }
    pub fn close(&mut self) {
        self.object = None;
        self.expected = None;
        self.contents = None;
        self.page = 0;
        self.selected = None;
        self.pending = None;
    }
    pub fn close_for(&mut self, object: u64) {
        if self.object == Some(object) || self.expected == Some(object) {
            self.close();
        }
    }
    pub fn reset(&mut self) {
        self.close();
        self.pending_senders.clear();
    }
    pub fn receive_contents(&mut self, contents: MailboxContents) {
        if self.expected != Some(contents.object) {
            return;
        }
        self.pending = None;
        if self
            .selected
            .is_some_and(|id| !contents.mails.iter().any(|m| m.mail_id == id))
        {
            self.selected = None;
        }
        self.page = self
            .page
            .min(contents.mails.len().div_ceil(PAGE_SIZE).saturating_sub(1));
        self.contents = Some(contents);
    }
    pub fn failed(&mut self, failed: MailFailed) -> Option<&'static str> {
        if self.expected != Some(failed.object) {
            return None;
        }
        self.pending = None;
        Some(failed.error.message())
    }
    fn mails(&self) -> &[MailHeader] {
        self.contents.as_ref().map_or(&[], |c| c.mails.as_slice())
    }
    fn selected_mail(&self) -> Option<&MailHeader> {
        self.mails()
            .iter()
            .find(|m| Some(m.mail_id) == self.selected)
    }

    pub fn click(&mut self, action: &str) -> Option<MailRequest> {
        let object = self.object?;
        if action == frame::ACTION_OPEN_CLOSE {
            self.selected = None;
            return None;
        }
        if action == frame::ACTION_PREV {
            self.page = self.page.saturating_sub(1);
            return None;
        }
        if action == frame::ACTION_NEXT {
            if self.page + 1 < self.mails().len().div_ceil(PAGE_SIZE) {
                self.page += 1;
            }
            return None;
        }
        if self.busy() {
            return None;
        }
        let (mail_id, action) = if let Some(id) = action
            .strip_prefix(frame::ACTION_OPEN_PREFIX)
            .and_then(|id| id.parse::<u64>().ok())
        {
            let read = self.mails().iter().find(|m| m.mail_id == id)?.read;
            self.selected = Some(id);
            if read {
                return None;
            }
            (id, MailAction::MarkRead)
        } else {
            let mail = self.selected_mail()?;
            let operation = if action == frame::ACTION_TAKE_MONEY && mail.money > 0 {
                MailAction::TakeMoney
            } else if let Some(slot) = action
                .strip_prefix(frame::ACTION_TAKE_ITEM_PREFIX)
                .and_then(|s| s.parse::<u8>().ok())
            {
                if mail.cod > 0 || !mail.attachments.iter().any(|a| a.slot == slot) {
                    return None;
                }
                MailAction::TakeAttachment { slot }
            } else {
                return None;
            };
            (mail.mail_id, operation)
        };
        let request = MailRequest {
            object,
            mail_id,
            action,
        };
        self.pending = Some(request);
        Some(request)
    }

    pub fn view(&self, money: u64) -> ReceivingMailState {
        let now = self.contents.as_ref().map_or(0, |c| c.now);
        let rows = self
            .mails()
            .iter()
            .skip(self.page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .map(|mail| {
                let first = mail.attachments.first();
                let left = mail.expires_at.saturating_sub(now);
                InboxRow {
                    mail_id: mail.mail_id,
                    sender: mail.sender.clone(),
                    subject: mail.subject.clone(),
                    icon_fdid: first.map_or(134327, |a| stack_slot(&a.item).icon_fdid),
                    count: first.map_or(0, |a| a.item.count),
                    read: mail.read,
                    cod: mail.cod > 0,
                    expires: time_left(left),
                    expires_soon: left < 86400,
                    selected: self.selected == Some(mail.mail_id),
                }
            })
            .collect();
        let open = self.selected_mail().map(|mail| OpenMailView {
            sender: mail.sender.clone(),
            subject: mail.subject.clone(),
            body: mail.body.clone(),
            money: mail.money,
            cod: mail.cod,
            attachments: mail
                .attachments
                .iter()
                .map(|a| OpenAttachment {
                    slot: a.slot,
                    item: SlotItem {
                        icon_fdid: stack_slot(&a.item).icon_fdid,
                        count: a.item.count,
                        quality_border: quality_color(a.quality).into(),
                    },
                })
                .collect(),
            can_reply: false,
            can_delete: mail.can_delete(),
        });
        ReceivingMailState {
            frame: MailFrameState {
                visible: self.is_open(),
                rows,
                page: self.page,
                page_count: self.mails().len().div_ceil(PAGE_SIZE).max(1),
                open,
                money,
                ..Default::default()
            },
            busy: self.busy(),
        }
    }
}
fn time_left(left: u64) -> String {
    if left >= 86400 {
        let days = left / 86400;
        format!("{days} {}", if days == 1 { "Day" } else { "Days" })
    } else if left >= 3600 {
        format!("{} Hr", left / 3600)
    } else {
        format!("{} Min", (left / 60).max(1))
    }
}
