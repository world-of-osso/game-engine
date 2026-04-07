use super::*;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn cat(name: &str, selected: bool) -> AchievementCategory {
    AchievementCategory {
        name: name.into(),
        is_child: false,
        selected,
    }
}

fn default_categories() -> Vec<AchievementCategory> {
    vec![
        cat("General", true),
        cat("Quests", false),
        cat("Exploration", false),
        cat("PvP", false),
        cat("Dungeons & Raids", false),
        cat("Professions", false),
        cat("Reputation", false),
        cat("World Events", false),
        cat("Feats of Strength", false),
    ]
}

fn sample_achievements() -> Vec<AchievementRow> {
    vec![
        AchievementRow {
            name: "Level 10".into(),
            description: "Reach level 10.".into(),
            points: 10,
            icon_fdid: 236562,
            completed: true,
            progress: 1.0,
            progress_text: "10 / 10".into(),
        },
        AchievementRow {
            name: "Level 20".into(),
            description: "Reach level 20.".into(),
            points: 10,
            icon_fdid: 236563,
            completed: false,
            progress: 0.75,
            progress_text: "15 / 20".into(),
        },
        AchievementRow {
            name: "Level 40".into(),
            description: "Reach level 40.".into(),
            points: 10,
            icon_fdid: 236565,
            completed: false,
            progress: 0.0,
            progress_text: "0 / 40".into(),
        },
    ]
}

fn make_test_state() -> AchievementFrameState {
    AchievementFrameState {
        visible: true,
        tabs: vec![
            AchievementTab {
                name: "Achievements".into(),
                active: true,
            },
            AchievementTab {
                name: "Statistics".into(),
                active: false,
            },
        ],
        categories: default_categories(),
        achievements: sample_achievements(),
        total_points: 0,
    }
}

#[test]
fn achievement_frame_builds_expected_frames() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    let mut screen = Screen::new(achievement_frame_screen);
    screen.sync(&shared, &mut registry);

    assert!(registry.get_by_name("AchievementFrame").is_some());
    assert!(registry.get_by_name("AchievementFrameTitle").is_some());
    assert!(registry.get_by_name("AchievementCategorySidebar").is_some());
    assert!(registry.get_by_name("AchievementContentArea").is_some());
    assert!(
        registry
            .get_by_name("AchievementContentPlaceholder")
            .is_some()
    );
}

#[test]
fn achievement_frame_builds_tabs() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

    assert!(registry.get_by_name("AchievementTab0").is_some());
    assert!(registry.get_by_name("AchievementTab1").is_some());
    assert!(registry.get_by_name("AchievementTab0Label").is_some());
    assert!(registry.get_by_name("AchievementTab1Label").is_some());
}

#[test]
fn achievement_frame_builds_category_rows() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

    for i in 0..9 {
        assert!(
            registry
                .get_by_name(&format!("AchievementCat{i}"))
                .is_some(),
            "AchievementCat{i} missing"
        );
        assert!(
            registry
                .get_by_name(&format!("AchievementCat{i}Label"))
                .is_some(),
            "AchievementCat{i}Label missing"
        );
    }
}

#[test]
fn achievement_frame_hidden_when_not_visible() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut state = make_test_state();
    state.visible = false;
    shared.insert(state);
    Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

    let frame_id = registry
        .get_by_name("AchievementFrame")
        .expect("AchievementFrame");
    let frame = registry.get(frame_id).expect("frame data");
    assert!(frame.hidden, "frame should be hidden when visible=false");
}

#[test]
fn achievement_frame_child_categories_indented() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut state = make_test_state();
    state.categories.insert(
        1,
        AchievementCategory {
            name: "Level".into(),
            is_child: true,
            selected: false,
        },
    );
    shared.insert(state);
    Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

    assert!(registry.get_by_name("AchievementCat1").is_some());
    assert!(registry.get_by_name("AchievementCat1Label").is_some());
}

