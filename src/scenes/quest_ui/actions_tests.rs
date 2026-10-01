use bevy::ecs::system::RunSystemOnce;
use game_engine::quest_runtime::QuestDialog;
use shared::protocol::{
    GossipMenu, QuestEntrySnapshot, QuestGiverOfferReward, QuestGiverQuestDetails,
    QuestGiverQuestEntry, QuestGiverQuestState, QuestLogSnapshot, QuestRepeatability,
    QuestRewardItem, QuestRewards,
};

use super::*;

const WILLEM: u64 = 823;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<PopupResult>()
        .add_message::<NpcInteractionRequest>()
        .init_resource::<QuestRuntime>()
        .init_resource::<QuestUiState>()
        .init_resource::<WindowManager>()
        .init_resource::<PopupStack>()
        .init_resource::<PendingAbandon>()
        .init_resource::<UiErrors>();
    app
}

fn dispatch(app: &mut App, action: &'static str) -> Vec<NpcInteractionRequest> {
    app.world_mut()
        .run_system_once(move |mut context: QuestUiContext| dispatch_action(action, &mut context))
        .unwrap()
}

fn runtime(app: &mut App) -> Mut<'_, QuestRuntime> {
    app.world_mut().resource_mut::<QuestRuntime>()
}

fn threat_within() -> QuestEntrySnapshot {
    QuestEntrySnapshot {
        quest_id: 783,
        title: "A Threat Within".into(),
        zone: String::new(),
        completed: true,
        repeatability: QuestRepeatability::Normal,
        objectives: vec![],
        level: 1,
        sort_id: 9,
        objectives_text: "Speak with Marshal McBride.".into(),
        completion_text: String::new(),
        watched: true,
        pois: vec![],
    }
}

fn greeting(app: &mut App) {
    let mut runtime = runtime(app);
    runtime.open_gossip(
        WILLEM,
        "Deputy Willem".into(),
        GossipMenu {
            menu_id: 57020,
            text: "Greetings.".into(),
            options: vec![],
        },
    );
    runtime.apply_quest_list(shared::protocol::QuestGiverQuestList {
        npc: WILLEM,
        quests: vec![
            QuestGiverQuestEntry {
                quest_id: 783,
                title: "A Threat Within".into(),
                level: 1,
                state: QuestGiverQuestState::Available,
            },
            QuestGiverQuestEntry {
                quest_id: 18,
                title: "Brotherhood of Thieves".into(),
                level: 4,
                state: QuestGiverQuestState::Incomplete,
            },
        ],
    });
}

#[test]
fn greeting_quests_query_details_or_start_the_turn_in() {
    let mut app = app();
    greeting(&mut app);
    assert_eq!(
        dispatch(&mut app, "quest_frame:quest:0"),
        vec![NpcInteractionRequest::QueryQuest {
            npc: WILLEM,
            quest_id: 783
        }]
    );
    assert_eq!(
        dispatch(&mut app, "quest_frame:quest:1"),
        vec![NpcInteractionRequest::Complete {
            npc: WILLEM,
            quest_id: 18
        }]
    );
}

#[test]
fn accept_sends_the_quest_and_closes_the_frame() {
    let mut app = app();
    runtime(&mut app).show_details(
        "Deputy Willem".into(),
        QuestGiverQuestDetails {
            npc: WILLEM,
            quest_id: 783,
            title: "A Threat Within".into(),
            description: String::new(),
            objectives_text: String::new(),
            level: 1,
            min_level: 1,
            suggested_group: 0,
            objectives: vec![],
            rewards: QuestRewards::default(),
        },
    );
    assert_eq!(
        dispatch(&mut app, "quest_frame:accept"),
        vec![
            NpcInteractionRequest::Accept {
                npc: WILLEM,
                quest_id: 783
            },
            NpcInteractionRequest::Close { npc: WILLEM }
        ]
    );
    assert!(runtime(&mut app).dialog.is_none());
}

