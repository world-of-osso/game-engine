use bevy::prelude::*;

use crate::auction_house_data::Money;

/// Texture FDIDs for the mail frame.
pub mod textures {
    /// Mail notification icon.
    pub const MAIL_ICON: u32 = 136382;
    /// Attachment item border.
    pub const ITEM_BORDER: u32 = 136383;
    /// Frame background.
    pub const FRAME_BG: u32 = 530419;
    /// Unread letter icon.
    pub const LETTER_UNREAD: u32 = 133457;
    /// Read letter icon.
    pub const LETTER_READ: u32 = 133462;
    /// Gold coin (shared with auction house).
    pub const GOLD_ICON: u32 = 237618;
    /// Silver coin (shared with auction house).
    pub const SILVER_ICON: u32 = 237620;
    /// Copper coin (shared with auction house).
    pub const COPPER_ICON: u32 = 237617;
}

/// Maximum number of item attachments per mail (WoW limit).
pub const MAX_MAIL_ATTACHMENTS: usize = 12;

#[derive(Clone, Debug, PartialEq)]
pub struct MailAttachment {
    pub item_name: String,
    pub icon_fdid: u32,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailMessage {
    pub id: u64,
    pub sender: String,
    pub subject: String,
    pub body: String,
    pub money: Money,
    pub attachments: Vec<MailAttachment>,
    pub read: bool,
    /// Seconds until expiry.
    pub expires_in: f32,
}

impl MailMessage {
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    pub fn has_money(&self) -> bool {
        self.money.0 > 0
    }

    pub fn expiry_text(&self) -> String {
        let hours = (self.expires_in / 3600.0).floor() as u32;
        if hours >= 24 {
            format!("{} days", hours / 24)
        } else {
            format!("{hours} hours")
        }
    }
}

/// Runtime mail state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct MailState {
    pub inbox: Vec<MailMessage>,
    pub send_recipient: String,
    pub send_subject: String,
    pub send_body: String,
    pub send_money: Money,
    pub send_attachments: Vec<Option<MailAttachment>>,
}

impl MailState {
    pub fn unread_count(&self) -> usize {
        self.inbox.iter().filter(|m| !m.read).count()
    }

    pub fn total_money_in_inbox(&self) -> Money {
        Money(self.inbox.iter().map(|m| m.money.0).sum())
    }

    /// Sort inbox: unread first, then by expiry (soonest first).
    pub fn sort_inbox(&mut self) {
        self.inbox.sort_by(|a, b| {
            a.read
                .cmp(&b.read)
                .then(a.expires_in.partial_cmp(&b.expires_in).unwrap())
        });
    }

    /// Collect all attachments across all inbox messages.
    pub fn all_attachments(&self) -> Vec<&MailAttachment> {
        self.inbox
            .iter()
            .flat_map(|m| m.attachments.iter())
            .collect()
    }

    /// Validate send money: must not exceed player balance, and must be non-negative.
    pub fn validate_send_money(&self, player_money: Money) -> bool {
        self.send_money.0 <= player_money.0
    }

    /// Count of non-empty send attachment slots.
    pub fn send_attachment_count(&self) -> usize {
        self.send_attachments.iter().filter(|s| s.is_some()).count()
    }

    /// Clear the send form fields.
    pub fn clear_send_form(&mut self) {
        self.send_recipient.clear();
        self.send_subject.clear();
        self.send_body.clear();
        self.send_money = Money::default();
        self.send_attachments.clear();
    }

    /// Find a message by ID.
    pub fn find_message(&self, mail_id: u64) -> Option<&MailMessage> {
        self.inbox.iter().find(|m| m.id == mail_id)
    }

    /// Remove a message from the inbox by ID. Returns true if found.
    pub fn remove_message(&mut self, mail_id: u64) -> bool {
        let before = self.inbox.len();
        self.inbox.retain(|m| m.id != mail_id);
        self.inbox.len() < before
    }

    /// Mark a message as read by ID.
    pub fn mark_read(&mut self, mail_id: u64) {
        if let Some(m) = self.inbox.iter_mut().find(|m| m.id == mail_id) {
            m.read = true;
        }
    }

