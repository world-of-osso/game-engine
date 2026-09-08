use super::*;

use bevy::ecs::system::RunSystemOnce;
use game_engine::network_runtime::connection::Client;
use game_engine::network_runtime::messages::{ConnectionSender, MessageSenders};
use shared::components::{
    EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Player as NetPlayer,
};

#[derive(Resource, Default)]
struct ObservedForcedDisconnect(Option<(String, bool)>);

fn apply_worker_connection_updates(app: &mut App) {
    use game_engine::network_runtime::worker::NetworkRuntime;
    app.world_mut()
        .resource_scope(|world, runtime: Mut<NetworkRuntime>| {
            runtime.drain_updates(world).unwrap();
        });
    game_engine::network_runtime::connection::apply_connection_events(app.world_mut());
}

#[test]
fn empty_forced_notice_inbox_does_not_require_worker() {
    let mut app = App::new();
    game_engine::network_events::register_message_handler::<ForcedDisconnect, _>(
        &mut app,
        receive_forced_disconnect,
        |_| true,
    );
    // No worker or pending-notice resource exists on a disconnected main world.
    game_engine::network_events::dispatch_incoming(app.world_mut());
}

#[test]
fn forced_notice_without_worker_fails_explicitly() {
    let mut app = App::new();
    app.init_resource::<crate::networking::PendingForcedDisconnect>();
    app.insert_resource(game_engine::network_runtime::messages::Inbox::new(vec![
        ForcedDisconnect {
            message: "invalid disconnected inbox".into(),
            reconnect_allowed: false,
        },
    ]));
    assert!(
        app.world_mut()
            .run_system_once(receive_forced_disconnect)
            .is_err()
    );
}

#[test]
fn forced_notice_disconnects_real_worker_and_preserves_notice_for_lifecycle() {
    use game_engine::network_runtime::{connection, messages::Inbox};
    use std::{
        net::UdpSocket,
        time::{Duration, Instant},
    };

    let server = UdpSocket::bind("127.0.0.1:0").unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut app = App::new();
    game_engine::network_events::initialize_dispatcher(&mut app);
    connection::initialize_connection_bridge(&mut app);
    app.init_resource::<crate::networking::PendingForcedDisconnect>();
    app.init_resource::<ObservedForcedDisconnect>();
    app.add_observer(
        |_: On<Add, connection::Disconnected>,
         pending: Res<crate::networking::PendingForcedDisconnect>,
         mut observed: ResMut<ObservedForcedDisconnect>| {
            let notice = pending
                .0
                .as_ref()
                .expect("notice must precede disconnect lifecycle");
            observed.0 = Some((notice.message.clone(), notice.reconnect_allowed));
        },
    );
    let client =
        connection::start_connection(app.world_mut(), server.local_addr().unwrap(), 9157).unwrap();
    let mut packet = [0; 2048];
    assert!(server.recv_from(&mut packet).unwrap().0 > 0);
    apply_worker_connection_updates(&mut app);
    assert!(
        app.world()
            .get::<connection::Disconnected>(client)
            .is_none()
    );

    app.insert_resource(Inbox::new(vec![ForcedDisconnect {
        message: "server requested disconnect".into(),
        reconnect_allowed: false,
    }]));
    app.world_mut()
        .run_system_once(receive_forced_disconnect)
        .unwrap();
    assert_eq!(
        app.world()
            .resource::<crate::networking::PendingForcedDisconnect>()
            .0
            .as_ref()
            .unwrap()
            .message,
        "server requested disconnect"
    );
    assert!(
        app.world()
            .get::<connection::Disconnected>(client)
            .is_none()
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        apply_worker_connection_updates(&mut app);
        if app
            .world()
            .get::<connection::Disconnected>(client)
            .is_some()
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        app.world()
            .get::<connection::Disconnected>(client)
            .is_some(),
        "a server notice must invoke the real worker client disconnect API"
    );
    assert_eq!(
        app.world().resource::<ObservedForcedDisconnect>().0,
        Some(("server requested disconnect".into(), false))
    );
    assert!(
        app.world()
            .resource::<crate::networking::PendingForcedDisconnect>()
            .0
            .is_some()
    );
    connection::stop_connection(app.world_mut()).unwrap();
}

