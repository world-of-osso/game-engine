use super::*;
use bevy::app::AppExit;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

struct WorkerTickFixture {
    runtime: game_engine::network_runtime::worker::NetworkRuntime,
    allow_tick: Sender<bool>,
    ready: Receiver<()>,
}

impl WorkerTickFixture {
    fn new() -> Self {
        let (allow_tick, allowed) = mpsc::channel();
        let (signal_ready, ready) = mpsc::channel();
        let allowed = Mutex::new(allowed);
        let runtime =
            game_engine::network_runtime::worker::NetworkRuntime::spawn(move |app, _updates| {
                app.add_systems(First, move |mut exit: MessageWriter<AppExit>| {
                    signal_ready.send(()).unwrap();
                    if !allowed.lock().unwrap().recv().unwrap() {
                        exit.write(AppExit::Success);
                    }
                });
            })
            .unwrap();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        Self {
            runtime,
            allow_tick,
            ready,
        }
    }

    fn deliver_ticks(&self, app: &mut App, count: u64) {
        for _ in 0..count {
            self.allow_tick.send(true).unwrap();
            // Entry into the next worker update proves the previous update's
            // real FIFO permit has been published, without a timing race.
            self.ready.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        self.runtime.drain_updates(app.world_mut()).unwrap();
    }
}

impl Drop for WorkerTickFixture {
    fn drop(&mut self) {
        self.allow_tick.send(false).unwrap();
        self.runtime.stop().unwrap();
    }
}

fn lifecycle_cadence_app() -> App {
    let mut app = App::new();
    app.add_plugins(game_engine::network_tick::NetworkTickPlugin);
    game_engine::network_runtime::connection::initialize_connection_bridge(&mut app);
    app.init_resource::<ReconnectState>()
        .init_resource::<PendingNetworkWorldReset>()
        .init_resource::<NetworkUpdateFrame>();
    register_network_lifecycle_systems(&mut app);
    app
}

#[test]
fn lifecycle_cadence_unchanged_time_preserves_pending_reset_and_reconnect() {
    let mut app = lifecycle_cadence_app();
    let replica = app.world_mut().spawn(Remote).id();
    app.world_mut().resource_mut::<PendingNetworkWorldReset>().0 = Some(0);
    app.world_mut().resource_mut::<ReconnectState>().phase = ReconnectPhase::PendingConnect;
    app.insert_resource(ServerAddr("127.0.0.1:9".parse().unwrap()));
    for _ in 0..20 {
        app.update();
    }
    assert!(
        app.world().get_entity(replica).is_ok(),
        "reset ran without a tick"
    );
    assert!(
        !app.world().contains_resource::<LocalClientId>(),
        "reconnect ran without a tick"
    );
    assert_eq!(app.world().resource::<NetworkUpdateFrame>().0, 0);
}

#[test]
fn lifecycle_cadence_finishes_ready_reconnect_only_on_tick() {
    let mut app = lifecycle_cadence_app();
    let worker = WorkerTickFixture::new();
    app.world_mut().spawn(LocalPlayer);
    {
        let mut reconnect = app.world_mut().resource_mut::<ReconnectState>();
        reconnect.phase = ReconnectPhase::AwaitingWorld;
        reconnect.terrain_refresh_seen = true;
    }
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::AwaitingWorld
    );
    worker.deliver_ticks(&mut app, 1);
    app.update();
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::Inactive
    );
}

#[test]
fn lifecycle_cadence_advances_exactly_sixty_times_per_second() {
    for render_hz in [30_u64, 60, 144, 400] {
        let mut app = lifecycle_cadence_app();
        let worker = WorkerTickFixture::new();
        let mut delivered = 0;
        for frame in 1..=render_hz {
            let due = frame * 60 / render_hz;
            worker.deliver_ticks(&mut app, due - delivered);
            delivered = due;
            app.update();
        }
        assert_eq!(
            app.world().resource::<NetworkUpdateFrame>().0,
            60,
            "render cadence {render_hz}"
        );
    }
}

#[test]
fn lifecycle_cadence_receive_commands_finish_before_next_tick_reset() {
    use game_engine::network_tick::{NetworkTick, NetworkTickSystems};
    let mut app = lifecycle_cadence_app();
    let worker = WorkerTickFixture::new();
    let replica = app.world_mut().spawn(Remote).id();
    app.add_systems(
        NetworkTick,
        (move |mut commands: Commands, mut once: Local<bool>| {
            if !*once {
                *once = true;
                crate::networking::reconnect::request_network_world_reset(&mut commands);
                commands.entity(replica).insert(LateReceiveMarker);
            }
        })
        .in_set(NetworkTickSystems::Receive),
    );
    worker.deliver_ticks(&mut app, 1);
    app.update();
    assert!(app.world().get::<LateReceiveMarker>(replica).is_some());
    assert_eq!(
        app.world().resource::<PendingNetworkWorldReset>().0,
        Some(1)
    );
    for _ in 0..20 {
        app.update();
    }
    assert!(app.world().get_entity(replica).is_ok());
    worker.deliver_ticks(&mut app, 1);
    app.update();
    assert!(app.world().get_entity(replica).is_err());
    assert_eq!(app.world().resource::<PendingNetworkWorldReset>().0, None);
    assert_eq!(app.world().resource::<NetworkUpdateFrame>().0, 2);
}

#[derive(Component)]
struct LateReceiveMarker;

#[derive(Resource)]
struct LateNetworkCommands {
    client: Entity,
    replicated: Entity,
    armed: bool,
}

#[derive(Resource)]
struct PendingDisconnectInsert {
    client: Entity,
    armed: bool,
}

