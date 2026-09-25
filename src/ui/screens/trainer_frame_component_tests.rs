use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: TrainerFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(trainer_frame_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn frame(reg: &FrameRegistry, name: &str) -> u64 {
    reg.get_by_name(name)
        .unwrap_or_else(|| panic!("{name} missing"))
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(frame(reg, name))
        .and_then(|f| f.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout rect"))
}

fn offset(reg: &FrameRegistry, name: &str) -> (f32, f32) {
    let (root, child) = (rect(reg, FRAME_NAME), rect(reg, name));
    (child.x - root.x, child.y - root.y)
}

fn text_color(reg: &FrameRegistry, name: &str) -> [f32; 4] {
    match reg.get(frame(reg, name)).unwrap().widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.color,
        _ => panic!("{name} is not a FontString"),
    }
}

/// Georgio Bolero's first services for a level 10 human with 5 copper: Tailoring
/// (10c, unaffordable) and White Linen Shirt (needs Classic Tailoring 1).
fn georgio() -> TrainerFrameState {
    TrainerFrameState {
        visible: true,
        title: "Georgio Bolero".into(),
        rank: None,
        rows: vec![
            ServiceRow {
                name: "Tailoring".into(),
                icon_fdid: 4_620_681,
                sub_text: "Requires Level 5".into(),
                sub_text_red: false,
                unavailable: false,
                cost: Some(10),
                cost_red: true,
                selected: true,
            },
            ServiceRow {
                name: "White Linen Shirt".into(),
                icon_fdid: 132_149,
                sub_text: "Requires Classic Tailoring (1)".into(),
                sub_text_red: true,
                unavailable: true,
                cost: Some(10),
                cost_red: true,
                selected: false,
            },
        ],
        train_enabled: false,
        money: 5,
    }
}

#[test]
fn rows_show_name_requirements_and_cost() {
    let reg = build(georgio());
    assert_eq!(
        fontstring_text(&reg, "ClassTrainerFrameSkill1Name"),
        "Tailoring"
    );
    assert_eq!(
        fontstring_text(&reg, "ClassTrainerFrameSkill2SubText"),
        "Requires Classic Tailoring (1)"
    );
    assert_eq!(
        fontstring_text(&reg, "ClassTrainerFrameSkill1MoneyFrameAmount0"),
        "10"
    );
    assert_eq!(
        text_color(&reg, "ClassTrainerFrameSkill1MoneyFrameAmount0"),
        [1.0, 0.1255, 0.1255, 1.0]
    );
    assert!(
        reg.get_by_name("ClassTrainerFrameSkill2DisabledBG")
            .is_some()
    );
    assert!(
        reg.get_by_name("ClassTrainerFrameSkill1DisabledBG")
            .is_none()
    );
    assert!(reg.get_by_name("ClassTrainerFrameSkill1Selected").is_some());
}

#[test]
fn rows_follow_the_scroll_box_geometry() {
    let reg = build(georgio());
    // ScrollBox TOPRIGHT at the Inset's −5,+5; rows 47 apart with 1 px padding.
    assert_eq!(offset(&reg, "ClassTrainerFrameSkill1"), (26.0, 56.0));
    assert_eq!(offset(&reg, "ClassTrainerFrameSkill2"), (26.0, 103.0));
    assert_eq!(rect(&reg, "ClassTrainerFrameSkill1").width, 298.0);
}

#[test]
fn train_button_is_inert_while_disabled_and_rows_select_by_index() {
    let reg = build(georgio());
    let train = reg.get(frame(&reg, "ClassTrainerTrainButton")).unwrap();
    assert!(train.onclick.as_deref().is_none_or(str::is_empty));
    let row = reg.get(frame(&reg, "ClassTrainerFrameSkill2")).unwrap();
    assert_eq!(row.onclick.as_deref(), Some("trainer_service:1"));
    let enabled = build(TrainerFrameState {
        train_enabled: true,
        ..georgio()
    });
    let train = enabled
        .get(frame(&enabled, "ClassTrainerTrainButton"))
        .unwrap();
    assert_eq!(train.onclick.as_deref(), Some(ACTION_TRAIN));
}

#[test]
fn rank_bar_shows_the_trainer_line_rank() {
    let reg = build(TrainerFrameState {
        rank: Some(("12/300".into(), 12.0 / 300.0)),
        ..georgio()
    });
    assert_eq!(
        fontstring_text(&reg, "ClassTrainerStatusBarSkillRank"),
        "12/300"
    );
    assert_eq!(
        offset(&reg, "ClassTrainerStatusBarBackground"),
        (64.0, 36.0)
    );
    assert!(
        build(georgio())
            .get_by_name("ClassTrainerStatusBarBackground")
            .is_none()
    );
}
