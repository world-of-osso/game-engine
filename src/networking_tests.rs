use super::*;
use bevy::ecs::system::RunSystemOnce;
use std::f32::consts::{FRAC_PI_2, PI};

use shared::components::CharacterAppearance;

fn make_state(direction: MoveDirection) -> MovementState {
    MovementState {
        direction,
        ..Default::default()
    }
}

fn make_facing(yaw: f32) -> CharacterFacing {
    CharacterFacing { yaw }
}

#[test]
fn idle_produces_zero_direction() {
    let dir = movement_to_direction(&make_state(MoveDirection::None), &make_facing(0.0));
    assert_eq!(dir, [0.0, 0.0, 0.0]);
}

#[test]
fn forward_at_zero_yaw() {
    let dir = movement_to_direction(&make_state(MoveDirection::Forward), &make_facing(0.0));
    // yaw=0: forward = [sin(0), 0, cos(0)] = [0, 0, 1]
    assert!(dir[0].abs() < 1e-6);
    assert_eq!(dir[1], 0.0);
    assert!((dir[2] - 1.0).abs() < 1e-6);
}

#[test]
fn forward_at_90_degrees() {
    let dir = movement_to_direction(&make_state(MoveDirection::Forward), &make_facing(FRAC_PI_2));
    // yaw=π/2: forward = [sin(π/2), 0, cos(π/2)] = [1, 0, 0]
    assert!((dir[0] - 1.0).abs() < 1e-6);
    assert!((dir[2]).abs() < 1e-6);
}

#[test]
fn backward_is_opposite_of_forward() {
    let facing = make_facing(0.5);
    let fwd = movement_to_direction(&make_state(MoveDirection::Forward), &facing);
    let bwd = movement_to_direction(&make_state(MoveDirection::Backward), &facing);
    assert!((fwd[0] + bwd[0]).abs() < 1e-6);
    assert!((fwd[2] + bwd[2]).abs() < 1e-6);
}

#[test]
fn left_is_perpendicular_to_forward() {
    let facing = make_facing(0.0);
    let fwd = movement_to_direction(&make_state(MoveDirection::Forward), &facing);
    let left = movement_to_direction(&make_state(MoveDirection::Left), &facing);
    // dot product should be zero (perpendicular)
    let dot = fwd[0] * left[0] + fwd[2] * left[2];
    assert!(dot.abs() < 1e-6);
}

#[test]
fn right_is_opposite_of_left() {
    let facing = make_facing(PI / 3.0);
    let left = movement_to_direction(&make_state(MoveDirection::Left), &facing);
    let right = movement_to_direction(&make_state(MoveDirection::Right), &facing);
    assert!((left[0] + right[0]).abs() < 1e-6);
    assert!((left[2] + right[2]).abs() < 1e-6);
}

#[test]
fn direction_is_unit_length() {
    for dir in [
        MoveDirection::Forward,
        MoveDirection::Backward,
        MoveDirection::Left,
        MoveDirection::Right,
    ] {
        let d = movement_to_direction(&make_state(dir), &make_facing(1.23));
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-6,
            "direction {dir:?} has length {len}"
        );
    }
}

#[test]
fn y_component_always_zero() {
    for yaw in [0.0, FRAC_PI_2, PI, -PI] {
        for dir in [
            MoveDirection::Forward,
            MoveDirection::Backward,
            MoveDirection::Left,
            MoveDirection::Right,
        ] {
            let d = movement_to_direction(&make_state(dir), &make_facing(yaw));
            assert_eq!(d[1], 0.0);
        }
    }
}

#[test]
fn disconnect_during_charselect_keeps_scene_alive() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(crate::game_state::GameState::CharSelect);
    app.init_resource::<AuthUiFeedback>();
    app.add_observer(handle_client_disconnected);

    let client = app.world_mut().spawn(Client::default()).id();
    app.world_mut().entity_mut(client).insert(Disconnected {
        reason: Some("Link failed: test".to_string()),
    });
    app.update();

    let state = app.world().resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::CharSelect);
    let feedback = app.world().resource::<AuthUiFeedback>();
    assert_eq!(
        feedback.0.as_deref(),
        Some("Connection lost. Char select is now offline.")
    );
}

