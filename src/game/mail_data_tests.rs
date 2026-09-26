use shared::protocol::ItemStack;

use super::*;

fn header(mail_id: u64) -> MailHeader {
    MailHeader {
        mail_id,
        sender: "Tradea".into(),
        subject: format!("Mail {mail_id}"),
        body: String::new(),
        money: 0,
        cod: 0,
        attachments: Vec::new(),
        expires_at: 1_800_000_000,
        read: false,
        returned: false,
        from_player: true,
        returnable: true,
    }
}

fn inbox(object: u64, ids: impl IntoIterator<Item = u64>) -> MailboxContents {
    MailboxContents {
        object,
        mails: ids.into_iter().map(header).collect(),
        now: 1_790_000_000,
    }
}

#[test]
fn inbox_pages_hold_seven_mails_and_the_page_follows_a_shrinking_inbox() {
    let mut state = MailState::default();
    state.open(9);
    state.apply(inbox(9, 1..=15));
    assert_eq!(state.page_count(), 3);
    state.next_page();
    state.next_page();
    state.next_page();
    assert_eq!(state.page, 2);
    assert_eq!(
        state
            .page_mails()
            .iter()
            .map(|m| m.mail_id)
            .collect::<Vec<_>>(),
        vec![15]
    );

    state.apply(inbox(9, 1..=8));

    assert_eq!(state.page, 1);
    assert_eq!(state.page_mails()[0].mail_id, 8);
}

#[test]
fn an_open_mail_closes_when_it_leaves_the_inbox() {
    let mut state = MailState::default();
    state.open(9);
    state.apply(inbox(9, [4, 5]));
    state.open_mail = Some(5);
    assert_eq!(state.opened().unwrap().subject, "Mail 5");

    state.apply(inbox(9, [4]));

    assert_eq!(state.open_mail, None);
}

#[test]
fn opening_keeps_an_inbox_that_arrived_first_and_the_pending_senders() {
    let mut state = MailState {
        pending_senders: vec!["Tradea".into()],
        ..MailState::default()
    };
    state.apply(inbox(9, [1]));
    state.open(9);
    assert_eq!(state.mails().len(), 1);
    state.close();
    assert!(state.contents.is_none() && !state.is_open());
    assert_eq!(state.pending_senders, vec!["Tradea".to_string()]);
}

#[test]
fn twelve_distinct_attachments_with_postage_per_item() {
    let mut state = MailState::default();
    assert_eq!(state.postage(), 30);
    for guid in 1..=12 {
        assert!(state.attach(guid));
    }
    assert!(!state.attach(13));
    assert!(!state.attach(1));
    assert_eq!(state.postage(), 360);
    state.detach(0);
    assert_eq!(state.attachments.first(), Some(&2));

    state.money_mode = SendMoneyMode::Cod;
    state.mail_sent();

    assert!(state.attachments.is_empty());
    assert_eq!(state.money_mode, SendMoneyMode::Money);
    assert_eq!(state.sent, 1);
}

#[test]
fn attachments_keep_their_item_view() {
    let mut mail = header(1);
    mail.attachments.push(shared::protocol::MailAttachment {
        slot: 3,
        item: ItemStack {
            item_guid: 90,
            item_id: 2589,
            count: 20,
            durability: None,
            soulbound: false,
        },
        name: "Linen Cloth".into(),
        quality: 1,
    });
    let mut state = MailState::default();
    state.open(9);
    state.apply(MailboxContents {
        object: 9,
        mails: vec![mail],
        now: 0,
    });
    assert_eq!(state.find(1).unwrap().attachments[0].slot, 3);
}
