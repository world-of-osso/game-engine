pub(super) use super::*;

mod dialog;
mod log;
mod queue;
mod values;

pub(super) fn sample_objective(current: u32, required: u32) -> QuestObjective {
    QuestObjective {
        text: "Kill mobs".into(),
        current,
        required,
    }
}

pub(super) fn sample_entry(quest_id: u32, zone: &str, complete: bool) -> QuestEntry {
    let current = if complete { 5 } else { 2 };
    QuestEntry {
        quest_id,
        title: format!("Quest {quest_id}"),
        level: 25,
        zone: zone.into(),
        quest_type: QuestType::Normal,
        description: "A test quest.".into(),
        objectives: vec![sample_objective(current, 5)],
        rewards: vec![QuestReward {
            name: "Sword".into(),
            icon_fdid: 100001,
            quantity: 1,
        }],
    }
}
