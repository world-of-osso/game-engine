use crate::quest_runtime::QuestDialog;
use shared::protocol::{
    GossipMenuOption, QuestGiverOfferReward, QuestGiverQuestEntry, QuestLogSnapshot,
    QuestObjectiveKind, QuestObjectiveSnapshot, QuestRepeatability,
};

use super::*;

fn theron() -> QuestTextTokens {
    QuestTextTokens {
        name: "Theron".into(),
        class: "Paladin".into(),
        race: "Human".into(),
        female: false,
    }
}

fn entry(quest_id: u32, title: &str, sort_id: i32) -> QuestEntrySnapshot {
    QuestEntrySnapshot {
        quest_id,
        title: title.into(),
        zone: String::new(),
        completed: false,
        repeatability: QuestRepeatability::Normal,
        objectives: vec![QuestObjectiveSnapshot {
            text: "Kobold Vermin slain".into(),
            current: 3,
            required: 8,
            completed: false,
            kind: QuestObjectiveKind::Monster,
            object_id: 6,
        }],
        level: 2,
        sort_id,
        objectives_text: "Kill 8 Kobold Vermin, then return to Marshal McBride.".into(),
        completion_text: String::new(),
        watched: true,
        pois: vec![],
    }
}

fn threat_within_details() -> QuestGiverQuestDetails {
    QuestGiverQuestDetails {
        npc: 823,
        quest_id: 783,
        title: "A Threat Within".into(),
        description:
            "I hope you strapped your belt on tight, young $c.$B$BSpeak with my superior, $N."
                .into(),
        objectives_text: "Speak with Marshal McBride.".into(),
        level: 1,
        min_level: 1,
        suggested_group: 0,
        objectives: vec![],
        rewards: QuestRewards::default(),
    }
}

#[test]
fn uifixes_npc_articles_and_player_tokens_resolve_for_a_human_paladin() {
    use shared::protocol::QuestGiverRequestItems;
    let raw = "Hey, $Gmister:miss;. You look like |5$C, $N.$B$B|5^ $R $r $c. Bring |5 apple and |5 banana.";
    let expected = "Hey, mister. You look like a Paladin, Theron.\n\nA Human human paladin. Bring an apple and a banana.";
    let mut details = threat_within_details();
    details.description = raw.into();
    details.objectives_text = raw.into();
    let pages = [
        QuestDialogPage::Greeting {
            text: raw.into(),
            options: vec![GossipMenuOption {
                option_id: 0,
                icon: 0,
                text: raw.into(),
            }],
            quests: vec![],
        },
        QuestDialogPage::Detail(details),
        QuestDialogPage::Progress(QuestGiverRequestItems {
            npc: 823,
            quest_id: 332,
            title: "Wine Shop Advert".into(),
            completion_text: raw.into(),
            required_items: vec![],
            can_complete: true,
        }),
        QuestDialogPage::Reward {
            offer: QuestGiverOfferReward {
                npc: 823,
                quest_id: 332,
                title: "Wine Shop Advert".into(),
                reward_text: raw.into(),
                rewards: QuestRewards::default(),
            },
            choice: None,
        },
    ];
    for page in pages {
        let dialog = QuestDialog {
            npc: 823,
            npc_name: "Renato Gallina".into(),
            page,
        };
        match quest_frame_state(Some(&dialog), &theron()).page {
            QuestFramePage::Greeting { text, options, .. } => {
                assert_eq!(text, expected);
                assert_eq!(options[0].text, expected);
            }
            QuestFramePage::Detail {
                description,
                objectives_text,
                ..
            } => {
                assert_eq!(description, expected);
                assert_eq!(objectives_text, expected);
            }
            QuestFramePage::Progress { text, .. } | QuestFramePage::Reward { text, .. } => {
                assert_eq!(text, expected)
            }
        }
    }
    assert_eq!(
        substitute_quest_text("|5Orc, |5Human, ||5 costs $5", &theron()),
        "an Orc, a Human, ||5 costs $5"
    );
}

#[test]
fn detail_page_substitutes_tokens() {
    let dialog = QuestDialog {
        npc: 823,
        npc_name: "Deputy Willem".into(),
        page: QuestDialogPage::Detail(threat_within_details()),
    };
    let state = quest_frame_state(Some(&dialog), &theron());
    assert!(state.visible);
    assert_eq!(state.npc_name, "Deputy Willem");
    let QuestFramePage::Detail { description, .. } = state.page else {
        panic!("detail page");
    };
    assert_eq!(
        description,
        "I hope you strapped your belt on tight, young paladin.\n\nSpeak with my superior, Theron."
    );
}