fn read_local_server_auth_test_token() -> String {
    let path = std::env::var_os("GAME_ENGINE_TEST_AUTH_TOKEN_FILE")
        .expect("set GAME_ENGINE_TEST_AUTH_TOKEN_FILE to a private local-server token file");
    let token = std::fs::read_to_string(path).expect("read private local-server auth token file");
    let token = token.trim().to_owned();
    assert!(
        !token.is_empty(),
        "private local-server auth token file is empty"
    );
    token
}

fn build_local_server_auth_startup_app(token: String) -> App {
    use game_engine::network_runtime::{connection, messages::Inbox};

    let mut app = App::new();
    game_engine::network_events::initialize_dispatcher(&mut app);
    connection::initialize_connection_bridge(&mut app);
    game_engine::network_events::register_message_handler::<LoginResponse, _>(
        &mut app,
        |_: ResMut<Inbox<LoginResponse>>| {
            panic!("auth startup regression must not dispatch main-world login callbacks");
        },
        |_| true,
    );
    app.insert_resource(LoginMode::Login)
        .init_resource::<LoginUsername>()
        .init_resource::<LoginPassword>()
        .insert_resource(AuthToken(Some(token)));
    app
}

#[test]
#[ignore = "requires the existing local server and GAME_ENGINE_TEST_AUTH_TOKEN_FILE; run alone"]
fn local_server_authenticates_while_main_callbacks_are_stalled() {
    use game_engine::network_runtime::{connection, messages::Inbox, worker::NetworkRuntime};
    use std::time::Duration;

    let mut app = build_local_server_auth_startup_app(read_local_server_auth_test_token());
    crate::networking::connect_to_server_inner(
        &mut app.world_mut().commands(),
        "127.0.0.1:5000".parse().expect("local test server address"),
    );
    app.world_mut().flush();
    let mut clients = app.world_mut().query_filtered::<Entity, With<Client>>();
    let client = clients
        .single(app.world())
        .expect("one main-world client proxy");
    assert!(app.world().get::<connection::Connected>(client).is_none());

    // No main app updates or connection-event application during authentication.
    std::thread::sleep(Duration::from_secs(2));
    let mut runtime = app
        .world_mut()
        .remove_resource::<NetworkRuntime>()
        .expect("connection creation starts the network worker");
    runtime.stop().expect("stop and join auth test worker");
    runtime
        .drain_updates(app.world_mut())
        .expect("drain retained worker updates after join");
    let main_connected = app.world().get::<connection::Connected>(client).is_some();
    let authenticated = app
        .world_mut()
        .resource_mut::<Inbox<LoginResponse>>()
        .receive()
        .any(|response| response.success);
    connection::stop_connection(app.world_mut()).expect("clear auth test connection state");

    assert!(
        !main_connected,
        "authentication must not require a main-world Connected marker"
    );
    assert!(
        authenticated,
        "worker must receive successful LoginResponse without main callbacks"
    );
}

#[derive(Resource, Default)]
struct ObservedLocalServerWorldEntry {
    selected_character: Option<CharacterListEntry>,
    responses: Vec<EnterWorldResponse>,
}

fn select_first_local_server_test_character(
    mut receivers: MessageReceivers<LoginResponse>,
    mut senders: MessageSenders<SelectCharacter>,
    mut observed: ResMut<ObservedLocalServerWorldEntry>,
) {
    for receiver in receivers.iter_mut() {
        for response in receiver.receive() {
            assert!(response.success, "local-server token login must succeed");
            let character = response
                .characters
                .first()
                .expect("local-server fixture account must own a character")
                .clone();
            let character_id = character.character_id;
            observed.selected_character = Some(character);
            for mut sender in senders.iter_mut() {
                sender.send::<AuthChannel>(SelectCharacter { character_id });
            }
        }
    }
}

fn record_local_server_world_entry(
    mut receivers: MessageReceivers<EnterWorldResponse>,
    mut observed: ResMut<ObservedLocalServerWorldEntry>,
) {
    for receiver in receivers.iter_mut() {
        observed.responses.extend(receiver.receive());
    }
}

