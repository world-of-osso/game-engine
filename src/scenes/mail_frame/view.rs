//! MailFrame view model from [`MailState`], the bags (attachment icons) and the
//! player's money.

use game_engine::bag_data::{InventorySlot, InventoryState, stack_slot};
use game_engine::mail_data::{MailState, MailTab, SendMoneyMode};
use game_engine::merchant_data::quality_color;
use game_engine::ui::screens::bank_art::SlotItem;
use game_engine::ui::screens::mail_frame_component::{
    InboxRow, MailFrameState, MailFrameTab, OpenAttachment, OpenMailView, SEND_ATTACHMENTS,
    SendView,
};
use shared::protocol::{MailAttachment, MailHeader};

/// `Interface\Icons\INV_Misc_Note_01`, the default stationery icon.
const STATIONERY_ICON: u32 = 134_327;

/// The bag slot holding `item_guid`.
pub fn bag_item(inventory: &InventoryState, item_guid: u64) -> Option<&InventorySlot> {
    inventory
        .slots
        .iter()
        .flatten()
        .find(|slot| slot.item_guid == item_guid && !slot.is_empty())
}

fn attachment_item(attached: &MailAttachment) -> SlotItem {
    let slot = stack_slot(&attached.item);
    SlotItem {
        icon_fdid: slot.icon_fdid,
        count: slot.count,
        quality_border: quality_color(attached.quality).into(),
    }
}

/// `DAYS_ABBR` for a day or more left, otherwise `HOURS_ABBR` / `MINUTES_ABBR`
/// (MF.lua:265-268); `true` under a day (red).
pub fn time_left(expires_at: u64, now: u64) -> (String, bool) {
    let left = expires_at.saturating_sub(now);
    let plural = |n: u64, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    if left >= 86_400 {
        (plural(left / 86_400, "Day", "Days"), false)
    } else if left >= 3_600 {
        (plural(left / 3_600, "Hr", "Hr"), true)
    } else {
        (plural((left / 60).max(1), "Min", "Min"), true)
    }
}

fn inbox_row(header: &MailHeader, now: u64, selected: bool) -> InboxRow {
    let first = header.attachments.first();
    let (expires, expires_soon) = time_left(header.expires_at, now);
    InboxRow {
        mail_id: header.mail_id,
        sender: header.sender.clone(),
        subject: header.subject.clone(),
        icon_fdid: first.map_or(STATIONERY_ICON, |attached| {
            attachment_item(attached).icon_fdid
        }),
        count: first.map_or(0, |attached| attached.item.count),
        read: header.read,
        cod: header.cod > 0,
        expires,
        expires_soon,
        selected,
    }
}

fn open_mail(header: &MailHeader) -> OpenMailView {
    OpenMailView {
        sender: header.sender.clone(),
        subject: header.subject.clone(),
        body: header.body.clone(),
        money: header.money,
        cod: header.cod,
        attachments: header
            .attachments
            .iter()
            .map(|attached| OpenAttachment {
                slot: attached.slot,
                item: attachment_item(attached),
            })
            .collect(),
        can_reply: header.from_player,
        can_delete: header.can_delete(),
    }
}

fn send_view(mail: &MailState, inventory: &InventoryState, money: u64) -> SendView {
    let attachments = (0..SEND_ATTACHMENTS)
        .map(|index| {
            let guid = mail.attachments.get(index)?;
            let slot = bag_item(inventory, *guid)?;
            Some(SlotItem {
                icon_fdid: slot.icon_fdid,
                count: slot.count,
                quality_border: "1.0,1.0,1.0,1.0".into(),
            })
        })
        .collect();
    SendView {
        attachments,
        postage: mail.postage(),
        postage_unaffordable: mail.postage() > money,
        cod: mail.money_mode == SendMoneyMode::Cod,
        cod_enabled: !mail.attachments.is_empty(),
        can_send: true,
    }
}

pub fn mail_frame_state(
    mail: &MailState,
    inventory: &InventoryState,
    window_open: bool,
    money: u64,
) -> MailFrameState {
    if !mail.is_open() {
        return MailFrameState::default();
    }
    let now = mail.contents.as_ref().map_or(0, |contents| contents.now);
    MailFrameState {
        visible: window_open,
        tab: match mail.tab {
            MailTab::Inbox => MailFrameTab::Inbox,
            MailTab::Send => MailFrameTab::Send,
        },
        rows: mail
            .page_mails()
            .iter()
            .map(|header| inbox_row(header, now, mail.open_mail == Some(header.mail_id)))
            .collect(),
        page: mail.page,
        page_count: mail.page_count(),
        send: send_view(mail, inventory, money),
        open: mail.opened().map(open_mail),
        money,
        ..Default::default()
    }
}
