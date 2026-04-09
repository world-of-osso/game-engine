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

pub const FRAME_W: f32 = 375.0;
pub const FRAME_H: f32 = 440.0;
const HEADER_H: f32 = 28.0;
const TAB_H: f32 = 28.0;
const TAB_GAP: f32 = 4.0;
const TAB_INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP + TAB_H + TAB_GAP;
const CONTENT_INSET: f32 = 8.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";

// Friends list layout
const FRIEND_ROW_H: f32 = 32.0;
const FRIEND_ROW_GAP: f32 = 1.0;
const FRIEND_INSET: f32 = 4.0;
const ADD_BUTTON_W: f32 = 100.0;
const ADD_BUTTON_H: f32 = 24.0;
const STATUS_ICON_SIZE: f32 = 12.0;
const FRIEND_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const FRIEND_GAME_COLOR: &str = "0.6,0.8,1.0,1.0";
const FRIEND_STATUS_ONLINE: &str = "0.0,1.0,0.0,1.0";
const FRIEND_STATUS_OFFLINE: &str = "0.5,0.5,0.5,1.0";
const ADD_BUTTON_BG: &str = "0.15,0.12,0.05,0.95";
const ADD_BUTTON_TEXT_COLOR: &str = "1.0,0.82,0.0,1.0";
const EMPTY_TEXT_COLOR: &str = "0.75,0.75,0.75,1.0";

pub const MAX_FRIENDS: usize = 15;
pub const ACTION_FRIENDS_TAB_PREFIX: &str = "friends_tab:";

#[derive(Clone, Debug, PartialEq)]
pub struct FriendsTab {
    pub name: String,
    pub active: bool,
    pub action: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FriendEntry {
    pub name: String,
    pub game: String,
    pub status: String,
    pub online: bool,
    pub is_bnet: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WhoEntry {
    pub name: String,
    pub details: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FriendsFrameTabKind {
    #[default]
    Friends,
    Who,
    Raid,
    QuickJoin,
}

impl FriendsFrameTabKind {
    pub fn from_action(action: &str) -> Option<Self> {
        match action.strip_prefix(ACTION_FRIENDS_TAB_PREFIX)? {
            "friends" => Some(Self::Friends),
            "who" => Some(Self::Who),
            "raid" => Some(Self::Raid),
            "quickjoin" => Some(Self::QuickJoin),
            _ => None,
        }
    }

    pub fn action(self) -> String {
        let suffix = match self {
            Self::Friends => "friends",
            Self::Who => "who",
            Self::Raid => "raid",
            Self::QuickJoin => "quickjoin",
        };
        format!("{ACTION_FRIENDS_TAB_PREFIX}{suffix}")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FriendsFrameState {
    pub visible: bool,
    pub active_tab: FriendsFrameTabKind,
    pub tabs: Vec<FriendsTab>,
    pub friends: Vec<FriendEntry>,
    pub who_query: String,
    pub who_results: Vec<WhoEntry>,
    pub status_text: String,
}

impl Default for FriendsFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            active_tab: FriendsFrameTabKind::Friends,
            tabs: vec![
                FriendsTab {
                    name: "Friends".into(),
                    active: true,
                    action: FriendsFrameTabKind::Friends.action(),
                },
                FriendsTab {
                    name: "Who".into(),
                    active: false,
                    action: FriendsFrameTabKind::Who.action(),
                },
                FriendsTab {
                    name: "Raid".into(),
                    active: false,
                    action: FriendsFrameTabKind::Raid.action(),
                },
                FriendsTab {
                    name: "Quick Join".into(),
                    active: false,
                    action: FriendsFrameTabKind::QuickJoin.action(),
                },
            ],
            friends: vec![],
            who_query: String::new(),
            who_results: vec![],
            status_text: String::new(),
        }
    }
}

pub fn friends_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<FriendsFrameState>()
        .expect("FriendsFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "FriendsFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "300",
                y: "-80",
            }
            {title_bar()}
            {tab_row(&state.tabs)}
            {content_area(state)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "FriendsFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Friends",
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

fn tab_row(tabs: &[FriendsTab]) -> Element {
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

fn tab_button(i: usize, tab: &FriendsTab, tab_w: f32, x: f32, y: f32) -> Element {
    let tab_id = DynName(format!("FriendsTab{i}"));
    let label_id = DynName(format!("FriendsTab{i}Label"));
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
            onclick: {tab.action.as_str()},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {friends_tab_label(label_id, &tab.name, tab_w, color)}
        }
    }
}

fn friends_tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
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

fn content_area(state: &FriendsFrameState) -> Element {
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - 2.0 * CONTENT_INSET;
    let content_h = FRAME_H - CONTENT_TOP - CONTENT_INSET;
    let body = content_area_body(state, content_w, content_h);
    rsx! {
        r#frame {
            name: "FriendsContentArea",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {CONTENT_INSET},
                y: {content_y},
            }
            {body}
        }
    }
}