fn build_local_server_world_entry_app(token: String) -> App {
    let mut app = App::new();
    app.add_plugins(game_engine::network_tick::NetworkTickPlugin);
    game_engine::network_runtime::connection::initialize_connection_bridge(&mut app);
    crate::networking::register_connection_tick_systems(&mut app);
    app.init_resource::<ObservedLocalServerWorldEntry>()
        .insert_resource(LoginMode::Login)
        .init_resource::<LoginUsername>()
        .init_resource::<LoginPassword>()
        .insert_resource(AuthToken(Some(token)));
    game_engine::network_events::register_message_handler::<LoginResponse, _>(
        &mut app,
        select_first_local_server_test_character,
        |_| true,
    );
    game_engine::network_events::register_message_handler::<EnterWorldResponse, _>(
        &mut app,
        record_local_server_world_entry,
        |_| true,
    );
    app
}

fn local_server_entered_player(world: &World) -> Option<(String, u8, u8)> {
    use game_engine::network_runtime::replication::ReplicationMirrorMap;

    let response = world
        .resource::<ObservedLocalServerWorldEntry>()
        .responses
        .first()?;
    let server_entity = Entity::try_from_bits(response.player_entity?)?;
    let entity = world
        .resource::<ReplicationMirrorMap>()
        .server_to_main(server_entity)?;
    let player = world.get::<NetPlayer>(entity)?;
    Some((player.name.clone(), player.race, player.class))
}

#[test]
#[ignore = "requires the existing local server and GAME_ENGINE_TEST_AUTH_TOKEN_FILE; run alone"]
fn local_server_enters_world_after_login_arrives_before_main_connection_marker() {
    use game_engine::network_runtime::connection;
    use std::time::{Duration, Instant};

    let mut app = build_local_server_world_entry_app(read_local_server_auth_test_token());
    crate::networking::connect_to_server_inner(
        &mut app.world_mut().commands(),
        "127.0.0.1:5000".parse().expect("local test server address"),
    );
    app.world_mut().flush();
    let mut clients = app.world_mut().query_filtered::<Entity, With<Client>>();
    let client = clients
        .single(app.world())
        .expect("one main-world client proxy");

    // Authentication progresses on the worker while the main lifecycle remains unapplied.
    std::thread::sleep(Duration::from_secs(2));
    assert!(app.world().get::<connection::Connected>(client).is_none());
    app.update();
    let deadline = Instant::now() + Duration::from_secs(5);
    while local_server_entered_player(app.world()).is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
        app.update();
    }

    let entered_player = local_server_entered_player(app.world());
    let observed = app
        .world_mut()
        .remove_resource::<ObservedLocalServerWorldEntry>()
        .expect("world-entry observations");
    connection::stop_connection(app.world_mut()).expect("stop and join world-entry test worker");

    let selected = observed
        .selected_character
        .expect("first main update must process the server's login roster");
    assert!(selected.character_id > 0, "select a persisted character ID");
    assert_eq!(
        observed.responses.len(),
        1,
        "selecting the first roster character must receive one actual EnterWorldResponse",
    );
    let response = &observed.responses[0];
    assert!(
        response.success,
        "server must accept the selected character ID"
    );
    assert!(
        response.player_entity.is_some(),
        "world entry must identify its server player"
    );
    assert_eq!(
        entered_player,
        Some((selected.name, selected.race, selected.class)),
        "the returned server entity must replicate the selected roster character",
    );
}

const VALID_TEST_UUID: &str = "22222222-2222-2222-2222-222222222222";

fn make_test_char(id: u64, name: &str) -> CharacterListEntry {
    CharacterListEntry {
        character_id: id,
        name: name.to_string(),
        level: 1,
        race: 1,
        class: 1,
        appearance: shared::components::CharacterAppearance::default(),
        equipment_appearance: shared::components::EquipmentAppearance::default(),
    }
}

struct LoginResponseResult {
    feedback: AuthUiFeedback,
    goes_login: bool,
    goes_charselect: bool,
    goes_loading: bool,
    reconnect: crate::networking::ReconnectState,
}

fn run_login_response_for_test(
    resp: LoginResponse,
    chars: Vec<CharacterListEntry>,
    reconnect: crate::networking::ReconnectState,
    auto_enter: bool,
) -> LoginResponseResult {
    let mut app = build_login_test_app(chars, reconnect, auto_enter);
    run_handle_login_response(&mut app, resp);
    app.update();
    extract_login_result(&app)
}