    /// Add an attachment to the next empty send slot. Returns the slot index, or None if full.
    pub fn add_send_attachment(&mut self, attachment: MailAttachment) -> Option<usize> {
        // Fill an existing empty slot first.
        if let Some(idx) = self.send_attachments.iter().position(|s| s.is_none()) {
            self.send_attachments[idx] = Some(attachment);
            return Some(idx);
        }
        // Append if under the limit.
        if self.send_attachments.len() < MAX_MAIL_ATTACHMENTS {
            let idx = self.send_attachments.len();
            self.send_attachments.push(Some(attachment));
            return Some(idx);
        }
        None
    }

    /// Remove an attachment from a send slot. Returns the removed attachment.
    pub fn remove_send_attachment(&mut self, slot: usize) -> Option<MailAttachment> {
        self.send_attachments.get_mut(slot)?.take()
    }

    /// Whether the send attachment slots are full.
    pub fn send_attachments_full(&self) -> bool {
        self.send_attachment_count() >= MAX_MAIL_ATTACHMENTS
    }

    /// Take a single attachment from a received mail message.
    /// Removes the attachment at the given index and returns it.
    pub fn take_inbox_attachment(&mut self, mail_id: u64, index: usize) -> Option<MailAttachment> {
        let msg = self.inbox.iter_mut().find(|m| m.id == mail_id)?;
        if index >= msg.attachments.len() {
            return None;
        }
        Some(msg.attachments.remove(index))
    }

    /// Take all attachments from a received mail message.
    pub fn take_all_inbox_attachments(&mut self, mail_id: u64) -> Vec<MailAttachment> {
        let Some(msg) = self.inbox.iter_mut().find(|m| m.id == mail_id) else {
            return vec![];
        };
        std::mem::take(&mut msg.attachments)
    }
}

// --- Client → server intents ---

/// A pending mail action to send to the server.
#[derive(Clone, Debug, PartialEq)]
pub enum MailIntent {
    /// Send a new mail.
    Send {
        recipient: String,
        subject: String,
        body: String,
        money: Money,
        /// Bag/slot pairs for items to attach.
        attachments: Vec<(u8, u8)>,
    },
    /// Take an item attachment from a received mail.
    TakeItem { mail_id: u64, attachment_index: u32 },
    /// Take money from a received mail.
    TakeMoney { mail_id: u64 },
    /// Delete a mail from the inbox.
    Delete { mail_id: u64 },
    /// Return a mail to its sender.
    ReturnToSender { mail_id: u64 },
    /// Open the mailbox (request inbox refresh).
    OpenMailbox,
}

/// Queue of mail intents waiting to be sent to the server.
#[derive(Resource, Default)]
pub struct MailIntentQueue {
    pub pending: Vec<MailIntent>,
}

impl MailIntentQueue {
    pub fn send_mail(
        &mut self,
        recipient: String,
        subject: String,
        body: String,
        money: Money,
        attachments: Vec<(u8, u8)>,
    ) {
        self.pending.push(MailIntent::Send {
            recipient,
            subject,
            body,
            money,
            attachments,
        });
    }

    pub fn take_item(&mut self, mail_id: u64, attachment_index: u32) {
        self.pending.push(MailIntent::TakeItem {
            mail_id,
            attachment_index,
        });
    }

    pub fn take_money(&mut self, mail_id: u64) {
        self.pending.push(MailIntent::TakeMoney { mail_id });
    }

    pub fn delete(&mut self, mail_id: u64) {
        self.pending.push(MailIntent::Delete { mail_id });
    }

    pub fn return_to_sender(&mut self, mail_id: u64) {
        self.pending.push(MailIntent::ReturnToSender { mail_id });
    }

    pub fn open_mailbox(&mut self) {
        self.pending.push(MailIntent::OpenMailbox);
    }

    pub fn drain(&mut self) -> Vec<MailIntent> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mail() -> MailMessage {
        MailMessage {
            id: 1,
            sender: "Auction House".into(),
            subject: "Auction Won".into(),
            body: "Your item sold.".into(),
            money: Money::from_gold_silver_copper(5, 30, 0),
            attachments: vec![MailAttachment {
                item_name: "Arcanite Bar".into(),
                icon_fdid: 135274,
                count: 1,
            }],
            read: false,
            expires_in: 86400.0,
        }
    }

    #[test]
    fn has_attachments_and_money() {
        let m = sample_mail();
        assert!(m.has_attachments());
        assert!(m.has_money());
    }