fn content_area_body(state: &FriendsFrameState, content_w: f32, content_h: f32) -> Element {
    match state.active_tab {
        FriendsFrameTabKind::Friends => friends_content_body(&state.friends, content_w, content_h),
        FriendsFrameTabKind::Who => who_content_body(
            &state.who_query,
            &state.who_results,
            &state.status_text,
            content_w,
            content_h,
        ),
        FriendsFrameTabKind::Raid => placeholder_content_body(
            "Raid",
            "Raid roster is not implemented yet.",
            content_w,
            content_h,
        ),
        FriendsFrameTabKind::QuickJoin => placeholder_content_body(
            "Quick Join",
            "Quick Join is not implemented yet.",
            content_w,
            content_h,
        ),
    }
}

fn friends_content_body(friends: &[FriendEntry], content_w: f32, content_h: f32) -> Element {
    let rows: Element = friends
        .iter()
        .enumerate()
        .take(MAX_FRIENDS)
        .flat_map(|(i, f)| friend_row(i, f, content_w))
        .collect();
    rsx! {
        {rows}
        {add_friend_button(content_w, content_h)}
    }
}

fn who_content_body(
    query: &str,
    results: &[WhoEntry],
    status_text: &str,
    w: f32,
    h: f32,
) -> Element {
    let header = if query.is_empty() {
        "Query: All Players".to_string()
    } else {
        format!("Query: {query}")
    };
    let rows: Element = results
        .iter()
        .enumerate()
        .take(MAX_FRIENDS)
        .flat_map(|(i, entry)| who_row(i, entry, w))
        .collect();
    let footer_text = if status_text.is_empty() {
        format!("Results: {}", results.len())
    } else {
        status_text.to_string()
    };
    rsx! {
        {section_header("WhoQueryLabel", &header, w)}
        {rows}
        {section_footer("WhoFooterLabel", &footer_text, w, h)}
        {empty_state(
            "WhoEmptyState",
            "No players matched the current query.",
            w,
            h,
            results.is_empty(),
        )}
    }
}

fn placeholder_content_body(title: &str, text: &str, w: f32, h: f32) -> Element {
    rsx! {
        {section_header("FriendsPlaceholderTitle", title, w)}
        {empty_state("FriendsPlaceholderBody", text, w, h, true)}
    }
}

fn friend_row(idx: usize, friend: &FriendEntry, parent_w: f32) -> Element {
    let row_id = DynName(format!("FriendRow{idx}"));
    let y = -(FRIEND_INSET + idx as f32 * (FRIEND_ROW_H + FRIEND_ROW_GAP));
    let row_w = parent_w - 2.0 * FRIEND_INSET;
    let status_color = if friend.online {
        FRIEND_STATUS_ONLINE
    } else {
        FRIEND_STATUS_OFFLINE
    };
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {FRIEND_ROW_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {FRIEND_INSET},
                y: {y},
            }
            {friend_name_label(DynName(format!("FriendRow{idx}Name")), &friend.name, row_w)}
            {friend_game_label(DynName(format!("FriendRow{idx}Game")), &friend.game, row_w)}
            {friend_status_icon(DynName(format!("FriendRow{idx}Status")), &friend.status, status_color)}
        }
    }
}

