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

pub const FRAME_W: f32 = 600.0;
pub const FRAME_H: f32 = 440.0;
const HEADER_H: f32 = 28.0;
const SIDEBAR_W: f32 = 160.0;
const SIDEBAR_INSET: f32 = 8.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 12.0;
const COMMUNITY_ROW_H: f32 = 28.0;
const COMMUNITY_ROW_GAP: f32 = 2.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
const CONTENT_GAP: f32 = 4.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const SIDEBAR_BG: &str = "0.0,0.0,0.0,0.4";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const COMMUNITY_SELECTED_BG: &str = "0.2,0.15,0.05,0.95";
const COMMUNITY_NORMAL_BG: &str = "0.0,0.0,0.0,0.0";
const COMMUNITY_SELECTED_COLOR: &str = "1.0,0.82,0.0,1.0";
const COMMUNITY_NORMAL_COLOR: &str = "1.0,1.0,1.0,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";

// Chat tab layout
const INPUT_H: f32 = 26.0;
const INPUT_INSET: f32 = 4.0;
const CHANNEL_SELECTOR_W: f32 = 100.0;
const CHANNEL_SELECTOR_H: f32 = 24.0;
const MSG_ROW_H: f32 = 16.0;
const INPUT_BG: &str = "0.1,0.1,0.1,0.9";
const INPUT_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const CHANNEL_BG: &str = "0.08,0.07,0.06,0.88";
const CHANNEL_COLOR: &str = "0.6,0.6,0.6,1.0";
const MSG_COLOR: &str = "1.0,1.0,1.0,1.0";
const MSG_SENDER_COLOR: &str = "0.6,0.8,1.0,1.0";

// Roster tab layout
const ROSTER_SEARCH_H: f32 = 26.0;
const ROSTER_SEARCH_INSET: f32 = 4.0;
const ROSTER_HEADER_H: f32 = 20.0;
const ROSTER_ROW_H: f32 = 20.0;
const ROSTER_ROW_GAP: f32 = 1.0;
const ROSTER_SEARCH_BG: &str = "0.1,0.1,0.1,0.9";
const ROSTER_SEARCH_TEXT: &str = "0.5,0.5,0.5,0.8";
const ROSTER_HEADER_BG: &str = "0.12,0.1,0.08,0.9";
const ROSTER_HEADER_COLOR: &str = "0.8,0.8,0.8,1.0";
const ROSTER_ROW_EVEN: &str = "0.04,0.04,0.04,0.6";
const ROSTER_ROW_ODD: &str = "0.06,0.06,0.06,0.6";
const ROSTER_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const ROSTER_ROLE_COLOR: &str = "0.6,0.8,1.0,1.0";

pub const MAX_COMMUNITIES: usize = 10;
pub const MAX_CHAT_MESSAGES: usize = 15;
pub const MAX_ROSTER_MEMBERS: usize = 15;
pub const ROSTER_COLUMNS: &[(&str, f32)] = &[
    ("Name", 0.35),
    ("Rank", 0.20),
    ("Role", 0.20),
    ("Status", 0.25),
];

#[derive(Clone, Debug, PartialEq)]
pub struct CommunityEntry {
    pub name: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommunityTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub sender: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RosterMember {
    pub name: String,
    pub rank: String,
    pub role: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommunitiesFrameState {
    pub visible: bool,
    pub communities: Vec<CommunityEntry>,
    pub tabs: Vec<CommunityTab>,
    pub chat_messages: Vec<ChatMessage>,
    pub chat_channel: String,
    pub roster_members: Vec<RosterMember>,
}

impl Default for CommunitiesFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            communities: vec![],
            tabs: vec![
                CommunityTab {
                    name: "Chat".into(),
                    active: true,
                },
                CommunityTab {
                    name: "Roster".into(),
                    active: false,
                },
                CommunityTab {
                    name: "Info".into(),
                    active: false,
                },
            ],
            chat_messages: vec![],
            chat_channel: "General".into(),
            roster_members: vec![],
        }
    }
}

pub fn communities_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CommunitiesFrameState>()
        .expect("CommunitiesFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "CommunitiesFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "200",
                y: "-80",
            }
            {title_bar()}
            {community_sidebar(&state.communities)}
            {tab_row(&state.tabs)}
            {chat_tab_content(&state.chat_messages, &state.chat_channel)}
            {roster_tab_content(&state.roster_members)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "CommunitiesFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Communities",
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

fn community_sidebar(communities: &[CommunityEntry]) -> Element {
    let sidebar_y = -HEADER_H;
    let sidebar_h = FRAME_H - HEADER_H - SIDEBAR_INSET;
    let rows: Element = communities
        .iter()
        .enumerate()
        .take(MAX_COMMUNITIES)
        .flat_map(|(i, c)| community_row(i, c))
        .collect();
    rsx! {
        r#frame {
            name: "CommunitiesSidebar",
            width: {SIDEBAR_W},
            height: {sidebar_h},
            background_color: SIDEBAR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SIDEBAR_INSET},
                y: {sidebar_y},
            }
            {rows}
        }
    }
}

