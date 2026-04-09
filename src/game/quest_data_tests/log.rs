use super::*;

#[test]
fn quest_log_selected_entry() {
    let state = QuestLogState {
        entries: vec![sample_entry(10, "Z", false), sample_entry(20, "Z", true)],
        selected_quest_id: Some(20),
        ..Default::default()
    };
    let selected = state.selected_entry().expect("should find quest 20");
    assert_eq!(selected.quest_id, 20);
}

#[test]
fn quest_log_selected_none() {
    let state = QuestLogState::default();
    assert!(state.selected_entry().is_none());
}

#[test]
fn quest_log_counts() {
    let state = QuestLogState {
        entries: vec![
            sample_entry(1, "A", true),
            sample_entry(2, "A", false),
            sample_entry(3, "B", true),
        ],
        ..Default::default()
    };
    assert_eq!(state.quest_count(), 3);
    assert_eq!(state.completed_count(), 2);
}

#[test]
fn quest_log_zones() {
    let state = QuestLogState {
        entries: vec![
            sample_entry(1, "Elwynn Forest", false),
            sample_entry(2, "Elwynn Forest", false),
            sample_entry(3, "Westfall", false),
        ],
        ..Default::default()
    };
    let zones = state.zones();
    assert_eq!(zones, vec!["Elwynn Forest", "Westfall"]);
}

#[test]
fn quest_log_dialog_presence() {
    let mut state = QuestLogState::default();
    assert!(!state.has_active_dialog());
    state.dialog = Some(NPCDialogState::default());
    assert!(state.has_active_dialog());
}

#[test]
fn quests_in_zone_filters() {
    let state = QuestLogState {
        entries: vec![
            sample_entry(1, "Elwynn Forest", false),
            sample_entry(2, "Westfall", false),
            sample_entry(3, "Elwynn Forest", true),
            sample_entry(4, "Duskwood", false),
        ],
        ..Default::default()
    };
    let elwynn = state.quests_in_zone("Elwynn Forest");
    assert_eq!(elwynn.len(), 2);
    assert_eq!(elwynn[0].quest_id, 1);
    assert_eq!(elwynn[1].quest_id, 3);
}

#[test]
fn quests_in_nonexistent_zone() {
    let state = QuestLogState {
        entries: vec![sample_entry(1, "Stormwind", false)],
        ..Default::default()
    };
    assert!(state.quests_in_zone("Nowhere").is_empty());
}

#[test]
fn zones_preserves_insertion_order() {
    let state = QuestLogState {
        entries: vec![
            sample_entry(1, "Westfall", false),
            sample_entry(2, "Elwynn Forest", false),
            sample_entry(3, "Westfall", false),
        ],
        ..Default::default()
    };
    let zones = state.zones();
    assert_eq!(zones, vec!["Westfall", "Elwynn Forest"]);
}

#[test]
fn quest_with_no_objectives_not_complete() {
    let entry = QuestEntry {
        objectives: vec![],
        ..sample_entry(1, "Z", false)
    };
    assert!(!entry.is_complete());
}

#[test]
fn quest_with_multiple_objectives_partial() {
    let entry = QuestEntry {
        objectives: vec![
            sample_objective(5, 5),
            sample_objective(3, 5),
            sample_objective(5, 5),
        ],
        ..sample_entry(1, "Z", false)
    };
    assert!(!entry.is_complete());
    assert_eq!(entry.objective_summary(), "2/3");
}

#[test]
fn quest_with_all_objectives_complete() {
    let entry = QuestEntry {
        objectives: vec![sample_objective(5, 5), sample_objective(10, 10)],
        ..sample_entry(1, "Z", false)
    };
    assert!(entry.is_complete());
    assert_eq!(entry.objective_summary(), "2/2");
}

#[test]
fn selected_quest_id_not_found() {
    let state = QuestLogState {
        entries: vec![sample_entry(1, "Z", false)],
        selected_quest_id: Some(999),
        ..Default::default()
    };
    assert!(state.selected_entry().is_none());
}

#[test]
fn quests_by_type_filters() {
    let mut entries = vec![sample_entry(1, "A", false), sample_entry(2, "A", false)];
    entries[1].quest_type = QuestType::Daily;
    let state = QuestLogState {
        entries,
        ..Default::default()
    };
    assert_eq!(state.quests_by_type(QuestType::Normal).len(), 1);
    assert_eq!(state.quests_by_type(QuestType::Daily).len(), 1);
    assert!(state.quests_by_type(QuestType::Campaign).is_empty());
}

#[test]
fn accept_quest_adds_to_log() {
    let mut state = QuestLogState::default();
    let entry = sample_entry(10, "Elwynn", false);
    assert!(state.accept_quest(entry));
    assert_eq!(state.quest_count(), 1);
}

#[test]
fn accept_duplicate_rejected() {
    let mut state = QuestLogState::default();
    state.accept_quest(sample_entry(10, "Z", false));
    assert!(!state.accept_quest(sample_entry(10, "Z", false)));
    assert_eq!(state.quest_count(), 1);
}

#[test]
fn abandon_quest_removes() {
    let mut state = QuestLogState::default();
    state.accept_quest(sample_entry(10, "Z", false));
    assert!(state.abandon_quest(10));
    assert_eq!(state.quest_count(), 0);
}

#[test]
fn abandon_nonexistent_returns_false() {
    let mut state = QuestLogState::default();
    assert!(!state.abandon_quest(999));
}

#[test]
fn turn_in_completed_quest() {
    let mut state = QuestLogState::default();
    state.accept_quest(sample_entry(10, "Z", true));
    assert!(state.turn_in_quest(10));
    assert_eq!(state.quest_count(), 0);
}

#[test]
fn turn_in_incomplete_fails() {
    let mut state = QuestLogState::default();
    state.accept_quest(sample_entry(10, "Z", false));
    assert!(!state.turn_in_quest(10));
    assert_eq!(state.quest_count(), 1);
}
