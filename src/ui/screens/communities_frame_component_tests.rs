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
fn chat_tab_builds_channel_tabs() {
    let reg = chat_registry();
    assert!(reg.get_by_name("CommunitiesChatChannelTabs").is_some());
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
fn coord_chat_channel_tabs() {
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
    let tabs = rect(&reg, "CommunitiesChatChannelTabs");
    assert!(
        (tabs.x - (content_x + CHAT_CHANNEL_TAB_INSET)).abs() < 1.0,
        "tabs.x={} expected={}",
        tabs.x,
        content_x + CHAT_CHANNEL_TAB_INSET
    );
    assert!(
        (tabs.y - (content_y + CHAT_CHANNEL_TAB_INSET)).abs() < 1.0,
        "tabs.y={} expected={}",
        tabs.y,
        content_y + CHAT_CHANNEL_TAB_INSET
    );
    assert!((tabs.height - CHAT_CHANNEL_TAB_H).abs() < 1.0);
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
