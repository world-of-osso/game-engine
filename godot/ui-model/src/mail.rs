//! Native mailbox session (Retail `Blizzard_MailFrame`): the open mailbox's inbox,
//! the Send Mail form and its attachments, the C.O.D. / delete confirmations and
//! Open All. One mail request is in flight at a time; every change waits for the
//! server's `MailboxContents`, `MailSent`, `MailFailed`, Gold and inventory updates.
use std::collections::{HashMap, HashSet};

use crate::bag_data::{InventorySlot, InventoryState, stack_slot};
use crate::bank_art::SlotItem;
use crate::mail_frame_component::{
    self as frame, BODY_BOX, InboxRow, MONEY_BOXES, MailFrameState, MailFrameTab, OpenAttachment,
    OpenMailView, SEND_ATTACHMENTS, SUBJECT_BOX, SendView, TO_BOX,
};
use crate::merchant_data::quality_color;
use crate::popup::PopupSpec;
use shared::protocol::{
    GAMEOBJECT_TYPE_MAILBOX, GameObjectInfo, MAIL_BODY_MAX_LETTERS, MAIL_SUBJECT_MAX_LETTERS,
    MAX_MAIL_ATTACHMENTS, MailAction, MailError, MailFailed, MailHeader, MailRequest, MailSent,
    MailboxContents, SendMail,
};

#[derive(Clone, Debug, PartialEq)]
pub struct NativeMailView {
    pub frame: MailFrameState,
    pub bags: crate::bag_frame_component::BagFrameState,
}

pub fn native_mail_screen(
    ctx: &ui_toolkit::screen::SharedContext,
) -> ui_toolkit::widget_def::Element {
    let view = ctx
        .get::<NativeMailView>()
        .expect("NativeMailView must be in SharedContext");
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(view.frame.clone());
    shared.insert(view.bags.clone());
    let mut elements = frame::mail_frame_screen(&shared);
    elements.extend(crate::bag_frame_component::bag_frame_screen(&shared));
    elements
}

/// Retail `INBOXITEMS_TO_DISPLAY`.
const PAGE_SIZE: usize = 7;
/// `Interface\Icons\INV_Misc_Note_01`, the stationery icon of a mail without items.
const STATIONERY_ICON: u32 = 134_327;
/// `MAX_COD_AMOUNT` gold (MF.lua:1121).
pub const MAX_COD_COPPER: u64 = 10_000 * 10_000;
/// `SendMailNameEditBox` letters (MF.xml:527).
pub const TO_BOX_LETTERS: usize = 77;

/// `StaticPopupDialogs` keys (GameDialogDefs.lua:935-1000).
pub const COD_ALERT_POPUP: &str = "COD_ALERT";
pub const COD_POPUP: &str = "COD_CONFIRMATION";
pub const DELETE_MAIL_POPUP: &str = "DELETE_MAIL";
pub const DELETE_MONEY_POPUP: &str = "DELETE_MONEY";
pub const POPUP_KEYS: [&str; 4] = [
    COD_ALERT_POPUP,
    COD_POPUP,
    DELETE_MAIL_POPUP,
    DELETE_MONEY_POPUP,
];

/// `ERR_MAIL_SENT`.
pub const MAIL_SENT_TEXT: &str = "Mail sent.";
/// `ERR_MAIL_INVALID_ATTACHMENT_SLOT`.
const ATTACHMENTS_FULL_TEXT: &str = "You cannot attach more than 12 items to mail.";

/// Text of the Send Mail edit boxes, read from the registry.
pub type MailTexts = HashMap<&'static str, String>;

pub fn input_names() -> Vec<&'static str> {
    let mut names = vec![TO_BOX, SUBJECT_BOX, BODY_BOX];
    names.extend(MONEY_BOXES.all());
    names
}

