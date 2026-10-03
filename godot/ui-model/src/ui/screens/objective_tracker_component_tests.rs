use shared::protocol::{
    QuestLogSnapshot, QuestObjectiveKind, QuestObjectiveSnapshot, QuestRepeatability,
};
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn entry(quest_id: u32, title: &str, current: u32) -> QuestEntrySnapshot {
    QuestEntrySnapshot {
        quest_id,
        title: title.into(),
        zone: String::new(),
        completed: current >= 8,
        repeatability: QuestRepeatability::Normal,
        objectives: vec![QuestObjectiveSnapshot {
            text: "Kobold Vermin slain".into(),
            current,
            required: 8,
            completed: current >= 8,
            kind: QuestObjectiveKind::Monster,
            object_id: 6,
        }],
        level: 2,
        sort_id: 9,
        objectives_text: "Kill 8 Kobold Vermin, then return to Marshal McBride.".into(),
        completion_text: String::new(),
        watched: true,
        pois: vec![],
    }
}

/// A Threat Within: no objectives, complete on accept, no completion log text.
fn threat_within() -> QuestEntrySnapshot {
    QuestEntrySnapshot {
        objectives: vec![],
        completed: true,
        objectives_text: "Speak with Marshal McBride.".into(),
        ..entry(783, "A Threat Within", 0)
    }
}

fn runtime(entries: Vec<QuestEntrySnapshot>, watched: Vec<u32>) -> QuestRuntime {
    let mut runtime = QuestRuntime::default();
    runtime.apply_snapshot(QuestLogSnapshot {
        entries,
        watched_quest_ids: watched,
    });
    runtime
}

