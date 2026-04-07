use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const FRAME_W: f32 = 360.0;
pub const FRAME_H: f32 = 440.0;
const HEADER_H: f32 = 28.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
const CONTENT_INSET: f32 = 8.0;
const INBOX_ROW_H: f32 = 36.0;
const INBOX_ROW_GAP: f32 = 1.0;
const INBOX_ICON_SIZE: f32 = 24.0;
const INBOX_INSET: f32 = 4.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";
const INBOX_ICON_BG: &str = "0.1,0.1,0.1,0.9";
const SUBJECT_COLOR: &str = "1.0,1.0,1.0,1.0";
const SENDER_COLOR: &str = "0.7,0.7,0.7,1.0";

// Send tab layout
const SEND_INSET: f32 = 8.0;
const SEND_LABEL_W: f32 = 70.0;
const SEND_INPUT_H: f32 = 22.0;
const SEND_BODY_H: f32 = 80.0;
const SEND_ROW_GAP: f32 = 6.0;
const ATTACH_SLOT_SIZE: f32 = 28.0;
const ATTACH_SLOT_GAP: f32 = 4.0;
const ATTACH_COLS: usize = 4;
const MONEY_INPUT_W: f32 = 50.0;
const MONEY_GAP: f32 = 4.0;
const SEND_BTN_W: f32 = 80.0;
const SEND_BTN_H: f32 = 24.0;
const SEND_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const SEND_INPUT_BG: &str = "0.1,0.1,0.1,0.9";
const SEND_INPUT_COLOR: &str = "1.0,1.0,1.0,1.0";
const ATTACH_BG: &str = "0.08,0.07,0.06,0.88";
const MONEY_LABEL_COLOR: &str = "1.0,0.82,0.0,1.0";
const SEND_BTN_BG: &str = "0.15,0.12,0.05,0.95";
const SEND_BTN_TEXT: &str = "1.0,0.82,0.0,1.0";

pub const INBOX_ROWS: usize = 7;

#[derive(Clone, Debug, PartialEq)]
pub struct MailTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InboxEntry {
    pub subject: String,
    pub sender: String,
    pub has_attachment: bool,
    pub read: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SendMailState {
    pub recipient: String,
    pub subject: String,
    pub body: String,
    pub gold: String,
    pub silver: String,
    pub copper: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MailFrameState {
    pub visible: bool,
    pub tabs: Vec<MailTab>,
    pub inbox: Vec<InboxEntry>,
    pub send: SendMailState,
}

impl Default for MailFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                MailTab {
                    name: "Inbox".into(),
                    active: true,
                },
                MailTab {
                    name: "Send".into(),
                    active: false,
                },
            ],
            inbox: vec![],
            send: SendMailState::default(),
        }
    }
}

pub fn mail_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<MailFrameState>()
        .expect("MailFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "MailFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "350",
                y: "-80",
            }
            {title_bar()}
            {tab_row(&state.tabs)}
            {inbox_list(&state.inbox)}
            {send_tab(&state.send)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "MailFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Mail",
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: "0",
            }
        }
    }
}

fn tab_row(tabs: &[MailTab]) -> Element {
    let count = tabs.len().max(1) as f32;
    let tab_w = (FRAME_W - 2.0 * TAB_INSET - (count - 1.0) * TAB_GAP) / count;
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let x = TAB_INSET + i as f32 * (tab_w + TAB_GAP);
            let y = -(HEADER_H + TAB_GAP);
            tab_button(i, tab, tab_w, x, y)
        })
        .collect()
}

fn tab_button(i: usize, tab: &MailTab, tab_w: f32, x: f32, y: f32) -> Element {
    let tab_id = DynName(format!("MailTab{i}"));
    let label_id = DynName(format!("MailTab{i}Label"));
    let (bg, color) = if tab.active {
        (TAB_BG_ACTIVE, TAB_TEXT_ACTIVE)
    } else {
        (TAB_BG_INACTIVE, TAB_TEXT_INACTIVE)
    };
    rsx! {
        r#frame {
            name: tab_id,
            width: {tab_w},
            height: {TAB_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {mail_tab_label(label_id, &tab.name, tab_w, color)}
        }
    }
}

