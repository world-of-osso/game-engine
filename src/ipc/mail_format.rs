//! `mail status` text, shared by the original engine (`src/ipc/mail.rs`) and the native
//! client: the mailbox, pending-mail senders, mails sent since it opened, and the inbox.

use shared::protocol::MailHeader;

pub fn format_mail_status(
    object: Option<u64>,
    pending_senders: &[String],
    sent: u32,
    mails: &[MailHeader],
) -> String {
    let mut lines = vec![
        format!(
            "mailbox: {}",
            object.map_or_else(|| "closed".to_string(), |object| format!("open {object}"))
        ),
        format!("pending_mail: {}", pending_senders.join(", ")),
        format!("sent: {sent}"),
        format!("inbox: {}", mails.len()),
    ];
    lines.extend(mails.iter().map(format_mail));
    lines.join("\n")
}

fn format_mail(mail: &MailHeader) -> String {
    let items = mail
        .attachments
        .iter()
        .map(|attached| {
            format!(
                "{}:{}x{}",
                attached.slot, attached.item.item_id, attached.item.count
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{} from={} subject={} money={} cod={} items=[{}] read={} returned={} expires_at={}",
        mail.mail_id,
        mail.sender,
        mail.subject,
        mail.money,
        mail.cod,
        items,
        mail.read,
        mail.returned,
        mail.expires_at
    )
}
