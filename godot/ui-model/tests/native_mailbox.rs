use game_engine_ui_model::bag_data::{InventorySlot, InventoryState};
use game_engine_ui_model::mail::{
    COD_ALERT_POPUP, COD_POPUP, DELETE_MAIL_POPUP, DELETE_MONEY_POPUP, MAIL_SENT_TEXT,
    MAX_COD_COPPER, MailEffect, MailOutgoing, MailSession, MailTexts, can_use_mailbox,
};
use game_engine_ui_model::mail_frame_component::{
    BODY_BOX, MONEY_BOXES, MailFrameTab, SUBJECT_BOX, TO_BOX,
};
use shared::protocol::*;

#[test]
fn uifixes_mail_body_is_multiline_and_spans_the_stationery_in_both_skins() {
    use game_engine_ui_model::mail_frame_component::{MailFrameState, mail_frame_screen};
    use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
    use ui_toolkit::frame::{Dimension, WidgetData};
    use ui_toolkit::layout_values::Val;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};
    use ui_toolkit::widgets::texture::TextureSource;
    configure_assets();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(MailFrameState {
            visible: true,
            tab: MailFrameTab::Send,
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(mail_frame_screen).sync(&shared, &mut registry);
        game_engine_ui_model::mail_frame_component::apply_mail_body_postsetup(&mut registry);
        let body = registry
            .get(registry.get_by_name(BODY_BOX).unwrap())
            .unwrap();
        let Some(WidgetData::EditBox(edit)) = &body.widget_data else {
            panic!("mail body missing")
        };
        assert!(edit.multi_line, "{skin:?}: mail body must retain newlines");
        assert_eq!(body.width, Dimension::Fixed(270.0));
        assert_eq!(body.height, Dimension::Fixed(134.0));
        assert_eq!(body.position.top, Val::Px(93.0));
        let paper = registry
            .get(
                registry
                    .get_by_name("SendStationeryBackgroundLeft")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(&paper.widget_data, Some(WidgetData::Texture(t)) if t.source == TextureSource::FileDataId(136859))
        );
        assert_eq!(paper.height, Dimension::Fixed(154.0));
    }
    set_active_skin(ActiveSkin::Modern);
}

const BOX: u64 = 517;

fn stack(guid: u64, item_id: u32, count: u32) -> ItemStack {
    ItemStack {
        item_guid: guid,
        item_id,
        count,
        durability: None,
        soulbound: false,
    }
}

fn attachment(slot: u8, guid: u64) -> MailAttachment {
    MailAttachment {
        slot,
        item: stack(guid, 2589, 5),
        name: "Linen Cloth".into(),
        quality: 1,
    }
}

fn mail(id: u64) -> MailHeader {
    MailHeader {
        mail_id: id,
        sender: "Auction House".into(),
        subject: "Auction won: Linen Cloth".into(),
        body: "Your won item is enclosed.".into(),
        money: 1200,
        cod: 0,
        attachments: vec![attachment(7, 81)],
        expires_at: 864100,
        read: false,
        returned: false,
        from_player: false,
        returnable: false,
    }
}

fn player_mail(id: u64) -> MailHeader {
    MailHeader {
        sender: "Fbalpha".into(),
        subject: "Linen".into(),
        from_player: true,
        returnable: true,
        read: true,
        ..mail(id)
    }
}

fn configure_assets() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
}

fn contents(object: u64, mails: Vec<MailHeader>) -> MailboxContents {
    MailboxContents {
        object,
        mails,
        now: 100,
    }
}

fn open(mails: Vec<MailHeader>) -> MailSession {
    let mut s = MailSession::default();
    s.expect_open(BOX);
    s.open(BOX);
    s.receive_contents(contents(BOX, mails));
    s
}

