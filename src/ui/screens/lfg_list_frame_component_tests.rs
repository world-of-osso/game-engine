use super::*;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn make_test_state() -> LFGListFrameState {
    LFGListFrameState {
        visible: true,
        ..Default::default()
    }
}

fn build_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
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
    assert!(reg.get_by_name("LFGListFrame").is_some());
    assert!(reg.get_by_name("LFGListFrameTitle").is_some());
}

#[test]
fn builds_role_checkboxes() {
    let reg = build_registry();
    for i in 0..3 {
        assert!(
            reg.get_by_name(&format!("LFGRoleCheck{i}")).is_some(),
            "LFGRoleCheck{i} missing"
        );
        assert!(
            reg.get_by_name(&format!("LFGRoleLabel{i}")).is_some(),
            "LFGRoleLabel{i} missing"
        );
    }
}

#[test]
fn builds_activity_dropdown() {
    let reg = build_registry();
    assert!(reg.get_by_name("LFGActivityDropdown").is_some());
    assert!(reg.get_by_name("LFGActivityDropdownText").is_some());
}

#[test]
fn builds_content_area() {
    let reg = build_registry();
    assert!(reg.get_by_name("LFGContentArea").is_some());
}

#[test]
fn hidden_when_not_visible() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(LFGListFrameState::default());
    Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("LFGListFrame").expect("frame");
    assert!(reg.get(id).expect("data").hidden);
}

// --- Coord validation ---

#[test]
fn coord_main_frame_centered() {
    let reg = layout_registry();
    let r = rect(&reg, "LFGListFrame");
    let expected_x = (1920.0 - FRAME_W) / 2.0;
    let expected_y = (1080.0 - FRAME_H) / 2.0;
    assert!((r.x - expected_x).abs() < 1.0);
    assert!((r.y - expected_y).abs() < 1.0);
    assert!((r.width - FRAME_W).abs() < 1.0);
}

#[test]
fn coord_first_role_checkbox() {
    let reg = layout_registry();
    let frame_x = (1920.0 - FRAME_W) / 2.0;
    let frame_y = (1080.0 - FRAME_H) / 2.0;
    let r = rect(&reg, "LFGRoleCheck0");
    assert!((r.x - (frame_x + ROLE_INSET)).abs() < 1.0);
    assert!((r.y - (frame_y + ROLE_ROW_Y)).abs() < 1.0);
    assert!((r.width - ROLE_CHECK_SIZE).abs() < 1.0);
}

#[test]
fn coord_activity_dropdown() {
    let reg = layout_registry();
    let r = rect(&reg, "LFGActivityDropdown");
    assert!((r.width - DROPDOWN_W).abs() < 1.0);
    assert!((r.height - DROPDOWN_H).abs() < 1.0);
}

// --- Group list tests ---

fn make_group_state() -> LFGListFrameState {
    let mut state = make_test_state();
    state.groups = vec![
        GroupListEntry {
            leader: "Arthas".into(),
            members: "3/5".into(),
            activity: "Deadmines".into(),
            note: "Need healer".into(),
        },
        GroupListEntry {
            leader: "Thrall".into(),
            members: "4/5".into(),
            activity: "SFK".into(),
            note: String::new(),
        },
    ];
    state
}

fn group_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_group_state());
    Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
    reg
}

#[test]
fn group_list_builds_header_and_rows() {
    let reg = group_registry();
    assert!(reg.get_by_name("LFGGroupHeader").is_some());
    for i in 0..GROUP_COLUMNS.len() {
        assert!(
            reg.get_by_name(&format!("LFGGroupCol{i}")).is_some(),
            "LFGGroupCol{i} missing"
        );
    }
    for i in 0..2 {
        assert!(
            reg.get_by_name(&format!("LFGGroup{i}")).is_some(),
            "LFGGroup{i} missing"
        );
        for col in 0..GROUP_COLUMNS.len() {
            assert!(
                reg.get_by_name(&format!("LFGGroup{i}Col{col}")).is_some(),
                "LFGGroup{i}Col{col} missing"
            );
        }
    }
}

#[test]
fn group_list_has_apply_button() {
    let reg = group_registry();
    assert!(reg.get_by_name("LFGApplyButton").is_some());
    assert!(reg.get_by_name("LFGApplyButtonText").is_some());
}

// --- Create group form tests ---

#[test]
fn create_form_builds_inputs() {
    let reg = build_registry();
    assert!(reg.get_by_name("LFGCreateGroupForm").is_some());
    assert!(reg.get_by_name("LFGFormTitleInput").is_some());
    assert!(reg.get_by_name("LFGFormDescInput").is_some());
    assert!(reg.get_by_name("LFGFormILevelInput").is_some());
}

#[test]
fn create_form_builds_voice_and_submit() {
    let reg = build_registry();
    assert!(reg.get_by_name("LFGFormVoiceCheck").is_some());
    assert!(reg.get_by_name("LFGFormVoiceLabel").is_some());
    assert!(reg.get_by_name("LFGFormSubmitButton").is_some());
}

#[test]
fn create_form_hidden_by_default() {
    let reg = build_registry();
    let id = reg.get_by_name("LFGCreateGroupForm").expect("form");
    assert!(reg.get(id).expect("data").hidden);
}

// --- Additional coord validation ---

#[test]
fn coord_group_header() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_group_state());
    Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);

    let content = rect(&reg, "LFGContentArea");
    let header = rect(&reg, "LFGGroupHeader");
    assert!((header.x - (content.x + GROUP_INSET)).abs() < 1.0);
    assert!((header.y - (content.y + GROUP_INSET)).abs() < 1.0);
    assert!((header.height - GROUP_HEADER_H).abs() < 1.0);
}

#[test]
fn coord_apply_button() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_group_state());
    Screen::new(lfg_list_frame_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);

    let r = rect(&reg, "LFGApplyButton");
    assert!((r.width - APPLY_BTN_W).abs() < 1.0);
    assert!((r.height - APPLY_BTN_H).abs() < 1.0);
}