fn mail_tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {TAB_H},
            text: text,
            font_size: 11.0,
            font_color: color,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn inbox_list(inbox: &[InboxEntry]) -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    let rows: Element = inbox
        .iter()
        .enumerate()
        .take(INBOX_ROWS)
        .flat_map(|(i, entry)| inbox_row(i, entry, content_w))
        .collect();
    rsx! {
        r#frame {
            name: "MailInboxList",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
            {rows}
        }
    }
}

fn inbox_row(idx: usize, entry: &InboxEntry, parent_w: f32) -> Element {
    let row_id = DynName(format!("MailInbox{idx}"));
    let y = -(INBOX_INSET + idx as f32 * (INBOX_ROW_H + INBOX_ROW_GAP));
    let row_w = parent_w - 2.0 * INBOX_INSET;
    let text_x = INBOX_ICON_SIZE + 8.0;
    let text_w = row_w - text_x;
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {INBOX_ROW_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INBOX_INSET},
                y: {y},
            }
            {inbox_icon(DynName(format!("MailInbox{idx}Icon")))}
            {inbox_subject(DynName(format!("MailInbox{idx}Subject")), &entry.subject, text_w, text_x)}
            {inbox_sender(DynName(format!("MailInbox{idx}Sender")), &entry.sender, text_w, text_x)}
        }
    }
}

fn inbox_icon(id: DynName) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {INBOX_ICON_SIZE},
            height: {INBOX_ICON_SIZE},
            background_color: INBOX_ICON_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: {-((INBOX_ROW_H - INBOX_ICON_SIZE) / 2.0)} }
        }
    }
}

fn inbox_subject(id: DynName, text: &str, w: f32, x: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: 16.0,
            text: text,
            font_size: 10.0,
            font_color: SUBJECT_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: "-2" }
        }
    }
}

fn inbox_sender(id: DynName, text: &str, w: f32, x: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: 14.0,
            text: text,
            font_size: 8.0,
            font_color: SENDER_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: "-18" }
        }
    }
}

// --- Send tab ---

fn send_tab(send: &SendMailState) -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    let input_w = content_w - SEND_LABEL_W - 3.0 * SEND_INSET;
    rsx! {
        r#frame {
            name: "MailSendTab",
            width: {content_w},
            height: {content_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
            {send_input_row("MailSendTo", "To:", 0, input_w)}
            {send_input_row("MailSendSubject", "Subject:", 1, input_w)}
            {send_body_area(input_w)}
            {send_attachments_grid()}
            {send_money_row(input_w)}
            {send_button()}
        }
    }
}

fn send_input_row(prefix: &str, label: &str, row: usize, input_w: f32) -> Element {
    let label_name = DynName(format!("{prefix}Label"));
    let input_name = DynName(format!("{prefix}Input"));
    let y = -(SEND_INSET + row as f32 * (SEND_INPUT_H + SEND_ROW_GAP));
    rsx! {
        fontstring {
            name: label_name,
            width: {SEND_LABEL_W},
            height: {SEND_INPUT_H},
            text: label,
            font_size: 10.0,
            font_color: SEND_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SEND_INSET},
                y: {y},
            }
        }
        r#frame {
            name: input_name,
            width: {input_w},
            height: {SEND_INPUT_H},
            background_color: SEND_INPUT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SEND_INSET + SEND_LABEL_W + SEND_INSET},
                y: {y},
            }
        }
    }
}