fn bags(items: &[(u64, &str, u32, bool)]) -> InventoryState {
    let mut inventory = InventoryState::default();
    for (index, (guid, name, count, soulbound)) in items.iter().enumerate() {
        inventory.set_item(
            0,
            index,
            InventorySlot {
                icon_fdid: 132_889,
                count: *count,
                name: (*name).into(),
                item_guid: *guid,
                item_id: 2589,
                soulbound: *soulbound,
                ..InventorySlot::default()
            },
        );
    }
    inventory
}

fn request(mail_id: u64, action: MailAction) -> Option<MailOutgoing> {
    Some(MailOutgoing::Request(MailRequest {
        object: BOX,
        mail_id,
        action,
    }))
}

fn click(s: &mut MailSession, action: &str) -> MailEffect {
    click_with(s, action, &MailTexts::new(), 1_000_000)
}

fn click_with(s: &mut MailSession, action: &str, texts: &MailTexts, money: u64) -> MailEffect {
    s.click(action, texts, money, &InventoryState::default())
}

/// The server carrying out `action` and resending the inbox.
fn serve(s: &mut MailSession, mail_id: u64, action: MailAction) {
    let mut mails = s.contents.clone().unwrap().mails;
    if let Some(index) = mails.iter().position(|m| m.mail_id == mail_id) {
        let mail = &mut mails[index];
        mail.read = true;
        match action {
            MailAction::TakeMoney => mail.money = 0,
            MailAction::TakeAttachment { slot } => {
                mail.attachments.retain(|a| a.slot != slot);
                mail.cod = 0;
            }
            MailAction::Return | MailAction::Delete => {
                mails.remove(index);
            }
            MailAction::MarkRead => {}
        }
    }
    s.receive_contents(contents(BOX, mails));
}

fn apply(texts: &mut MailTexts, effect: &MailEffect) {
    for (name, text) in &effect.edits {
        texts.insert(name, text.clone());
    }
}

#[test]
fn native_mailbox_claims_are_gated_and_never_change_authoritative_contents_locally() {
    configure_assets();
    let mut s = MailSession::default();
    s.expect_open(BOX);
    s.receive_contents(contents(BOX, vec![mail(9)])); // MailChannel can precede InteractionChannel.
    assert!(!s.is_open());
    s.open(BOX);
    assert!(s.is_open());
    assert_eq!(
        click(&mut s, "mail_open:9").outgoing,
        request(9, MailAction::MarkRead)
    );
    assert_eq!(click(&mut s, "mail_take_money").outgoing, None); // One request at a time.
    let mut read = mail(9);
    read.read = true;
    s.receive_contents(contents(BOX, vec![read.clone()]));
    assert_eq!(
        click(&mut s, "mail_take_money").outgoing,
        request(9, MailAction::TakeMoney)
    );
    assert_eq!(s.contents.as_ref().unwrap().mails[0].money, 1200);
    s.receive_contents(contents(999, vec![]));
    assert!(s.busy()); // Wrong object's response must not unlock the current request.
    assert_eq!(
        s.failed(MailFailed {
            object: BOX,
            error: MailError::InventoryFull
        }),
        Some("Inventory is full.")
    );
    assert_eq!(
        click(&mut s, "mail_take_item:7").outgoing,
        request(9, MailAction::TakeAttachment { slot: 7 })
    );
    assert_eq!(s.contents.as_ref().unwrap().mails[0].attachments.len(), 1);
    read.attachments.clear();
    read.money = 0;
    s.receive_contents(contents(BOX, vec![read]));
    assert_eq!(click(&mut s, "mail_take_item:7").outgoing, None);
    assert_eq!(click(&mut s, "mail_take_money").outgoing, None);
    s.close();
    s.receive_contents(contents(BOX, vec![mail(9)]));
    assert!(!s.is_open());
    assert!(s.contents.is_none());
}

