use super::*;
use crate::terrain_heightmap::TerrainHeightmap;
use bevy::ecs::system::RunSystemOnce;

fn whisper_message(sender: &str, target: &str, content: &str) -> ChatMessage {
    ChatMessage {
        sender: sender.into(),
        content: content.into(),
        channel: shared::protocol::ChatType::Whisper(target.into()),
    }
}

fn default_chat_state() -> ChatState {
    ChatState {
        max_messages: MAX_CHAT_LOG,
        ..Default::default()
    }
}

#[test]
fn map_runtime_chat_channel_maps_emotes() {
    let (channel, name) =
        map_runtime_chat_channel(&shared::protocol::ChatType::Emote, "Alice", Some("Theron"));
    assert_eq!(channel, ChatChannelType::Emote);
    assert!(name.is_empty());
}

#[test]
fn resolve_emote_visual_entity_prefers_mounted_visual_child() {
    let mut app = App::new();
    let parent = app.world_mut().spawn_empty().id();
    let mounted_child = app
        .world_mut()
        .spawn(crate::networking_player::MountedVisualRoot)
        .id();
    let other_child = app.world_mut().spawn_empty().id();
    app.world_mut()
        .entity_mut(parent)
        .add_children(&[other_child, mounted_child]);

    let entity = app
        .world_mut()
        .run_system_once(
            move |children_query: Query<&Children>,
                  mounted_visual_roots: Query<
                (),
                With<crate::networking_player::MountedVisualRoot>,
            >,
                  existing_entities: Query<(), ()>| {
                resolve_emote_visual_entity(
                    parent.to_bits(),
                    &children_query,
                    &mounted_visual_roots,
                    &existing_entities,
                )
            },
        )
        .expect("resolve emote entity");
    assert_eq!(entity, Some(mounted_child));
}

#[test]
fn resolve_emote_visual_entity_ignores_unknown_entity() {
    let mut app = App::new();
    let entity = app
        .world_mut()
        .run_system_once(
            move |children_query: Query<&Children>,
                  mounted_visual_roots: Query<
                (),
                With<crate::networking_player::MountedVisualRoot>,
            >,
                  existing_entities: Query<(), ()>| {
                resolve_emote_visual_entity(
                    Entity::from_bits(999_999).to_bits(),
                    &children_query,
                    &mounted_visual_roots,
                    &existing_entities,
                )
            },
        )
        .expect("resolve emote entity");
    assert_eq!(entity, None);
}

#[test]
fn outgoing_whisper_updates_recent_targets() {
    let mut whisper_state = WhisperState {
        max_recent: 10,
        ..Default::default()
    };

    apply_outgoing_chat_message(
        &whisper_message("Theron", "Alice", "hey"),
        &mut whisper_state,
    );

    assert_eq!(whisper_state.reply_target, None);
    assert_eq!(whisper_state.recent_targets, vec!["Alice"]);
}

#[test]
fn map_change_load_terrain_enters_loading_and_reseeds_map() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<AdtManager>();
    app.init_resource::<TerrainHeightmap>();
    app.init_resource::<NextState<GameState>>();
    {
        let mut adt_manager = app.world_mut().resource_mut::<AdtManager>();
        adt_manager.map_name = "azeroth".into();
        adt_manager.initial_tile = (32, 48);
    }

    app.world_mut()
        .run_system_once(
            |mut commands: Commands,
             mut adt_manager: ResMut<AdtManager>,
             mut heightmap: ResMut<TerrainHeightmap>,
             mut next_state: ResMut<NextState<GameState>>| {
                apply_load_terrain_message(
                    &mut commands,
                    &mut adt_manager,
                    &mut heightmap,
                    None,
                    &mut next_state,
                    LoadTerrain {
                        map_name: "kalimdor".into(),
                        initial_tile_y: 20,
                        initial_tile_x: 21,
                    },
                );
            },
        )
        .expect("apply load terrain");

    let adt_manager = app.world().resource::<AdtManager>();
    assert_eq!(adt_manager.map_name, "kalimdor");
    assert_eq!(adt_manager.initial_tile, (20, 21));
    assert!(adt_manager.server_requested.contains(&(20, 21)));
    assert!(matches!(
        app.world().resource::<NextState<GameState>>(),
        NextState::Pending(GameState::Loading)
    ));
}

#[test]
fn incoming_whisper_sets_reply_target_and_runtime_message() {
    let mut chat_log = ChatLog::default();
    let mut chat_state = default_chat_state();
    let mut whisper_state = WhisperState {
        max_recent: 10,
        ..Default::default()
    };

    apply_incoming_chat_message(
        &whisper_message("Alice", "Theron", "psst"),
        Some("Theron"),
        &IgnoreListStatusSnapshot::default(),
        &mut chat_log,
        &mut chat_state,
        &mut whisper_state,
    );

    assert_eq!(chat_log.messages.len(), 1);
    assert_eq!(whisper_state.reply_target.as_deref(), Some("Alice"));
    assert_eq!(whisper_state.recent_targets, vec!["Alice"]);
    assert_eq!(chat_state.messages.len(), 1);
    assert_eq!(
        chat_state.messages[0].channel_type,
        ChatChannelType::Whisper
    );
    assert_eq!(chat_state.messages[0].channel_name, "");
}

#[test]
fn outgoing_whisper_message_tracks_recipient_without_reply_target() {
    let mut chat_log = ChatLog::default();
    let mut chat_state = default_chat_state();
    let mut whisper_state = WhisperState {
        max_recent: 10,
        ..Default::default()
    };

    apply_incoming_chat_message(
        &whisper_message("Theron", "Alice", "hello"),
        Some("Theron"),
        &IgnoreListStatusSnapshot::default(),
        &mut chat_log,
        &mut chat_state,
        &mut whisper_state,
    );

    assert_eq!(whisper_state.reply_target, None);
    assert_eq!(whisper_state.recent_targets, vec!["Alice"]);
    assert_eq!(chat_state.messages[0].channel_name, "Alice");
}

#[test]
fn ignored_sender_message_is_not_added_to_chat_log() {
    let mut chat_log = ChatLog::default();
    let mut chat_state = default_chat_state();
    let mut whisper_state = WhisperState {
        max_recent: 10,
        ..Default::default()
    };
    let ignore_list = IgnoreListStatusSnapshot {
        names: vec!["Alice".into()],
        ..Default::default()
    };

    apply_incoming_chat_message(
        &whisper_message("Alice", "Theron", "psst"),
        Some("Theron"),
        &ignore_list,
        &mut chat_log,
        &mut chat_state,
        &mut whisper_state,
    );

    assert!(chat_log.messages.is_empty());
    assert!(chat_state.messages.is_empty());
    assert_eq!(whisper_state.reply_target, None);
}
