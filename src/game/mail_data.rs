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
}