#[test]
fn send_mail_attaches_bag_items_fills_the_subject_and_sends_the_typed_form() {
    configure_assets();
    let mut s = open(vec![]);
    let inventory = bags(&[(40, "Linen Cloth", 20, false), (41, "Hearthstone", 1, true)]);
    let mut texts = MailTexts::new();
    click(&mut s, "mail_tab_send");
    assert_eq!(s.tab, MailFrameTab::Send);
    let refused = s.attach(inventory.slot(0, 1).unwrap(), &texts, &inventory);
    assert_eq!(refused.error, Some("You can't mail soulbound items."));
    let attached = s.attach(inventory.slot(0, 0).unwrap(), &texts, &inventory);
    apply(&mut texts, &attached);
    assert_eq!(texts[SUBJECT_BOX], "Linen Cloth (20)");
    assert_eq!(s.attachments, vec![40]);
    let view = s.view(29, &inventory, &texts);
    assert_eq!(view.send.postage, 30);
    assert!(view.send.postage_unaffordable && view.send.cod_enabled);
    assert!(!view.send.can_send); // No recipient yet.
    texts.insert(TO_BOX, " Fbbravo ".into());
    texts.insert(BODY_BOX, "For your tailoring.".into());
    texts.insert(MONEY_BOXES.silver, "5".into());
    assert!(s.view(10_000, &inventory, &texts).send.can_send);
    let sent = s.click("mail_send", &texts, 10_000, &inventory);
    assert_eq!(
        sent.outgoing,
        Some(MailOutgoing::Send(SendMail {
            object: BOX,
            recipient: "Fbbravo".into(),
            subject: "Linen Cloth (20)".into(),
            body: "For your tailoring.".into(),
            attachments: vec![40],
            money: 500,
            cod: 0,
        }))
    );
    assert!(s.busy());
    // The sender's mailbox resend arrives with MailSent; only MailSent ends the send.
    s.receive_contents(contents(BOX, vec![]));
    assert!(s.busy());
    let done = s.mail_sent(MailSent { object: BOX }).unwrap();
    assert_eq!(done.error, Some(MAIL_SENT_TEXT));
    apply(&mut texts, &done);
    assert!(texts.values().all(String::is_empty));
    assert!(s.attachments.is_empty() && !s.busy());
}

#[test]
fn cod_needs_an_attachment_and_at_most_ten_thousand_gold() {
    configure_assets();
    let mut s = open(vec![]);
    let inventory = bags(&[(40, "Linen Cloth", 1, false)]);
    let mut texts = MailTexts::from([(TO_BOX, "Fbbravo".to_string())]);
    click(&mut s, "mail_mode_cod");
    assert!(!s.cod_mode);
    let attached = s.attach(inventory.slot(0, 0).unwrap(), &texts, &inventory);
    apply(&mut texts, &attached);
    assert_eq!(texts[SUBJECT_BOX], "Linen Cloth");
    click(&mut s, "mail_mode_cod");
    assert!(s.cod_mode);
    texts.insert(MONEY_BOXES.gold, (MAX_COD_COPPER / 10_000 + 1).to_string());
    assert!(!s.can_send(&texts));
    texts.insert(MONEY_BOXES.gold, "2".into());
    let Some(MailOutgoing::Send(send)) = s.click("mail_send", &texts, 0, &inventory).outgoing
    else {
        panic!("C.O.D. form did not send")
    };
    assert_eq!((send.money, send.cod), (0, 20_000));
    // Taking the only attachment off clears C.O.D. and the subject it filled in.
    s.failed(MailFailed {
        object: BOX,
        error: MailError::RecipientNotFound,
    });
    let detached = s.click("mail_attachment:0", &texts, 0, &inventory);
    apply(&mut texts, &detached);
    assert!(!s.cod_mode && texts[SUBJECT_BOX].is_empty());
}

