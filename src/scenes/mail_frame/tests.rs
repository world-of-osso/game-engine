use game_engine::bag_data::InventorySlot;
use shared::protocol::{ItemStack, MailAttachment, MailHeader, MailboxContents};

use super::*;

const MAILBOX: u64 = 0x0000_0001_0000_2A01;

fn linen(slot: u8) -> MailAttachment {
    MailAttachment {
        slot,
        item: ItemStack {
            item_guid: 90 + u64::from(slot),
            item_id: 2589,
            count: 20,
            durability: None,
            soulbound: false,
        },
        name: "Linen Cloth".into(),
        quality: 1,
    }
}

fn header(mail_id: u64) -> MailHeader {
    MailHeader {
        mail_id,
        sender: "Tradea".into(),
        subject: "Linen".into(),
        body: "For your tailoring.".into(),
        money: 0,
        cod: 0,
        attachments: Vec::new(),
        expires_at: 1_790_000_000 + 30 * 86_400,
        read: false,
        returned: false,
        from_player: true,
        returnable: true,
    }
}

fn opened(mails: Vec<MailHeader>) -> MailState {
    let mut state = MailState::default();
    state.open(MAILBOX);
    state.apply(MailboxContents {
        object: MAILBOX,
        mails,
        now: 1_790_000_000,
    });
    state
}

fn registry() -> FrameRegistry {
    FrameRegistry::new(1920.0, 1080.0)
}

fn send_form_registry(mail: &MailState) -> FrameRegistry {
    let mut reg = registry();
    let mut shared = SharedContext::new();
    let mut state = mail_frame_state(mail, &InventoryState::default(), true, 0);
    state.tab = mail_ui::MailFrameTab::Send;
    shared.insert(state);
    Screen::new(mail_frame_screen).sync(&shared, &mut reg);
    reg
}

fn bags_with_linen() -> InventoryState {
    let mut inventory = InventoryState::default();
    inventory.set_item(
        0,
        2,
        InventorySlot {
            icon_fdid: 132_889,
            count: 20,
            name: "Linen Cloth".into(),
            item_guid: 41,
            item_id: 2589,
            ..Default::default()
        },
    );
    inventory
}

#[test]
fn opening_an_unread_mail_reads_it_and_shows_it() {
    let mut mail = opened(vec![header(4)]);
    let click = mail_click(
        "mail_open:4",
        &mut mail,
        &registry(),
        &InventoryState::default(),
    );
    assert_eq!(
        click.requests,
        vec![MailRequest::Act {
            mail_id: 4,
            action: MailAction::MarkRead
        }]
    );
    assert_eq!(mail.open_mail, Some(4));
    let state = mail_frame_state(&mail, &InventoryState::default(), true, 0);
    assert_eq!(state.open.unwrap().body, "For your tailoring.");
    assert!(state.rows[0].selected);
    assert_eq!(state.rows[0].expires, "30 Days");
}

#[test]
fn a_cod_item_asks_first_and_player_mail_with_items_returns() {
    let mut cod = header(4);
    cod.cod = 2_000;
    cod.attachments.push(linen(0));
    let mut mail = opened(vec![cod]);
    mail.open_mail = Some(4);
    let click = mail_click(
        "mail_take_item:0",
        &mut mail,
        &registry(),
        &InventoryState::default(),
    );
    let (popup, request) = click.popup.unwrap();
    assert_eq!(popup.key, COD_POPUP);
    assert_eq!(popup.text, "Accepting this item will cost:\n20s");
    assert_eq!(
        request,
        MailRequest::Act {
            mail_id: 4,
            action: MailAction::TakeAttachment { slot: 0 }
        }
    );
    assert!(click.requests.is_empty());

    let click = mail_click(
        "mail_delete",
        &mut mail,
        &registry(),
        &InventoryState::default(),
    );
    assert_eq!(
        click.requests,
        vec![MailRequest::Act {
            mail_id: 4,
            action: MailAction::Return
        }]
    );
    assert_eq!(mail.open_mail, None);
}

