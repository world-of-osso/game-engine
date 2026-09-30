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
fn configure_assets() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
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
    configure_assets();
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
    configure_assets();
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
fn native_mailbox_authored_receiving_ui_disables_pending_claims_and_excludes_sending() {
    use game_engine_ui_model::bag_frame_component::BagFrameState;
    use game_engine_ui_model::mail::{NativeMailView, native_mail_screen};
    use ui_toolkit::{
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    configure_assets();
    let mut session = MailSession::default();
    session.expect_open(517);
    session.open(517);
    let mut header = mail(9);
    header.read = true;
    session.receive_contents(contents(517, vec![header.clone()]));
    session.click("mail_open:9");
    let build = |session: &MailSession| {
        let mut shared = SharedContext::new();
        shared.insert(NativeMailView {
            inbox: session.view(77),
            bags: BagFrameState::default(),
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(native_mail_screen).sync(&shared, &mut registry);
        registry
    };
    let registry = build(&session);
    let frame = |name: &str| registry.get(registry.get_by_name(name).unwrap()).unwrap();
    assert_eq!(
        frame("OpenMailMoneyButton").onclick.as_deref(),
        Some("mail_take_money")
    );
    assert_eq!(
        frame("OpenMailAttachmentButton8").onclick.as_deref(),
        Some("mail_take_item:7")
    );
    let Some(WidgetData::FontString(subject)) = &frame("OpenMailSubject").widget_data else {
        panic!("Missing subject label")
    };
    assert_eq!(subject.text, header.subject);
    assert!(registry.get_by_name("SendMailNameEditBox").is_none());
    assert!(registry.get_by_name("MailFrameTab2").is_none());
    assert_eq!(
        frame("OpenMailReplyButton")
            .onclick
            .as_deref()
            .unwrap_or(""),
        ""
    );
    assert_eq!(
        frame("OpenMailDeleteButton")
            .onclick
            .as_deref()
            .unwrap_or(""),
        ""
    );
    session.click("mail_take_money");
    let busy = build(&session);
    for name in ["OpenMailMoneyButton", "OpenMailAttachmentButton8"] {
        assert_eq!(
            busy.get(busy.get_by_name(name).unwrap())
                .unwrap()
                .onclick
                .as_deref()
                .unwrap_or(""),
            ""
        );
    }
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
