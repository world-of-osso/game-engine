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

pub const FRAME_W: f32 = 500.0;
pub const FRAME_H: f32 = 440.0;
const HEADER_H: f32 = 28.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
const CONTENT_INSET: f32 = 8.0;
const CURRENCY_H: f32 = 20.0;
const CURRENCY_GAP: f32 = 16.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";
const CURRENCY_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const CURRENCY_VALUE_COLOR: &str = "1.0,0.82,0.0,1.0";
const BRACKET_ROW_H: f32 = 48.0;
const BRACKET_ROW_GAP: f32 = 4.0;
const BRACKET_INSET: f32 = 8.0;
const QUEUE_BTN_W: f32 = 120.0;
const QUEUE_BTN_H: f32 = 28.0;
const WARGAME_BTN_W: f32 = 140.0;
const WARGAME_BTN_H: f32 = 28.0;
const BTN_GAP: f32 = 12.0;
const QUEUE_BTN_BG: &str = "0.15,0.25,0.1,0.95";
const QUEUE_BTN_TEXT: &str = "0.2,1.0,0.2,1.0";
const WARGAME_BTN_BG: &str = "0.15,0.12,0.05,0.95";
const WARGAME_BTN_TEXT: &str = "1.0,0.82,0.0,1.0";
const BRACKET_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const BRACKET_RATING_COLOR: &str = "1.0,0.82,0.0,1.0";
const BRACKET_STATS_COLOR: &str = "0.7,0.7,0.7,1.0";
const BRACKET_BG: &str = "0.04,0.04,0.04,0.6";

#[derive(Clone, Debug, PartialEq)]
pub struct BracketEntry {
    pub name: String,
    pub rating: String,
    pub season_wins: String,
    pub season_losses: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PVPTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PVPFrameState {
    pub visible: bool,
    pub tabs: Vec<PVPTab>,
    pub honor: String,
    pub conquest: String,
    pub brackets: Vec<BracketEntry>,
}

impl Default for PVPFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                PVPTab {
                    name: "Honor".into(),
                    active: true,
                },
                PVPTab {
                    name: "Conquest".into(),
                    active: false,
                },
                PVPTab {
                    name: "War Games".into(),
                    active: false,
                },
            ],
            honor: "0".into(),
            conquest: "0".into(),
            brackets: vec![],
        }
    }
}

pub fn pvp_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<PVPFrameState>()
        .expect("PVPFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "PVPFrame",
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
            {title_bar()}
            {tab_row(&state.tabs)}
            {currency_display(&state.honor, &state.conquest)}
            {bracket_list(&state.brackets)}
            {queue_and_wargame_buttons()}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "PVPFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Player vs. Player",
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

fn tab_row(tabs: &[PVPTab]) -> Element {
    let count = tabs.len().max(1) as f32;
    let tab_w = (FRAME_W - 2.0 * TAB_INSET - (count - 1.0) * TAB_GAP) / count;
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let x = TAB_INSET + i as f32 * (tab_w + TAB_GAP);
            pvp_tab_button(i, tab, tab_w, x)
        })
        .collect()
}

fn pvp_tab_button(i: usize, tab: &PVPTab, w: f32, x: f32) -> Element {
    let tab_id = DynName(format!("PVPTab{i}"));
    let label_id = DynName(format!("PVPTab{i}Label"));
    let (bg, color) = if tab.active {
        (TAB_BG_ACTIVE, TAB_TEXT_ACTIVE)
    } else {
        (TAB_BG_INACTIVE, TAB_TEXT_INACTIVE)
    };
    let y = -(HEADER_H + TAB_GAP);
    rsx! {
        r#frame {
            name: tab_id,
            width: {w},
            height: {TAB_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {pvp_tab_label(label_id, &tab.name, w, color)}
        }
    }
}

fn pvp_tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
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

fn currency_pair(
    label_name: &str,
    label: &str,
    label_w: f32,
    value_name: &str,
    value: &str,
    x: f32,
    y: f32,
) -> Element {
    rsx! {
        fontstring {
            name: DynName(label_name.into()),
            width: {label_w},
            height: {CURRENCY_H},
            text: label,
            font_size: 10.0,
            font_color: CURRENCY_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} }
        }
        fontstring {
            name: DynName(value_name.into()),
            width: 80.0,
            height: {CURRENCY_H},
            text: value,
            font_size: 10.0,
            font_color: CURRENCY_VALUE_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x + label_w + 4.0}, y: {y} }
        }
    }
}

fn currency_display(honor: &str, conquest: &str) -> Element {
    let y = -(HEADER_H + TAB_GAP + TAB_H + TAB_GAP);
    rsx! {
        {currency_pair("PVPHonorLabel", "Honor:", 60.0, "PVPHonorValue", honor, CONTENT_INSET, y)}
        {currency_pair("PVPConquestLabel", "Conquest:", 70.0, "PVPConquestValue", conquest, FRAME_W / 2.0, y)}
    }
}

fn bracket_list(brackets: &[BracketEntry]) -> Element {
    let content_y = -(CONTENT_TOP + CURRENCY_H + 4.0);
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CURRENCY_H - 4.0 - CONTENT_INSET;
    let rows: Element = brackets
        .iter()
        .enumerate()
        .flat_map(|(i, b)| bracket_row(i, b, content_w))
        .collect();
    rsx! {
        r#frame {
            name: "PVPContentArea",
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

fn bracket_row(idx: usize, bracket: &BracketEntry, parent_w: f32) -> Element {
    let row_id = DynName(format!("PVPBracket{idx}"));
    let row_w = parent_w - 2.0 * BRACKET_INSET;
    let y = -(BRACKET_INSET + idx as f32 * (BRACKET_ROW_H + BRACKET_ROW_GAP));
    let stats_text = format!("{} - {}", bracket.season_wins, bracket.season_losses);
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {BRACKET_ROW_H},
            background_color: BRACKET_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {BRACKET_INSET},
                y: {y},
            }
            {bracket_name(DynName(format!("PVPBracket{idx}Name")), &bracket.name, row_w)}
            {bracket_rating(DynName(format!("PVPBracket{idx}Rating")), &bracket.rating, row_w)}
            {bracket_stats(DynName(format!("PVPBracket{idx}Stats")), &stats_text, row_w)}
        }
    }
}

