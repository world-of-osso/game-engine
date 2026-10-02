use bevy::ecs::system::RunSystemOnce;
use game_engine::network_runtime::messages::Inbox;
use game_engine::quest_runtime::QuestDialogPage;
use shared::protocol::{
    GossipMenu, QuestEntrySnapshot, QuestFailedReason, QuestGiverQuestEntry, QuestGiverQuestState,
    QuestGiverStatusEntry, QuestMarkerClass, QuestRepeatability, QuestRewards,
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
        .add_message::<NpcFrameEvent>()
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

fn frame_events(app: &mut App) -> Vec<NpcFrameEvent> {
    app.world_mut()
        .resource_mut::<Messages<NpcFrameEvent>>()
        .drain()
        .collect()
}

#[test]
fn auctioneer_role_picked_from_gossip_closes_the_greeting_and_opens_its_frame() {
    let Fixture { mut app, .. } = fixture();
    deliver(
        &mut app,
        vec![
            InteractionOpened {
                npc: WILLEM_SERVER,
                kind: InteractionKind::Gossip(GossipMenu {
                    menu_id: 0,
                    text: String::new(),
                    options: vec![],
                }),
            },
            InteractionOpened {
                npc: WILLEM_SERVER,
                kind: InteractionKind::Role(NpcRole::AuctionHouse),
            },
        ],
    );
    app.world_mut()
        .run_system_once(receive_quest_dialog)
        .unwrap();

    assert!(app.world().resource::<QuestRuntime>().dialog.is_none());
    assert_eq!(
        frame_events(&mut app),
        vec![NpcFrameEvent::Opened {
            npc: WILLEM_SERVER,
            role: NpcRole::AuctionHouse,
        }]
    );

    deliver(&mut app, vec![InteractionClosed { npc: WILLEM_SERVER }]);
    app.world_mut()
        .run_system_once(receive_quest_dialog)
        .unwrap();
    assert_eq!(
        frame_events(&mut app),
        vec![NpcFrameEvent::Closed { npc: WILLEM_SERVER }]
    );
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
                status: QuestGiverStatus::Available(QuestMarkerClass::Normal),
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
                status: QuestGiverStatus::Incomplete(QuestMarkerClass::Normal),
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
        quest_indicator(QuestGiverStatus::Trivial(QuestMarkerClass::Normal)),
        QuestIndicator::None
    );
    assert_eq!(
        quest_indicator(QuestGiverStatus::Reward(QuestMarkerClass::Normal)),
        QuestIndicator::TurnIn
    );
    assert_eq!(
        quest_indicator(QuestGiverStatus::Future(QuestMarkerClass::Normal)),
        QuestIndicator::Unavailable
    );
}

#[test]
fn chain_offer_in_the_same_batch_as_the_turn_in_stays_open() {
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

    // Marshal McBride turns in 783 and offers 7 in one network batch.
    deliver(
        &mut app,
        vec![QuestGiverQuestComplete {
            quest_id: 783,
            money: 0,
            items: vec![],
            xp: 100,
        }],
    );
    deliver(
        &mut app,
        vec![QuestGiverQuestDetails {
            npc: WILLEM_SERVER,
            quest_id: 7,
            title: "Kobold Camp Cleanup".into(),
            description: String::new(),
            objectives_text: "Kill 8 Kobold Vermin, then return to Marshal McBride.".into(),
            level: 2,
            min_level: 1,
            suggested_group: 0,
            objectives: vec![],
            rewards: QuestRewards::default(),
        }],
    );
    app.world_mut()
        .run_system_once(receive_quest_dialog)
        .unwrap();

    let dialog = app.world().resource::<QuestRuntime>().dialog.clone();
    assert!(
        matches!(dialog.map(|d| d.page), Some(QuestDialogPage::Detail(details)) if details.quest_id == 7)
    );
    assert_eq!(
        chat_lines(&app),
        vec![
            "A Threat Within completed.".to_string(),
            "Experience gained: 100.".to_string()
        ]
    );
}

#[test]
fn using_a_mirrored_game_object_sends_use_game_object_and_its_role_opens_a_frame() {
    use game_engine::network_runtime::messages::ConnectionSender;
    use game_engine::network_runtime::worker::NetworkCommand;
    const VAULT_SERVER: u64 = 0x0000_0001_0000_1D01;
    let Fixture { mut app, .. } = fixture();
    let (sender, commands) = std::sync::mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    let vault = app.world_mut().spawn_empty().id();
    let unmapped = app.world_mut().spawn_empty().id();
    app.world_mut()
        .resource_mut::<ReplicationMirrorMap>()
        .insert(Entity::from_bits(VAULT_SERVER), vault);
    app.world_mut()
        .write_message(NpcInteractionRequest::UseObject(unmapped));
    app.world_mut()
        .write_message(NpcInteractionRequest::UseObject(vault));
    app.world_mut()
        .run_system_once(send_interaction_requests)
        .unwrap();
    assert!(matches!(commands.try_recv(), Ok(NetworkCommand::Apply(_))));
    assert!(commands.try_recv().is_err());

    deliver(
        &mut app,
        vec![InteractionOpened {
            npc: VAULT_SERVER,
            kind: InteractionKind::Role(NpcRole::GuildBanker),
        }],
    );
    app.world_mut()
        .run_system_once(receive_quest_dialog)
        .unwrap();
    assert_eq!(
        frame_events(&mut app),
        vec![NpcFrameEvent::Opened {
            npc: VAULT_SERVER,
            role: NpcRole::GuildBanker,
        }]
    );
}