#[test]
fn achievement_frame_builds_achievement_rows() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

    for i in 0..3 {
        assert!(
            registry
                .get_by_name(&format!("AchievementRow{i}"))
                .is_some(),
            "AchievementRow{i} missing"
        );
        assert!(
            registry
                .get_by_name(&format!("AchievementRow{i}Icon"))
                .is_some(),
            "AchievementRow{i}Icon missing"
        );
        assert!(
            registry
                .get_by_name(&format!("AchievementRow{i}Name"))
                .is_some(),
            "AchievementRow{i}Name missing"
        );
        assert!(
            registry
                .get_by_name(&format!("AchievementRow{i}Desc"))
                .is_some(),
            "AchievementRow{i}Desc missing"
        );
        assert!(
            registry
                .get_by_name(&format!("AchievementRow{i}ProgressBg"))
                .is_some(),
            "AchievementRow{i}ProgressBg missing"
        );
        assert!(
            registry
                .get_by_name(&format!("AchievementRow{i}ProgressFill"))
                .is_some(),
            "AchievementRow{i}ProgressFill missing"
        );
        assert!(
            registry
                .get_by_name(&format!("AchievementRow{i}Check"))
                .is_some(),
            "AchievementRow{i}Check missing"
        );
        assert!(
            registry
                .get_by_name(&format!("AchievementRow{i}Points"))
                .is_some(),
            "AchievementRow{i}Points missing"
        );
    }
}

#[test]
fn achievement_row_icons_have_texture_frames() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

    for i in 0..3 {
        assert!(
            registry
                .get_by_name(&format!("AchievementRow{i}IconTex"))
                .is_some(),
            "AchievementRow{i}IconTex missing"
        );
    }
}

#[test]
fn achievement_frame_empty_shows_placeholder() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut state = make_test_state();
    state.achievements.clear();
    shared.insert(state);
    Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

    let ph_id = registry
        .get_by_name("AchievementContentPlaceholder")
        .expect("placeholder");
    let ph = registry.get(ph_id).expect("frame data");
    assert!(
        !ph.hidden,
        "placeholder should be visible when no achievements"
    );
}

#[test]
fn achievement_frame_with_rows_hides_placeholder() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

    let ph_id = registry
        .get_by_name("AchievementContentPlaceholder")
        .expect("placeholder");
    let ph = registry.get(ph_id).expect("frame data");
    assert!(
        ph.hidden,
        "placeholder should be hidden when achievements present"
    );
}

#[test]
fn achievement_row_progress_fill_width_matches_fraction() {
    use ui_toolkit::frame::Dimension;

    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(achievement_frame_screen).sync(&shared, &mut registry);

    // Row 0 is completed (progress=1.0), fill should be full width
    let fill_id = registry
        .get_by_name("AchievementRow0ProgressFill")
        .expect("fill");
    let fill = registry.get(fill_id).expect("frame data");
    assert_eq!(fill.width, Dimension::Fixed(PROGRESS_BAR_W));

    // Row 1 has progress=0.75
    let fill_id = registry
        .get_by_name("AchievementRow1ProgressFill")
        .expect("fill");
    let fill = registry.get(fill_id).expect("frame data");
    assert_eq!(fill.width, Dimension::Fixed(PROGRESS_BAR_W * 0.75));
}

// --- Coord validation helpers ---

