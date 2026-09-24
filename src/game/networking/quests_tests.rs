use bevy::ecs::system::RunSystemOnce;
use game_engine::network_runtime::messages::Inbox;
use game_engine::quest_runtime::QuestDialogPage;
use shared::protocol::{
    GossipMenu, QuestEntrySnapshot, QuestFailedReason, QuestGiverQuestEntry, QuestGiverQuestState,
    QuestGiverStatusEntry, QuestRepeatability, QuestRewards,
};

use super::*;

/// Server entity bits of Deputy Willem in these fixtures.
const WILLEM_SERVER: u64 = 0x0000_0001_0000_0337;

struct Fixture {
    app: App,
    willem: Entity,
}

fn fixture() -> Fixture {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<NpcInteractionRequest>()
        .init_resource::<QuestRuntime>()
        .init_resource::<QuestLogStatusSnapshot>()
        .init_resource::<ChatState>()
        .init_resource::<UiErrors>()
        .init_resource::<ReplicationMirrorMap>();
    app.init_resource::<Inbox<QuestLogSnapshot>>()
        .init_resource::<Inbox<QuestLogUpdate>>()
        .init_resource::<Inbox<QuestGiverStatusMultiple>>()
        .init_resource::<Inbox<InteractionOpened>>()
        .init_resource::<Inbox<InteractionFailed>>()
        .init_resource::<Inbox<InteractionClosed>>()
        .init_resource::<Inbox<QuestGiverQuestList>>()
        .init_resource::<Inbox<QuestGiverQuestDetails>>()
        .init_resource::<Inbox<QuestGiverRequestItems>>()
        .init_resource::<Inbox<QuestGiverOfferReward>>()
        .init_resource::<Inbox<QuestGiverQuestComplete>>()
        .init_resource::<Inbox<QuestFailed>>();
    let willem = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 823,
                name: "Deputy Willem".into(),
            },
            NpcFlags(NpcFlags::GOSSIP | NpcFlags::QUESTGIVER),
        ))
        .id();
    app.world_mut()
        .resource_mut::<ReplicationMirrorMap>()
        .insert(Entity::from_bits(WILLEM_SERVER), willem);
    Fixture { app, willem }
}

fn deliver<M: lightyear::prelude::Message>(app: &mut App, messages: Vec<M>) {
    app.world_mut().insert_resource(Inbox::new(messages));
}

fn requests(app: &mut App) -> Vec<NpcInteractionRequest> {
    app.world_mut()
        .resource_mut::<Messages<NpcInteractionRequest>>()
        .drain()
        .collect()
}

fn chat_lines(app: &App) -> Vec<String> {
    app.world()
        .resource::<ChatState>()
        .messages
        .iter()
        .filter(|message| message.channel_type == ChatChannelType::System)
        .map(|message| message.text.clone())
        .collect()
}

#[test]
fn gossip_from_a_quest_giver_opens_the_greeting_and_asks_for_its_quests() {
    let Fixture { mut app, .. } = fixture();
    deliver(
        &mut app,
        vec![InteractionOpened {
            npc: WILLEM_SERVER,
            kind: InteractionKind::Gossip(GossipMenu {
                menu_id: 57020,
                text: "Greetings, $n.".into(),
                options: vec![],
            }),
        }],
    );
    app.world_mut()
        .run_system_once(receive_quest_dialog)
        .unwrap();

    let dialog = app
        .world()
        .resource::<QuestRuntime>()
        .dialog
        .clone()
        .unwrap();
    assert_eq!(dialog.npc_name, "Deputy Willem");
    assert!(matches!(dialog.page, QuestDialogPage::Greeting { .. }));
    assert_eq!(
        requests(&mut app),
        vec![NpcInteractionRequest::Hello { npc: WILLEM_SERVER }]
    );

    deliver(
        &mut app,
        vec![QuestGiverQuestList {
            npc: WILLEM_SERVER,
            quests: vec![QuestGiverQuestEntry {
                quest_id: 783,
                title: "A Threat Within".into(),
                level: 1,
                state: QuestGiverQuestState::Available,
            }],
        }],
    );
    app.world_mut()
        .run_system_once(receive_quest_dialog)
        .unwrap();
    let Some(QuestDialogPage::Greeting { quests, .. }) = app
        .world()
        .resource::<QuestRuntime>()
        .dialog
        .as_ref()
        .map(|dialog| dialog.page.clone())
    else {
        panic!("greeting open");
    };
    assert_eq!(quests[0].quest_id, 783);
}