#[test]
fn cod_items_confirm_the_charge_and_alert_without_the_money() {
    configure_assets();
    let mut cod = player_mail(3);
    cod.money = 0;
    cod.cod = 25_000;
    let mut s = open(vec![cod]);
    click(&mut s, "mail_open:3");
    let alert = click_with(&mut s, "mail_take_item:7", &MailTexts::new(), 24_999);
    assert_eq!(alert.popup.as_ref().unwrap().key, COD_ALERT_POPUP);
    assert_eq!(
        alert.popup.unwrap().text,
        "You do not have enough money to pay the C.O.D. charges."
    );
    assert_eq!(s.popup_result(COD_ALERT_POPUP, true), None);
    let confirm = click_with(&mut s, "mail_take_item:7", &MailTexts::new(), 25_000);
    assert_eq!(confirm.outgoing, None);
    let spec = confirm.popup.unwrap();
    assert_eq!(spec.key, COD_POPUP);
    assert_eq!(spec.text, "Accepting this item will cost:\n2g 50s");
    assert_eq!(s.popup_result(COD_POPUP, false), None);
    click_with(&mut s, "mail_take_item:7", &MailTexts::new(), 25_000);
    assert_eq!(
        s.popup_result(COD_POPUP, true),
        request(3, MailAction::TakeAttachment { slot: 7 })
    );
    assert!(s.busy());
}

#[test]
fn delete_returns_player_mail_with_contents_and_confirms_destroying_the_rest() {
    configure_assets();
    let mut letter = player_mail(4);
    letter.money = 0;
    letter.attachments.clear();
    let mut money = mail(5);
    money.attachments.clear();
    money.read = true;
    let mut s = open(vec![player_mail(2), mail(3), letter, money]);
    click(&mut s, "mail_open:2");
    assert_eq!(
        click(&mut s, "mail_delete").outgoing,
        request(2, MailAction::Return)
    );
    assert_eq!(s.selected, None);
    serve(&mut s, 2, MailAction::Return);
    click(&mut s, "mail_open:3");
    serve(&mut s, 3, MailAction::MarkRead);
    let item = click(&mut s, "mail_delete");
    let spec = item.popup.unwrap();
    assert_eq!(
        (spec.key.as_str(), spec.text.as_str()),
        (
            DELETE_MAIL_POPUP,
            "Deleting this mail will also destroy Linen Cloth"
        )
    );
    assert_eq!(
        s.popup_result(DELETE_MAIL_POPUP, true),
        request(3, MailAction::Delete)
    );
    assert_eq!(s.selected, None);
    serve(&mut s, 3, MailAction::Delete);
    click(&mut s, "mail_open:5");
    let spec = click(&mut s, "mail_delete").popup.unwrap();
    assert_eq!(
        (spec.key.as_str(), spec.text.as_str()),
        (
            DELETE_MONEY_POPUP,
            "Deleting this mail will also destroy:\n12s"
        )
    );
    assert_eq!(s.popup_result(DELETE_MONEY_POPUP, false), None);
    click(&mut s, "mail_open:4");
    assert_eq!(
        click(&mut s, "mail_delete").outgoing,
        request(4, MailAction::Delete)
    );
}

#[test]
fn reply_fills_the_send_form_and_a_sent_reply_returns_to_the_inbox() {
    configure_assets();
    let mut s = open(vec![player_mail(6), mail(7)]);
    click(&mut s, "mail_open:7");
    serve(&mut s, 7, MailAction::MarkRead);
    assert!(click(&mut s, "mail_reply").edits.is_empty()); // Auction House mail.
    click(&mut s, "mail_open:6");
    let mut texts = MailTexts::new();
    apply(&mut texts, &click(&mut s, "mail_reply"));
    assert_eq!(s.tab, MailFrameTab::Send);
    assert_eq!(
        (texts[TO_BOX].as_str(), texts[SUBJECT_BOX].as_str()),
        ("Fbalpha", "RE: Linen")
    );
    assert!(
        click_with(&mut s, "mail_send", &texts, 0)
            .outgoing
            .is_some()
    );
    s.mail_sent(MailSent { object: BOX }).unwrap();
    assert_eq!(s.tab, MailFrameTab::Inbox);
}

