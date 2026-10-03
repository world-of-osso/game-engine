use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: QuestLogFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(quest_log_frame_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn row(quest_id: u32, title: &str, complete: bool, selected: bool) -> QuestLogRow {
    QuestLogRow {
        quest_id,
        title: title.into(),
        complete,
        watched: true,
        selected,
    }
}

fn northshire_log(collapsed: bool) -> QuestLogFrameState {
    QuestLogFrameState {
        visible: true,
        quest_count: 2,
        max_quests: 35,
        groups: vec![QuestLogGroup {
            sort_id: 9,
            name: "Northshire Valley".into(),
            collapsed,
            quests: vec![
                row(783, "A Threat Within", true, true),
                row(7, "Kobold Camp Cleanup", false, false),
            ],
        }],
        details: Some(QuestLogDetails {
            quest_id: 783,
            title: "A Threat Within".into(),
            objectives_text: "Speak with Marshal McBride.".into(),
            objectives: vec![],
            description: Some("I hope you strapped your belt on tight.".into()),
            rewards: None,
            watched: true,
        }),
    }
}

fn onclick(reg: &FrameRegistry, name: &str) -> Option<String> {
    reg.get(reg.get_by_name(name).expect(name))
        .unwrap()
        .onclick
        .clone()
}

#[test]
fn list_groups_quests_under_zone_headers_with_count() {
    let reg = build(northshire_log(false));
    assert_eq!(fontstring_text(&reg, "QuestLogFrameTitleText"), "Quest Log");
    assert_eq!(fontstring_text(&reg, "QuestLogCount"), "Quests: 2/35");
    assert_eq!(
        fontstring_text(&reg, "QuestLogHeader9Text"),
        "Northshire Valley"
    );
    assert_eq!(
        fontstring_text(&reg, "QuestLogTitle783Text"),
        "A Threat Within"
    );
    assert_eq!(
        fontstring_text(&reg, "QuestLogTitle7Text"),
        "Kobold Camp Cleanup"
    );
    assert!(reg.get_by_name("QuestLogTitle783Selected").is_some());
    assert!(reg.get_by_name("QuestLogTitle7Selected").is_none());
    assert_eq!(
        onclick(&reg, "QuestLogTitle7Text").as_deref(),
        Some("quest_log:select:7")
    );
    assert_eq!(
        onclick(&reg, "QuestLogHeader9Text").as_deref(),
        Some("quest_log:header:9")
    );
}

#[test]
fn collapsed_header_hides_its_quests() {
    let reg = build(northshire_log(true));
    assert_eq!(fontstring_text(&reg, "QuestLogHeader9Marker"), "+");
    assert!(reg.get_by_name("QuestLogTitle783Text").is_none());
}

#[test]
fn details_show_objectives_description_and_track_state() {
    let reg = build(northshire_log(false));
    assert_eq!(
        fontstring_text(&reg, "QuestLogDetailsTitle"),
        "A Threat Within"
    );
    assert_eq!(
        fontstring_text(&reg, "QuestLogDetailsObjectivesText"),
        "Speak with Marshal McBride."
    );
    assert_eq!(
        fontstring_text(&reg, "QuestLogDetailsDescription"),
        "I hope you strapped your belt on tight."
    );
    assert_eq!(fontstring_text(&reg, "QuestLogTrackButton"), "Untrack");
    assert_eq!(
        onclick(&reg, "QuestLogAbandonButton").as_deref(),
        Some(ABANDON_ACTION)
    );

    let mut untracked = northshire_log(false);
    untracked.details.as_mut().unwrap().watched = false;
    assert_eq!(
        fontstring_text(&build(untracked), "QuestLogTrackButton"),
        "Track"
    );
}

#[test]
fn empty_log_shows_the_retail_hint_and_no_details_buttons() {
    let reg = build(QuestLogFrameState {
        visible: true,
        max_quests: 35,
        ..QuestLogFrameState::default()
    });
    assert!(fontstring_text(&reg, "QuestLogNoQuestsText").starts_with("No quests available"));
    assert!(reg.get_by_name("QuestLogAbandonButton").is_none());
}
