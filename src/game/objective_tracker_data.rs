use bevy::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct QuestObjective {
    pub text: String,
    pub current: u32,
    pub required: u32,
}

impl QuestObjective {
    pub fn is_complete(&self) -> bool {
        self.current >= self.required
    }

    pub fn progress_text(&self) -> String {
        format!("{}/{}", self.current, self.required)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackedQuestData {
    pub quest_id: u32,
    pub title: String,
    pub objectives: Vec<QuestObjective>,
    pub collapsed: bool,
}

impl TrackedQuestData {
    pub fn is_complete(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(|o| o.is_complete())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BonusObjectiveData {
    pub name: String,
    pub current: u32,
    pub required: u32,
}

impl BonusObjectiveData {
    pub fn progress(&self) -> f32 {
        if self.required == 0 {
            return 1.0;
        }
        self.current as f32 / self.required as f32
    }

    pub fn progress_text(&self) -> String {
        format!("{}/{}", self.current, self.required)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioData {
    pub name: String,
    pub current_stage: usize,
    pub stages: Vec<String>,
}

impl ScenarioData {
    pub fn is_stage_complete(&self, index: usize) -> bool {
        index < self.current_stage
    }
}

/// Runtime objective tracker state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct ObjectiveTrackerData {
    pub tracked_quests: Vec<TrackedQuestData>,
    pub bonus_objectives: Vec<BonusObjectiveData>,
    pub scenario: Option<ScenarioData>,
}

impl ObjectiveTrackerData {
    pub fn completed_quest_count(&self) -> usize {
        self.tracked_quests
            .iter()
            .filter(|q| q.is_complete())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quest() -> TrackedQuestData {
        TrackedQuestData {
            quest_id: 1,
            title: "Test Quest".into(),
            objectives: vec![
                QuestObjective {
                    text: "Kill 5 mobs".into(),
                    current: 5,
                    required: 5,
                },
                QuestObjective {
                    text: "Collect 3 items".into(),
                    current: 1,
                    required: 3,
                },
            ],
            collapsed: false,
        }
    }

    #[test]
    fn objective_completion() {
        let done = QuestObjective {
            text: "x".into(),
            current: 5,
            required: 5,
        };
        assert!(done.is_complete());
        assert_eq!(done.progress_text(), "5/5");
        let partial = QuestObjective {
            text: "x".into(),
            current: 2,
            required: 5,
        };
        assert!(!partial.is_complete());
    }

    #[test]
    fn quest_complete_requires_all() {
        let q = quest();
        assert!(!q.is_complete());
        let done = TrackedQuestData {
            objectives: vec![QuestObjective {
                text: "a".into(),
                current: 1,
                required: 1,
            }],
            ..quest()
        };
        assert!(done.is_complete());
    }

    #[test]
    fn bonus_objective_progress() {
        let b = BonusObjectiveData {
            name: "Bridge".into(),
            current: 3,
            required: 5,
        };
        assert!((b.progress() - 0.6).abs() < 0.01);
        assert_eq!(b.progress_text(), "3/5");
    }

    #[test]
    fn scenario_stage_completion() {
        let s = ScenarioData {
            name: "Proving Grounds".into(),
            current_stage: 2,
            stages: vec!["Wave 1".into(), "Wave 2".into(), "Wave 3".into()],
        };
        assert!(s.is_stage_complete(0));
        assert!(s.is_stage_complete(1));
        assert!(!s.is_stage_complete(2));
    }

    #[test]
    fn completed_quest_count() {
        let data = ObjectiveTrackerData {
            tracked_quests: vec![
                TrackedQuestData {
                    objectives: vec![QuestObjective {
                        text: "a".into(),
                        current: 1,
                        required: 1,
                    }],
                    ..quest()
                },
                quest(),
            ],
            ..Default::default()
        };
        assert_eq!(data.completed_quest_count(), 1);
    }
}