fn community_row(idx: usize, entry: &CommunityEntry) -> Element {
    let row_id = DynName(format!("CommunityRow{idx}"));
    let label_id = DynName(format!("CommunityRow{idx}Label"));
    let (bg, color) = if entry.selected {
        (COMMUNITY_SELECTED_BG, COMMUNITY_SELECTED_COLOR)
    } else {
        (COMMUNITY_NORMAL_BG, COMMUNITY_NORMAL_COLOR)
    };
    let y = -(idx as f32 * (COMMUNITY_ROW_H + COMMUNITY_ROW_GAP));
    rsx! {
        r#frame {
            name: row_id,
            width: {SIDEBAR_W},
            height: {COMMUNITY_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            {community_row_label(label_id, &entry.name, color)}
        }
    }
}

fn community_row_label(id: DynName, text: &str, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {SIDEBAR_W - 8.0},
            height: {COMMUNITY_ROW_H},
            text: text,
            font_size: 10.0,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
        }
    }
}

fn tab_row(tabs: &[CommunityTab]) -> Element {
    let tab_area_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let tab_area_w = FRAME_W - tab_area_x - SIDEBAR_INSET;
    let count = tabs.len().max(1) as f32;
    let tab_w = (tab_area_w - (count - 1.0) * TAB_GAP) / count;
    let tab_y = -(HEADER_H + TAB_GAP);
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let x = tab_area_x + i as f32 * (tab_w + TAB_GAP);
            tab_button(i, tab, tab_w, x, tab_y)
        })
        .collect()
}

fn tab_button(i: usize, tab: &CommunityTab, tab_w: f32, x: f32, y: f32) -> Element {
    let tab_id = DynName(format!("CommunitiesTab{i}"));
    let label_id = DynName(format!("CommunitiesTab{i}Label"));
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
            {communities_tab_label(label_id, &tab.name, tab_w, color)}
        }
    }
}

fn communities_tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
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

fn chat_tab_content(messages: &[ChatMessage], channel: &str) -> Element {
    let content_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - content_x - SIDEBAR_INSET;
    let content_h = FRAME_H - CONTENT_TOP - SIDEBAR_INSET;
    rsx! {
        r#frame {
            name: "CommunitiesContentArea",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {content_x},
                y: {content_y},
            }
            {chat_channel_selector(channel, content_w)}
            {chat_message_list(messages, content_w, content_h)}
            {chat_input_box(content_w, content_h)}
        }
    }
}

fn chat_channel_selector(channel: &str, parent_w: f32) -> Element {
    rsx! {
        r#frame {
            name: "CommunitiesChatChannelSelector",
            width: {CHANNEL_SELECTOR_W},
            height: {CHANNEL_SELECTOR_H},
            background_color: CHANNEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INPUT_INSET},
                y: {-INPUT_INSET},
            }
            fontstring {
                name: "CommunitiesChatChannelText",
                width: {CHANNEL_SELECTOR_W - 8.0},
                height: {CHANNEL_SELECTOR_H},
                text: channel,
                font_size: 10.0,
                font_color: CHANNEL_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
        }
    }
}

fn chat_message_list(messages: &[ChatMessage], parent_w: f32, parent_h: f32) -> Element {
    let list_y = -(INPUT_INSET + CHANNEL_SELECTOR_H + INPUT_INSET);
    let list_h =
        parent_h - INPUT_INSET - CHANNEL_SELECTOR_H - INPUT_INSET - INPUT_H - INPUT_INSET * 2.0;
    let list_w = parent_w - 2.0 * INPUT_INSET;
    let rows: Element = messages
        .iter()
        .enumerate()
        .take(MAX_CHAT_MESSAGES)
        .flat_map(|(i, msg)| chat_message_row(i, msg, list_w))
        .collect();
    rsx! {
        r#frame {
            name: "CommunitiesChatMessageList",
            width: {list_w},
            height: {list_h},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INPUT_INSET},
                y: {list_y},
            }
            {rows}
        }
    }
}