fn send_body_area(input_w: f32) -> Element {
    let y = -(SEND_INSET + 2.0 * (SEND_INPUT_H + SEND_ROW_GAP));
    rsx! {
        fontstring {
            name: "MailSendBodyLabel",
            width: {SEND_LABEL_W},
            height: {SEND_INPUT_H},
            text: "Body:",
            font_size: 10.0,
            font_color: SEND_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SEND_INSET},
                y: {y},
            }
        }
        r#frame {
            name: "MailSendBodyInput",
            width: {input_w},
            height: {SEND_BODY_H},
            background_color: SEND_INPUT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SEND_INSET + SEND_LABEL_W + SEND_INSET},
                y: {y},
            }
        }
    }
}

fn send_attachments_grid() -> Element {
    let base_y = SEND_INSET + 2.0 * (SEND_INPUT_H + SEND_ROW_GAP) + SEND_BODY_H + SEND_ROW_GAP;
    let x_start = SEND_INSET + SEND_LABEL_W + SEND_INSET;
    (0..(ATTACH_COLS * 2))
        .flat_map(|i| {
            let col = i % ATTACH_COLS;
            let row = i / ATTACH_COLS;
            let x = x_start + col as f32 * (ATTACH_SLOT_SIZE + ATTACH_SLOT_GAP);
            let y = -(base_y + row as f32 * (ATTACH_SLOT_SIZE + ATTACH_SLOT_GAP));
            let slot_name = DynName(format!("MailSendAttach{i}"));
            rsx! {
                r#frame {
                    name: slot_name,
                    width: {ATTACH_SLOT_SIZE},
                    height: {ATTACH_SLOT_SIZE},
                    background_color: ATTACH_BG,
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: {x},
                        y: {y},
                    }
                }
            }
        })
        .collect()
}

fn send_money_row(input_w: f32) -> Element {
    let base_y = SEND_INSET
        + 2.0 * (SEND_INPUT_H + SEND_ROW_GAP)
        + SEND_BODY_H
        + SEND_ROW_GAP
        + 2.0 * (ATTACH_SLOT_SIZE + ATTACH_SLOT_GAP)
        + SEND_ROW_GAP;
    let x_start = SEND_INSET + SEND_LABEL_W + SEND_INSET;
    rsx! {
        fontstring {
            name: "MailSendMoneyLabel",
            width: {SEND_LABEL_W},
            height: {SEND_INPUT_H},
            text: "Money:",
            font_size: 10.0,
            font_color: SEND_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SEND_INSET},
                y: {-base_y},
            }
        }
        r#frame {
            name: "MailSendGoldInput",
            width: {MONEY_INPUT_W},
            height: {SEND_INPUT_H},
            background_color: SEND_INPUT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x_start},
                y: {-base_y},
            }
        }
        r#frame {
            name: "MailSendSilverInput",
            width: {MONEY_INPUT_W},
            height: {SEND_INPUT_H},
            background_color: SEND_INPUT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x_start + MONEY_INPUT_W + MONEY_GAP},
                y: {-base_y},
            }
        }
        r#frame {
            name: "MailSendCopperInput",
            width: {MONEY_INPUT_W},
            height: {SEND_INPUT_H},
            background_color: SEND_INPUT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x_start + 2.0 * (MONEY_INPUT_W + MONEY_GAP)},
                y: {-base_y},
            }
        }
    }
}

fn send_button() -> Element {
    let base_y = SEND_INSET
        + 2.0 * (SEND_INPUT_H + SEND_ROW_GAP)
        + SEND_BODY_H
        + SEND_ROW_GAP
        + 2.0 * (ATTACH_SLOT_SIZE + ATTACH_SLOT_GAP)
        + SEND_ROW_GAP
        + SEND_INPUT_H
        + SEND_ROW_GAP;
    let x = SEND_INSET + SEND_LABEL_W + SEND_INSET;
    rsx! {
        r#frame {
            name: "MailSendButton",
            width: {SEND_BTN_W},
            height: {SEND_BTN_H},
            background_color: SEND_BTN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-base_y},
            }
            fontstring {
                name: "MailSendButtonText",
                width: {SEND_BTN_W},
                height: {SEND_BTN_H},
                text: "Send Mail",
                font_size: 10.0,
                font_color: SEND_BTN_TEXT,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