/// Retail `letters` limit of each Send Mail edit box.
pub fn input_letters(name: &str) -> Option<usize> {
    match name {
        TO_BOX => Some(TO_BOX_LETTERS),
        SUBJECT_BOX => Some(MAIL_SUBJECT_MAX_LETTERS),
        BODY_BOX => Some(MAIL_BODY_MAX_LETTERS),
        _ if MONEY_BOXES.all().contains(&name) => {
            Some(if name == MONEY_BOXES.gold { 7 } else { 2 })
        }
        _ => None,
    }
}

fn text<'a>(texts: &'a MailTexts, name: &str) -> &'a str {
    texts.get(name).map_or("", String::as_str)
}

fn money_input(texts: &MailTexts) -> u64 {
    let part = |name| text(texts, name).trim().parse::<u64>().unwrap_or(0);
    part(MONEY_BOXES.gold)
        .saturating_mul(10_000)
        .saturating_add(part(MONEY_BOXES.silver).saturating_mul(100))
        .saturating_add(part(MONEY_BOXES.copper))
}

/// Retail `GetMoneyString` without coin textures.
pub fn money_text(copper: u64) -> String {
    let parts: Vec<String> = [
        (copper / 10_000, "g"),
        (copper / 100 % 100, "s"),
        (copper % 100, "c"),
    ]
    .iter()
    .filter(|(amount, _)| *amount > 0)
    .map(|(amount, unit)| format!("{amount}{unit}"))
    .collect();
    if parts.is_empty() {
        "0c".into()
    } else {
        parts.join(" ")
    }
}

pub fn can_use_mailbox(info: &GameObjectInfo, distance: f32) -> bool {
    info.go_type == GAMEOBJECT_TYPE_MAILBOX && distance <= 5.0
}

/// A message for the open mailbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailOutgoing {
    Request(MailRequest),
    Send(SendMail),
}

/// What a frame input asks the host to do.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MailEffect {
    pub outgoing: Option<MailOutgoing>,
    pub popup: Option<PopupSpec>,
    /// Edit box texts to set.
    pub edits: Vec<(&'static str, String)>,
    /// UI error text.
    pub error: Option<&'static str>,
}