#[test]
fn turn_in_posts_completion_and_experience_lines_and_closes_the_frame() {
    let Fixture { mut app, .. } = fixture();
    deliver(
        &mut app,
        vec![QuestGiverOfferReward {
            npc: WILLEM_SERVER,
            quest_id: 783,
            title: "A Threat Within".into(),
            reward_text: "Ah, good.".into(),
            rewards: QuestRewards::default(),
        }],
    );
    app.world_mut()
        .run_system_once(receive_quest_dialog)
        .unwrap();
    assert!(app.world().resource::<QuestRuntime>().dialog.is_some());

    deliver(
        &mut app,
        vec![QuestGiverQuestComplete {
            quest_id: 783,
            money: 0,
            items: vec![],
            xp: 40,
        }],
    );
    app.world_mut()
        .run_system_once(receive_quest_dialog)
        .unwrap();

    assert!(app.world().resource::<QuestRuntime>().dialog.is_none());
    assert_eq!(
        chat_lines(&app),
        vec![
            "A Threat Within completed.".to_string(),
            "Experience gained: 40.".to_string()
        ]
    );
}

#[test]
fn rejections_show_retail_error_text() {
    let Fixture { mut app, .. } = fixture();
    deliver(
        &mut app,
        vec![QuestFailed {
            quest_id: 7,
            reason: QuestFailedReason::QuestLogFull,
        }],
    );
    deliver(
        &mut app,
        vec![InteractionFailed {
            npc: WILLEM_SERVER,
            error: shared::protocol::InteractionError::TooFarAway,
        }],
    );
    app.world_mut()
        .run_system_once(receive_quest_dialog)
        .unwrap();
    let texts: Vec<String> = app
        .world()
        .resource::<UiErrors>()
        .lines
        .iter()
        .map(|line| line.text.clone())
        .collect();
    assert!(
        texts.contains(&"Your quest log is full.".to_string()),
        "{texts:?}"
    );
    assert!(
        texts.contains(&"You are too far away.".to_string()),
        "{texts:?}"
    );
}

#[test]
fn log_snapshot_and_update_feed_runtime_and_ipc_status() {
    let Fixture { mut app, .. } = fixture();
    let entry = QuestEntrySnapshot {
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
    };
    deliver(
        &mut app,
        vec![QuestLogUpdate {
            changed: vec![entry],
            removed: vec![],
            watched_quest_ids: vec![783],
        }],
    );
    app.world_mut().run_system_once(receive_quest_log).unwrap();

    let status = app.world().resource::<QuestLogStatusSnapshot>();
    assert_eq!(status.entries[0].title, "A Threat Within");
    assert_eq!(status.watched_quest_ids, vec![783]);
    assert_eq!(
        chat_lines(&app),
        vec!["Quest accepted: A Threat Within".to_string()]
    );
}

#[test]
fn quest_giver_markers_follow_the_server_status() {
    let Fixture { mut app, willem } = fixture();
    deliver(
        &mut app,
        vec![QuestGiverStatusMultiple {
            statuses: vec![QuestGiverStatusEntry {
                npc: WILLEM_SERVER,
                status: QuestGiverStatus::Available,
            }],
        }],
    );
    app.world_mut().run_system_once(receive_quest_log).unwrap();
    app.world_mut()
        .run_system_once(sync_quest_indicators)
        .unwrap();
    assert_eq!(
        app.world().get::<NpcQuestIndicator>(willem).map(|qi| qi.0),
        Some(QuestIndicator::Available)
    );

    deliver(
        &mut app,
        vec![QuestGiverStatusMultiple {
            statuses: vec![QuestGiverStatusEntry {
                npc: WILLEM_SERVER,
                status: QuestGiverStatus::Incomplete,
            }],
        }],
    );
    app.world_mut().run_system_once(receive_quest_log).unwrap();
    app.world_mut()
        .run_system_once(sync_quest_indicators)
        .unwrap();
    assert_eq!(
        app.world().get::<NpcQuestIndicator>(willem).map(|qi| qi.0),
        Some(QuestIndicator::Incomplete)
    );
}

#[test]
fn trivial_quests_show_no_marker_like_retail_default_tracking() {
    assert_eq!(
        quest_indicator(QuestGiverStatus::LowLevelAvailable),
        QuestIndicator::None
    );
    assert_eq!(
        quest_indicator(QuestGiverStatus::Reward),
        QuestIndicator::TurnIn
    );
    assert_eq!(
        quest_indicator(QuestGiverStatus::Unavailable),
        QuestIndicator::Unavailable
    );
}