#[test]
fn disconnect_during_connecting_is_ignored() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(crate::game_state::GameState::Connecting);
    app.init_resource::<AuthUiFeedback>();
    app.add_observer(handle_client_disconnected);

    let client = app.world_mut().spawn(Client::default()).id();
    app.world_mut().entity_mut(client).insert(Disconnected {
        reason: Some("Link failed: test".to_string()),
    });
    app.update();
    app.update();

    let state = app.world().resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::Connecting);
    let feedback = app.world().resource::<AuthUiFeedback>();
    assert_eq!(feedback.0.as_deref(), None);
}

#[test]
fn disconnect_during_inworld_arms_reconnect_and_preserves_scene_state() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(crate::game_state::GameState::InWorld);
    app.init_resource::<AuthUiFeedback>();
    app.init_resource::<ReconnectState>();
    app.insert_resource(AuthToken(Some("saved-token".to_string())));
    app.insert_resource(selected_with_name("Theron"));
    app.insert_resource(game_engine::targeting::CurrentTarget(Some(Entity::from_bits(77))));
    app.init_resource::<CurrentZone>();
    app.init_resource::<LocalAliveState>();
    app.init_resource::<ChatLog>();
    app.init_resource::<ChatInput>();
    app.add_observer(handle_client_disconnected);

    let client = app.world_mut().spawn(Client::default()).id();
    let receiver = app.world_mut().spawn_empty().id();
    let replicated = app
        .world_mut()
        .spawn((
            Replicated { receiver },
            RemoteEntity,
            NetPlayer {
                name: "Theron".to_string(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
        ))
        .id();
    app.world_mut().resource_mut::<ChatLog>().messages.push((
        "system".to_string(),
        "stale".to_string(),
        ChatType::Say,
    ));
    app.world_mut().entity_mut(client).insert(Disconnected {
        reason: Some("Link failed: test".to_string()),
    });

    app.update();
    app.update();

    let state = app.world().resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::InWorld);
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::PendingConnect
    );
    assert!(app.world().contains_resource::<crate::char_select::AutoEnterWorld>());
    assert_eq!(
        app.world()
            .resource::<crate::char_select::PreselectedCharName>()
            .0,
        "Theron"
    );
    assert!(app.world().get_entity(client).is_err());
    assert!(app.world().get_entity(replicated).is_err());
    assert!(
        app.world()
            .resource::<game_engine::targeting::CurrentTarget>()
            .0
            .is_none()
    );
    assert!(app.world().resource::<ChatLog>().messages.is_empty());
}

#[test]
fn reset_network_world_preserves_selected_character_for_reconnect() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(selected_with_name("Theron"));

    let _ = app.world_mut().run_system_once(|mut commands: Commands| {
        commands.queue(reset_network_world);
    });
    app.update();

    let selected = app.world().resource::<SelectedCharacterId>();
    assert_eq!(selected.character_name.as_deref(), Some("Theron"));
}

#[test]
fn gameplay_input_is_disabled_during_reconnect() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(ReconnectState {
        phase: ReconnectPhase::PendingConnect,
        terrain_refresh_seen: false,
    });

    let allowed = app
        .world_mut()
        .run_system_once(|reconnect: Option<Res<ReconnectState>>| gameplay_input_allowed(reconnect))
        .expect("run gameplay_input_allowed");
    assert!(!allowed);
}

#[test]
fn reconnect_does_not_finish_until_fresh_terrain_signal_arrives() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(ReconnectState {
        phase: ReconnectPhase::AwaitingWorld,
        terrain_refresh_seen: false,
    });
    let mut adt = crate::terrain::AdtManager::default();
    adt.map_name = "azeroth".to_string();
    app.insert_resource(adt);
    app.add_systems(Update, finish_reconnect_when_world_ready);
    app.world_mut().spawn(LocalPlayer);

    app.update();

    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::AwaitingWorld
    );
}

