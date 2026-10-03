use shared::protocol::{
    QuestGiverQuestState, QuestObjectiveKind, QuestObjectiveSnapshot, QuestRepeatability,
    QuestRewardItem, QuestRewards,
};

use super::*;

const WILLEM: u64 = 823;
const MCBRIDE: u64 = 197;

fn kobold_camp_cleanup(current: u32) -> QuestEntrySnapshot {
    QuestEntrySnapshot {
        quest_id: 7,
        title: "Kobold Camp Cleanup".into(),
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
        completion_text: "Return to Marshal McBride at Northshire Abbey in Elwynn Forest.".into(),
        watched: true,
        pois: vec![],
    }
}

fn threat_within_offer() -> QuestGiverOfferReward {
    QuestGiverOfferReward {
        npc: MCBRIDE,
        quest_id: 783,
        title: "A Threat Within".into(),
        reward_text: "Ah, good.  Another volunteer.".into(),
        rewards: QuestRewards::default(),
    }
}

#[test]
fn update_adds_changes_and_removes_entries_and_announces_accepts() {
    let mut runtime = QuestRuntime::default();
    runtime.apply_snapshot(QuestLogSnapshot {
        entries: vec![kobold_camp_cleanup(2)],
        watched_quest_ids: vec![7],
    });

    let mut threat = kobold_camp_cleanup(0);
    threat.quest_id = 783;
    threat.title = "A Threat Within".into();
    let notices = runtime.apply_update(QuestLogUpdate {
        changed: vec![kobold_camp_cleanup(3), threat],
        removed: vec![],
        watched_quest_ids: vec![7, 783],
    });

    assert_eq!(notices, vec!["Quest accepted: A Threat Within".to_string()]);
    assert_eq!(runtime.entry(7).unwrap().objectives[0].current, 3);
    assert_eq!(
        runtime
            .watched_entries()
            .iter()
            .map(|entry| entry.quest_id)
            .collect::<Vec<_>>(),
        vec![7, 783]
    );

    runtime.apply_update(QuestLogUpdate {
        changed: vec![],
        removed: vec![7],
        watched_quest_ids: vec![783],
    });
    assert!(runtime.entry(7).is_none());
    assert!(!runtime.is_watched(7));
}

#[test]
fn quest_list_fills_the_open_greeting_of_the_same_npc_only() {
    let mut runtime = QuestRuntime::default();
    runtime.open_gossip(
        WILLEM,
        "Deputy Willem".into(),
        GossipMenu {
            menu_id: 57020,
            text: "Greetings, $n.".into(),
            options: vec![],
        },
    );
    let entry = QuestGiverQuestEntry {
        quest_id: 783,
        title: "A Threat Within".into(),
        level: 1,
        state: QuestGiverQuestState::Available,
    };
    runtime.apply_quest_list(QuestGiverQuestList {
        npc: MCBRIDE,
        quests: vec![entry.clone()],
    });
    let Some(QuestDialog {
        page: QuestDialogPage::Greeting { quests, .. },
        ..
    }) = &runtime.dialog
    else {
        panic!("greeting stays open");
    };
    assert!(quests.is_empty(), "another NPC's list is ignored");

    runtime.apply_quest_list(QuestGiverQuestList {
        npc: WILLEM,
        quests: vec![entry.clone()],
    });
    let Some(QuestDialog {
        page: QuestDialogPage::Greeting { quests, text, .. },
        ..
    }) = &runtime.dialog
    else {
        panic!("greeting stays open");
    };
    assert_eq!(quests, &vec![entry]);
    assert_eq!(text, "Greetings, $n.");
}

#[test]
fn reward_choice_is_bounded_by_the_offered_choices() {
    let mut runtime = QuestRuntime::default();
    let mut offer = threat_within_offer();
    offer.rewards.choice_items = vec![
        QuestRewardItem {
            item_id: 2055,
            name: "Small Wooden Hammer".into(),
            count: 1,
        },
        QuestRewardItem {
            item_id: 2057,
            name: "Pitted Defias Shortsword".into(),
            count: 1,
        },
    ];
    runtime.show_reward("Marshal McBride".into(), offer);

    runtime.choose_reward(5);
    runtime.choose_reward(1);
    let Some(QuestDialog {
        page: QuestDialogPage::Reward { choice, .. },
        ..
    }) = &runtime.dialog
    else {
        panic!("reward page open");
    };
    assert_eq!(*choice, Some(1));
}

#[test]
fn completing_a_quest_closes_the_reward_page_with_retail_lines() {
    let mut runtime = QuestRuntime::default();
    runtime.show_reward("Marshal McBride".into(), threat_within_offer());

    let notices = runtime.complete_quest(&QuestGiverQuestComplete {
        quest_id: 783,
        money: 25,
        items: vec![QuestRewardItem {
            item_id: 2055,
            name: "Small Wooden Hammer".into(),
            count: 1,
        }],
        xp: 40,
    });

    assert!(runtime.dialog.is_none());
    assert_eq!(
        notices,
        vec![
            "A Threat Within completed.".to_string(),
            "Experience gained: 40.".to_string(),
            "Received 25 Copper.".to_string(),
            "You receive item: [Small Wooden Hammer].".to_string(),
        ]
    );
}

#[test]
fn interaction_close_only_closes_that_npcs_dialog() {
    let mut runtime = QuestRuntime::default();
    runtime.show_reward("Marshal McBride".into(), threat_within_offer());
    assert!(!runtime.close_dialog_for(WILLEM));
    assert!(runtime.dialog.is_some());
    assert!(runtime.close_dialog_for(MCBRIDE));
    assert!(runtime.dialog.is_none());
}

#[test]
fn money_text_splits_gold_silver_copper() {
    assert_eq!(money_text(25), "25 Copper");
    assert_eq!(money_text(10_503), "1 Gold 5 Silver 3 Copper");
    assert_eq!(money_text(200), "2 Silver");
}

#[test]
fn quest_text_tokens_substitute_name_class_race_breaks_and_gender() {
    let tokens = QuestTextTokens {
        name: "Theron".into(),
        class: "Paladin".into(),
        race: "Human".into(),
        female: false,
    };
    let text = "I hope you strapped your belt on tight, young $c, because there is work.$B$BIt is a battle, $N. Well met, $Glad:lass;! A $R $r.";
    assert_eq!(
        substitute_quest_text(text, &tokens),
        "I hope you strapped your belt on tight, young paladin, because there is work.\n\nIt is a battle, Theron. Well met, lad! A Human human."
    );
    let female = QuestTextTokens {
        female: true,
        ..tokens
    };
    assert_eq!(substitute_quest_text("$Glad:lass;", &female), "lass");
    assert_eq!(substitute_quest_text("costs $5", &female), "costs $5");
}