fn build(state: ObjectiveTrackerState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(objective_tracker_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|f| f.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

fn visible(reg: &FrameRegistry, name: &str) -> bool {
    reg.get(reg.get_by_name(name).expect(name)).unwrap().visible
}

#[test]
fn watched_quests_become_blocks_in_watch_order_and_unwatched_are_skipped() {
    let runtime = runtime(
        vec![
            entry(7, "Kobold Camp Cleanup", 3),
            threat_within(),
            entry(18, "Brotherhood of Thieves", 0),
        ],
        vec![783, 7],
    );
    let state = ObjectiveTrackerState::from_runtime(&runtime, false, false);

    assert_eq!(
        state.quests.iter().map(|q| q.quest_id).collect::<Vec<_>>(),
        vec![783, 7]
    );
    assert_eq!(
        state.quests[1].lines,
        vec![ObjectiveLine {
            text: "3/8 Kobold Vermin slain".into(),
            style: ObjectiveLineStyle::InProgress,
        }]
    );
    assert_eq!(
        state.quests[0].lines,
        vec![ObjectiveLine {
            text: READY_FOR_TURN_IN.into(),
            style: ObjectiveLineStyle::Completed,
        }]
    );
}

#[test]
fn finished_quest_with_completion_log_shows_only_that_text() {
    let mut done = entry(7, "Kobold Camp Cleanup", 8);
    done.completion_text = "Return to Marshal McBride at Northshire Abbey in Elwynn Forest.".into();
    let state = ObjectiveTrackerState::from_runtime(&runtime(vec![done], vec![7]), false, false);
    assert_eq!(
        state.quests[0].lines,
        vec![ObjectiveLine {
            text: "Return to Marshal McBride at Northshire Abbey in Elwynn Forest.".into(),
            style: ObjectiveLineStyle::CompletionText,
        }]
    );
    assert!(state.quests[0].complete);
}

#[test]
fn tracker_renders_headers_titles_and_lines_at_the_retail_anchor() {
    let runtime = runtime(
        vec![entry(7, "Kobold Camp Cleanup", 3), threat_within()],
        vec![7, 783],
    );
    let reg = build(ObjectiveTrackerState::from_runtime(&runtime, false, false));

    let frame = rect(&reg, TRACKER_FRAME);
    assert!((frame.x - (1920.0 - TRACKER_RIGHT - TRACKER_W)).abs() < 1.0);
    assert!((frame.y - TRACKER_TOP).abs() < 1.0);
    assert_eq!(
        fontstring_text(&reg, "ObjectiveTrackerFrameHeaderText"),
        "All Objectives"
    );
    assert_eq!(
        fontstring_text(&reg, "QuestObjectiveTrackerHeaderText"),
        "Quests"
    );
    assert_eq!(
        fontstring_text(&reg, "QuestBlock7HeaderText"),
        "Kobold Camp Cleanup"
    );
    assert_eq!(
        fontstring_text(&reg, "QuestBlock7Line0Text"),
        "3/8 Kobold Vermin slain"
    );
    assert_eq!(fontstring_text(&reg, "QuestBlock7Line0Dash"), "- ");
    assert_eq!(
        fontstring_text(&reg, "QuestBlock783Line0Text"),
        READY_FOR_TURN_IN
    );
    assert_eq!(fontstring_text(&reg, "QuestBlock783Line0Dash"), "");
    assert!(reg.get_by_name("QuestBlock783Line0Check").is_some());
    assert!(reg.get_by_name("QuestBlock783POIButtonTurnIn").is_some());
    assert!(reg.get_by_name("QuestBlock7POIButtonInProgress").is_some());

    let first = rect(&reg, "QuestBlock7HeaderText");
    let second = rect(&reg, "QuestBlock783HeaderText");
    assert!(
        second.y > rect(&reg, "QuestBlock7Line0Text").y,
        "blocks stack downwards"
    );
    assert!((first.x - (frame.x + 20.0)).abs() < 1.0, "blockOffsetX 20");
    assert!(
        frame.height >= second.y - frame.y + 12.0,
        "frame grows to its blocks"
    );
}

#[test]
fn collapsing_hides_blocks_and_empty_tracker_shows_only_its_header() {
    let runtime = runtime(vec![entry(7, "Kobold Camp Cleanup", 3)], vec![7]);
    let collapsed = build(ObjectiveTrackerState::from_runtime(&runtime, true, false));
    assert!(collapsed.get_by_name("QuestBlock7HeaderText").is_none());
    assert!(
        collapsed
            .get_by_name("QuestObjectiveTrackerHeaderText")
            .is_none()
    );
    assert!((rect(&collapsed, TRACKER_FRAME).height - 32.0).abs() < 1.0);

    let module_collapsed = build(ObjectiveTrackerState::from_runtime(&runtime, false, true));
    assert!(
        module_collapsed
            .get_by_name("QuestBlock7HeaderText")
            .is_none()
    );
    assert_eq!(
        fontstring_text(&module_collapsed, "QuestObjectiveTrackerHeaderText"),
        "Quests"
    );

    let empty = build(ObjectiveTrackerState::default());
    assert!(visible(&empty, TRACKER_FRAME));
    assert_eq!(
        fontstring_text(&empty, "ObjectiveTrackerFrameHeaderText"),
        "All Objectives"
    );
    assert!(
        empty
            .get_by_name("QuestObjectiveTrackerHeaderText")
            .is_none()
    );
    assert!((rect(&empty, TRACKER_FRAME).height - 32.0).abs() < 1.0);
}

#[test]
fn block_title_and_poi_open_the_quest() {
    let runtime = runtime(vec![entry(7, "Kobold Camp Cleanup", 3)], vec![7]);
    let reg = build(ObjectiveTrackerState::from_runtime(&runtime, false, false));
    let onclick = |name: &str| {
        reg.get(reg.get_by_name(name).unwrap())
            .unwrap()
            .onclick
            .clone()
    };
    assert_eq!(
        onclick("QuestBlock7HeaderText").as_deref(),
        Some("quest_tracker:open:7")
    );
    assert_eq!(
        onclick("QuestBlock7POIButton").as_deref(),
        Some("quest_tracker:open:7")
    );
    assert_eq!(
        onclick("ObjectiveTrackerFrameHeaderMinimizeButton").as_deref(),
        Some(TOGGLE_ACTION)
    );
}