fn queue_disconnect_during_update(
    mut pending: ResMut<PendingDisconnectInsert>,
    mut commands: Commands,
) {
    if !pending.armed {
        return;
    }
    pending.armed = false;
    commands.entity(pending.client).insert(Disconnected {
        reason: Some("Link failed: test".to_string()),
    });
}

fn queue_late_network_entity_commands(
    mut late: ResMut<LateNetworkCommands>,
    mut commands: Commands,
) {
    if !late.armed {
        return;
    }
    late.armed = false;
    commands.entity(late.client).insert(LateReceiveMarker);
    commands.entity(late.replicated).insert(LateReceiveMarker);
}

#[test]
fn disconnect_during_charselect_arms_reconnect_when_token_exists() {
    let mut app = charselect_disconnect_app(Some("saved-token"));
    let client = trigger_disconnect(&mut app);
    app.update();
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
    let client = app.world_mut().spawn(Client::default()).id();
    app.world_mut().resource_mut::<PendingForcedDisconnect>().0 = Some(ForcedDisconnect {
        message: "Account banned: cheating".to_string(),
        reconnect_allowed: false,
    });
    trigger_disconnect_entity(&mut app, client);
    for _ in 0..5 {
        app.update();
    }

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
    assert!(app.world().get_entity(client).is_err());
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
fn disconnect_defers_world_reset_until_after_late_network_commands() {
    let mut app = inworld_disconnect_base_app();
    let (client, replicated) = populate_inworld_disconnect_entities(&mut app);
    app.insert_resource(LateNetworkCommands {
        client,
        replicated,
        armed: true,
    });
    app.insert_resource(PendingDisconnectInsert {
        client,
        armed: true,
    });
    app.add_systems(Update, queue_disconnect_during_update);
    app.add_systems(Last, queue_late_network_entity_commands);

    let first_update = catch_unwind(AssertUnwindSafe(|| {
        app.update();
    }));
    assert!(first_update.is_ok(), "disconnect frame should not panic");
    assert!(app.world().get_entity(client).is_ok());
    assert!(app.world().get_entity(replicated).is_ok());
    assert!(app.world().entity(client).contains::<LateReceiveMarker>());
    assert!(
        app.world()
            .entity(replicated)
            .contains::<LateReceiveMarker>()
    );
    assert_eq!(
        app.world().resource::<PendingNetworkWorldReset>().0,
        Some(1)
    );

    app.update();

    assert_inworld_reconnect_state(&app, client, replicated);
    assert!(
        app.world()
            .resource::<PendingNetworkWorldReset>()
            .0
            .is_none()
    );
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

#[test]
fn initial_netcode_disconnected_marker_does_not_restart_inworld_reconnect() {
    let mut app = App::new();
    game_engine::network_events::initialize_dispatcher(&mut app);
    game_engine::network_runtime::connection::initialize_connection_bridge(&mut app);
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(crate::game_state::GameState::InWorld);
    app.init_resource::<AuthUiFeedback>();
    app.init_resource::<PendingForcedDisconnect>();
    app.insert_resource(AuthToken(Some("saved-token".to_string())));
    app.insert_resource(selected_with_name("Elara"));
    app.insert_resource(ReconnectState {
        phase: ReconnectPhase::PendingConnect,
        terrain_refresh_seen: false,
    });
    app.init_resource::<PendingNetworkWorldReset>();
    app.init_resource::<NetworkUpdateFrame>();
    app.insert_resource(ServerAddr("127.0.0.1:5000".parse().unwrap()));
    app.insert_resource(LoginMode::Login);
    app.insert_resource(LoginUsername(String::new()));
    app.insert_resource(LoginPassword(String::new()));
    app.add_systems(
        Update,
        (
            flush_pending_network_world_reset.run_if(network_world_reset_is_due),
            drive_inworld_reconnect,
        )
            .chain(),
    );
    app.add_systems(Last, advance_network_update_frame);
    app.add_observer(handle_client_disconnected);

    app.update();

    let mut clients = app.world_mut().query_filtered::<Entity, With<Client>>();
    let first_clients = clients.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(first_clients.len(), 1, "reconnect should spawn one client");
    let first_client = first_clients[0];
    let first_client_id = app.world().resource::<LocalClientId>().0;
    assert_eq!(
        app.world().resource::<PendingNetworkWorldReset>().0,
        None,
        "the required initial Disconnected marker must not schedule a world reset"
    );
    assert!(
        !app.world()
            .contains_resource::<crate::scenes::char_select::AutoEnterWorld>(),
        "the initial marker must not re-enter the world state"
    );
    assert!(
        !app.world()
            .contains_resource::<crate::scenes::char_select::PreselectedCharName>(),
        "the initial marker must not reinsert the selected-character transition"
    );

    for _ in 0..3 {
        app.update();

        let mut clients = app.world_mut().query_filtered::<Entity, With<Client>>();
        let current_clients = clients.iter(app.world()).collect::<Vec<_>>();
        assert_eq!(
            current_clients,
            vec![first_client],
            "reconnect must retain one client entity instead of churning entities"
        );
        assert_eq!(
            app.world().resource::<LocalClientId>().0,
            first_client_id,
            "reconnect must retain the same client id"
        );
        assert_eq!(
            app.world().resource::<PendingNetworkWorldReset>().0,
            None,
            "initial marker handling must not schedule repeated world resets"
        );
    }
}

#[test]
fn failed_pending_inworld_connection_still_rearms_reconnect() {
    let mut app = inworld_disconnect_base_app();
    app.world_mut().resource_mut::<ReconnectState>().phase = ReconnectPhase::PendingConnect;
    let (client, replicated) = populate_inworld_disconnect_entities(&mut app);

    trigger_disconnect_entity(&mut app, client);
    app.update();
    app.update();

    assert_inworld_reconnect_state(&app, client, replicated);
}