#[test]
fn open_all_takes_money_then_items_newest_first_skipping_cod_and_failures() {
    configure_assets();
    let mut cod = player_mail(1);
    cod.cod = 100;
    let mut two = mail(2);
    two.attachments.push(attachment(9, 82));
    let mut s = open(vec![cod.clone(), two.clone(), mail(3)]);
    assert_eq!(s.next_open_all(5), None); // Not started.
    click(&mut s, "mail_open_all");
    assert!(
        s.view(0, &InventoryState::default(), &MailTexts::new())
            .opening_all
    );
    let mut steps = Vec::new();
    let mut mails = vec![cod, two, mail(3)];
    while let Some(next) = s.next_open_all(5) {
        assert_eq!(s.next_open_all(5), None); // One request in flight.
        steps.push((next.mail_id, next.action));
        let mail = mails
            .iter_mut()
            .find(|m| m.mail_id == next.mail_id)
            .unwrap();
        match next.action {
            MailAction::TakeMoney => mail.money = 0,
            MailAction::TakeAttachment { slot: 9 } => {
                // A failed item is skipped, not retried.
                s.failed(MailFailed {
                    object: BOX,
                    error: MailError::Internal,
                });
                continue;
            }
            MailAction::TakeAttachment { slot } => mail.attachments.retain(|a| a.slot != slot),
            _ => unreachable!(),
        }
        s.receive_contents(contents(BOX, mails.clone()));
    }
    assert_eq!(
        steps,
        vec![
            (2, MailAction::TakeMoney),
            (2, MailAction::TakeAttachment { slot: 9 }),
            (2, MailAction::TakeAttachment { slot: 7 }),
            (3, MailAction::TakeMoney),
            (3, MailAction::TakeAttachment { slot: 7 }),
        ]
    );
    assert!(!s.opening_all());
    assert_eq!(mails[0].attachments.len(), 1); // C.O.D. mail untouched.
    s.receive_contents(contents(BOX, vec![mail(8)]));
    click(&mut s, "mail_open_all");
    assert_eq!(s.next_open_all(0), None); // No free bag slot stops it.
    assert!(!s.opening_all());
    click(&mut s, "mail_open_all");
    s.next_open_all(1).unwrap();
    s.failed(MailFailed {
        object: BOX,
        error: MailError::InventoryFull,
    });
    assert!(!s.opening_all());
}

#[test]
fn native_mailbox_pages_and_stale_opening_stay_bounded() {
    configure_assets();
    let mut s = open((1..=9).map(mail).collect());
    let none = InventoryState::default();
    let texts = MailTexts::new();
    assert_eq!(s.view(77, &none, &texts).rows.len(), 7);
    click(&mut s, "mail_next_page");
    assert_eq!(
        s.view(77, &none, &texts)
            .rows
            .iter()
            .map(|r| r.mail_id)
            .collect::<Vec<_>>(),
        vec![8, 9]
    );
    s.receive_contents(contents(BOX, vec![mail(1)]));
    assert_eq!(s.page, 0);
    s.expect_open(600);
    s.open(BOX);
    assert!(!s.is_open());
    s.receive_contents(contents(BOX, vec![mail(1)]));
    s.open(600);
    assert!(s.contents.is_none());
    s.close_for(BOX);
    assert!(s.is_open());
    s.close_for(600);
    assert!(!s.is_open());
}