fn layout_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(achievement_frame_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|f| f.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

fn assert_rect(reg: &FrameRegistry, name: &str, expected: LayoutRect) {
    let actual = rect(reg, name);
    let ok = (actual.x - expected.x).abs() < 1.0
        && (actual.y - expected.y).abs() < 1.0
        && (actual.width - expected.width).abs() < 1.0
        && (actual.height - expected.height).abs() < 1.0;
    assert!(
        ok,
        "{name} rect mismatch:\n  expected: ({}, {}, {}×{})\n  actual:   ({}, {}, {}×{})",
        expected.x,
        expected.y,
        expected.width,
        expected.height,
        actual.x,
        actual.y,
        actual.width,
        actual.height,
    );
}

// --- Coord validation tests ---

const FRAME_X: f32 = 370.0;
const FRAME_Y: f32 = 80.0;

#[test]
fn coord_main_frame() {
    let reg = layout_registry();
    assert_rect(
        &reg,
        "AchievementFrame",
        LayoutRect {
            x: FRAME_X,
            y: FRAME_Y,
            width: FRAME_W,
            height: FRAME_H,
        },
    );
}

#[test]
fn coord_title() {
    let reg = layout_registry();
    assert_rect(
        &reg,
        "AchievementFrameTitle",
        LayoutRect {
            x: FRAME_X,
            y: FRAME_Y,
            width: FRAME_W,
            height: HEADER_H,
        },
    );
}

#[test]
fn coord_tabs() {
    let reg = layout_registry();
    let tab_count = 2.0_f32;
    let tab_w = (FRAME_W - 2.0 * TAB_INSET - (tab_count - 1.0) * TAB_GAP) / tab_count;
    let tab_y = FRAME_Y + HEADER_H + TAB_GAP;
    assert_rect(
        &reg,
        "AchievementTab0",
        LayoutRect {
            x: FRAME_X + TAB_INSET,
            y: tab_y,
            width: tab_w,
            height: TAB_H,
        },
    );
    assert_rect(
        &reg,
        "AchievementTab1",
        LayoutRect {
            x: FRAME_X + TAB_INSET + tab_w + TAB_GAP,
            y: tab_y,
            width: tab_w,
            height: TAB_H,
        },
    );
}

#[test]
fn coord_sidebar() {
    let reg = layout_registry();
    let sidebar_y = FRAME_Y + SIDEBAR_TOP;
    let sidebar_h = FRAME_H - SIDEBAR_TOP - SIDEBAR_INSET;
    assert_rect(
        &reg,
        "AchievementCategorySidebar",
        LayoutRect {
            x: FRAME_X + SIDEBAR_INSET,
            y: sidebar_y,
            width: SIDEBAR_W,
            height: sidebar_h,
        },
    );
}

#[test]
fn coord_content_area() {
    let reg = layout_registry();
    let content_x = FRAME_X + SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET;
    let content_y = FRAME_Y + SIDEBAR_TOP;
    let content_w = FRAME_W - (SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET) - SIDEBAR_INSET;
    let content_h = FRAME_H - SIDEBAR_TOP - SIDEBAR_INSET;
    assert_rect(
        &reg,
        "AchievementContentArea",
        LayoutRect {
            x: content_x,
            y: content_y,
            width: content_w,
            height: content_h,
        },
    );
}

#[test]
fn coord_first_category_row() {
    let reg = layout_registry();
    let sidebar_x = FRAME_X + SIDEBAR_INSET;
    let sidebar_y = FRAME_Y + SIDEBAR_TOP;
    assert_rect(
        &reg,
        "AchievementCat0",
        LayoutRect {
            x: sidebar_x,
            y: sidebar_y,
            width: SIDEBAR_W,
            height: CAT_ROW_H,
        },
    );
}

#[test]
fn coord_first_achievement_row() {
    let reg = layout_registry();
    let content_x = FRAME_X + SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET;
    let content_y = FRAME_Y + SIDEBAR_TOP;
    let content_w = FRAME_W - (SIDEBAR_INSET + SIDEBAR_W + CONTENT_INSET) - SIDEBAR_INSET;
    let row_w = content_w - 2.0 * ROW_INSET;
    assert_rect(
        &reg,
        "AchievementRow0",
        LayoutRect {
            x: content_x + ROW_INSET,
            y: content_y + ROW_INSET,
            width: row_w,
            height: ROW_H,
        },
    );
}