fn chat_message_row(idx: usize, msg: &ChatMessage, list_w: f32) -> Element {
    let sender_id = DynName(format!("CommunitiesChatMsg{idx}Sender"));
    let text_id = DynName(format!("CommunitiesChatMsg{idx}Text"));
    let y = -(idx as f32 * MSG_ROW_H);
    let sender_w = 80.0;
    rsx! {
        {chat_line(sender_id, &msg.sender, sender_w, MSG_SENDER_COLOR, 0.0, y)}
        {chat_line(text_id, &msg.text, list_w - sender_w, MSG_COLOR, sender_w, y)}
    }
}

fn chat_line(id: DynName, text: &str, w: f32, color: &str, x: f32, y: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {MSG_ROW_H},
            text: text,
            font_size: 9.0,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} }
        }
    }
}

fn chat_input_box(parent_w: f32, parent_h: f32) -> Element {
    let input_w = parent_w - 2.0 * INPUT_INSET;
    let input_y = -(parent_h - INPUT_H - INPUT_INSET);
    rsx! {
        r#frame {
            name: "CommunitiesChatInputBox",
            width: {input_w},
            height: {INPUT_H},
            background_color: INPUT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INPUT_INSET},
                y: {input_y},
            }
            fontstring {
                name: "CommunitiesChatInputText",
                width: {input_w - 8.0},
                height: {INPUT_H},
                text: "",
                font_size: 10.0,
                font_color: INPUT_TEXT_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
        }
    }
}

// --- Roster tab ---

fn roster_tab_content(members: &[RosterMember]) -> Element {
    let content_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - content_x - SIDEBAR_INSET;
    let content_h = FRAME_H - CONTENT_TOP - SIDEBAR_INSET;
    rsx! {
        r#frame {
            name: "CommunitiesRosterTab",
            width: {content_w},
            height: {content_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {content_x},
                y: {content_y},
            }
            {roster_search_bar(content_w)}
            {roster_header(content_w)}
            {roster_rows(members, content_w)}
        }
    }
}

fn roster_search_bar(parent_w: f32) -> Element {
    let bar_w = parent_w - 2.0 * ROSTER_SEARCH_INSET;
    rsx! {
        r#frame {
            name: "CommunitiesRosterSearch",
            width: {bar_w},
            height: {ROSTER_SEARCH_H},
            background_color: ROSTER_SEARCH_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ROSTER_SEARCH_INSET},
                y: {-ROSTER_SEARCH_INSET},
            }
            fontstring {
                name: "CommunitiesRosterSearchText",
                width: {bar_w - 8.0},
                height: {ROSTER_SEARCH_H},
                text: "Search members...",
                font_size: 10.0,
                font_color: ROSTER_SEARCH_TEXT,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
        }
    }
}

fn roster_header(parent_w: f32) -> Element {
    let header_y = -(ROSTER_SEARCH_INSET + ROSTER_SEARCH_H + ROSTER_SEARCH_INSET);
    let header_w = parent_w - 2.0 * ROSTER_SEARCH_INSET;
    let cols: Element = ROSTER_COLUMNS
        .iter()
        .enumerate()
        .flat_map(|(i, (name, _))| {
            let x = roster_col_x(header_w, i);
            let w = roster_col_w(header_w, i);
            roster_header_cell(i, name, x, w)
        })
        .collect();
    rsx! {
        r#frame {
            name: "CommunitiesRosterHeader",
            width: {header_w},
            height: {ROSTER_HEADER_H},
            background_color: ROSTER_HEADER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ROSTER_SEARCH_INSET},
                y: {header_y},
            }
            {cols}
        }
    }
}

fn roster_header_cell(idx: usize, text: &str, x: f32, w: f32) -> Element {
    let cell_id = DynName(format!("CommunitiesRosterCol{idx}"));
    rsx! {
        fontstring {
            name: cell_id,
            width: {w},
            height: {ROSTER_HEADER_H},
            text,
            font_size: 9.0,
            font_color: ROSTER_HEADER_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "0",
            }
        }
    }
}

fn roster_rows(members: &[RosterMember], parent_w: f32) -> Element {
    let row_w = parent_w - 2.0 * ROSTER_SEARCH_INSET;
    let top = ROSTER_SEARCH_INSET + ROSTER_SEARCH_H + ROSTER_SEARCH_INSET + ROSTER_HEADER_H;
    members
        .iter()
        .enumerate()
        .take(MAX_ROSTER_MEMBERS)
        .flat_map(|(i, member)| roster_row(i, member, row_w, top))
        .collect()
}