impl MailEffect {
    fn request(request: MailRequest) -> Self {
        Self {
            outgoing: Some(MailOutgoing::Request(request)),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Pending {
    Request(MailRequest),
    Send,
}

/// Attachments Open All gave up on after a failure (`OpenAllMailMixin:AddFailedItem`):
/// `None` is the mail's money.
type Skipped = HashSet<(u64, Option<u8>)>;

#[derive(Default)]
pub struct MailSession {
    pub object: Option<u64>,
    expected: Option<u64>,
    pub contents: Option<MailboxContents>,
    pub page: usize,
    pub selected: Option<u64>,
    pending: Option<Pending>,
    pub pending_senders: Vec<String>,
    pub tab: MailFrameTab,
    /// Bag item guids on the Send Mail attachment buttons, in button order.
    pub attachments: Vec<u64>,
    /// `SendMailCODButton` checked.
    pub cod_mode: bool,
    /// The confirmation popup on screen for the open mail and the request it stands for.
    confirm: Option<(&'static str, MailRequest)>,
    open_all: Option<Skipped>,
    /// `SendMailFrame.sendMode == "reply"`: a sent reply returns to the inbox.
    replying: bool,
    /// The subject Send Mail filled in from the first attachment (`previousItem`).
    item_subject: Option<String>,
    /// Mails sent since the mailbox opened (`MAIL_SEND_SUCCESS`; IPC `mail status`).
    pub sent: u32,
}

impl MailSession {
    pub fn is_open(&self) -> bool {
        self.object.is_some()
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn opening_all(&self) -> bool {
        self.open_all.is_some()
    }
    /// The confirmation popup that still answers for the open mail.
    pub fn confirming(&self) -> Option<&'static str> {
        self.confirm.map(|(key, _)| key)
    }
    /// Attached bag items are locked in the bags.
    pub fn is_attached(&self, item_guid: u64) -> bool {
        self.attachments.contains(&item_guid)
    }
    /// Opening another mail, or none, drops the open mail's confirmation
    /// (`OpenMailFrame_OnHide` hides `DELETE_MAIL`).
    fn select(&mut self, mail_id: Option<u64>) {
        if self.selected != mail_id {
            self.confirm = None;
        }
        self.selected = mail_id;
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
        let pending_senders = std::mem::take(&mut self.pending_senders);
        *self = Self {
            pending_senders,
            ..Self::default()
        };
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
        // The server also resends the inbox on its own (delivery, expiry, other
        // senders); only contents showing the request done answer it.
        if let Some(Pending::Request(request)) = self.pending
            && request_done(request, &contents.mails)
        {
            self.pending = None;
        }
        if self
            .selected
            .is_some_and(|id| !contents.mails.iter().any(|m| m.mail_id == id))
        {
            self.select(None);
        }
        self.page = self
            .page
            .min(contents.mails.len().div_ceil(PAGE_SIZE).saturating_sub(1));
        self.contents = Some(contents);
    }

    /// `MAIL_SEND_SUCCESS`: `SendMailFrame_Reset`, and a sent reply goes back to the
    /// inbox (MF.lua:96-101).
    pub fn mail_sent(&mut self, sent: MailSent) -> Option<MailEffect> {
        if self.expected != Some(sent.object) || self.pending != Some(Pending::Send) {
            return None;
        }
        self.pending = None;
        self.sent += 1;
        self.attachments.clear();
        self.cod_mode = false;
        self.item_subject = None;
        if std::mem::take(&mut self.replying) {
            self.tab = MailFrameTab::Inbox;
        }
        Some(MailEffect {
            edits: cleared_form(),
            error: Some(MAIL_SENT_TEXT),
            ..MailEffect::default()
        })
    }

    /// `MAIL_FAILED`: the Retail text. Open All skips what failed and stops once the
    /// bags are full.
    pub fn failed(&mut self, failed: MailFailed) -> Option<&'static str> {
        if self.expected != Some(failed.object) {
            return None;
        }
        let pending = self.pending.take();
        if let Some(skipped) = self.open_all.as_mut() {
            match (failed.error, pending) {
                (MailError::InventoryFull, _) => self.open_all = None,
                (_, Some(Pending::Request(request))) => match request.action {
                    MailAction::TakeMoney => {
                        skipped.insert((request.mail_id, None));
                    }
                    MailAction::TakeAttachment { slot } => {
                        skipped.insert((request.mail_id, Some(slot)));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
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
    fn request(&mut self, mail_id: u64, action: MailAction) -> Option<MailRequest> {
        let request = MailRequest {
            object: self.object?,
            mail_id,
            action,
        };
        self.pending = Some(Pending::Request(request));
        Some(request)
    }

    /// One frame click. `money` is the player's copper, `inventory` the bags.
    pub fn click(
        &mut self,
        action: &str,
        texts: &MailTexts,
        money: u64,
        inventory: &InventoryState,
    ) -> MailEffect {
        if self.object.is_none() {
            return MailEffect::default();
        }
        if let Some(effect) = self.click_form(action) {
            return effect;
        }
        if self.busy() {
            return MailEffect::default();
        }
        match action {
            frame::ACTION_SEND => self.send(texts),
            frame::ACTION_OPEN_ALL => {
                if !self.mails().is_empty() {
                    self.open_all = Some(Skipped::new());
                }
                MailEffect::default()
            }
            frame::ACTION_TAKE_MONEY => {
                let mail = self.selected_mail().filter(|mail| mail.money > 0);
                let id = mail.map(|mail| mail.mail_id);
                id.and_then(|id| self.request(id, MailAction::TakeMoney))
                    .map_or_else(MailEffect::default, MailEffect::request)
            }
            frame::ACTION_REPLY => self.reply(),
            frame::ACTION_DELETE => self.delete(),
            _ => self.click_item(action, texts, money, inventory),
        }
    }

    /// Tabs, paging and the Send Mail form, which work while a request is in flight.
    fn click_form(&mut self, action: &str) -> Option<MailEffect> {
        match action {
            frame::ACTION_TAB_INBOX => self.tab = MailFrameTab::Inbox,
            frame::ACTION_TAB_SEND => self.tab = MailFrameTab::Send,
            frame::ACTION_OPEN_CLOSE => self.select(None),
            frame::ACTION_PREV => self.page = self.page.saturating_sub(1),
            frame::ACTION_NEXT => {
                if self.page + 1 < self.mails().len().div_ceil(PAGE_SIZE) {
                    self.page += 1;
                }
            }
            frame::ACTION_MODE_MONEY => self.cod_mode = false,
            frame::ACTION_MODE_COD => self.cod_mode = !self.attachments.is_empty(),
            frame::ACTION_SEND_CANCEL => {
                self.attachments.clear();
                self.cod_mode = false;
                self.item_subject = None;
                self.replying = false;
                return Some(MailEffect {
                    edits: cleared_form(),
                    ..MailEffect::default()
                });
            }
            _ => return None,
        }
        Some(MailEffect::default())
    }

    fn click_item(
        &mut self,
        action: &str,
        texts: &MailTexts,
        money: u64,
        inventory: &InventoryState,
    ) -> MailEffect {
        if let Some(id) = parse::<u64>(action, frame::ACTION_OPEN_PREFIX) {
            // `InboxFrame_OnClick`: opening an unread mail reads it.
            let Some(read) = self
                .mails()
                .iter()
                .find(|m| m.mail_id == id)
                .map(|m| m.read)
            else {
                return MailEffect::default();
            };
            self.select(Some(id));
            if read {
                return MailEffect::default();
            }
            return self
                .request(id, MailAction::MarkRead)
                .map_or_else(MailEffect::default, MailEffect::request);
        }
        if let Some(slot) = parse::<u8>(action, frame::ACTION_TAKE_ITEM_PREFIX) {
            return self.take_item(slot, money);
        }
        if let Some(index) = parse::<usize>(action, frame::ACTION_ATTACHMENT_PREFIX)
            && index < self.attachments.len()
        {
            self.attachments.remove(index);
            if self.attachments.is_empty() {
                self.cod_mode = false;
            }
            return self.fill_subject(texts, inventory);
        }
        MailEffect::default()
    }

    /// `OpenMailAttachment_OnClick` (MF.lua:913-925).
    fn take_item(&mut self, slot: u8, money: u64) -> MailEffect {
        let Some(mail) = self.selected_mail() else {
            return MailEffect::default();
        };
        if !mail.attachments.iter().any(|a| a.slot == slot) {
            return MailEffect::default();
        }
        let (id, cod) = (mail.mail_id, mail.cod);
        let action = MailAction::TakeAttachment { slot };
        if cod > money {
            return popup(
                COD_ALERT_POPUP,
                MailError::CodInsufficientMoney.message().into(),
                "Close",
                None,
            );
        }
        if cod > 0 {
            self.confirm = self.object.map(|object| {
                let take = MailRequest {
                    object,
                    mail_id: id,
                    action,
                };
                (COD_POPUP, take)
            });
            let text = format!("Accepting this item will cost:\n{}", money_text(cod));
            return popup(COD_POPUP, text, "Accept", Some("Cancel"));
        }
        self.request(id, action)
            .map_or_else(MailEffect::default, MailEffect::request)
    }

    /// `OpenMail_Reply`: Send Mail to the sender with "RE: subject".
    fn reply(&mut self) -> MailEffect {
        let Some(mail) = self.selected_mail().filter(|mail| mail.from_player) else {
            return MailEffect::default();
        };
        let edits = vec![
            (TO_BOX, mail.sender.clone()),
            (SUBJECT_BOX, format!("RE: {}", mail.subject)),
        ];
        self.tab = MailFrameTab::Send;
        self.replying = true;
        MailEffect {
            edits,
            ..MailEffect::default()
        }
    }

    /// `OpenMail_Delete` (MF.lua:865-880): returnable mail holding something goes
    /// back; deleting mail with an item or money asks first.
    fn delete(&mut self) -> MailEffect {
        let Some(mail) = self.selected_mail().cloned() else {
            return MailEffect::default();
        };
        let Some(object) = self.object else {
            return MailEffect::default();
        };
        let delete = MailRequest {
            object,
            mail_id: mail.mail_id,
            action: MailAction::Delete,
        };
        if !mail.can_delete() {
            self.select(None);
            return self
                .request(mail.mail_id, MailAction::Return)
                .map_or_else(MailEffect::default, MailEffect::request);
        }
        let text = if let Some(item) = mail.attachments.first() {
            Some((
                DELETE_MAIL_POPUP,
                format!("Deleting this mail will also destroy {}", item.name),
            ))
        } else if mail.money > 0 {
            Some((
                DELETE_MONEY_POPUP,
                format!(
                    "Deleting this mail will also destroy:\n{}",
                    money_text(mail.money)
                ),
            ))
        } else {
            None
        };
        match text {
            Some((key, text)) => {
                self.confirm = Some((key, delete));
                popup(key, text, "Accept", Some("Cancel"))
            }
            None => {
                self.select(None);
                self.request(mail.mail_id, MailAction::Delete)
                    .map_or_else(MailEffect::default, MailEffect::request)
            }
        }
    }

    /// `SendMailMailButton_OnClick`: the typed form as money or C.O.D.
    fn send(&mut self, texts: &MailTexts) -> MailEffect {
        let Some(object) = self.object.filter(|_| self.can_send(texts)) else {
            return MailEffect::default();
        };
        let copper = money_input(texts);
        let (money, cod) = if self.cod_mode {
            (0, copper)
        } else {
            (copper, 0)
        };
        self.pending = Some(Pending::Send);
        MailEffect {
            outgoing: Some(MailOutgoing::Send(SendMail {
                object,
                recipient: text(texts, TO_BOX).trim().to_string(),
                subject: text(texts, SUBJECT_BOX).to_string(),
                body: text(texts, BODY_BOX).to_string(),
                attachments: self.attachments.clone(),
                money,
                cod,
            })),
            ..MailEffect::default()
        }
    }

    /// IPC `mail send`: `mail` addressed to the open mailbox, as the original sends its
    /// `MailRequest::Send` draft (src/game/networking/mail.rs:118-127).
    pub fn ipc_send(&mut self, mail: SendMail) -> Option<SendMail> {
        let object = self.object?;
        self.pending = Some(Pending::Send);
        Some(SendMail { object, ..mail })
    }

    /// IPC `mail act`: one inbox action for the open mailbox.
    pub fn ipc_act(&mut self, mail_id: u64, action: MailAction) -> Option<MailRequest> {
        self.request(mail_id, action)
    }

    /// `SendMailFrame_CanSend` (MF.lua:1106-1135).
    pub fn can_send(&self, texts: &MailTexts) -> bool {
        !text(texts, TO_BOX).trim().is_empty()
            && !text(texts, SUBJECT_BOX).is_empty()
            && !(self.cod_mode && money_input(texts) > MAX_COD_COPPER)
    }

    /// A bag item right-clicked while Send Mail shows takes the next free button
    /// (`ClickSendMailItemButton`). Soulbound items are refused like the server does.
    pub fn attach(
        &mut self,
        item: &InventorySlot,
        texts: &MailTexts,
        inventory: &InventoryState,
    ) -> MailEffect {
        if self.object.is_none()
            || self.busy()
            || item.is_empty()
            || self.attachments.contains(&item.item_guid)
        {
            return MailEffect::default();
        }
        if item.soulbound {
            return MailEffect {
                error: Some(MailError::BoundItem.message()),
                ..MailEffect::default()
            };
        }
        if self.attachments.len() >= MAX_MAIL_ATTACHMENTS {
            return MailEffect {
                error: Some(ATTACHMENTS_FULL_TEXT),
                ..MailEffect::default()
            };
        }
        self.attachments.push(item.item_guid);
        self.fill_subject(texts, inventory)
    }

    /// Attachments no longer in the bags fall off the form.
    pub fn retain_attachments(&mut self, inventory: &InventoryState) {
        self.attachments
            .retain(|guid| bag_item(inventory, *guid).is_some());
        if self.attachments.is_empty() {
            self.cod_mode = false;
        }
    }

    /// `SendMailFrame_Update`: an empty subject, or the one it filled in before, takes
    /// the first attachment's name ("name (count)" for a stack).
    fn fill_subject(&mut self, texts: &MailTexts, inventory: &InventoryState) -> MailEffect {
        let subject = text(texts, SUBJECT_BOX);
        if !subject.is_empty() && self.item_subject.as_deref() != Some(subject) {
            return MailEffect::default();
        }
        let title = self
            .attachments
            .first()
            .and_then(|guid| bag_item(inventory, *guid))
            .map(|slot| {
                if slot.count <= 1 {
                    slot.name.clone()
                } else {
                    format!("{} ({})", slot.name, slot.count)
                }
            })
            .unwrap_or_default();
        self.item_subject = (!title.is_empty()).then(|| title.clone());
        MailEffect {
            edits: vec![(SUBJECT_BOX, title)],
            ..MailEffect::default()
        }
    }

    /// A closed mail popup: Accept sends what it stood for.
    pub fn popup_result(&mut self, key: &str, accepted: bool) -> Option<MailOutgoing> {
        if !POPUP_KEYS.contains(&key) {
            return None;
        }
        let (shown, request) = self.confirm?;
        if shown != key {
            return None;
        }
        self.confirm = None;
        if !accepted || self.busy() || self.object != Some(request.object) {
            return None;
        }
        if request.action == MailAction::Delete {
            self.select(None);
        }
        self.pending = Some(Pending::Request(request));
        Some(MailOutgoing::Request(request))
    }

    /// `OpenAllMailMixin:AdvanceAndProcessNextItem` once the last request is answered:
    /// newest mail first, its money then its items from the last slot, skipping C.O.D.
    /// mail and failed items, stopping with no free bag slot or nothing left.
    pub fn next_open_all(&mut self, free_bag_slots: usize) -> Option<MailRequest> {
        if self.busy() || self.confirm.is_some() {
            return None;
        }
        let skipped = self.open_all.as_ref()?;
        let next = (free_bag_slots > 0)
            .then(|| next_open_all_item(self.mails(), skipped))
            .flatten();
        let Some((mail_id, action)) = next else {
            self.open_all = None;
            return None;
        };
        self.request(mail_id, action)
    }

    fn send_view(&self, money: u64, inventory: &InventoryState, texts: &MailTexts) -> SendView {
        let postage = shared::mail::postage(self.attachments.len());
        SendView {
            attachments: (0..SEND_ATTACHMENTS)
                .map(|index| {
                    let slot = bag_item(inventory, *self.attachments.get(index)?)?;
                    Some(SlotItem {
                        icon_fdid: slot.icon_fdid,
                        count: slot.count,
                        quality_border: slot.quality.border_color().into(),
                    })
                })
                .collect(),
            postage,
            postage_unaffordable: postage > money,
            cod: self.cod_mode,
            cod_enabled: !self.attachments.is_empty(),
            can_send: self.can_send(texts),
        }
    }

    pub fn view(
        &self,
        money: u64,
        inventory: &InventoryState,
        texts: &MailTexts,
    ) -> MailFrameState {
        let now = self.contents.as_ref().map_or(0, |c| c.now);
        let rows = self
            .mails()
            .iter()
            .skip(self.page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .map(|mail| inbox_row(mail, now, self.selected == Some(mail.mail_id)))
            .collect();
        let send = self.send_view(money, inventory, texts);
        MailFrameState {
            visible: self.is_open(),
            tab: self.tab,
            rows,
            page: self.page,
            page_count: self.mails().len().div_ceil(PAGE_SIZE).max(1),
            send,
            open: self.selected_mail().map(open_mail),
            money,
            busy: self.busy(),
            opening_all: self.opening_all(),
        }
    }
}

/// Whether `mails` shows `request` carried out.
fn request_done(request: MailRequest, mails: &[MailHeader]) -> bool {
    let Some(mail) = mails.iter().find(|mail| mail.mail_id == request.mail_id) else {
        return true;
    };
    match request.action {
        MailAction::MarkRead => mail.read,
        MailAction::TakeMoney => mail.money == 0,
        MailAction::TakeAttachment { slot } => !mail.attachments.iter().any(|a| a.slot == slot),
        MailAction::Return | MailAction::Delete => false,
    }
}

fn next_open_all_item(mails: &[MailHeader], skipped: &Skipped) -> Option<(u64, MailAction)> {
    mails.iter().filter(|mail| mail.cod == 0).find_map(|mail| {
        if mail.money > 0 && !skipped.contains(&(mail.mail_id, None)) {
            return Some((mail.mail_id, MailAction::TakeMoney));
        }
        mail.attachments
            .iter()
            .rev()
            .find(|a| !skipped.contains(&(mail.mail_id, Some(a.slot))))
            .map(|a| (mail.mail_id, MailAction::TakeAttachment { slot: a.slot }))
    })
}

fn popup(key: &str, text: String, accept: &str, cancel: Option<&str>) -> MailEffect {
    MailEffect {
        popup: Some(PopupSpec {
            key: key.into(),
            text,
            accept_label: accept.into(),
            cancel_label: cancel.map(Into::into),
            timeout: None,
            confirm_text: None,
        }),
        ..MailEffect::default()
    }
}

fn cleared_form() -> Vec<(&'static str, String)> {
    input_names()
        .into_iter()
        .map(|name| (name, String::new()))
        .collect()
}

fn parse<T: std::str::FromStr>(action: &str, prefix: &str) -> Option<T> {
    action.strip_prefix(prefix)?.parse().ok()
}

/// The bag slot holding `item_guid`.
pub fn bag_item(inventory: &InventoryState, item_guid: u64) -> Option<&InventorySlot> {
    inventory
        .slots
        .iter()
        .flatten()
        .find(|slot| slot.item_guid == item_guid && !slot.is_empty())
}

fn attachment_item(attached: &shared::protocol::MailAttachment) -> SlotItem {
    SlotItem {
        icon_fdid: stack_slot(&attached.item).icon_fdid,
        count: attached.item.count,
        quality_border: quality_color(attached.quality).into(),
    }
}

fn inbox_row(mail: &MailHeader, now: u64, selected: bool) -> InboxRow {
    let first = mail.attachments.first();
    let left = mail.expires_at.saturating_sub(now);
    InboxRow {
        mail_id: mail.mail_id,
        sender: mail.sender.clone(),
        subject: mail.subject.clone(),
        icon_fdid: first.map_or(STATIONERY_ICON, |a| attachment_item(a).icon_fdid),
        count: first.map_or(0, |a| a.item.count),
        read: mail.read,
        cod: mail.cod > 0,
        expires: time_left(left),
        expires_soon: left < 86400,
        selected,
    }
}

fn open_mail(mail: &MailHeader) -> OpenMailView {
    OpenMailView {
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
                item: attachment_item(a),
            })
            .collect(),
        can_reply: mail.from_player,
        can_delete: mail.can_delete(),
    }
}

/// `DAYS_ABBR` for a day or more left, otherwise `HOURS_ABBR` / `MINUTES_ABBR`
/// (MF.lua:265-268).
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