#[test]
fn deleting_auction_mail_with_an_item_confirms_with_the_item_name() {
    let mut auction = header(5);
    auction.sender = "Auction House".into();
    auction.from_player = false;
    auction.returnable = false;
    auction.attachments.push(linen(0));
    let mut mail = opened(vec![auction]);
    mail.open_mail = Some(5);
    let click = mail_click(
        "mail_delete",
        &mut mail,
        &registry(),
        &InventoryState::default(),
    );
    let (popup, request) = click.popup.unwrap();
    assert_eq!(
        popup.text,
        "Deleting this mail will also destroy Linen Cloth"
    );
    assert_eq!(
        request,
        MailRequest::Act {
            mail_id: 5,
            action: MailAction::Delete
        }
    );
}

#[test]
fn reply_fills_to_and_subject_on_the_send_tab() {
    let mut mail = opened(vec![header(4)]);
    mail.open_mail = Some(4);
    let click = mail_click(
        "mail_reply",
        &mut mail,
        &registry(),
        &InventoryState::default(),
    );
    assert_eq!(mail.tab, MailTab::Send);
    assert_eq!(
        click.texts,
        vec![
            (mail_ui::TO_BOX, "Tradea".to_string()),
            (mail_ui::SUBJECT_BOX, "RE: Linen".to_string())
        ]
    );
}

#[test]
fn send_reads_the_form_and_cod_mode_sends_the_amount_as_cod() {
    let mut mail = opened(Vec::new());
    mail.attach(41);
    mail.money_mode = SendMoneyMode::Cod;
    let mut reg = send_form_registry(&mail);
    frame_input::set_text(&mut reg, mail_ui::TO_BOX, "Tradeb");
    frame_input::set_text(&mut reg, mail_ui::MONEY_BOXES.silver, "50");

    let click = mail_click(mail_ui::ACTION_SEND, &mut mail, &reg, &bags_with_linen());

    assert_eq!(
        click.requests,
        vec![MailRequest::Send(MailDraft {
            recipient: "Tradeb".into(),
            subject: "Linen Cloth".into(),
            body: String::new(),
            attachments: vec![41],
            money: 0,
            cod: 5_000,
        })]
    );
    // No recipient: nothing is sent.
    frame_input::set_text(&mut reg, mail_ui::TO_BOX, " ");
    let click = mail_click(mail_ui::ACTION_SEND, &mut mail, &reg, &bags_with_linen());
    assert!(click.requests.is_empty());
}

#[test]
fn open_all_takes_everything_but_cod_mail() {
    let mut money = header(1);
    money.money = 500;
    let mut parcel = header(2);
    parcel.attachments = vec![linen(0), linen(3)];
    let mut cod = header(3);
    cod.cod = 10;
    cod.attachments.push(linen(0));
    let mut mail = opened(vec![money, parcel, cod]);
    let click = mail_click(
        mail_ui::ACTION_OPEN_ALL,
        &mut mail,
        &registry(),
        &InventoryState::default(),
    );
    assert_eq!(
        click.requests,
        vec![
            MailRequest::Act {
                mail_id: 1,
                action: MailAction::TakeMoney
            },
            MailRequest::Act {
                mail_id: 2,
                action: MailAction::TakeAttachment { slot: 0 }
            },
            MailRequest::Act {
                mail_id: 2,
                action: MailAction::TakeAttachment { slot: 3 }
            },
        ]
    );
}

#[test]
fn attached_bag_items_show_on_the_send_buttons_with_postage() {
    let mut mail = opened(Vec::new());
    mail.attach(41);
    let state = mail_frame_state(&mail, &bags_with_linen(), true, 20);
    assert_eq!(state.send.attachments.len(), 12);
    assert_eq!(state.send.attachments[0].as_ref().unwrap().count, 20);
    assert_eq!(state.send.postage, 30);
    assert!(state.send.postage_unaffordable && state.send.cod_enabled);
    let click = mail_click(
        "mail_attachment:0",
        &mut mail,
        &registry(),
        &bags_with_linen(),
    );
    assert_eq!(click, MailClick::default());
    assert!(mail.attachments.is_empty());
}

#[test]
fn time_left_is_days_then_hours_then_minutes() {
    assert_eq!(view::time_left(3 * 86_400 + 5, 0), ("3 Days".into(), false));
    assert_eq!(view::time_left(86_400, 0), ("1 Day".into(), false));
    assert_eq!(view::time_left(7_300, 0), ("2 Hr".into(), true));
    assert_eq!(view::time_left(30, 0), ("1 Min".into(), true));
}