#[test]
fn greeting_maps_quest_states_and_options() {
    let dialog = QuestDialog {
        npc: 197,
        npc_name: "Marshal McBride".into(),
        page: QuestDialogPage::Greeting {
            text: "Greetings, $n.".into(),
            options: vec![GossipMenuOption {
                option_id: 0,
                icon: 0,
                text: "Tell me about Northshire.".into(),
            }],
            quests: vec![
                QuestGiverQuestEntry {
                    quest_id: 783,
                    title: "A Threat Within".into(),
                    level: 1,
                    state: QuestGiverQuestState::Complete,
                },
                QuestGiverQuestEntry {
                    quest_id: 7,
                    title: "Kobold Camp Cleanup".into(),
                    level: 2,
                    state: QuestGiverQuestState::LowLevelAvailable,
                },
            ],
        },
    };
    let QuestFramePage::Greeting {
        text,
        options,
        quests,
    } = quest_frame_state(Some(&dialog), &theron()).page
    else {
        panic!("greeting");
    };
    assert_eq!(text, "Greetings, Theron.");
    assert_eq!(options[0].text, "Tell me about Northshire.");
    assert_eq!(quests[0].kind, GreetingQuestKind::Complete);
    assert_eq!(quests[1].kind, GreetingQuestKind::Available);
    assert_eq!(quests[1].index, 1);
}

#[test]
fn reward_page_carries_the_selected_choice_and_resolved_icons() {
    let mut rewards = QuestRewards {
        money: 25,
        ..QuestRewards::default()
    };
    rewards.choice_items.push(QuestRewardItem {
        item_id: 5580,
        name: "Brotherhood reward".into(),
        count: 1,
    });
    let dialog = QuestDialog {
        npc: 823,
        npc_name: "Deputy Willem".into(),
        page: QuestDialogPage::Reward {
            offer: QuestGiverOfferReward {
                npc: 823,
                quest_id: 18,
                title: "Brotherhood of Thieves".into(),
                reward_text: "Well done, $N.".into(),
                rewards,
            },
            choice: Some(0),
        },
    };
    let QuestFramePage::Reward { text, rewards, .. } =
        quest_frame_state(Some(&dialog), &theron()).page
    else {
        panic!("reward");
    };
    assert_eq!(text, "Well done, Theron.");
    assert_eq!(rewards.selected_choice, Some(0));
    assert_eq!(rewards.money, 25);
    assert_eq!(rewards.choices[0].icon_fdid, Some(133_057));
}

#[test]
fn log_groups_by_sort_id_and_uses_cached_giver_text() {
    let mut runtime = QuestRuntime::default();
    runtime.apply_snapshot(QuestLogSnapshot {
        entries: vec![
            entry(783, "A Threat Within", 9),
            entry(7, "Kobold Camp Cleanup", 9),
            entry(60, "Kobold Candles", 87),
        ],
        watched_quest_ids: vec![7],
    });
    let mut cache = QuestDetailsCache::new();
    cache.insert(783, threat_within_details());
    let ui = QuestUiState::default();
    let mut names = |sort_id: i32| match sort_id {
        9 => "Northshire Valley".to_string(),
        other => format!("Area {other}"),
    };

    let state = quest_log_state(&runtime, &ui, &cache, &theron(), &mut names, 5, true);

    assert_eq!(state.quest_count, 3);
    assert_eq!(state.max_quests, 35);
    assert_eq!(state.groups.len(), 2);
    assert_eq!(state.groups[0].name, "Northshire Valley");
    assert_eq!(
        state.groups[0]
            .quests
            .iter()
            .map(|q| q.quest_id)
            .collect::<Vec<_>>(),
        vec![783, 7]
    );
    assert!(state.groups[0].quests[1].watched);
    let details = state.details.expect("first quest selected by default");
    assert_eq!(details.quest_id, 783);
    assert!(details.description.unwrap().contains("young paladin"));
    assert_eq!(details.objectives[0].text, "3/8 Kobold Vermin slain");

    let selected = QuestUiState {
        log_selected: Some(60),
        ..QuestUiState::default()
    };
    let state = quest_log_state(&runtime, &selected, &cache, &theron(), &mut names, 5, true);
    let details = state.details.unwrap();
    assert_eq!(details.quest_id, 60);
    assert_eq!(details.description, None, "no giver text seen this session");
}

/// `GetRelativeDifficultyColor` bands for a level 12 player: a scaling quest (-1)
/// shows the player's level in the difficult colour.
#[test]
fn log_rows_carry_level_and_relative_difficulty() {
    let levelled = |quest_id, level| QuestEntrySnapshot {
        level,
        ..entry(quest_id, "Quest", 12)
    };
    let mut runtime = QuestRuntime::default();
    runtime.apply_snapshot(QuestLogSnapshot {
        entries: vec![
            levelled(1, 17),
            levelled(2, 15),
            levelled(3, 8),
            levelled(4, 7),
            levelled(5, -1),
        ],
        watched_quest_ids: vec![],
    });
    let state = quest_log_state(
        &runtime,
        &QuestUiState::default(),
        &QuestDetailsCache::new(),
        &theron(),
        &mut |_| "Elwynn Forest".into(),
        12,
        true,
    );
    let rows: Vec<_> = state.groups[0]
        .quests
        .iter()
        .map(|row| (row.level, row.difficulty))
        .collect();
    assert_eq!(
        rows,
        vec![
            (17, QuestDifficulty::Impossible),
            (15, QuestDifficulty::VeryDifficult),
            (8, QuestDifficulty::Difficult),
            (7, QuestDifficulty::Trivial),
            (12, QuestDifficulty::Difficult),
        ]
    );
}