fn roster_row(idx: usize, member: &RosterMember, row_w: f32, top: f32) -> Element {
    let row_id = DynName(format!("CommunitiesRosterRow{idx}"));
    let y = -(top + idx as f32 * (ROSTER_ROW_H + ROSTER_ROW_GAP));
    let bg = if idx % 2 == 0 {
        ROSTER_ROW_EVEN
    } else {
        ROSTER_ROW_ODD
    };
    let cells = roster_row_cells(idx, member, row_w);
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {ROSTER_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {ROSTER_SEARCH_INSET},
                y: {y},
            }
            {cells}
        }
    }
}

fn roster_row_cells(idx: usize, member: &RosterMember, row_w: f32) -> Element {
    let values = [&member.name, &member.rank, &member.role, &member.status];
    values
        .iter()
        .enumerate()
        .flat_map(|(col, text)| {
            let cell_id = DynName(format!("CommunitiesRosterRow{idx}Col{col}"));
            let x = roster_col_x(row_w, col);
            let w = roster_col_w(row_w, col);
            let color = if col == 2 {
                ROSTER_ROLE_COLOR
            } else {
                ROSTER_TEXT_COLOR
            };
            rsx! {
                fontstring {
                    name: cell_id,
                    width: {w},
                    height: {ROSTER_ROW_H},
                    text: {text.as_str()},
                    font_size: 9.0,
                    font_color: color,
                    justify_h: "LEFT",
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                        x: {x},
                        y: "0",
                    }
                }
            }
        })
        .collect()
}

fn roster_col_x(row_w: f32, col: usize) -> f32 {
    let mut x = 4.0;
    for i in 0..col {
        x += ROSTER_COLUMNS[i].1 * row_w;
    }
    x
}

