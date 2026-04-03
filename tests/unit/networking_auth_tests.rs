use super::*;

use bevy::ecs::system::RunSystemOnce;
use lightyear::prelude::client::Client;
use shared::components::{
    EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Player as NetPlayer,
};

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
    let _ = app.world_mut().run_system_once(
        move |mut auth_token: ResMut<AuthToken>,
              mut auth_feedback: ResMut<AuthUiFeedback>,
              mut char_list: ResMut<CharacterList>,
              auto_enter_world: Option<Res<crate::scenes::char_select::AutoEnterWorld>>,
              mut selected_char_idx: ResMut<crate::scenes::char_select::SelectedCharIndex>,
              mut next_state: ResMut<NextState<GameState>>,
              mut select_senders: Query<&mut MessageSender<SelectCharacter>>,
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
    );
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
fn login_failure_despawns_live_client() {
    let mut app = build_login_test_app(Vec::new(), Default::default(), false);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(GameState::Connecting);
    let client = app.world_mut().spawn(Client::default()).id();

    let resp = LoginResponse {
        success: false,
        token: String::new(),
        characters: Vec::new(),
        error: Some("Invalid password".to_string()),
    };
    run_handle_login_response(&mut app, resp);
    app.update();
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
