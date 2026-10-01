use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: MailFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(mail_frame_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn exists(reg: &FrameRegistry, name: &str) -> bool {
    reg.get_by_name(name).is_some()
}

fn onclick(reg: &FrameRegistry, name: &str) -> Option<String> {
    reg.get(reg.get_by_name(name).expect(name))
        .unwrap()
        .onclick
        .clone()
        .filter(|action| !action.is_empty())
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout rect"))
}

fn offset(reg: &FrameRegistry, name: &str) -> (f32, f32) {
    let root = rect(reg, FRAME_NAME);
    let child = rect(reg, name);
    (child.x - root.x, child.y - root.y)
}

fn text_color(reg: &FrameRegistry, name: &str) -> [f32; 4] {
    match reg
        .get(reg.get_by_name(name).expect(name))
        .unwrap()
        .widget_data
        .as_ref()
    {
        Some(WidgetData::FontString(fs)) => fs.color,
        _ => panic!("{name} is not a FontString"),
    }
}

fn row(mail_id: u64, read: bool) -> InboxRow {
    InboxRow {
        mail_id,
        sender: "Tradea".into(),
        subject: format!("Mail {mail_id}"),
        icon_fdid: 132_889,
        count: 20,
        read,
        cod: mail_id == 2,
        expires: "30 Days".into(),
        expires_soon: false,
        selected: false,
    }
}

fn inbox() -> MailFrameState {
    MailFrameState {
        visible: true,
        rows: vec![row(1, false), row(2, true)],
        page: 0,
        page_count: 2,
        ..Default::default()
    }
}

fn linen() -> SlotItem {
    SlotItem {
        icon_fdid: 132_889,
        count: 20,
        quality_border: WHITE.into(),
    }
}

#[test]
fn inbox_rows_open_their_mail_and_unread_mail_is_gold() {
    let reg = build(inbox());
    assert_eq!(fontstring_text(&reg, "MailFrameTitleText"), "Inbox");
    assert_eq!(offset(&reg, "MailItem1Button"), (17.0, 73.0));
    assert_eq!(offset(&reg, "MailItem2Button"), (17.0, 118.0));
    assert_eq!(
        onclick(&reg, "MailItem2Button").as_deref(),
        Some("mail_open:2")
    );
    assert_eq!(text_color(&reg, "MailItem1Sender"), [1.0, 0.82, 0.0, 1.0]);
    assert_eq!(text_color(&reg, "MailItem2Sender"), [0.75, 0.75, 0.75, 1.0]);
    assert!(exists(&reg, "MailItem2ButtonCOD") && !exists(&reg, "MailItem1ButtonCOD"));
    assert_eq!(fontstring_text(&reg, "MailItem1ExpireTime"), "30 Days");
    assert!(!exists(&reg, "MailItem3Button"));
}

#[test]
fn paging_buttons_act_only_when_there_is_a_page_to_go_to() {
    let reg = build(inbox());
    assert_eq!(onclick(&reg, "InboxPrevPageButton"), None);
    assert_eq!(
        onclick(&reg, "InboxNextPageButton").as_deref(),
        Some(ACTION_NEXT)
    );
    let mut last = inbox();
    last.page = 1;
    let reg = build(last);
    assert_eq!(
        onclick(&reg, "InboxPrevPageButton").as_deref(),
        Some(ACTION_PREV)
    );
    assert_eq!(onclick(&reg, "InboxNextPageButton"), None);
}

#[test]
fn send_mail_has_the_form_twelve_attachment_buttons_and_the_postage() {
    let mut state = inbox();
    state.tab = MailFrameTab::Send;
    let mut attachments = vec![None; SEND_ATTACHMENTS];
    attachments[0] = Some(linen());
    state.send = SendView {
        attachments,
        postage: 30,
        postage_unaffordable: false,
        cod: false,
        cod_enabled: true,
        can_send: true,
    };
    let reg = build(state);
    assert_eq!(fontstring_text(&reg, "MailFrameTitleText"), "Send Mail");
    for name in [TO_BOX, SUBJECT_BOX, BODY_BOX, MONEY_BOXES.gold] {
        assert!(exists(&reg, name), "{name}");
    }
    assert!(!exists(&reg, "MailItem1Button"));
    assert_eq!(offset(&reg, "SendMailAttachment1"), (15.0, 253.0));
    assert_eq!(offset(&reg, "SendMailAttachment8"), (15.0, 297.0));
    assert!(exists(&reg, "SendMailAttachment12") && !exists(&reg, "SendMailAttachment13"));
    assert_eq!(
        onclick(&reg, "SendMailAttachment1").as_deref(),
        Some("mail_attachment:0")
    );
    assert_eq!(onclick(&reg, "SendMailAttachment2"), None);
    assert_eq!(fontstring_text(&reg, "SendMailCostMoneyFrameAmount0"), "30");
    assert_eq!(
        onclick(&reg, "SendMailCODButton").as_deref(),
        Some(ACTION_MODE_COD)
    );
    assert_eq!(fontstring_text(&reg, "SendMailMoneyText"), "Send Money:");
}

#[test]
fn send_waits_for_a_sendable_form_and_a_free_mailbox() {
    let mut state = inbox();
    state.tab = MailFrameTab::Send;
    state.send.attachments = vec![Some(linen())];
    state.send.can_send = true;
    assert_eq!(
        onclick(&build(state.clone()), "SendMailMailButton").as_deref(),
        Some(ACTION_SEND)
    );
    state.busy = true;
    let reg = build(state.clone());
    assert_eq!(onclick(&reg, "SendMailMailButton"), None);
    assert_eq!(onclick(&reg, "SendMailAttachment1"), None);
    state.busy = false;
    state.send.can_send = false;
    assert_eq!(onclick(&build(state), "SendMailMailButton"), None);
}

#[test]
fn open_all_shows_opening_while_it_takes_attachments() {
    let mut state = inbox();
    assert_eq!(
        onclick(&build(state.clone()), "OpenAllMail").as_deref(),
        Some(ACTION_OPEN_ALL)
    );
    state.opening_all = true;
    let reg = build(state);
    assert_eq!(fontstring_text(&reg, "OpenAllMail"), "Opening...");
    assert_eq!(onclick(&reg, "OpenAllMail"), None);
}

#[test]
fn cod_needs_an_attachment() {
    let mut state = inbox();
    state.tab = MailFrameTab::Send;
    state.send.cod_enabled = false;
    let reg = build(state);
    assert_eq!(onclick(&reg, "SendMailCODButton"), None);
}

#[test]
fn open_mail_takes_money_and_items_and_returns_undeletable_player_mail() {
    let mut state = inbox();
    state.open = Some(OpenMailView {
        sender: "Tradea".into(),
        subject: "Linen".into(),
        body: "For your tailoring.".into(),
        money: 5_000,
        cod: 0,
        attachments: vec![OpenAttachment {
            slot: 3,
            item: linen(),
        }],
        can_reply: true,
        can_delete: false,
    });
    let reg = build(state);
    assert_eq!(offset(&reg, OPEN_MAIL_NAME), (384.0, 0.0));
    assert_eq!(fontstring_text(&reg, "OpenMailSenderName"), "Tradea");
    assert_eq!(
        fontstring_text(&reg, "OpenMailBodyText"),
        "For your tailoring."
    );
    assert_eq!(
        onclick(&reg, "OpenMailMoneyButton").as_deref(),
        Some(ACTION_TAKE_MONEY)
    );
    assert_eq!(
        onclick(&reg, "OpenMailAttachmentButton4").as_deref(),
        Some("mail_take_item:3")
    );
    assert_eq!(fontstring_text(&reg, "OpenMailDeleteButton"), "Return");
    assert_eq!(
        onclick(&reg, "OpenMailReplyButton").as_deref(),
        Some(ACTION_REPLY)
    );
    assert_eq!(
        fontstring_text(&reg, "OpenMailAttachmentText"),
        "Take Attachments:"
    );
}
