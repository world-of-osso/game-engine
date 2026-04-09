use game_engine::ui::screens::friends_frame_component::*;
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

fn friends_layout_registry() -> FrameRegistry {
    let mut reg = friends_registry();
    recompute_layouts(&mut reg);
    reg
}

fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
    use ui_toolkit::frame::WidgetData;

    let id = reg.get_by_name(name).expect(name);
    let frame = reg.get(id).expect("frame data");
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        _ => panic!("{name} is not a FontString"),
    }
}

const FRAME_X: f32 = 300.0;
const FRAME_Y: f32 = 80.0;

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