fn bracket_name(id: DynName, text: &str, row_w: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {row_w * 0.4},
            height: 18.0,
            text: text,
            font_size: 12.0,
            font_color: BRACKET_NAME_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "8", y: "-4" }
        }
    }
}

fn bracket_rating(id: DynName, text: &str, row_w: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {row_w * 0.4},
            height: 16.0,
            text: text,
            font_size: 14.0,
            font_color: BRACKET_RATING_COLOR,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight, x: "-8", y: "-4" }
        }
    }
}

fn bracket_stats(id: DynName, text: &str, row_w: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {row_w - 16.0},
            height: 14.0,
            text: text,
            font_size: 9.0,
            font_color: BRACKET_STATS_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "8", y: "-26" }
        }
    }
}

fn pvp_action_button(
    name: &str,
    label: &str,
    w: f32,
    h: f32,
    bg: &str,
    color: &str,
    x: f32,
    y: f32,
) -> Element {
    let btn_id = DynName(name.into());
    let text_id = DynName(format!("{name}Text"));
    rsx! {
        r#frame {
            name: btn_id,
            width: {w},
            height: {h},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: text_id,
                width: {w},
                height: {h},
                text: label,
                font_size: 11.0,
                font_color: color,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn queue_and_wargame_buttons() -> Element {
    let y = -(FRAME_H - QUEUE_BTN_H - 8.0);
    let center = FRAME_W / 2.0;
    rsx! {
        {pvp_action_button("PVPQueueButton", "Join Queue", QUEUE_BTN_W, QUEUE_BTN_H, QUEUE_BTN_BG, QUEUE_BTN_TEXT, center - QUEUE_BTN_W - BTN_GAP / 2.0, y)}
        {pvp_action_button("PVPWarGamesButton", "War Games", WARGAME_BTN_W, WARGAME_BTN_H, WARGAME_BTN_BG, WARGAME_BTN_TEXT, center + BTN_GAP / 2.0, y)}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(PVPFrameState {
            visible: true,
            ..Default::default()
        });
        Screen::new(pvp_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("PVPFrame").is_some());
        assert!(reg.get_by_name("PVPFrameTitle").is_some());
    }

    #[test]
    fn builds_three_tabs() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(reg.get_by_name(&format!("PVPTab{i}")).is_some());
        }
    }

    #[test]
    fn builds_currency_display() {
        let reg = build_registry();
        assert!(reg.get_by_name("PVPHonorLabel").is_some());
        assert!(reg.get_by_name("PVPHonorValue").is_some());
        assert!(reg.get_by_name("PVPConquestLabel").is_some());
        assert!(reg.get_by_name("PVPConquestValue").is_some());
    }

    #[test]
    fn builds_content_area() {
        let reg = build_registry();
        assert!(reg.get_by_name("PVPContentArea").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(PVPFrameState::default());
        Screen::new(pvp_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("PVPFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "PVPFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_content_area() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let r = rect(&reg, "PVPContentArea");
        assert!((r.x - (frame_x + CONTENT_INSET)).abs() < 1.0);
    }

    // --- Bracket list tests ---

    fn make_bracket_state() -> PVPFrameState {
        PVPFrameState {
            visible: true,
            brackets: vec![
                BracketEntry {
                    name: "2v2".into(),
                    rating: "1850".into(),
                    season_wins: "42".into(),
                    season_losses: "18".into(),
                },
                BracketEntry {
                    name: "3v3".into(),
                    rating: "2100".into(),
                    season_wins: "30".into(),
                    season_losses: "12".into(),
                },
                BracketEntry {
                    name: "RBG".into(),
                    rating: "1600".into(),
                    season_wins: "15".into(),
                    season_losses: "10".into(),
                },
                BracketEntry {
                    name: "Solo Shuffle".into(),
                    rating: "1900".into(),
                    season_wins: "50".into(),
                    season_losses: "25".into(),
                },
            ],
            ..Default::default()
        }
    }

    fn bracket_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_bracket_state());
        Screen::new(pvp_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn bracket_list_builds_rows() {
        let reg = bracket_registry();
        for i in 0..4 {
            assert!(
                reg.get_by_name(&format!("PVPBracket{i}")).is_some(),
                "PVPBracket{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("PVPBracket{i}Name")).is_some(),
                "PVPBracket{i}Name missing"
            );
            assert!(
                reg.get_by_name(&format!("PVPBracket{i}Rating")).is_some(),
                "PVPBracket{i}Rating missing"
            );
            assert!(
                reg.get_by_name(&format!("PVPBracket{i}Stats")).is_some(),
                "PVPBracket{i}Stats missing"
            );
        }
    }

    #[test]
    fn builds_queue_and_wargame_buttons() {
        let reg = build_registry();
        assert!(reg.get_by_name("PVPQueueButton").is_some());
        assert!(reg.get_by_name("PVPQueueButtonText").is_some());
        assert!(reg.get_by_name("PVPWarGamesButton").is_some());
        assert!(reg.get_by_name("PVPWarGamesButtonText").is_some());
    }
}
