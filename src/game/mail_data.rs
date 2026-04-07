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
}
