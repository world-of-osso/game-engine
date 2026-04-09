use super::*;

#[test]
fn quest_type_labels() {
    assert_eq!(QuestType::Normal.label(), "Quest");
    assert_eq!(QuestType::Daily.label(), "Daily Quest");
    assert_eq!(QuestType::Weekly.label(), "Weekly Quest");
    assert_eq!(QuestType::Campaign.label(), "Campaign Quest");
}

#[test]
fn objective_completion() {
    assert!(sample_objective(5, 5).is_complete());
    assert!(sample_objective(6, 5).is_complete());
    assert!(!sample_objective(4, 5).is_complete());
}

#[test]
fn objective_progress_text() {
    let multi = QuestObjective {
        text: "Kill mobs".into(),
        current: 3,
        required: 5,
    };
    assert_eq!(multi.progress_text(), "Kill mobs: 3/5");

    let single = QuestObjective {
        text: "Talk to NPC".into(),
        current: 0,
        required: 1,
    };
    assert_eq!(single.progress_text(), "Talk to NPC");
}

#[test]
fn objective_progress_fraction() {
    let obj = sample_objective(3, 10);
    assert!((obj.progress_fraction() - 0.3).abs() < 0.01);

    let zero = sample_objective(0, 0);
    assert_eq!(zero.progress_fraction(), 1.0);

    let over = sample_objective(12, 10);
    assert_eq!(over.progress_fraction(), 1.0);
}

#[test]
fn reward_display_name() {
    let single = QuestReward {
        name: "Sword".into(),
        icon_fdid: 1,
        quantity: 1,
    };
    assert_eq!(single.display_name(), "Sword");

    let multi = QuestReward {
        name: "Gold Dust".into(),
        icon_fdid: 2,
        quantity: 5,
    };
    assert_eq!(multi.display_name(), "Gold Dust x5");
}

#[test]
fn quest_entry_completion() {
    assert!(sample_entry(1, "Z", true).is_complete());
    assert!(!sample_entry(2, "Z", false).is_complete());
}

#[test]
fn quest_entry_objective_summary() {
    let entry = QuestEntry {
        objectives: vec![sample_objective(5, 5), sample_objective(2, 5)],
        ..sample_entry(1, "Z", false)
    };
    assert_eq!(entry.objective_summary(), "1/2");
}

#[test]
fn required_item_satisfaction() {
    let sat = RequiredItem {
        name: "A".into(),
        icon_fdid: 1,
        current: 5,
        required: 5,
    };
    assert!(sat.is_satisfied());

    let not = RequiredItem {
        name: "B".into(),
        icon_fdid: 2,
        current: 3,
        required: 5,
    };
    assert!(!not.is_satisfied());
}

#[test]
fn required_item_count_text() {
    let item = RequiredItem {
        name: "X".into(),
        icon_fdid: 1,
        current: 3,
        required: 10,
    };
    assert_eq!(item.count_text(), "3/10");
}

#[test]
fn texture_fdids_are_nonzero() {
    assert_ne!(textures::QUEST_LOG_PANE_LEFT, 0);
    assert_ne!(textures::QUEST_LOG_PANE_RIGHT, 0);
    assert_ne!(textures::QUEST_LOG_TOP_LEFT, 0);
    assert_ne!(textures::QUEST_LOG_BOT_LEFT, 0);
    assert_ne!(textures::QUEST_LOG_TITLE_HIGHLIGHT, 0);
    assert_ne!(textures::QUEST_LOG_BOOK_ICON, 0);
    assert_ne!(textures::HORIZONTAL_BREAK, 0);
    assert_ne!(textures::BULLET_POINT, 0);
    assert_ne!(textures::QUEST_BANG_NORMAL, 0);
    assert_ne!(textures::QUEST_TURNIN, 0);
    assert_ne!(textures::QUEST_BANG_DAILY, 0);
    assert_ne!(textures::QUEST_BANG_CAMPAIGN, 0);
    assert_ne!(textures::QUEST_TURNIN_CAMPAIGN, 0);
    assert_ne!(textures::QUEST_DIALOG_TOP_LEFT, 0);
    assert_ne!(textures::QUEST_DIALOG_BOT_LEFT, 0);
    assert_ne!(textures::NPC_PORTRAIT_RING, 0);
    assert_ne!(textures::NPC_PORTRAIT_RING_HIGHLIGHT, 0);
    assert_ne!(textures::REWARD_ICON_SWORD, 0);
    assert_ne!(textures::REWARD_ICON_SHIELD, 0);
    assert_ne!(textures::REWARD_ICON_GOLD, 0);
    assert_ne!(textures::REWARD_ICON_XP, 0);
}