#[test]
fn native_mailbox_authored_ui_has_both_tabs_and_disables_actions_while_pending() {
    use game_engine_ui_model::bag_frame_component::BagFrameState;
    use game_engine_ui_model::mail::{NativeMailView, native_mail_screen};
    use ui_toolkit::{
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    configure_assets();
    let mut header = player_mail(9);
    header.read = true;
    let mut session = open(vec![header.clone()]);
    click(&mut session, "mail_open:9");
    let build = |session: &MailSession| {
        let mut shared = SharedContext::new();
        shared.insert(NativeMailView {
            frame: session.view(77, &InventoryState::default(), &MailTexts::new()),
            bags: BagFrameState::default(),
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(native_mail_screen).sync(&shared, &mut registry);
        registry
    };
    let onclick = |registry: &FrameRegistry, name: &str| {
        registry
            .get(registry.get_by_name(name).unwrap())
            .unwrap()
            .onclick
            .clone()
            .unwrap_or_default()
    };
    let registry = build(&session);
    assert_eq!(onclick(&registry, "OpenMailMoneyButton"), "mail_take_money");
    assert_eq!(
        onclick(&registry, "OpenMailAttachmentButton8"),
        "mail_take_item:7"
    );
    assert_eq!(onclick(&registry, "OpenMailReplyButton"), "mail_reply");
    assert_eq!(onclick(&registry, "OpenMailDeleteButton"), "mail_delete");
    assert_eq!(onclick(&registry, "MailFrameTab2"), "mail_tab_send");
    let Some(WidgetData::FontString(subject)) = &registry
        .get(registry.get_by_name("OpenMailSubject").unwrap())
        .unwrap()
        .widget_data
    else {
        panic!("Missing subject label")
    };
    assert_eq!(subject.text, header.subject);
    click(&mut session, "mail_take_money");
    let busy = build(&session);
    for name in [
        "OpenMailMoneyButton",
        "OpenMailAttachmentButton8",
        "OpenMailDeleteButton",
        "OpenAllMail",
    ] {
        assert_eq!(onclick(&busy, name), "", "{name}");
    }
    click(&mut session, "mail_tab_send");
    let send = build(&session);
    assert!(send.get_by_name(TO_BOX).is_some());
    assert_eq!(onclick(&send, "SendMailMailButton"), "");
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

#[test]
fn minimap_mail_icon_shows_with_unread_mail_and_names_its_senders() {
    use game_engine_ui_model::minimap::{
        MINIMAP_MAIL_FRAME, MinimapClusterState, mail_tooltip_lines, minimap_cluster_screen,
    };
    use ui_toolkit::{
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    let build = |has_mail: bool| {
        let mut shared = SharedContext::new();
        shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
        shared.insert(MinimapClusterState {
            has_mail,
            ..MinimapClusterState::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(minimap_cluster_screen).sync(&shared, &mut registry);
        registry
    };
    assert!(build(false).get_by_name(MINIMAP_MAIL_FRAME).is_none());
    let registry = build(true);
    let icon = registry
        .get(registry.get_by_name(MINIMAP_MAIL_FRAME).unwrap())
        .unwrap();
    assert!(icon.mouse_enabled);
    assert_eq!(
        mail_tooltip_lines(&["Fbalpha".into(), "Auction House".into()]),
        (
            "Unread mail from:",
            vec!["Fbalpha".to_string(), "Auction House".to_string()]
        )
    );
    assert_eq!(mail_tooltip_lines(&[]), ("You have unread mail", vec![]));
}

#[test]
fn unread_senders_survive_closing_the_mailbox_but_not_a_new_connection() {
    // PendingMail arrives once per change, often while the world is still loading.
    let mut s = MailSession::default();
    s.pending_senders = vec!["Postalpha".into()];
    s.expect_open(BOX);
    s.open(BOX);
    s.close();
    assert_eq!(s.pending_senders, vec!["Postalpha".to_string()]);
    s.reset();
    assert!(s.pending_senders.is_empty());
}

#[test]
fn each_popup_answers_only_for_its_own_request() {
    configure_assets();
    let mut cod = player_mail(3);
    cod.money = 0;
    cod.cod = 50_000;
    let mut s = open(vec![mail(2), cod]);
    s.receive_contents(contents(BOX, s.contents.clone().unwrap().mails));
    click(&mut s, "mail_open:2");
    s.receive_contents(contents(BOX, {
        let mut read = mail(2);
        read.read = true;
        let mut cod = player_mail(3);
        cod.money = 0;
        cod.cod = 50_000;
        vec![read, cod]
    }));
    assert_eq!(
        click(&mut s, "mail_delete").popup.unwrap().key,
        DELETE_MAIL_POPUP
    );
    assert_eq!(s.confirming(), Some(DELETE_MAIL_POPUP));
    // The C.O.D. alert's Close must not answer the delete still on screen.
    click(&mut s, "mail_open:3");
    assert_eq!(s.confirming(), None); // Another mail opened: the delete prompt is stale.
    let alert = click_with(&mut s, "mail_take_item:7", &MailTexts::new(), 10);
    assert_eq!(alert.popup.unwrap().key, COD_ALERT_POPUP);
    assert_eq!(s.popup_result(COD_ALERT_POPUP, true), None);
    assert_eq!(s.popup_result(DELETE_MAIL_POPUP, true), None);
    // A C.O.D. prompt is answered only by its own popup.
    click_with(&mut s, "mail_take_item:7", &MailTexts::new(), 50_000);
    assert_eq!(s.confirming(), Some(COD_POPUP));
    assert_eq!(s.popup_result(DELETE_MAIL_POPUP, true), None);
    assert_eq!(
        s.popup_result(COD_POPUP, true),
        request(3, MailAction::TakeAttachment { slot: 7 })
    );
    // Closing the open mail drops its prompt.
    s.failed(MailFailed {
        object: BOX,
        error: MailError::Internal,
    });
    click_with(&mut s, "mail_take_item:7", &MailTexts::new(), 50_000);
    click(&mut s, "mail_open_close");
    assert_eq!(s.confirming(), None);
    assert_eq!(s.popup_result(COD_POPUP, true), None);
}

#[test]
fn only_contents_showing_the_request_done_end_it() {
    configure_assets();
    let mut s = open(vec![mail(9)]);
    assert_eq!(
        click(&mut s, "mail_open:9").outgoing,
        request(9, MailAction::MarkRead)
    );
    // An unrelated resend (delivery tick, another sender) leaves the request pending.
    s.receive_contents(contents(BOX, vec![mail(9), mail(10)]));
    assert!(s.busy());
    let mut read = mail(9);
    read.read = true;
    s.receive_contents(contents(BOX, vec![read.clone(), mail(10)]));
    assert!(!s.busy());
    click(&mut s, "mail_take_item:7");
    s.receive_contents(contents(BOX, vec![read.clone()]));
    assert!(s.busy());
    read.attachments.clear();
    s.receive_contents(contents(BOX, vec![read.clone()]));
    assert!(!s.busy());
    click(&mut s, "mail_take_money");
    read.money = 0;
    s.receive_contents(contents(BOX, vec![read.clone()]));
    assert!(!s.busy());
    // Emptied Auction House mail is deleted without a prompt; only its absence ends it.
    assert_eq!(
        click(&mut s, "mail_delete").outgoing,
        request(9, MailAction::Delete)
    );
    s.receive_contents(contents(BOX, vec![read, mail(10)]));
    assert!(s.busy());
    s.receive_contents(contents(BOX, vec![mail(10)]));
    assert!(!s.busy());
}

#[test]
fn the_form_keeps_its_attachments_while_a_send_is_in_flight() {
    configure_assets();
    let mut s = open(vec![]);
    let inventory = bags(&[(40, "Linen Cloth", 20, false), (41, "Wool Cloth", 5, false)]);
    let mut texts = MailTexts::from([(TO_BOX, "Fbbravo".to_string())]);
    let attached = s.attach(inventory.slot(0, 0).unwrap(), &texts, &inventory);
    apply(&mut texts, &attached);
    assert!(
        s.click("mail_send", &texts, 1_000, &inventory)
            .outgoing
            .is_some()
    );
    let late = s.attach(inventory.slot(0, 1).unwrap(), &texts, &inventory);
    assert_eq!(late, MailEffect::default());
    assert_eq!(s.attachments, vec![40]);
    assert!(s.is_attached(40) && !s.is_attached(41));
}