#[test]
fn reconnect_finishes_after_local_player_and_terrain_signal() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(ReconnectState {
        phase: ReconnectPhase::AwaitingWorld,
        terrain_refresh_seen: true,
    });
    app.add_systems(Update, finish_reconnect_when_world_ready);
    app.world_mut().spawn(LocalPlayer);

    app.update();

    let reconnect = app.world().resource::<ReconnectState>();
    assert_eq!(reconnect.phase, ReconnectPhase::Inactive);
    assert!(!reconnect.terrain_refresh_seen);
}

#[test]
fn sync_updates_rotation_target() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, sync_replicated_transforms);

    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            NetRotation {
                x: 0.0,
                y: 1.5,
                z: 0.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            RotationTarget { yaw: 0.0 },
            RemoteEntity,
        ))
        .id();

    app.update();

    let rot = app.world().get::<RotationTarget>(entity).unwrap();
    assert!(
        (rot.yaw - 1.5).abs() < 1e-6,
        "rotation target should be 1.5, got {}",
        rot.yaw
    );
}

#[test]
fn sync_updates_interpolation_target() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, sync_replicated_transforms);

    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            RemoteEntity,
        ))
        .id();

    app.update();

    let interp = app.world().get::<InterpolationTarget>(entity).unwrap();
    // Server sends Bevy-space positions — no conversion.
    assert_eq!(interp.target, Vec3::new(10.0, 20.0, 30.0));
}

#[test]
fn sync_skips_entities_without_remote_marker() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, sync_replicated_transforms);

    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            // no RemoteEntity marker
        ))
        .id();

    app.update();

    let interp = app.world().get::<InterpolationTarget>(entity).unwrap();
    assert_eq!(interp.target, Vec3::ZERO);
}

#[test]
fn interpolation_moves_toward_target() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, interpolate_remote_entities);

    let start = Vec3::ZERO;
    let target = Vec3::new(10.0, 0.0, 0.0);
    let entity = app
        .world_mut()
        .spawn((
            InterpolationTarget { target },
            Transform::from_translation(start),
            RemoteEntity,
        ))
        .id();

    // First update has zero delta_time; run twice so time advances.
    app.update();
    app.update();

    let pos = app.world().get::<Transform>(entity).unwrap().translation;
    // Should have moved toward target but not reached it in one frame
    assert!(pos.x > 0.0, "should move toward target");
    assert!(pos.x < 10.0, "should not snap to target");
    assert!((pos.y).abs() < 1e-6, "y should stay zero");
}

#[test]
fn chat_log_caps_at_max() {
    let mut log = ChatLog::default();
    for i in 0..101 {
        log.messages
            .push((format!("player{i}"), format!("msg{i}"), ChatType::Say));
        if log.messages.len() > MAX_CHAT_LOG {
            log.messages.remove(0);
        }
    }
    assert_eq!(log.messages.len(), MAX_CHAT_LOG);
    assert_eq!(log.messages[0].0, "player1");
    assert_eq!(log.messages[99].0, "player100");
}

fn selected_with_name(name: &str) -> SelectedCharacterId {
    SelectedCharacterId {
        character_id: Some(1),
        character_name: Some(name.to_string()),
    }
}

#[test]
fn is_local_player_entity_matches_selected() {
    let selected = selected_with_name("Theron");
    assert!(is_local_player_entity("Theron", Some(&selected)));
}

#[test]
fn is_local_player_entity_rejects_different() {
    let selected = selected_with_name("Theron");
    assert!(!is_local_player_entity("OtherPlayer", Some(&selected)));
}

#[test]
fn is_local_player_entity_none_without_resource() {
    assert!(!is_local_player_entity("Theron", None));
}

#[test]
fn is_local_player_entity_none_when_not_selected() {
    let selected = SelectedCharacterId::default();
    assert!(!is_local_player_entity("Theron", Some(&selected)));
}