fn who_row(idx: usize, entry: &WhoEntry, parent_w: f32) -> Element {
    let row_id = DynName(format!("WhoRow{idx}"));
    let y = -(FRIEND_INSET + 20.0 + idx as f32 * (FRIEND_ROW_H + FRIEND_ROW_GAP));
    let row_w = parent_w - 2.0 * FRIEND_INSET;
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {FRIEND_ROW_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {FRIEND_INSET},
                y: {y},
            }
            {friend_name_label(DynName(format!("WhoRow{idx}Name")), &entry.name, row_w)}
            {friend_game_label(DynName(format!("WhoRow{idx}Details")), &entry.details, row_w)}
        }
    }
}

fn section_header(name: &str, text: &str, width: f32) -> Element {
    let id = DynName(name.to_string());
    rsx! {
        fontstring {
            name: id,
            width: {width - 2.0 * FRIEND_INSET},
            height: 16.0,
            text: text,
            font_size: 10.0,
            font_color: FRIEND_GAME_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {FRIEND_INSET},
                y: "-2",
            }
        }
    }
}

fn section_footer(name: &str, text: &str, width: f32, height: f32) -> Element {
    let id = DynName(name.to_string());
    let y = -(height - ADD_BUTTON_H - FRIEND_INSET);
    rsx! {
        fontstring {
            name: id,
            width: {width - 2.0 * FRIEND_INSET},
            height: {ADD_BUTTON_H},
            text: text,
            font_size: 10.0,
            font_color: ADD_BUTTON_TEXT_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {FRIEND_INSET},
                y: {y},
            }
        }
    }
}

fn empty_state(name: &str, text: &str, width: f32, height: f32, visible: bool) -> Element {
    let id = DynName(name.to_string());
    let hide = !visible;
    rsx! {
        fontstring {
            name: id,
            width: {width - 2.0 * FRIEND_INSET},
            height: 24.0,
            hidden: hide,
            text: text,
            font_size: 10.0,
            font_color: EMPTY_TEXT_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: {-(height / 2.0)},
            }
        }
    }
}

fn friend_name_label(id: DynName, text: &str, row_w: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {row_w * 0.45},
            height: 16.0,
            text: text,
            font_size: 10.0,
            font_color: FRIEND_NAME_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: "-2" }
        }
    }
}

fn friend_game_label(id: DynName, text: &str, row_w: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {row_w * 0.45},
            height: 14.0,
            text: text,
            font_size: 8.0,
            font_color: FRIEND_GAME_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "0", y: "-18" }
        }
    }
}

fn friend_status_icon(id: DynName, text: &str, color: &str) -> Element {
    let status_y = -((FRIEND_ROW_H - STATUS_ICON_SIZE) / 2.0);
    rsx! {
        fontstring {
            name: id,
            width: {STATUS_ICON_SIZE},
            height: {STATUS_ICON_SIZE},
            text: text,
            font_size: 8.0,
            font_color: color,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight, x: "0", y: {status_y} }
        }
    }
}

fn add_friend_button(parent_w: f32, parent_h: f32) -> Element {
    let x = (parent_w - ADD_BUTTON_W) / 2.0;
    let y = -(parent_h - ADD_BUTTON_H - FRIEND_INSET);
    rsx! {
        r#frame {
            name: "FriendsAddButton",
            width: {ADD_BUTTON_W},
            height: {ADD_BUTTON_H},
            background_color: ADD_BUTTON_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: "FriendsAddButtonText",
                width: {ADD_BUTTON_W},
                height: {ADD_BUTTON_H},
                text: "Add Friend",
                font_size: 10.0,
                font_color: ADD_BUTTON_TEXT_COLOR,
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
#[path = "../../../tests/unit/friends_frame_component_tests.rs"]
mod tests;
