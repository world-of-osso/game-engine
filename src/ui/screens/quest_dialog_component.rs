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

// --- Layout constants ---

/// Matches wow-ui-sim QuestFrame (350×450).
pub const FRAME_W: f32 = 350.0;
pub const FRAME_H: f32 = 450.0;
const HEADER_H: f32 = 28.0;
const INSET: f32 = 10.0;
const CONTENT_TOP: f32 = HEADER_H + 4.0;

const PORTRAIT_SIZE: f32 = 60.0;
const PORTRAIT_INSET: f32 = 12.0;
const NPC_NAME_H: f32 = 20.0;

const TEXT_TOP: f32 = CONTENT_TOP + PORTRAIT_SIZE + 8.0;
const TEXT_INSET: f32 = 12.0;
const TEXT_AREA_H: f32 = 130.0;

const REQ_HEADER_H: f32 = 20.0;
const REQ_ROW_H: f32 = 28.0;
const REQ_ICON_SIZE: f32 = 24.0;
const REQ_GAP: f32 = 4.0;
const REQ_NAME_W: f32 = 200.0;

const BTN_W: f32 = 100.0;
const BTN_H: f32 = 26.0;
const BTN_GAP: f32 = 12.0;

// --- Colors ---

const FRAME_BG: &str = "0.06,0.05,0.04,0.95";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const PORTRAIT_BG: &str = "0.08,0.08,0.08,0.9";
const NPC_NAME_COLOR: &str = "1.0,0.82,0.0,1.0";
const QUEST_TEXT_COLOR: &str = "0.85,0.85,0.85,1.0";
const TEXT_AREA_BG: &str = "0.0,0.0,0.0,0.2";
const REQ_HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
const REQ_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const REQ_COUNT_COLOR: &str = "0.7,0.7,0.7,1.0";
const REQ_ICON_BG: &str = "0.08,0.08,0.08,0.8";
const ACCEPT_BTN_BG: &str = "0.15,0.25,0.1,0.95";
const ACCEPT_BTN_TEXT: &str = "0.2,1.0,0.2,1.0";
const DECLINE_BTN_BG: &str = "0.2,0.08,0.08,0.95";
const DECLINE_BTN_TEXT: &str = "1.0,0.3,0.3,1.0";
const COMPLETE_BTN_BG: &str = "0.15,0.25,0.1,0.95";
const COMPLETE_BTN_TEXT: &str = "0.2,1.0,0.2,1.0";

// --- Data types ---

#[derive(Clone, Debug, PartialEq)]
pub struct RequiredItem {
    pub name: String,
    pub icon_fdid: u32,
    pub current: u32,
    pub required: u32,
}

impl RequiredItem {
    pub fn is_satisfied(&self) -> bool {
        self.current >= self.required
    }

    pub fn count_text(&self) -> String {
        format!("{}/{}", self.current, self.required)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuestDialogMode {
    /// NPC offering a new quest — show Accept/Decline.
    Offer,
    /// NPC ready to accept turn-in — show Complete/Cancel.
    TurnIn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuestDialogState {
    pub visible: bool,
    pub mode: QuestDialogMode,
    pub npc_name: String,
    pub quest_title: String,
    pub quest_text: String,
    pub required_items: Vec<RequiredItem>,
}

impl Default for QuestDialogState {
    fn default() -> Self {
        Self {
            visible: false,
            mode: QuestDialogMode::Offer,
            npc_name: String::new(),
            quest_title: String::new(),
            quest_text: String::new(),
            required_items: vec![],
        }
    }
}

// --- Screen entry ---

pub fn quest_dialog_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<QuestDialogState>()
        .expect("QuestDialogState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "QuestDialogFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: "0",
                y: "0",
            }
            {dialog_title(&state.quest_title)}
            {npc_portrait(&state.npc_name)}
            {quest_text_area(&state.quest_text)}
            {requirement_items(&state.required_items)}
            {dialog_buttons(&state.mode)}
        }
    }
}

// --- Title bar ---

fn dialog_title(quest_title: &str) -> Element {
    rsx! {
        fontstring {
            name: "QuestDialogTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: quest_title,
            font_size: 14.0,
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

// --- NPC portrait + name ---

fn npc_portrait(npc_name: &str) -> Element {
    let portrait_y = -CONTENT_TOP;
    rsx! {
        r#frame {
            name: "QuestDialogPortrait",
            width: {PORTRAIT_SIZE},
            height: {PORTRAIT_SIZE},
            background_color: PORTRAIT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {PORTRAIT_INSET},
                y: {portrait_y},
            }
        }
        fontstring {
            name: "QuestDialogNPCName",
            width: {FRAME_W - PORTRAIT_INSET - PORTRAIT_SIZE - 8.0},
            height: {NPC_NAME_H},
            text: npc_name,
            font_size: 12.0,
            font_color: NPC_NAME_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {PORTRAIT_INSET + PORTRAIT_SIZE + 8.0},
                y: {portrait_y},
            }
        }
    }
}

