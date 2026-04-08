use bevy::prelude::*;

use crate::quest_data::QuestLogState;

/// Marks a world entity as related to a quest objective.
/// When the objective is active and incomplete, the entity shows a sparkle effect.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct QuestTrackedItem {
    pub quest_id: u32,
    pub objective_index: usize,
}

/// Determines whether a tracked item should display its sparkle effect.
/// Returns true when the quest is in the log and the specific objective is incomplete.
pub fn should_sparkle(tracked: &QuestTrackedItem, quest_log: &QuestLogState) -> bool {
    quest_log.entries.iter().any(|entry| {
        entry.quest_id == tracked.quest_id
            && entry
                .objectives
                .get(tracked.objective_index)
                .is_some_and(|obj| !obj.is_complete())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest_data::{QuestEntry, QuestObjective, QuestType};

    fn make_objective(current: u32, required: u32) -> QuestObjective {
        QuestObjective {
            text: "Kill mobs".into(),
            current,
            required,
        }
    }

    fn make_entry(quest_id: u32, objectives: Vec<QuestObjective>) -> QuestEntry {
        QuestEntry {
            quest_id,
            title: format!("Quest {quest_id}"),
            level: 10,
            zone: "Test".into(),
            quest_type: QuestType::Normal,
            description: String::new(),
            objectives,
            rewards: vec![],
        }
    }

    #[test]
    fn sparkle_when_objective_incomplete() {
        let log = QuestLogState {
            entries: vec![make_entry(100, vec![make_objective(2, 5)])],
            ..Default::default()
        };
        let tracked = QuestTrackedItem {
            quest_id: 100,
            objective_index: 0,
        };
        assert!(should_sparkle(&tracked, &log));
    }

    #[test]
    fn no_sparkle_when_objective_complete() {
        let log = QuestLogState {
            entries: vec![make_entry(100, vec![make_objective(5, 5)])],
            ..Default::default()
        };
        let tracked = QuestTrackedItem {
            quest_id: 100,
            objective_index: 0,
        };
        assert!(!should_sparkle(&tracked, &log));
    }

    #[test]
    fn no_sparkle_when_quest_not_in_log() {
        let log = QuestLogState::default();
        let tracked = QuestTrackedItem {
            quest_id: 100,
            objective_index: 0,
        };
        assert!(!should_sparkle(&tracked, &log));
    }

    #[test]
    fn no_sparkle_when_objective_index_out_of_range() {
        let log = QuestLogState {
            entries: vec![make_entry(100, vec![make_objective(2, 5)])],
            ..Default::default()
        };
        let tracked = QuestTrackedItem {
            quest_id: 100,
            objective_index: 5,
        };
        assert!(!should_sparkle(&tracked, &log));
    }

    #[test]
    fn sparkle_targets_specific_objective() {
        let log = QuestLogState {
            entries: vec![make_entry(
                100,
                vec![
                    make_objective(5, 5), // complete
                    make_objective(1, 3), // incomplete
                ],
            )],
            ..Default::default()
        };
        let complete = QuestTrackedItem {
            quest_id: 100,
            objective_index: 0,
        };
        assert!(!should_sparkle(&complete, &log));

        let incomplete = QuestTrackedItem {
            quest_id: 100,
            objective_index: 1,
        };
        assert!(should_sparkle(&incomplete, &log));
    }

    #[test]
    fn sparkle_only_matching_quest_id() {
        let log = QuestLogState {
            entries: vec![make_entry(100, vec![make_objective(2, 5)])],
            ..Default::default()
        };
        let tracked = QuestTrackedItem {
            quest_id: 200,
            objective_index: 0,
        };
        assert!(!should_sparkle(&tracked, &log));
    }

    #[test]
    fn sparkle_with_zero_required_objective() {
        let log = QuestLogState {
            entries: vec![make_entry(100, vec![make_objective(0, 0)])],
            ..Default::default()
        };
        let tracked = QuestTrackedItem {
            quest_id: 100,
            objective_index: 0,
        };
        // 0/0 is complete per QuestObjective::is_complete
        assert!(!should_sparkle(&tracked, &log));
    }
}