    #[test]
    fn empty_mail_flags() {
        let m = MailMessage {
            money: Money(0),
            attachments: vec![],
            ..sample_mail()
        };
        assert!(!m.has_attachments());
        assert!(!m.has_money());
    }

    #[test]
    fn expiry_text_days() {
        let m = sample_mail();
        assert_eq!(m.expiry_text(), "1 days");
    }

    #[test]
    fn expiry_text_hours() {
        let m = MailMessage {
            expires_in: 7200.0,
            ..sample_mail()
        };
        assert_eq!(m.expiry_text(), "2 hours");
    }

    #[test]
    fn unread_count() {
        let mut state = MailState::default();
        state.inbox.push(sample_mail());
        state.inbox.push(MailMessage {
            read: true,
            ..sample_mail()
        });
        state.inbox.push(MailMessage {
            id: 3,
            read: false,
            ..sample_mail()
        });
        assert_eq!(state.unread_count(), 2);
    }

    #[test]
    fn total_money_in_inbox() {
        let mut state = MailState::default();
        state.inbox.push(sample_mail());
        state.inbox.push(MailMessage {
            money: Money::from_gold_silver_copper(2, 0, 0),
            ..sample_mail()
        });
        let total = state.total_money_in_inbox();
        assert_eq!(total, Money(53000 + 20000));
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::MAIL_ICON, 0);
        assert_ne!(textures::ITEM_BORDER, 0);
        assert_ne!(textures::FRAME_BG, 0);
        assert_ne!(textures::LETTER_UNREAD, 0);
        assert_ne!(textures::LETTER_READ, 0);
        assert_ne!(textures::GOLD_ICON, 0);
    }

    // --- Inbox sorting ---

    fn mail(id: u64, read: bool, expires: f32) -> MailMessage {
        MailMessage {
            id,
            sender: format!("Sender{id}"),
            subject: format!("Mail {id}"),
            body: String::new(),
            money: Money(0),
            attachments: vec![],
            read,
            expires_in: expires,
        }
    }

    #[test]
    fn sort_inbox_unread_first() {
        let mut state = MailState::default();
        state.inbox = vec![
            mail(1, true, 3600.0),
            mail(2, false, 7200.0),
            mail(3, true, 1800.0),
        ];
        state.sort_inbox();
        assert!(!state.inbox[0].read); // unread first
        assert!(state.inbox[1].read);
        assert!(state.inbox[2].read);
    }

    #[test]
    fn sort_inbox_by_expiry_within_status() {
        let mut state = MailState::default();
        state.inbox = vec![
            mail(1, false, 7200.0),
            mail(2, false, 1800.0),
            mail(3, false, 3600.0),
        ];
        state.sort_inbox();
        assert_eq!(state.inbox[0].id, 2); // soonest expiry first
        assert_eq!(state.inbox[1].id, 3);
        assert_eq!(state.inbox[2].id, 1);
    }

    // --- Attachment extraction ---

    #[test]
    fn all_attachments_across_messages() {
        let mut state = MailState::default();
        state.inbox = vec![
            MailMessage {
                attachments: vec![
                    MailAttachment {
                        item_name: "Ore".into(),
                        icon_fdid: 1,
                        count: 20,
                    },
                    MailAttachment {
                        item_name: "Bar".into(),
                        icon_fdid: 2,
                        count: 5,
                    },
                ],
                ..sample_mail()
            },
            MailMessage {
                id: 2,
                attachments: vec![MailAttachment {
                    item_name: "Gem".into(),
                    icon_fdid: 3,
                    count: 1,
                }],
                ..sample_mail()
            },
        ];
        let all = state.all_attachments();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].item_name, "Ore");
        assert_eq!(all[2].item_name, "Gem");
    }

    #[test]
    fn all_attachments_empty_inbox() {
        let state = MailState::default();
        assert!(state.all_attachments().is_empty());
    }

    // --- Money validation ---

    #[test]
    fn validate_send_money_sufficient() {
        let mut state = MailState::default();
        state.send_money = Money::from_gold_silver_copper(5, 0, 0);
        assert!(state.validate_send_money(Money::from_gold_silver_copper(10, 0, 0)));
    }

    #[test]
    fn validate_send_money_exact() {
        let mut state = MailState::default();
        state.send_money = Money::from_gold_silver_copper(10, 0, 0);
        assert!(state.validate_send_money(Money::from_gold_silver_copper(10, 0, 0)));
    }

    #[test]
    fn validate_send_money_insufficient() {
        let mut state = MailState::default();
        state.send_money = Money::from_gold_silver_copper(10, 0, 0);
        assert!(!state.validate_send_money(Money::from_gold_silver_copper(5, 0, 0)));
    }

    #[test]
    fn validate_send_money_zero() {
        let state = MailState::default();
        assert!(state.validate_send_money(Money(0)));
    }

    #[test]
    fn send_attachment_count() {
        let mut state = MailState::default();
        state.send_attachments = vec![
            Some(MailAttachment {
                item_name: "A".into(),
                icon_fdid: 1,
                count: 1,
            }),
            None,
            Some(MailAttachment {
                item_name: "B".into(),
                icon_fdid: 2,
                count: 1,
            }),
            None,
        ];
        assert_eq!(state.send_attachment_count(), 2);
    }

    // --- MailState management ---

    #[test]
    fn clear_send_form() {
        let mut state = MailState {
            send_recipient: "Alice".into(),
            send_subject: "Hello".into(),
            send_body: "Hi there".into(),
            send_money: Money(5000),
            send_attachments: vec![Some(MailAttachment {
                item_name: "X".into(),
                icon_fdid: 1,
                count: 1,
            })],
            ..Default::default()
        };
        state.clear_send_form();
        assert!(state.send_recipient.is_empty());
        assert!(state.send_subject.is_empty());
        assert!(state.send_body.is_empty());
        assert_eq!(state.send_money, Money(0));
        assert!(state.send_attachments.is_empty());
    }

    #[test]
    fn find_message_by_id() {
        let mut state = MailState::default();
        state.inbox = vec![mail(10, false, 3600.0), mail(20, true, 7200.0)];
        assert_eq!(state.find_message(20).unwrap().id, 20);
        assert!(state.find_message(99).is_none());
    }

    #[test]
    fn remove_message_by_id() {
        let mut state = MailState::default();
        state.inbox = vec![mail(10, false, 3600.0), mail(20, true, 7200.0)];
        assert!(state.remove_message(10));
        assert_eq!(state.inbox.len(), 1);
        assert_eq!(state.inbox[0].id, 20);
        assert!(!state.remove_message(99));
    }

    #[test]
    fn mark_read_by_id() {
        let mut state = MailState::default();
        state.inbox = vec![mail(10, false, 3600.0)];
        state.mark_read(10);
        assert!(state.inbox[0].read);
    }

    // --- MailIntentQueue ---

    #[test]
    fn intent_send_mail() {
        let mut queue = MailIntentQueue::default();
        queue.send_mail(
            "Bob".into(),
            "Hi".into(),
            "Body".into(),
            Money(100),
            vec![(0, 3)],
        );
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(
            drained[0],
            MailIntent::Send {
                recipient: "Bob".into(),
                subject: "Hi".into(),
                body: "Body".into(),
                money: Money(100),
                attachments: vec![(0, 3)],
            }
        );
    }

    #[test]
    fn intent_take_item() {
        let mut queue = MailIntentQueue::default();
        queue.take_item(42, 0);
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            MailIntent::TakeItem {
                mail_id: 42,
                attachment_index: 0
            }
        );
    }

    #[test]
    fn intent_take_money() {
        let mut queue = MailIntentQueue::default();
        queue.take_money(42);
        let drained = queue.drain();
        assert_eq!(drained[0], MailIntent::TakeMoney { mail_id: 42 });
    }

    #[test]
    fn intent_delete() {
        let mut queue = MailIntentQueue::default();
        queue.delete(5);
        let drained = queue.drain();
        assert_eq!(drained[0], MailIntent::Delete { mail_id: 5 });
    }

    #[test]
    fn intent_return_to_sender() {
        let mut queue = MailIntentQueue::default();
        queue.return_to_sender(7);
        let drained = queue.drain();
        assert_eq!(drained[0], MailIntent::ReturnToSender { mail_id: 7 });
    }

    #[test]
    fn intent_open_mailbox() {
        let mut queue = MailIntentQueue::default();
        queue.open_mailbox();
        let drained = queue.drain();
        assert_eq!(drained[0], MailIntent::OpenMailbox);
    }

    #[test]
    fn intent_drain_clears() {
        let mut queue = MailIntentQueue::default();
        queue.take_money(1);
        queue.delete(2);
        assert_eq!(queue.drain().len(), 2);
        assert!(queue.pending.is_empty());
    }

    // --- Send attachment management ---

    fn attachment(name: &str) -> MailAttachment {
        MailAttachment {
            item_name: name.into(),
            icon_fdid: 1,
            count: 1,
        }
    }

    #[test]
    fn add_send_attachment() {
        let mut state = MailState::default();
        let idx = state.add_send_attachment(attachment("Sword"));
        assert_eq!(idx, Some(0));
        assert_eq!(state.send_attachment_count(), 1);
        assert_eq!(
            state.send_attachments[0].as_ref().unwrap().item_name,
            "Sword"
        );
    }

    #[test]
    fn add_send_attachment_fills_empty_slot() {
        let mut state = MailState::default();
        state.send_attachments = vec![Some(attachment("A")), None, Some(attachment("C"))];
        let idx = state.add_send_attachment(attachment("B"));
        assert_eq!(idx, Some(1));
        assert_eq!(state.send_attachment_count(), 3);
    }

    #[test]
    fn add_send_attachment_full() {
        let mut state = MailState::default();
        for i in 0..MAX_MAIL_ATTACHMENTS {
            state
                .send_attachments
                .push(Some(attachment(&format!("I{i}"))));
        }
        assert!(state.send_attachments_full());
        assert_eq!(state.add_send_attachment(attachment("Extra")), None);
    }

    #[test]
    fn remove_send_attachment() {
        let mut state = MailState::default();
        state.add_send_attachment(attachment("Sword"));
        state.add_send_attachment(attachment("Shield"));
        let removed = state.remove_send_attachment(0);
        assert_eq!(removed.unwrap().item_name, "Sword");
        assert!(state.send_attachments[0].is_none());
        assert_eq!(state.send_attachment_count(), 1);
    }

    #[test]
    fn remove_send_attachment_empty_slot() {
        let mut state = MailState::default();
        state.send_attachments = vec![None];
        assert!(state.remove_send_attachment(0).is_none());
    }

    #[test]
    fn remove_send_attachment_out_of_range() {
        let state = MailState::default();
        assert!(state.send_attachments.get(5).is_none());
    }

    // --- Inbox attachment handling ---

    #[test]
    fn take_inbox_attachment() {
        let mut state = MailState::default();
        state.inbox = vec![MailMessage {
            attachments: vec![attachment("Gem"), attachment("Ore")],
            ..sample_mail()
        }];
        let taken = state.take_inbox_attachment(1, 0);
        assert_eq!(taken.unwrap().item_name, "Gem");
        assert_eq!(state.inbox[0].attachments.len(), 1);
        assert_eq!(state.inbox[0].attachments[0].item_name, "Ore");
    }

    #[test]
    fn take_inbox_attachment_bad_index() {
        let mut state = MailState::default();
        state.inbox = vec![MailMessage {
            attachments: vec![attachment("A")],
            ..sample_mail()
        }];
        assert!(state.take_inbox_attachment(1, 5).is_none());
    }

    #[test]
    fn take_inbox_attachment_bad_mail_id() {
        let mut state = MailState::default();
        assert!(state.take_inbox_attachment(999, 0).is_none());
    }

    #[test]
    fn take_all_inbox_attachments() {
        let mut state = MailState::default();
        state.inbox = vec![MailMessage {
            attachments: vec![attachment("A"), attachment("B"), attachment("C")],
            ..sample_mail()
        }];
        let taken = state.take_all_inbox_attachments(1);
        assert_eq!(taken.len(), 3);
        assert!(state.inbox[0].attachments.is_empty());
    }

    #[test]
    fn take_all_inbox_attachments_bad_id() {
        let mut state = MailState::default();
        let taken = state.take_all_inbox_attachments(999);
        assert!(taken.is_empty());
    }

    #[test]
    fn max_mail_attachments_is_twelve() {
        assert_eq!(MAX_MAIL_ATTACHMENTS, 12);
    }
}