// --- Quest text scroll area ---

fn quest_text_area(quest_text: &str) -> Element {
    let text_w = FRAME_W - 2.0 * TEXT_INSET;
    rsx! {
        r#frame {
            name: "QuestDialogTextArea",
            width: {text_w},
            height: {TEXT_AREA_H},
            background_color: TEXT_AREA_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {TEXT_INSET},
                y: {-TEXT_TOP},
            }
            fontstring {
                name: "QuestDialogText",
                width: {text_w - 8.0},
                height: {TEXT_AREA_H - 8.0},
                text: quest_text,
                font_size: 11.0,
                font_color: QUEST_TEXT_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "-4",
                }
            }
        }
    }
}

// --- Requirement items ---

fn reqs_header_label(w: f32) -> Element {
    rsx! {
        fontstring {
            name: "QuestDialogReqsHeader",
            width: {w},
            height: {REQ_HEADER_H},
            text: "Requirements",
            font_size: 12.0,
            font_color: REQ_HEADER_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: "0" }
        }
    }
}

fn requirement_items(items: &[RequiredItem]) -> Element {
    let hide_reqs = items.is_empty();
    let req_top = TEXT_TOP + TEXT_AREA_H + 8.0;
    let req_w = FRAME_W - 2.0 * INSET;
    let rows: Element = items
        .iter()
        .enumerate()
        .flat_map(|(i, item)| {
            let row_y = REQ_HEADER_H + REQ_GAP + i as f32 * (REQ_ROW_H + REQ_GAP);
            requirement_row(i, item, row_y)
        })
        .collect();
    rsx! {
        r#frame {
            name: "QuestDialogReqs",
            width: {req_w},
            height: {REQ_HEADER_H + items.len() as f32 * (REQ_ROW_H + REQ_GAP) + REQ_GAP},
            hidden: hide_reqs,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-req_top},
            }
            {reqs_header_label(req_w)}
            {rows}
        }
    }
}

fn requirement_row(idx: usize, item: &RequiredItem, y: f32) -> Element {
    let row_id = DynName(format!("QuestDialogReq{idx}"));
    let count_text = item.count_text();
    rsx! {
        r#frame {
            name: row_id,
            width: {REQ_ICON_SIZE + 8.0 + REQ_NAME_W + 60.0},
            height: {REQ_ROW_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "4",
                y: {-y},
            }
            {req_icon(DynName(format!("QuestDialogReq{idx}Icon")))}
            {req_name_label(DynName(format!("QuestDialogReq{idx}Name")), &item.name)}
            {req_count_label(DynName(format!("QuestDialogReq{idx}Count")), &count_text)}
        }
    }
}

fn req_icon(id: DynName) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {REQ_ICON_SIZE},
            height: {REQ_ICON_SIZE},
            background_color: REQ_ICON_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: "-2" }
        }
    }
}

fn req_name_label(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {REQ_NAME_W},
            height: {REQ_ROW_H},
            text: text,
            font_size: 10.0,
            font_color: REQ_NAME_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {REQ_ICON_SIZE + 8.0}, y: "0" }
        }
    }
}

fn req_count_label(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: 50.0,
            height: {REQ_ROW_H},
            text: text,
            font_size: 10.0,
            font_color: REQ_COUNT_COLOR,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {REQ_ICON_SIZE + 8.0 + REQ_NAME_W}, y: "0" }
        }
    }
}

// --- Dialog buttons ---

