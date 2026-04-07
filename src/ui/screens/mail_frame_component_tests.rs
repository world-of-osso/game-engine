use super::*;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn make_test_state() -> MailFrameState {
    MailFrameState {
        visible: true,
        inbox: vec![
            InboxEntry {
                subject: "Auction Won".into(),
                sender: "Auction House".into(),
                has_attachment: true,
                read: false,
            },
            InboxEntry {
                subject: "Hello!".into(),
                sender: "Alice".into(),
                has_attachment: false,
                read: true,
            },
            InboxEntry {
                subject: "Gold enclosed".into(),
                sender: "Bob".into(),
                has_attachment: true,
                read: false,
            },
        ],
        ..Default::default()
    }
}

fn build_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(mail_frame_screen).sync(&shared, &mut reg);
    reg
}

fn layout_registry() -> FrameRegistry {
    let mut reg = build_registry();
    recompute_layouts(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|f| f.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

#[test]
fn builds_frame_and_title() {
    let reg = build_registry();
    assert!(reg.get_by_name("MailFrame").is_some());
    assert!(reg.get_by_name("MailFrameTitle").is_some());
}

#[test]
fn builds_tabs() {
    let reg = build_registry();
    assert!(reg.get_by_name("MailTab0").is_some());
    assert!(reg.get_by_name("MailTab1").is_some());
}

#[test]
fn builds_inbox_rows() {
    let reg = build_registry();
    assert!(reg.get_by_name("MailInboxList").is_some());
    for i in 0..3 {
        assert!(
            reg.get_by_name(&format!("MailInbox{i}")).is_some(),
            "MailInbox{i} missing"
        );
        assert!(
            reg.get_by_name(&format!("MailInbox{i}Icon")).is_some(),
            "MailInbox{i}Icon missing"
        );
        assert!(
            reg.get_by_name(&format!("MailInbox{i}Subject")).is_some(),
            "MailInbox{i}Subject missing"
        );
        assert!(
            reg.get_by_name(&format!("MailInbox{i}Sender")).is_some(),
            "MailInbox{i}Sender missing"
        );
    }
}

#[test]
fn hidden_when_not_visible() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(MailFrameState::default());
    Screen::new(mail_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("MailFrame").expect("frame");
    assert!(reg.get(id).expect("data").hidden);
}

// --- Coord validation ---

const FRAME_X: f32 = 350.0;
const FRAME_Y: f32 = 80.0;

#[test]
fn coord_main_frame() {
    let reg = layout_registry();
    let r = rect(&reg, "MailFrame");
    assert!((r.x - FRAME_X).abs() < 1.0);
    assert!((r.y - FRAME_Y).abs() < 1.0);
    assert!((r.width - FRAME_W).abs() < 1.0);
}

#[test]
fn coord_inbox_list() {
    let reg = layout_registry();
    let r = rect(&reg, "MailInboxList");
    assert!((r.x - (FRAME_X + CONTENT_INSET)).abs() < 1.0);
    assert!((r.y - (FRAME_Y + CONTENT_TOP)).abs() < 1.0);
}

#[test]
fn coord_first_inbox_row() {
    let reg = layout_registry();
    let list = rect(&reg, "MailInboxList");
    let row = rect(&reg, "MailInbox0");
    assert!((row.x - (list.x + INBOX_INSET)).abs() < 1.0);
    assert!((row.y - (list.y + INBOX_INSET)).abs() < 1.0);
    assert!((row.height - INBOX_ROW_H).abs() < 1.0);
}

// --- Send tab tests ---

#[test]
fn send_tab_builds_inputs() {
    let reg = build_registry();
    assert!(reg.get_by_name("MailSendTab").is_some());
    assert!(reg.get_by_name("MailSendToInput").is_some());
    assert!(reg.get_by_name("MailSendSubjectInput").is_some());
    assert!(reg.get_by_name("MailSendBodyInput").is_some());
}

#[test]
fn send_tab_builds_attachments() {
    let reg = build_registry();
    for i in 0..(ATTACH_COLS * 2) {
        assert!(
            reg.get_by_name(&format!("MailSendAttach{i}")).is_some(),
            "MailSendAttach{i} missing"
        );
    }
}

#[test]
fn send_tab_builds_money_and_button() {
    let reg = build_registry();
    assert!(reg.get_by_name("MailSendGoldInput").is_some());
    assert!(reg.get_by_name("MailSendSilverInput").is_some());
    assert!(reg.get_by_name("MailSendCopperInput").is_some());
    assert!(reg.get_by_name("MailSendButton").is_some());
}

#[test]
fn send_tab_hidden_by_default() {
    let reg = build_registry();
    let id = reg.get_by_name("MailSendTab").expect("send tab");
    assert!(reg.get(id).expect("data").hidden);
}

// --- Text content tests ---

fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
    use ui_toolkit::frame::WidgetData;
    let id = reg.get_by_name(name).expect(name);
    let frame = reg.get(id).expect("frame data");
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        _ => panic!("{name} is not a FontString"),
    }
}

