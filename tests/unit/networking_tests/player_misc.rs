use super::*;

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
    let theron = net_player("Theron");
    let other_player = net_player("Other");

    let (chosen, matches) = choose_local_player_entity(
        "Theron",
        [(older, &theron), (other, &other_player), (newer, &theron)].into_iter(),
    );

    assert_eq!(matches, 2);
    assert_eq!(chosen, Some(newer));
}

#[test]
fn choose_local_player_entity_returns_none_when_name_missing() {
    let player = net_player("Other");
    let (chosen, matches) =
        choose_local_player_entity("Theron", [(Entity::from_bits(1), &player)].into_iter());

    assert_eq!(matches, 0);
    assert_eq!(chosen, None);
}

#[test]
fn net_position_to_bevy_passes_through_unchanged() {
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
            eye_color: 0,
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
