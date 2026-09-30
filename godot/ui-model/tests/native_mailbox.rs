use game_engine_ui_model::mail::{MailSession, can_use_mailbox};
use shared::protocol::*;

fn mail(id: u64) -> MailHeader {
    MailHeader {
        mail_id: id,
        sender: "Auction House".into(),
        subject: "Auction won: Linen Cloth".into(),
        body: "Your won item is enclosed.".into(),
        money: 1200,
        cod: 0,
        attachments: vec![MailAttachment {
            slot: 7,
            item: ItemStack {
                item_guid: 81,
                item_id: 2589,
                count: 5,
                durability: None,
                soulbound: false,
            },
            name: "Linen Cloth".into(),
            quality: 1,
        }],
        expires_at: 864100,
        read: false,
        returned: false,
        from_player: false,
        returnable: false,
    }
}
fn contents(object: u64, mails: Vec<MailHeader>) -> MailboxContents {
    MailboxContents {
        object,
        mails,
        now: 100,
    }
}

#[test]
fn native_mailbox_claims_are_gated_and_never_change_authoritative_contents_locally() {
    let mut s = MailSession::default();
    s.expect_open(517);
    s.receive_contents(contents(517, vec![mail(9)])); // MailChannel can precede InteractionChannel.
    assert!(!s.is_open());
    s.open(517);
    assert!(s.is_open());
    assert_eq!(
        s.click("mail_open:9"),
        Some(MailRequest {
            object: 517,
            mail_id: 9,
            action: MailAction::MarkRead
        })
    );
    assert!(s.click("mail_take_money").is_none()); // One pending request at a time.
    let mut read = mail(9);
    read.read = true;
    s.receive_contents(contents(517, vec![read.clone()]));
    assert_eq!(
        s.click("mail_take_money"),
        Some(MailRequest {
            object: 517,
            mail_id: 9,
            action: MailAction::TakeMoney
        })
    );
    assert_eq!(s.contents.as_ref().unwrap().mails[0].money, 1200);
    s.receive_contents(contents(999, vec![]));
    assert!(s.busy()); // Wrong object's response must not unlock the current request.
    assert_eq!(
        s.failed(MailFailed {
            object: 517,
            error: MailError::InventoryFull
        }),
        Some("Inventory is full.")
    );
    assert_eq!(
        s.click("mail_take_item:7"),
        Some(MailRequest {
            object: 517,
            mail_id: 9,
            action: MailAction::TakeAttachment { slot: 7 }
        })
    );
    assert_eq!(s.contents.as_ref().unwrap().mails[0].attachments.len(), 1);
    read.attachments.clear();
    read.money = 0;
    s.receive_contents(contents(517, vec![read]));
    assert!(s.click("mail_take_item:7").is_none());
    assert!(s.click("mail_take_money").is_none());
    s.close();
    s.receive_contents(contents(517, vec![mail(9)]));
    assert!(!s.is_open());
    assert!(s.contents.is_none());
}

#[test]
fn native_mailbox_pages_cod_and_stale_opening_stay_bounded() {
    let mut s = MailSession::default();
    s.expect_open(517);
    s.open(517);
    s.receive_contents(contents(517, (1..=9).map(mail).collect()));
    assert_eq!(s.view(77).frame.rows.len(), 7);
    s.click("mail_next_page");
    assert_eq!(
        s.view(77)
            .frame
            .rows
            .iter()
            .map(|r| r.mail_id)
            .collect::<Vec<_>>(),
        vec![8, 9]
    );
    s.receive_contents(contents(517, vec![mail(1)]));
    assert_eq!(s.page, 0);
    s.click("mail_open:1");
    let mut cod = mail(1);
    cod.read = true;
    cod.cod = 500;
    s.receive_contents(contents(517, vec![cod]));
    assert!(s.click("mail_take_item:7").is_none());
    for action in [
        "mail_send",
        "mail_reply",
        "mail_delete",
        "mail_tab_send",
        "mail_take_item:0",
    ] {
        assert!(s.click(action).is_none());
    }
    s.expect_open(600);
    s.open(517);
    assert!(!s.is_open());
    s.receive_contents(contents(517, vec![mail(1)]));
    s.open(600);
    assert!(s.contents.is_none());
    s.close_for(517);
    assert!(s.is_open());
    s.close_for(600);
    assert!(!s.is_open());
}

#[test]
fn native_mailbox_interaction_uses_replicated_type_and_range() {
    let mut info = GameObjectInfo {
        entry: 140907,
        go_type: GAMEOBJECT_TYPE_MAILBOX,
        display_id: 1727,
        name: "Stormwind Mailbox".into(),
        scale: 1.0,
    };
    assert!(can_use_mailbox(&info, 5.0));
    assert!(!can_use_mailbox(&info, 5.001));
    assert!(!can_use_mailbox(&info, f32::NAN));
    info.go_type = 0;
    assert!(!can_use_mailbox(&info, 1.0));
}