#[test]
fn complete_quest_waits_for_a_choice_then_sends_it() {
    let mut app = app();
    runtime(&mut app).show_reward(
        "Deputy Willem".into(),
        QuestGiverOfferReward {
            npc: WILLEM,
            quest_id: 18,
            title: "Brotherhood of Thieves".into(),
            reward_text: String::new(),
            rewards: QuestRewards {
                choice_items: vec![
                    QuestRewardItem {
                        item_id: 2224,
                        name: "Militia Dagger".into(),
                        count: 1,
                    },
                    QuestRewardItem {
                        item_id: 5580,
                        name: "Militia Hammer".into(),
                        count: 1,
                    },
                ],
                ..QuestRewards::default()
            },
        },
    );
    assert!(dispatch(&mut app, "quest_frame:complete").is_empty());
    assert_eq!(
        app.world().resource::<UiErrors>().lines[0].text,
        "You must choose a reward."
    );
    assert!(dispatch(&mut app, "quest_frame:choice:1").is_empty());
    assert_eq!(
        dispatch(&mut app, "quest_frame:complete"),
        vec![NpcInteractionRequest::ChooseReward {
            npc: WILLEM,
            quest_id: 18,
            choice: Some(1)
        }]
    );
    let dialog: Option<QuestDialog> = runtime(&mut app).dialog.clone();
    assert!(
        dialog.is_some(),
        "frame stays until QuestGiverQuestComplete"
    );
}

#[test]
fn abandon_asks_for_confirmation_and_only_accept_sends() {
    let mut app = app();
    runtime(&mut app).apply_snapshot(QuestLogSnapshot {
        entries: vec![threat_within()],
        watched_quest_ids: vec![783],
    });
    assert!(dispatch(&mut app, "quest_log:abandon").is_empty());
    let visible = app.world().resource::<PopupStack>().visible();
    assert_eq!(visible[0].spec.key, ABANDON_QUEST_POPUP);
    assert_eq!(visible[0].spec.text, "Abandon \"A Threat Within\"?");

    let id = visible[0].id;
    let handler = app.world_mut().register_system(handle_abandon_popup);
    app.world_mut().write_message(PopupResult {
        id,
        key: ABANDON_QUEST_POPUP.into(),
        outcome: PopupOutcome::Cancelled,
    });
    app.world_mut().run_system(handler).unwrap();
    assert!(drain(&mut app).is_empty());

    dispatch(&mut app, "quest_log:abandon");
    app.world_mut().write_message(PopupResult {
        id,
        key: ABANDON_QUEST_POPUP.into(),
        outcome: PopupOutcome::Accepted,
    });
    app.world_mut().run_system(handler).unwrap();
    assert_eq!(
        drain(&mut app),
        vec![NpcInteractionRequest::Abandon { quest_id: 783 }]
    );
}

fn drain(app: &mut App) -> Vec<NpcInteractionRequest> {
    app.world_mut()
        .resource_mut::<Messages<NpcInteractionRequest>>()
        .drain()
        .collect()
}

#[test]
fn track_toggles_the_selected_quest_and_tracker_click_opens_the_log() {
    let mut app = app();
    runtime(&mut app).apply_snapshot(QuestLogSnapshot {
        entries: vec![threat_within()],
        watched_quest_ids: vec![783],
    });
    assert_eq!(
        dispatch(&mut app, "quest_log:track"),
        vec![NpcInteractionRequest::SetWatched {
            quest_id: 783,
            watched: false
        }]
    );

    dispatch(&mut app, "quest_tracker:open:783");
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::QuestLog)
    );
    assert_eq!(
        app.world().resource::<QuestUiState>().log_selected,
        Some(783)
    );

    dispatch(&mut app, "quest_tracker:toggle");
    assert!(app.world().resource::<QuestUiState>().tracker_collapsed);
    dispatch(&mut app, "quest_log:header:9");
    assert!(
        app.world()
            .resource::<QuestUiState>()
            .collapsed_headers
            .contains(&9)
    );
}