#[test]
fn title_text() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MailFrameTitle"), "Mail");
}

#[test]
fn tab_labels() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MailTab0Label"), "Inbox");
    assert_eq!(fontstring_text(&reg, "MailTab1Label"), "Send");
}

#[test]
fn inbox_row_subject_and_sender() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MailInbox0Subject"), "Auction Won");
    assert_eq!(fontstring_text(&reg, "MailInbox0Sender"), "Auction House");
    assert_eq!(fontstring_text(&reg, "MailInbox1Subject"), "Hello!");
    assert_eq!(fontstring_text(&reg, "MailInbox1Sender"), "Alice");
    assert_eq!(fontstring_text(&reg, "MailInbox2Subject"), "Gold enclosed");
    assert_eq!(fontstring_text(&reg, "MailInbox2Sender"), "Bob");
}

#[test]
fn send_tab_field_labels() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MailSendToLabel"), "To:");
    assert_eq!(fontstring_text(&reg, "MailSendSubjectLabel"), "Subject:");
    assert_eq!(fontstring_text(&reg, "MailSendBodyLabel"), "Body:");
    assert_eq!(fontstring_text(&reg, "MailSendMoneyLabel"), "Money:");
}

#[test]
fn send_button_text() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "MailSendButtonText"), "Send Mail");
}

#[test]
fn empty_inbox_no_rows() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(MailFrameState {
        visible: true,
        ..Default::default()
    });
    Screen::new(mail_frame_screen).sync(&shared, &mut reg);
    assert!(reg.get_by_name("MailInboxList").is_some());
    assert!(reg.get_by_name("MailInbox0").is_none());
}

#[test]
fn inbox_rows_capped() {
    let inbox: Vec<InboxEntry> = (0..12)
        .map(|i| InboxEntry {
            subject: format!("Mail {i}"),
            sender: format!("Sender {i}"),
            has_attachment: false,
            read: false,
        })
        .collect();
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(MailFrameState {
        visible: true,
        inbox,
        ..Default::default()
    });
    Screen::new(mail_frame_screen).sync(&shared, &mut reg);
    for i in 0..INBOX_ROWS {
        assert!(
            reg.get_by_name(&format!("MailInbox{i}")).is_some(),
            "MailInbox{i} missing"
        );
    }
    assert!(reg.get_by_name(&format!("MailInbox{INBOX_ROWS}")).is_none());
}

#[test]
fn attachment_slot_count() {
    let reg = build_registry();
    let total = ATTACH_COLS * 2;
    for i in 0..total {
        assert!(
            reg.get_by_name(&format!("MailSendAttach{i}")).is_some(),
            "MailSendAttach{i} missing"
        );
    }
    assert!(reg.get_by_name(&format!("MailSendAttach{total}")).is_none());
}

#[test]
fn money_input_fields_exist() {
    let reg = build_registry();
    assert!(reg.get_by_name("MailSendGoldInput").is_some());
    assert!(reg.get_by_name("MailSendSilverInput").is_some());
    assert!(reg.get_by_name("MailSendCopperInput").is_some());
}