fn build_login_test_app(
    chars: Vec<CharacterListEntry>,
    reconnect: crate::networking::ReconnectState,
    auto_enter: bool,
) -> App {
    let mut app = game_engine::test_harness::headless_app();
    app.insert_resource(AuthToken(None));
    app.init_resource::<AuthUiFeedback>();
    app.init_resource::<ConnectionSender>();
    app.insert_resource(CharacterList(chars));
    app.init_resource::<crate::scenes::char_select::SelectedCharIndex>();
    app.init_resource::<NextState<GameState>>();
    app.insert_resource(reconnect);
    if auto_enter {
        app.insert_resource(crate::scenes::char_select::AutoEnterWorld);
    }
    app
}

fn run_handle_login_response(app: &mut App, resp: LoginResponse) {
    app.world_mut()
        .run_system_once(
            move |mut auth_token: ResMut<AuthToken>,
                  mut auth_feedback: ResMut<AuthUiFeedback>,
                  mut char_list: ResMut<CharacterList>,
                  auto_enter_world: Option<Res<crate::scenes::char_select::AutoEnterWorld>>,
                  mut selected_char_idx: ResMut<crate::scenes::char_select::SelectedCharIndex>,
                  mut next_state: ResMut<NextState<GameState>>,
                  mut select_senders: MessageSenders<SelectCharacter>,
                  mut reconnect: ResMut<crate::networking::ReconnectState>,
                  mut commands: Commands| {
                handle_login_response(
                    resp.clone(),
                    &mut auth_token,
                    &mut auth_feedback,
                    &mut char_list,
                    auto_enter_world.as_ref(),
                    None,
                    None,
                    &mut selected_char_idx,
                    &mut select_senders,
                    &mut next_state,
                    Some(&mut reconnect),
                    None,
                    &mut commands,
                );
            },
        )
        .expect("login response system must have its adapter resources");
}

fn extract_login_result(app: &App) -> LoginResponseResult {
    LoginResponseResult {
        feedback: app.world().resource::<AuthUiFeedback>().clone(),
        goes_login: matches!(
            app.world().resource::<NextState<GameState>>(),
            NextState::Pending(GameState::Login)
        ),
        goes_charselect: matches!(
            app.world().resource::<NextState<GameState>>(),
            NextState::Pending(GameState::CharSelect)
        ),
        goes_loading: matches!(
            app.world().resource::<NextState<GameState>>(),
            NextState::Pending(GameState::Loading)
        ),
        reconnect: *app.world().resource::<crate::networking::ReconnectState>(),
    }
}

#[test]
fn build_login_request_omits_cached_token_for_password_login() {
    let request = build_login_request(
        &AuthToken(Some(VALID_TEST_UUID.to_string())),
        &LoginUsername("alice".to_string()),
        &LoginPassword("secret".to_string()),
    );

    assert!(request.token.is_none());
    assert_eq!(request.username, "alice");
    assert_eq!(request.password, "secret");
}

#[test]
fn build_login_request_allows_token_only_login() {
    let request = build_login_request(
        &AuthToken(Some(VALID_TEST_UUID.to_string())),
        &LoginUsername(String::new()),
        &LoginPassword(String::new()),
    );

    assert_eq!(request.token.as_deref(), Some(VALID_TEST_UUID));
    assert!(request.username.is_empty());
    assert!(request.password.is_empty());
}

#[test]
fn build_login_request_trims_cached_token() {
    let request = build_login_request(
        &AuthToken(Some(format!(" {VALID_TEST_UUID} \n"))),
        &LoginUsername(String::new()),
        &LoginPassword(String::new()),
    );

    assert_eq!(request.token.as_deref(), Some(VALID_TEST_UUID));
}

#[test]
fn build_login_request_drops_placeholder_cached_token() {
    let request = build_login_request(
        &AuthToken(Some("saved-token".to_string())),
        &LoginUsername(String::new()),
        &LoginPassword(String::new()),
    );

    assert!(request.token.is_none());
}

#[test]
fn normalize_auth_token_rejects_blank_and_placeholder_values() {
    assert_eq!(normalize_auth_token("   "), None);
    assert_eq!(normalize_auth_token("saved-token"), None);
    assert_eq!(normalize_auth_token(" SAVED-TOKEN \n"), None);
    assert_eq!(normalize_auth_token(TEST_PLACEHOLDER_UUID), None);
}