#[test]
fn choose_local_player_entity_prefers_newest_matching_entity() {
    let older = Entity::from_bits(10);
    let newer = Entity::from_bits(20);
    let other = Entity::from_bits(30);
    let theron = NetPlayer {
        name: "Theron".into(),
        race: 1,
        class: 1,
        appearance: CharacterAppearance::default(),
    };
    let other_player = NetPlayer {
        name: "Other".into(),
        race: 1,
        class: 1,
        appearance: CharacterAppearance::default(),
    };

    let (chosen, matches) = choose_local_player_entity(
        "Theron",
        [(older, &theron), (other, &other_player), (newer, &theron)].into_iter(),
    );

    assert_eq!(matches, 2);
    assert_eq!(chosen, Some(newer));
}

#[test]
fn choose_local_player_entity_returns_none_when_name_missing() {
    let player = NetPlayer {
        name: "Other".into(),
        race: 1,
        class: 1,
        appearance: CharacterAppearance::default(),
    };

    let (chosen, matches) =
        choose_local_player_entity("Theron", [(Entity::from_bits(1), &player)].into_iter());

    assert_eq!(matches, 0);
    assert_eq!(chosen, None);
}

#[test]
fn net_position_to_bevy_passes_through_unchanged() {
    // Server already sends Bevy-space coordinates.
    let pos = NetPosition {
        x: -8949.0,
        y: 83.0,
        z: 132.0,
    };

    assert_eq!(net_position_to_bevy(&pos), Vec3::new(-8949.0, 83.0, 132.0));
}

#[test]
fn net_player_customization_selection_uses_player_race_class_and_appearance() {
    let player = NetPlayer {
        name: "Theron".into(),
        race: 10,
        class: 8,
        appearance: CharacterAppearance {
            sex: 1,
            skin_color: 2,
            face: 3,
            hair_style: 4,
            hair_color: 5,
            facial_style: 6,
        },
    };

    let selection = net_player_customization_selection(&player);

    assert_eq!(selection.race, 10);
    assert_eq!(selection.class, 8);
    assert_eq!(selection.sex, 1);
    assert_eq!(selection.appearance, player.appearance);
}

#[test]
fn resolve_player_model_path_uses_player_race_and_sex() {
    let player = NetPlayer {
        name: "Theron".into(),
        race: 10,
        class: 8,
        appearance: CharacterAppearance {
            sex: 1,
            ..Default::default()
        },
    };

    let path = resolve_player_model_path(&player).expect("player model path should resolve");

    assert!(
        path.to_string_lossy()
            .to_ascii_lowercase()
            .contains("bloodelffemale"),
        "expected bloodelf female model path, got {}",
        path.display()
    );
}

#[test]
fn terrain_messages_are_processed_before_inworld_transition() {
    assert!(terrain_messages_allowed_in_state(
        crate::game_state::GameState::CharSelect
    ));
}

#[test]
fn queue_despawn_if_exists_removes_live_entity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let entity = app.world_mut().spawn_empty().id();

    let _ = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            queue_despawn_if_exists(&mut commands, entity);
        });
    app.update();

    assert!(app.world().get_entity(entity).is_err());
}

#[test]
fn queue_despawn_if_exists_ignores_missing_entity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let entity = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(entity).despawn();

    let _ = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            queue_despawn_if_exists(&mut commands, entity);
        });
    app.update();

    assert!(app.world().get_entity(entity).is_err());
}

#[test]
fn npc_visibility_policy_hides_debug_pedestals_and_turkeys() {
    assert_eq!(npc_visibility_policy(26741), NpcVisibilityPolicy::Hidden);
    assert_eq!(npc_visibility_policy(32820), NpcVisibilityPolicy::Hidden);
}

#[test]
fn npc_visibility_policy_only_shows_spirit_healer_when_dead() {
    assert_eq!(npc_visibility_policy(6491), NpcVisibilityPolicy::DeadOnly);
}

#[test]
fn sync_local_alive_state_tracks_local_player_health() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<LocalAliveState>();
    app.add_systems(Update, sync_local_alive_state);
    app.world_mut().spawn((
        LocalPlayer,
        NetHealth {
            current: 0.0,
            max: 100.0,
        },
    ));

    app.update();

    assert!(!app.world().resource::<LocalAliveState>().0);
}