fn roster_col_w(row_w: f32, col: usize) -> f32 {
    ROSTER_COLUMNS[col].1 * row_w
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> CommunitiesFrameState {
        CommunitiesFrameState {
            visible: true,
            communities: vec![
                CommunityEntry {
                    name: "My Guild".into(),
                    selected: true,
                },
                CommunityEntry {
                    name: "Arena Team".into(),
                    selected: false,
                },
            ],
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(communities_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("CommunitiesFrame").is_some());
        assert!(reg.get_by_name("CommunitiesFrameTitle").is_some());
    }

    #[test]
    fn builds_sidebar_with_communities() {
        let reg = build_registry();
        assert!(reg.get_by_name("CommunitiesSidebar").is_some());
        assert!(reg.get_by_name("CommunityRow0").is_some());
        assert!(reg.get_by_name("CommunityRow1").is_some());
        assert!(reg.get_by_name("CommunityRow0Label").is_some());
    }

    #[test]
    fn builds_three_tabs() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("CommunitiesTab{i}")).is_some(),
                "CommunitiesTab{i} missing"
            );
        }
    }

    #[test]
    fn builds_content_area() {
        let reg = build_registry();
        assert!(reg.get_by_name("CommunitiesContentArea").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.visible = false;
        shared.insert(state);
        Screen::new(communities_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("CommunitiesFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    const FRAME_X: f32 = 200.0;
    const FRAME_Y: f32 = 80.0;

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        let r = rect(&reg, "CommunitiesFrame");
        assert!((r.x - FRAME_X).abs() < 1.0);
        assert!((r.y - FRAME_Y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_sidebar() {
        let reg = layout_registry();
        let r = rect(&reg, "CommunitiesSidebar");
        assert!((r.x - (FRAME_X + SIDEBAR_INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + HEADER_H)).abs() < 1.0);
        assert!((r.width - SIDEBAR_W).abs() < 1.0);
    }

    #[test]
    fn coord_content_area() {
        let reg = layout_registry();
        let r = rect(&reg, "CommunitiesContentArea");
        let expected_x = FRAME_X + SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - (FRAME_Y + CONTENT_TOP)).abs() < 1.0);
    }

    // --- Chat tab tests ---

    fn make_chat_state() -> CommunitiesFrameState {
        CommunitiesFrameState {
            visible: true,
            communities: vec![CommunityEntry {
                name: "Guild".into(),
                selected: true,
            }],
            chat_messages: vec![
                ChatMessage {
                    sender: "Alice".into(),
                    text: "Hello!".into(),
                },
                ChatMessage {
                    sender: "Bob".into(),
                    text: "Hi there".into(),
                },
            ],
            chat_channel: "Guild".into(),
            ..Default::default()
        }
    }

    fn chat_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_chat_state());
        Screen::new(communities_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn chat_tab_builds_channel_selector() {
        let reg = chat_registry();
        assert!(reg.get_by_name("CommunitiesChatChannelSelector").is_some());
        assert!(reg.get_by_name("CommunitiesChatChannelText").is_some());
    }

    #[test]
    fn chat_tab_builds_message_list() {
        let reg = chat_registry();
        assert!(reg.get_by_name("CommunitiesChatMessageList").is_some());
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("CommunitiesChatMsg{i}Sender"))
                    .is_some(),
                "CommunitiesChatMsg{i}Sender missing"
            );
            assert!(
                reg.get_by_name(&format!("CommunitiesChatMsg{i}Text"))
                    .is_some(),
                "CommunitiesChatMsg{i}Text missing"
            );
        }
    }

    #[test]
    fn chat_tab_builds_input_box() {
        let reg = chat_registry();
        assert!(reg.get_by_name("CommunitiesChatInputBox").is_some());
        assert!(reg.get_by_name("CommunitiesChatInputText").is_some());
    }

    // --- Roster tab tests ---

    fn make_roster_state() -> CommunitiesFrameState {
        CommunitiesFrameState {
            visible: true,
            communities: vec![CommunityEntry {
                name: "Guild".into(),
                selected: true,
            }],
            roster_members: vec![
                RosterMember {
                    name: "Alice".into(),
                    rank: "Officer".into(),
                    role: "Tank".into(),
                    status: "Online".into(),
                },
                RosterMember {
                    name: "Bob".into(),
                    rank: "Member".into(),
                    role: "Healer".into(),
                    status: "Offline".into(),
                },
            ],
            ..Default::default()
        }
    }

    fn roster_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_roster_state());
        Screen::new(communities_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn roster_tab_builds_root_and_search() {
        let reg = roster_registry();
        assert!(reg.get_by_name("CommunitiesRosterTab").is_some());
        assert!(reg.get_by_name("CommunitiesRosterSearch").is_some());
        assert!(reg.get_by_name("CommunitiesRosterSearchText").is_some());
    }

    #[test]
    fn roster_tab_builds_header_columns() {
        let reg = roster_registry();
        assert!(reg.get_by_name("CommunitiesRosterHeader").is_some());
        for i in 0..ROSTER_COLUMNS.len() {
            assert!(
                reg.get_by_name(&format!("CommunitiesRosterCol{i}"))
                    .is_some(),
                "CommunitiesRosterCol{i} missing"
            );
        }
    }

    #[test]
    fn roster_tab_builds_member_rows() {
        let reg = roster_registry();
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("CommunitiesRosterRow{i}"))
                    .is_some(),
                "CommunitiesRosterRow{i} missing"
            );
            for col in 0..ROSTER_COLUMNS.len() {
                assert!(
                    reg.get_by_name(&format!("CommunitiesRosterRow{i}Col{col}"))
                        .is_some(),
                    "CommunitiesRosterRow{i}Col{col} missing"
                );
            }
        }
    }

    // --- Additional coord validation ---

    #[test]
    fn coord_first_tab() {
        let reg = layout_registry();
        let tab_area_x = FRAME_X + SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
        let tab_y = FRAME_Y + HEADER_H + TAB_GAP;
        let t = rect(&reg, "CommunitiesTab0");
        assert!(
            (t.x - tab_area_x).abs() < 1.0,
            "tab x: expected {tab_area_x}, got {}",
            t.x
        );
        assert!((t.y - tab_y).abs() < 1.0);
        assert!((t.height - TAB_H).abs() < 1.0);
    }

    #[test]
    fn coord_chat_channel_selector() {
        let reg = {
            let mut r = FrameRegistry::new(1920.0, 1080.0);
            let mut s = SharedContext::new();
            s.insert(make_chat_state());
            Screen::new(communities_frame_screen).sync(&s, &mut r);
            recompute_layouts(&mut r);
            r
        };
        let content_x = FRAME_X + SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
        let content_y = FRAME_Y + CONTENT_TOP;
        let r = rect(&reg, "CommunitiesChatChannelSelector");
        assert!((r.x - (content_x + INPUT_INSET)).abs() < 1.0);
        assert!((r.y - (content_y + INPUT_INSET)).abs() < 1.0);
        assert!((r.width - CHANNEL_SELECTOR_W).abs() < 1.0);
    }

    #[test]
    fn coord_chat_input_box() {
        let reg = {
            let mut r = FrameRegistry::new(1920.0, 1080.0);
            let mut s = SharedContext::new();
            s.insert(make_chat_state());
            Screen::new(communities_frame_screen).sync(&s, &mut r);
            recompute_layouts(&mut r);
            r
        };
        let r = rect(&reg, "CommunitiesChatInputBox");
        assert!((r.height - INPUT_H).abs() < 1.0);
    }
}
