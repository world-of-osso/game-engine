use super::*;

#[test]
fn disconnect_during_charselect_arms_reconnect_when_token_exists() {
    let mut app = charselect_disconnect_app(Some("saved-token"));
    let client = trigger_disconnect(&mut app);
    app.update();

    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::CharSelect);
    let feedback = app.world().resource::<AuthUiFeedback>();
    assert_eq!(feedback.0.as_deref(), None);
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::PendingConnect
    );
    assert_eq!(app.world().resource::<LoginUsername>().0, "");
    assert_eq!(app.world().resource::<LoginPassword>().0, "");
    assert!(app.world().get_entity(client).is_err());
}

#[test]
fn disconnect_during_charselect_without_token_stays_offline() {
    let mut app = charselect_disconnect_app(None);
    let client = trigger_disconnect(&mut app);
    app.update();

    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::CharSelect);
    let feedback = app.world().resource::<AuthUiFeedback>();
    assert_eq!(
        feedback.0.as_deref(),
        Some("Connection lost. Char select is now offline.")
    );
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::Inactive
    );
    assert!(app.world().get_entity(client).is_ok());
}

#[test]
fn forced_disconnect_during_charselect_with_token_returns_to_login() {
    let mut app = charselect_disconnect_app(Some("saved-token"));
    app.world_mut().resource_mut::<PendingForcedDisconnect>().0 = Some(ForcedDisconnect {
        message: "Account banned: cheating".to_string(),
        reconnect_allowed: false,
    });
    trigger_disconnect(&mut app);
    app.update();
    app.update();

    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::Login);
    assert_eq!(
        app.world().resource::<AuthUiFeedback>().0.as_deref(),
        Some("Account banned: cheating")
    );
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::Inactive
    );
}

#[test]
fn disconnect_during_connecting_is_ignored() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(crate::game_state::GameState::Connecting);
    app.init_resource::<AuthUiFeedback>();
    app.init_resource::<PendingForcedDisconnect>();
    app.add_observer(handle_client_disconnected);
    trigger_disconnect(&mut app);
    app.update();
    app.update();

    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::Connecting);
    let feedback = app.world().resource::<AuthUiFeedback>();
    assert_eq!(feedback.0.as_deref(), None);
}

#[test]
fn disconnect_during_inworld_arms_reconnect_and_preserves_scene_state() {
    let mut app = inworld_disconnect_base_app();
    let (client, replicated) = populate_inworld_disconnect_entities(&mut app);
    trigger_disconnect_entity(&mut app, client);

    app.update();
    app.update();

    assert_inworld_reconnect_state(&app, client, replicated);
}

#[test]
fn forced_disconnect_during_inworld_goes_to_login_without_reconnect() {
    let mut app = inworld_disconnect_base_app();
    let (client, replicated) = populate_inworld_disconnect_entities(&mut app);
    app.world_mut().resource_mut::<PendingForcedDisconnect>().0 = Some(ForcedDisconnect {
        message: "You were kicked: testing".to_string(),
        reconnect_allowed: false,
    });
    trigger_disconnect_entity(&mut app, client);

    app.update();
    app.update();

    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::Login);
    assert_eq!(
        app.world().resource::<AuthUiFeedback>().0.as_deref(),
        Some("You were kicked: testing")
    );
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::Inactive
    );
    assert!(app.world().get_entity(client).is_err());
    assert!(app.world().get_entity(replicated).is_err());
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
fn disconnect_during_game_menu_reconnects_without_bouncing_to_login() {
    let mut app = disconnect_app_with_state(crate::game_state::GameState::GameMenu);
    let (client, replicated) = populate_inworld_disconnect_entities(&mut app);
    trigger_disconnect_entity(&mut app, client);

    app.update();
    app.update();

    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::InWorld);
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::PendingConnect
    );
    assert!(app.world().get_entity(client).is_err());
    assert!(app.world().get_entity(replicated).is_err());
}