#[test]
fn normalize_auth_token_trims_valid_token() {
    assert_eq!(
        normalize_auth_token(&format!(" {VALID_TEST_UUID} \n")),
        Some(VALID_TEST_UUID.to_string())
    );
}

#[test]
fn invalid_password_error_is_normalized_for_login_screen() {
    assert_eq!(
        user_facing_login_error("Invalid password"),
        "Incorrect username or password"
    );
}

#[test]
fn resolve_selected_char_index_prefers_named_character() {
    let chars = vec![make_test_char(1, "Elara"), make_test_char(2, "Borin")];
    assert_eq!(resolve_selected_char_index(&chars, Some("borin")), Some(1));
}

#[test]
fn resolve_selected_char_index_falls_back_to_first_character() {
    let chars = vec![make_test_char(7, "Elara")];
    assert_eq!(
        resolve_selected_char_index(&chars, Some("missing")),
        Some(0)
    );
    assert_eq!(resolve_selected_char_index(&chars, None), Some(0));
}

#[test]
fn login_success_auto_enter_skips_charselect_when_character_exists() {
    let chars = vec![make_test_char(7, "Elara")];
    assert_eq!(
        decide_login_success_action(&chars, None, true, None),
        LoginSuccessAction {
            selected_idx: Some(0),
            enter_world_character_id: Some(7),
            next_state: Some(GameState::Loading),
        }
    );
}

#[test]
fn login_success_without_auto_enter_still_goes_to_charselect() {
    let chars = vec![make_test_char(7, "Elara")];
    assert_eq!(
        decide_login_success_action(&chars, None, false, None),
        LoginSuccessAction {
            selected_idx: Some(0),
            enter_world_character_id: None,
            next_state: Some(GameState::CharSelect),
        }
    );
}

fn live_helm_appearance() -> EquipmentAppearance {
    EquipmentAppearance {
        entries: vec![EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(9001),
            display_info_id: Some(1234),
            inventory_type: 1,
            hidden: false,
        }],
    }
}

#[test]
fn sync_selected_character_roster_entry_copies_live_equipment_appearance() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(CharacterList(vec![make_test_char(7, "Elara")]));
    app.insert_resource(SelectedCharacterId {
        character_id: Some(7),
        character_name: Some("Elara".to_string()),
    });
    app.world_mut().spawn((
        crate::networking::LocalPlayer,
        NetPlayer {
            name: "Elara".to_string(),
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
        },
        live_helm_appearance(),
    ));

    let _ = app
        .world_mut()
        .run_system_once(sync_selected_character_roster_entry);

    let char_list = app.world().resource::<CharacterList>();
    assert_eq!(char_list.0[0].equipment_appearance, live_helm_appearance());
}

#[test]
fn login_success_can_route_directly_to_charcreate() {
    let chars = vec![make_test_char(7, "Elara")];
    assert_eq!(
        decide_login_success_action(&chars, None, false, Some(GameState::CharCreate)),
        LoginSuccessAction {
            selected_idx: Some(0),
            enter_world_character_id: None,
            next_state: Some(GameState::CharCreate),
        }
    );
}

#[test]
fn enter_world_success_sets_selected_character_and_goes_to_loading() {
    let mut selected = SelectedCharacterId::default();
    let char_list = CharacterList(vec![make_test_char(7, "Elara")]);
    let char_idx = crate::scenes::char_select::SelectedCharIndex(Some(0));
    let mut next_state = NextState::<GameState>::default();

    handle_enter_world_response(
        EnterWorldResponse {
            success: true,
            player_entity: Some(42),
            error: None,
        },
        &mut selected,
        &char_list,
        &char_idx,
        &GameState::CharSelect,
        None,
        &mut next_state,
    );

    assert_eq!(selected.character_id, Some(7));
    assert_eq!(selected.character_name.as_deref(), Some("Elara"));
    assert!(matches!(next_state, NextState::Pending(GameState::Loading)));
}