fn dialog_btn(name: &str, label: &str, bg: &str, color: &str, x: f32, y: f32) -> Element {
    let btn_id = DynName(name.into());
    let text_id = DynName(format!("{name}Text"));
    rsx! {
        r#frame {
            name: btn_id,
            width: {BTN_W},
            height: {BTN_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: text_id,
                width: {BTN_W},
                height: {BTN_H},
                text: label,
                font_size: 11.0,
                font_color: color,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn dialog_buttons(mode: &QuestDialogMode) -> Element {
    let y = -(FRAME_H - BTN_H - 10.0);
    let center = FRAME_W / 2.0;
    let left_x = center - BTN_W - BTN_GAP / 2.0;
    let right_x = center + BTN_GAP / 2.0;
    let (left_label, left_bg, left_color, right_label, right_bg, right_color) = match mode {
        QuestDialogMode::Offer => (
            "Accept",
            ACCEPT_BTN_BG,
            ACCEPT_BTN_TEXT,
            "Decline",
            DECLINE_BTN_BG,
            DECLINE_BTN_TEXT,
        ),
        QuestDialogMode::TurnIn => (
            "Complete",
            COMPLETE_BTN_BG,
            COMPLETE_BTN_TEXT,
            "Cancel",
            DECLINE_BTN_BG,
            DECLINE_BTN_TEXT,
        ),
    };
    rsx! {
        {dialog_btn("QuestDialogLeftBtn", left_label, left_bg, left_color, left_x, y)}
        {dialog_btn("QuestDialogRightBtn", right_label, right_bg, right_color, right_x, y)}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn offer_state() -> QuestDialogState {
        QuestDialogState {
            visible: true,
            mode: QuestDialogMode::Offer,
            npc_name: "Marshal Gryan".into(),
            quest_title: "The People's Militia".into(),
            quest_text: "The Defias Brotherhood is causing trouble.".into(),
            required_items: vec![],
        }
    }

    fn turnin_state() -> QuestDialogState {
        QuestDialogState {
            visible: true,
            mode: QuestDialogMode::TurnIn,
            npc_name: "Marshal Gryan".into(),
            quest_title: "The People's Militia".into(),
            quest_text: "Bring me the bandanas.".into(),
            required_items: vec![
                RequiredItem {
                    name: "Red Bandana".into(),
                    icon_fdid: 300001,
                    current: 8,
                    required: 10,
                },
                RequiredItem {
                    name: "Defias Orders".into(),
                    icon_fdid: 300002,
                    current: 1,
                    required: 1,
                },
            ],
        }
    }

    fn build_offer_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(offer_state());
        Screen::new(quest_dialog_screen).sync(&shared, &mut reg);
        reg
    }

    fn build_turnin_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(turnin_state());
        Screen::new(quest_dialog_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_offer_registry() -> FrameRegistry {
        let mut reg = build_offer_registry();
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
        let id = reg.get_by_name(name).expect(name);
        let frame = reg.get(id).expect("frame data");
        match frame.widget_data.as_ref() {
            Some(WidgetData::FontString(fs)) => fs.text.clone(),
            _ => panic!("{name} is not a FontString"),
        }
    }

    // --- Structure tests ---

    #[test]
    fn builds_frame_and_title() {
        let reg = build_offer_registry();
        assert!(reg.get_by_name("QuestDialogFrame").is_some());
        assert!(reg.get_by_name("QuestDialogTitle").is_some());
    }

    #[test]
    fn builds_npc_portrait_and_name() {
        let reg = build_offer_registry();
        assert!(reg.get_by_name("QuestDialogPortrait").is_some());
        assert!(reg.get_by_name("QuestDialogNPCName").is_some());
    }

    #[test]
    fn builds_quest_text_area() {
        let reg = build_offer_registry();
        assert!(reg.get_by_name("QuestDialogTextArea").is_some());
        assert!(reg.get_by_name("QuestDialogText").is_some());
    }

    #[test]
    fn builds_dialog_buttons() {
        let reg = build_offer_registry();
        assert!(reg.get_by_name("QuestDialogLeftBtn").is_some());
        assert!(reg.get_by_name("QuestDialogRightBtn").is_some());
        assert!(reg.get_by_name("QuestDialogLeftBtnText").is_some());
        assert!(reg.get_by_name("QuestDialogRightBtnText").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(QuestDialogState::default());
        Screen::new(quest_dialog_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("QuestDialogFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Mode-specific button labels ---

    #[test]
    fn offer_mode_shows_accept_decline() {
        let reg = build_offer_registry();
        assert_eq!(fontstring_text(&reg, "QuestDialogLeftBtnText"), "Accept");
        assert_eq!(fontstring_text(&reg, "QuestDialogRightBtnText"), "Decline");
    }

    #[test]
    fn turnin_mode_shows_complete_cancel() {
        let reg = build_turnin_registry();
        assert_eq!(fontstring_text(&reg, "QuestDialogLeftBtnText"), "Complete");
        assert_eq!(fontstring_text(&reg, "QuestDialogRightBtnText"), "Cancel");
    }

    // --- Requirement items ---

    #[test]
    fn builds_requirement_rows() {
        let reg = build_turnin_registry();
        assert!(reg.get_by_name("QuestDialogReqs").is_some());
        assert!(reg.get_by_name("QuestDialogReqsHeader").is_some());
        assert!(reg.get_by_name("QuestDialogReq0").is_some());
        assert!(reg.get_by_name("QuestDialogReq0Icon").is_some());
        assert!(reg.get_by_name("QuestDialogReq0Name").is_some());
        assert!(reg.get_by_name("QuestDialogReq0Count").is_some());
        assert!(reg.get_by_name("QuestDialogReq1").is_some());
    }

    #[test]
    fn reqs_hidden_when_empty() {
        let reg = build_offer_registry();
        let id = reg.get_by_name("QuestDialogReqs").expect("reqs frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Data model tests ---

    #[test]
    fn required_item_satisfaction() {
        let satisfied = RequiredItem {
            name: "A".into(),
            icon_fdid: 1,
            current: 5,
            required: 5,
        };
        assert!(satisfied.is_satisfied());
        let partial = RequiredItem {
            name: "B".into(),
            icon_fdid: 2,
            current: 3,
            required: 5,
        };
        assert!(!partial.is_satisfied());
    }

    #[test]
    fn required_item_count_text() {
        let item = RequiredItem {
            name: "X".into(),
            icon_fdid: 1,
            current: 3,
            required: 10,
        };
        assert_eq!(item.count_text(), "3/10");
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_offer_registry();
        let r = rect(&reg, "QuestDialogFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_portrait() {
        let reg = layout_offer_registry();
        let frame_r = rect(&reg, "QuestDialogFrame");
        let r = rect(&reg, "QuestDialogPortrait");
        assert!((r.x - (frame_r.x + PORTRAIT_INSET)).abs() < 1.0);
        assert!((r.width - PORTRAIT_SIZE).abs() < 1.0);
        assert!((r.height - PORTRAIT_SIZE).abs() < 1.0);
    }

    #[test]
    fn coord_text_area() {
        let reg = layout_offer_registry();
        let frame_r = rect(&reg, "QuestDialogFrame");
        let r = rect(&reg, "QuestDialogTextArea");
        assert!((r.x - (frame_r.x + TEXT_INSET)).abs() < 1.0);
        assert!((r.height - TEXT_AREA_H).abs() < 1.0);
    }

    #[test]
    fn coord_npc_name_right_of_portrait() {
        let reg = layout_offer_registry();
        let portrait_r = rect(&reg, "QuestDialogPortrait");
        let name_r = rect(&reg, "QuestDialogNPCName");
        let expected_x = portrait_r.x + PORTRAIT_SIZE + 8.0;
        assert!((name_r.x - expected_x).abs() < 1.0);
        assert!((name_r.y - portrait_r.y).abs() < 1.0);
    }

    #[test]
    fn coord_text_below_portrait() {
        let reg = layout_offer_registry();
        let frame_r = rect(&reg, "QuestDialogFrame");
        let text_r = rect(&reg, "QuestDialogTextArea");
        let expected_y = frame_r.y + TEXT_TOP;
        assert!((text_r.y - expected_y).abs() < 1.0);
    }

    #[test]
    fn coord_buttons_centered_at_bottom() {
        let reg = layout_offer_registry();
        let frame_r = rect(&reg, "QuestDialogFrame");
        let left_r = rect(&reg, "QuestDialogLeftBtn");
        let right_r = rect(&reg, "QuestDialogRightBtn");
        // Buttons pinned 10px + BTN_H from bottom
        let expected_btn_y = frame_r.y + FRAME_H - BTN_H - 10.0;
        assert!((left_r.y - expected_btn_y).abs() < 1.0);
        assert!((right_r.y - expected_btn_y).abs() < 1.0);
        // Buttons centered: left_x + BTN_W + gap == right_x
        let center = frame_r.x + FRAME_W / 2.0;
        let expected_left_x = center - BTN_W - BTN_GAP / 2.0;
        let expected_right_x = center + BTN_GAP / 2.0;
        assert!((left_r.x - expected_left_x).abs() < 1.0);
        assert!((right_r.x - expected_right_x).abs() < 1.0);
        assert!((left_r.width - BTN_W).abs() < 1.0);
        assert!((right_r.width - BTN_W).abs() < 1.0);
    }

    #[test]
    fn coord_requirement_rows() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(turnin_state());
        Screen::new(quest_dialog_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);

        let frame_r = rect(&reg, "QuestDialogFrame");
        let reqs_r = rect(&reg, "QuestDialogReqs");
        // Requirements below text area
        let expected_reqs_y = frame_r.y + TEXT_TOP + TEXT_AREA_H + 8.0;
        assert!((reqs_r.y - expected_reqs_y).abs() < 1.0);
        assert!((reqs_r.x - (frame_r.x + INSET)).abs() < 1.0);

        // First row has icon + name + count
        let row0_r = rect(&reg, "QuestDialogReq0");
        assert!((row0_r.height - REQ_ROW_H).abs() < 1.0);
    }

    #[test]
    fn coord_frame_matches_wowuisim_dimensions() {
        // wow-ui-sim QuestFrame: 350×450
        let reg = layout_offer_registry();
        let r = rect(&reg, "QuestDialogFrame");
        assert!((r.width - 350.0).abs() < 1.0);
        assert!((r.height - 450.0).abs() < 1.0);
    }
}
