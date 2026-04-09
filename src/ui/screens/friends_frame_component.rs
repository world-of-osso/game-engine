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
    let body = match state.active_tab {
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
    };
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
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> FriendsFrameState {
        FriendsFrameState {
            visible: true,
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(friends_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("FriendsFrame").is_some());
        assert!(reg.get_by_name("FriendsFrameTitle").is_some());
    }

    #[test]
    fn builds_four_tabs() {
        let reg = build_registry();
        for i in 0..4 {
            assert!(
                reg.get_by_name(&format!("FriendsTab{i}")).is_some(),
                "FriendsTab{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("FriendsTab{i}Label")).is_some(),
                "FriendsTab{i}Label missing"
            );
        }
    }

    #[test]
    fn builds_content_area() {
        let reg = build_registry();
        assert!(reg.get_by_name("FriendsContentArea").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(FriendsFrameState::default());
        Screen::new(friends_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("FriendsFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    const FRAME_X: f32 = 300.0;
    const FRAME_Y: f32 = 80.0;

    #[test]
    fn coord_main_frame() {
        let reg = layout_registry();
        let r = rect(&reg, "FriendsFrame");
        assert!((r.x - FRAME_X).abs() < 1.0);
        assert!((r.y - FRAME_Y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_tabs() {
        let reg = layout_registry();
        let tab_count = 4.0_f32;
        let tab_w = (FRAME_W - 2.0 * TAB_INSET - (tab_count - 1.0) * TAB_GAP) / tab_count;
        let t0 = rect(&reg, "FriendsTab0");
        let t3 = rect(&reg, "FriendsTab3");
        assert!((t0.x - (FRAME_X + TAB_INSET)).abs() < 1.0);
        assert!((t0.width - tab_w).abs() < 1.0);
        let expected_x3 = FRAME_X + TAB_INSET + 3.0 * (tab_w + TAB_GAP);
        assert!((t3.x - expected_x3).abs() < 1.0);
    }

    #[test]
    fn coord_content_area() {
        let reg = layout_registry();
        let r = rect(&reg, "FriendsContentArea");
        assert!((r.x - (FRAME_X + CONTENT_INSET)).abs() < 1.0);
        assert!((r.y - (FRAME_Y + CONTENT_TOP)).abs() < 1.0);
    }

    // --- Friends list tests ---

    fn make_friends_state() -> FriendsFrameState {
        FriendsFrameState {
            visible: true,
            friends: vec![
                FriendEntry {
                    name: "Alice#1234".into(),
                    game: "World of Warcraft".into(),
                    status: "Online".into(),
                    online: true,
                    is_bnet: true,
                },
                FriendEntry {
                    name: "Bobchar".into(),
                    game: String::new(),
                    status: "Offline".into(),
                    online: false,
                    is_bnet: false,
                },
            ],
            ..Default::default()
        }
    }

    fn friends_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_friends_state());
        Screen::new(friends_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn friends_list_builds_rows() {
        let reg = friends_registry();
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("FriendRow{i}")).is_some(),
                "FriendRow{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("FriendRow{i}Name")).is_some(),
                "FriendRow{i}Name missing"
            );
            assert!(
                reg.get_by_name(&format!("FriendRow{i}Game")).is_some(),
                "FriendRow{i}Game missing"
            );
            assert!(
                reg.get_by_name(&format!("FriendRow{i}Status")).is_some(),
                "FriendRow{i}Status missing"
            );
        }
    }

    #[test]
    fn friends_list_has_add_button() {
        let reg = friends_registry();
        assert!(reg.get_by_name("FriendsAddButton").is_some());
        assert!(reg.get_by_name("FriendsAddButtonText").is_some());
    }

    // --- Additional coord validation ---

    fn friends_layout_registry() -> FrameRegistry {
        let mut reg = friends_registry();
        recompute_layouts(&mut reg);
        reg
    }

    #[test]
    fn coord_first_friend_row() {
        let reg = friends_layout_registry();
        let content = rect(&reg, "FriendsContentArea");
        let row = rect(&reg, "FriendRow0");
        assert!((row.x - (content.x + FRIEND_INSET)).abs() < 1.0);
        assert!((row.y - (content.y + FRIEND_INSET)).abs() < 1.0);
        assert!((row.height - FRIEND_ROW_H).abs() < 1.0);
    }

    #[test]
    fn coord_friend_row_spacing() {
        let reg = friends_layout_registry();
        let r0 = rect(&reg, "FriendRow0");
        let r1 = rect(&reg, "FriendRow1");
        let spacing = r1.y - r0.y;
        let expected = FRIEND_ROW_H + FRIEND_ROW_GAP;
        assert!(
            (spacing - expected).abs() < 1.0,
            "row spacing: expected {expected}, got {spacing}"
        );
    }

    #[test]
    fn coord_add_button_dimensions() {
        let reg = friends_layout_registry();
        let r = rect(&reg, "FriendsAddButton");
        assert!((r.width - ADD_BUTTON_W).abs() < 1.0);
        assert!((r.height - ADD_BUTTON_H).abs() < 1.0);
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
        assert_eq!(fontstring_text(&reg, "FriendsFrameTitle"), "Friends");
    }

    #[test]
    fn tab_labels() {
        let reg = build_registry();
        assert_eq!(fontstring_text(&reg, "FriendsTab0Label"), "Friends");
        assert_eq!(fontstring_text(&reg, "FriendsTab1Label"), "Who");
        assert_eq!(fontstring_text(&reg, "FriendsTab2Label"), "Raid");
        assert_eq!(fontstring_text(&reg, "FriendsTab3Label"), "Quick Join");
    }

    #[test]
    fn friend_row_name_and_game() {
        let reg = friends_registry();
        assert_eq!(fontstring_text(&reg, "FriendRow0Name"), "Alice#1234");
        assert_eq!(fontstring_text(&reg, "FriendRow0Game"), "World of Warcraft");
        assert_eq!(fontstring_text(&reg, "FriendRow1Name"), "Bobchar");
        assert_eq!(fontstring_text(&reg, "FriendRow1Game"), "");
    }

    #[test]
    fn friend_row_status_text() {
        let reg = friends_registry();
        assert_eq!(fontstring_text(&reg, "FriendRow0Status"), "Online");
        assert_eq!(fontstring_text(&reg, "FriendRow1Status"), "Offline");
    }

    #[test]
    fn add_button_text() {
        let reg = friends_registry();
        assert_eq!(fontstring_text(&reg, "FriendsAddButtonText"), "Add Friend");
    }

    #[test]
    fn empty_friends_list_no_rows() {
        let reg = build_registry();
        assert!(reg.get_by_name("FriendRow0").is_none());
        assert!(reg.get_by_name("FriendsAddButton").is_some());
    }

    #[test]
    fn max_friends_capped() {
        let friends: Vec<FriendEntry> = (0..20)
            .map(|i| FriendEntry {
                name: format!("Friend{i}"),
                game: String::new(),
                status: "Online".into(),
                online: true,
                is_bnet: false,
            })
            .collect();
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(FriendsFrameState {
            visible: true,
            friends,
            ..Default::default()
        });
        Screen::new(friends_frame_screen).sync(&shared, &mut reg);
        for i in 0..MAX_FRIENDS {
            assert!(
                reg.get_by_name(&format!("FriendRow{i}")).is_some(),
                "FriendRow{i} missing"
            );
        }
        assert!(
            reg.get_by_name(&format!("FriendRow{MAX_FRIENDS}"))
                .is_none()
        );
    }

    #[test]
    fn who_tab_renders_query_and_rows() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(FriendsFrameState {
            visible: true,
            active_tab: FriendsFrameTabKind::Who,
            tabs: vec![
                FriendsTab {
                    name: "Friends".into(),
                    active: false,
                    action: FriendsFrameTabKind::Friends.action(),
                },
                FriendsTab {
                    name: "Who".into(),
                    active: true,
                    action: FriendsFrameTabKind::Who.action(),
                },
            ],
            who_query: "ali".into(),
            who_results: vec![WhoEntry {
                name: "Alice".into(),
                details: "Lvl 42 Mage Zone 12".into(),
            }],
            status_text: "who: 1 result(s)".into(),
            ..Default::default()
        });
        Screen::new(friends_frame_screen).sync(&shared, &mut reg);

        assert_eq!(fontstring_text(&reg, "WhoQueryLabel"), "Query: ali");
        assert_eq!(fontstring_text(&reg, "WhoRow0Name"), "Alice");
        assert_eq!(
            fontstring_text(&reg, "WhoRow0Details"),
            "Lvl 42 Mage Zone 12"
        );
        assert_eq!(fontstring_text(&reg, "WhoFooterLabel"), "who: 1 result(s)");
    }
}