#[test]
fn login_failure_despawns_live_client() {
    let mut app = build_login_test_app(Vec::new(), Default::default(), false);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(GameState::Connecting);
    let client = app.world_mut().spawn(Client).id();

    let resp = LoginResponse {
        success: false,
        token: String::new(),
        characters: Vec::new(),
        error: Some("Invalid password".to_string()),
    };
    run_handle_login_response(&mut app, resp);
    app.update();
    app.world_mut()
        .insert_resource(crate::networking::NetworkUpdateFrame(1));
    crate::networking::flush_pending_network_world_reset(app.world_mut());
    app.update();

    assert!(app.world().get_entity(client).is_err());
    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Login
    );
    assert_eq!(
        app.world().resource::<AuthUiFeedback>().0.as_deref(),
        Some("Incorrect username or password")
    );
}

#[test]
fn banned_login_failure_preserves_server_message() {
    let result = run_login_response_for_test(
        LoginResponse {
            success: false,
            token: String::new(),
            characters: Vec::new(),
            error: Some("Account banned: cheating".to_string()),
        },
        Vec::new(),
        crate::networking::ReconnectState::default(),
        false,
    );

    assert_eq!(
        result.feedback.0.as_deref(),
        Some("Account banned: cheating")
    );
    assert!(result.goes_login);
}

#[test]
fn login_success_auto_enter_falls_back_to_charselect_when_list_is_empty() {
    assert_eq!(
        decide_login_success_action(&[], None, true, None),
        LoginSuccessAction {
            selected_idx: None,
            enter_world_character_id: None,
            next_state: Some(GameState::CharSelect),
        }
    );
}

#[test]
fn reconnect_login_failure_clears_reconnect_state() {
    let result = run_login_response_for_test(
        LoginResponse {
            success: false,
            token: String::new(),
            characters: Vec::new(),
            error: Some("invalid password".to_string()),
        },
        Vec::new(),
        crate::networking::ReconnectState {
            phase: crate::networking::ReconnectPhase::AwaitingWorld,
            terrain_refresh_seen: false,
        },
        false,
    );

    assert_eq!(
        result.feedback.0.as_deref(),
        Some("Incorrect username or password")
    );
    assert!(result.goes_login);
    assert!(!result.goes_charselect);
    assert_eq!(
        result.reconnect.phase,
        crate::networking::ReconnectPhase::Inactive
    );
}

#[test]
fn reconnect_login_fallback_to_charselect_clears_reconnect_state() {
    let result = run_login_response_for_test(
        LoginResponse {
            success: true,
            token: VALID_TEST_UUID.to_string(),
            characters: Vec::new(),
            error: None,
        },
        Vec::new(),
        crate::networking::ReconnectState {
            phase: crate::networking::ReconnectPhase::AwaitingWorld,
            terrain_refresh_seen: false,
        },
        true,
    );

    assert!(!result.goes_login);
    assert!(result.goes_charselect);
    assert!(!result.goes_loading);
    assert_eq!(
        result.reconnect.phase,
        crate::networking::ReconnectPhase::Inactive
    );
}

#[test]
fn auto_enter_login_success_goes_to_loading() {
    let result = run_login_response_for_test(
        LoginResponse {
            success: true,
            token: VALID_TEST_UUID.to_string(),
            characters: vec![make_test_char(7, "Elara")],
            error: None,
        },
        Vec::new(),
        crate::networking::ReconnectState::default(),
        true,
    );

    assert!(!result.goes_login);
    assert!(!result.goes_charselect);
    assert!(result.goes_loading);
}

#[test]
fn auth_token_path_worldofosso_uses_default() {
    let default_path = auth_token_path(None);
    let woo_path = auth_token_path(Some("game.worldofosso.com:5000"));
    let bare_path = auth_token_path(Some("worldofosso.com:5000"));
    assert_eq!(default_path, woo_path);
    assert_eq!(default_path, bare_path);
    assert!(default_path.ends_with("data/auth_token"));
}

#[test]
fn auth_token_path_local_server_is_separate() {
    let path = auth_token_path(Some("127.0.0.1:5000"));
    assert!(path.ends_with("data/auth_token.127.0.0.1_5000"));
}

#[test]
fn is_worldofosso_host_matches_subdomains() {
    assert!(is_worldofosso_host("worldofosso.com:5000"));
    assert!(is_worldofosso_host("game.worldofosso.com:5000"));
    assert!(is_worldofosso_host("GAME.WORLDOFOSSO.COM:5000"));
    assert!(!is_worldofosso_host("127.0.0.1:5000"));
    assert!(!is_worldofosso_host("localhost:5000"));
    assert!(!is_worldofosso_host("notworldofosso.com:5000"));
}
