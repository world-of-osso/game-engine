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

#[test]
fn player_zone_comes_from_the_terrain_chunk_underfoot() {
    let data = std::fs::read("data/terrain/azeroth_32_48.adt").expect("azeroth_32_48.adt");
    let adt = crate::asset::adt::load_adt(&data).expect("parse ADT");
    let mut heightmap = TerrainHeightmap::default();
    heightmap.insert_tile(32, 48, &adt);
    // Northshire Abbey steps, the human starting position.
    let [bx, _, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
    let mut world = World::new();
    world.insert_resource(heightmap);
    world.insert_resource(CurrentZone::default());
    world.spawn((Player, Transform::from_xyz(bx, 0.0, bz)));

    world.run_system_once(track_player_zone).unwrap();

    let zone = world.resource::<CurrentZone>();
    assert_eq!(zone.zone_id, 12, "Elwynn Forest");
    let area = crate::zone_names::zone_id_to_name(zone.area_id);
    assert!(
        area.starts_with("Northshire"),
        "area {} = {area}",
        zone.area_id
    );
}

fn creature_line(channel: shared::protocol::ChatType, sender: &str, content: &str) -> String {
    let mut chat_log = ChatLog::default();
    let mut chat_state = default_chat_state();
    apply_incoming_chat_message(
        &ChatMessage {
            sender: sender.into(),
            content: content.into(),
            channel,
        },
        Some("Fixshout"),
        &IgnoreListStatusSnapshot::default(),
        &mut chat_log,
        &mut chat_state,
        &mut WhisperState::default(),
    );
    game_engine::ui::chat_frame::chat_message_line(&chat_state.messages[0])
        .plain_text(|_| String::new())
}

#[test]
fn stockade_creature_texts_read_as_retail_monster_chat_lines() {
    use shared::protocol::ChatType;
    assert_eq!(
        creature_line(ChatType::MonsterYell(7), "Hogger", "Forest just setback!"),
        "Hogger yells: Forest just setback!"
    );
    assert_eq!(
        creature_line(
            ChatType::MonsterSay(8),
            "Warden Thelwater",
            "He's...he's dead? "
        ),
        "Warden Thelwater says: He's...he's dead? "
    );
    // BroadcastText 46561 already names the speaker; creature_text's "%s" form works too.
    assert_eq!(
        creature_line(
            ChatType::MonsterEmote(9),
            "Mortimer Moloch",
            "Mortimer Moloch collapses from a heart attack!"
        ),
        "Mortimer Moloch collapses from a heart attack!"
    );
    assert_eq!(
        creature_line(ChatType::RaidBossEmote(7), "Hogger", "%s Enrages!"),
        "Hogger Enrages!"
    );
}

#[test]
fn a_raid_boss_emote_also_shows_center_screen() {
    use game_engine::network_runtime::messages::Inbox;
    let mut app = App::new();
    app.init_resource::<ChatLog>()
        .insert_resource(default_chat_state())
        .init_resource::<WhisperState>()
        .init_resource::<IgnoreListStatusSnapshot>()
        .init_resource::<game_engine::ui::raid_warning::RaidWarnings>()
        .insert_resource(Inbox::new(vec![ChatMessage {
            sender: "Hogger".into(),
            content: "Hogger enrages!".into(),
            channel: shared::protocol::ChatType::RaidBossEmote(7),
        }]));
    app.world_mut()
        .run_system_once(receive_chat_messages)
        .unwrap();
    let warnings = app
        .world()
        .resource::<game_engine::ui::raid_warning::RaidWarnings>();
    assert_eq!(warnings.lines.len(), 1);
    assert_eq!(warnings.lines[0].text, "Hogger enrages!");
    assert_eq!(warnings.lines[0].color, [1.0, 0.867, 0.0]);
}

#[test]
fn player_input_reports_where_the_local_movement_put_the_player() {
    use crate::camera::{MoveDirection, MovementState};
    let facing = CharacterFacing { yaw: 0.0 };
    let running_forward = MovementState {
        direction: MoveDirection::Forward,
        ..Default::default()
    };
    let at = Vec3::new(-8949.0, 112.88, 1.4);
    let input = player_input(&running_forward, &facing, at, 2).unwrap();
    assert_eq!(input.position, at.to_array());
    assert_eq!(input.epoch, 2);
    assert_eq!(input.direction, [0.0, 0.0, 1.0]);
    assert!(input.running);
    assert!(player_input(&MovementState::default(), &facing, at, 2).is_none());
}
