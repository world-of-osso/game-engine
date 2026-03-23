use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AuthChannel, CharacterListEntry, CreateCharacterResponse, DeleteCharacterResponse,
    EnterWorldResponse, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse,
    SelectCharacter,
};
use std::path::PathBuf;

use crate::game_state::GameState;

/// Persisted auth token for reconnection.
#[derive(Resource)]
pub struct AuthToken(pub Option<String>);

/// Pending auth feedback to surface when the login screen is shown again.
#[derive(Resource, Default, Clone)]
pub struct AuthUiFeedback(pub Option<String>);

/// Character list populated by LoginResponse.
#[derive(Resource, Default)]
pub struct CharacterList(pub Vec<CharacterListEntry>);

/// Info about the selected character, set when entering the world.
#[derive(Resource, Default)]
pub struct SelectedCharacterId {
    /// DB character_id (for looking up stats from CharacterList).
    pub character_id: Option<u64>,
    /// Character name (for matching against replicated NetPlayer entities).
    pub character_name: Option<String>,
}

/// Username captured from the login screen.
#[derive(Resource, Default)]
pub struct LoginUsername(pub String);

/// Password captured from the login screen (cleared after sending).
#[derive(Resource, Default)]
pub struct LoginPassword(pub String);

/// Whether the user is logging in or registering a new account.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoginMode {
    #[default]
    Login,
    Register,
}

const AUTH_TOKEN_PATH: &str = "data/auth_token";
const TEST_PLACEHOLDER_UUID: &str = "11111111-1111-1111-1111-111111111111";

fn auth_token_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(AUTH_TOKEN_PATH)
}

pub(crate) fn token_debug_label(token: Option<&str>) -> String {
    match token.map(str::trim) {
        Some("") | None => "none".to_string(),
        Some(token) => {
            let prefix: String = token.chars().take(8).collect();
            format!("present(len={}, prefix={prefix})", token.len())
        }
    }
}

fn normalize_auth_token(token: &str) -> Option<String> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    if token.eq_ignore_ascii_case("saved-token") {
        warn!(
            "Ignoring cached auth token from {}: found test placeholder value",
            auth_token_path().display()
        );
        return None;
    }

    if token == TEST_PLACEHOLDER_UUID {
        warn!(
            "Ignoring cached auth token from {}: found test sentinel UUID",
            auth_token_path().display()
        );
        return None;
    }

    Some(token.to_string())
}

pub fn load_auth_token() -> Option<String> {
    std::fs::read_to_string(auth_token_path())
        .ok()
        .and_then(|token| normalize_auth_token(&token))
}

fn save_auth_token(token: &str) {
    let path = auth_token_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, token) {
        warn!("Failed to save auth token: {e}");
    }
}

/// Send LoginRequest or RegisterRequest depending on mode.
pub fn send_auth_request(
    auth_token: &AuthToken,
    username: &LoginUsername,
    password: &LoginPassword,
    mode: &LoginMode,
    login_senders: &mut Query<&mut MessageSender<LoginRequest>>,
    register_senders: &mut Query<&mut MessageSender<RegisterRequest>>,
) {
    match mode {
        LoginMode::Login => send_login(auth_token, username, password, login_senders),
        LoginMode::Register => send_register(username, password, register_senders),
    }
}

fn send_login(
    auth_token: &AuthToken,
    username: &LoginUsername,
    password: &LoginPassword,
    senders: &mut Query<&mut MessageSender<LoginRequest>>,
) {
    let request = build_login_request(auth_token, username, password);
    let request_token_label = token_debug_label(request.token.as_deref());
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
    }
    info!(
        "Sent LoginRequest username='{}' password_present={} token={}",
        username.0,
        !password.0.is_empty(),
        request_token_label,
    );
}

fn build_login_request(
    auth_token: &AuthToken,
    username: &LoginUsername,
    password: &LoginPassword,
) -> LoginRequest {
    let token = if username.0.trim().is_empty() && password.0.trim().is_empty() {
        auth_token.0.as_deref().and_then(normalize_auth_token)
    } else {
        None
    };
    LoginRequest {
        token,
        username: username.0.clone(),
        password: password.0.clone(),
    }
}

fn send_register(
    username: &LoginUsername,
    password: &LoginPassword,
    senders: &mut Query<&mut MessageSender<RegisterRequest>>,
) {
    let request = RegisterRequest {
        username: username.0.clone(),
        password: password.0.clone(),
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
    }
    info!("Sent RegisterRequest for '{}'", username.0);
}

/// Handle LoginResponse: save token, populate character list, transition state.
pub fn receive_login_response(
    mut receivers: Query<&mut MessageReceiver<LoginResponse>>,
    mut auth_token: ResMut<AuthToken>,
    mut auth_feedback: ResMut<AuthUiFeedback>,
    mut char_list: ResMut<CharacterList>,
    auto_enter_world: Option<Res<crate::char_select::AutoEnterWorld>>,
    preselected: Option<Res<crate::char_select::PreselectedCharName>>,
    mut selected_char_idx: ResMut<crate::char_select::SelectedCharIndex>,
    mut select_senders: Query<&mut MessageSender<SelectCharacter>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut reconnect: Option<ResMut<crate::networking::ReconnectState>>,
    mut commands: Commands,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            handle_login_response(
                resp,
                &mut auth_token,
                &mut auth_feedback,
                &mut char_list,
                auto_enter_world.as_ref(),
                preselected.as_ref(),
                &mut selected_char_idx,
                &mut select_senders,
                &mut next_state,
                reconnect.as_deref_mut(),
                &mut commands,
            );
        }
    }
}

fn handle_login_response(
    resp: LoginResponse,
    auth_token: &mut AuthToken,
    auth_feedback: &mut AuthUiFeedback,
    char_list: &mut CharacterList,
    auto_enter_world: Option<&Res<crate::char_select::AutoEnterWorld>>,
    preselected: Option<&Res<crate::char_select::PreselectedCharName>>,
    selected_char_idx: &mut crate::char_select::SelectedCharIndex,
    select_senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    next_state: &mut NextState<GameState>,
    reconnect: Option<&mut crate::networking::ReconnectState>,
    commands: &mut Commands,
) {
    if resp.success {
        save_auth_token(&resp.token);
        auth_token.0 = Some(resp.token);
        auth_feedback.0 = None;
        char_list.0 = resp.characters;
        info!(
            "Login success, {} characters, token={}",
            char_list.0.len(),
            token_debug_label(auth_token.0.as_deref()),
        );
        let action = decide_login_success_action(
            &char_list.0,
            preselected.map(|name| name.0.as_str()),
            auto_enter_world.is_some(),
        );
        info!(
            "Post-login action: selected_idx={:?} auto_enter={} enter_world_character_id={:?} next_state={:?} preselected={:?}",
            action.selected_idx,
            auto_enter_world.is_some(),
            action.enter_world_character_id,
            action.next_state,
            preselected.map(|name| name.0.as_str()),
        );
        selected_char_idx.0 = action.selected_idx;
        if let Some(character_id) = action.enter_world_character_id {
            send_enter_world(character_id, select_senders);
            commands.remove_resource::<crate::char_select::AutoEnterWorld>();
        }
        if let Some(state) = action.next_state {
            clear_reconnect_if_not_entering_world(reconnect, true);
            next_state.set(state);
        }
    } else {
        let err = resp.error.unwrap_or_default();
        error!(
            "Login failed: {err}; preselected={:?} auto_enter={}",
            preselected.map(|name| name.0.as_str()),
            auto_enter_world.is_some(),
        );
        commands.queue(crate::networking::reset_network_world);
        auth_feedback.0 = Some(user_facing_login_error(&err).to_string());
        clear_reconnect_if_not_entering_world(reconnect, false);
        next_state.set(GameState::Login);
    }
}

fn clear_reconnect_if_not_entering_world(
    reconnect: Option<&mut crate::networking::ReconnectState>,
    login_succeeded: bool,
) {
    let Some(reconnect) = reconnect else { return };
    if reconnect.is_active() {
        if login_succeeded {
            info!("Reconnect fallback stayed out of world; hiding reconnect overlay");
        } else {
            warn!("Reconnect login failed; hiding reconnect overlay");
        }
        reconnect.phase = crate::networking::ReconnectPhase::Inactive;
        reconnect.terrain_refresh_seen = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoginSuccessAction {
    selected_idx: Option<usize>,
    enter_world_character_id: Option<u64>,
    next_state: Option<GameState>,
}

fn decide_login_success_action(
    characters: &[CharacterListEntry],
    preselected_name: Option<&str>,
    auto_enter_world: bool,
) -> LoginSuccessAction {
    let selected_idx = resolve_selected_char_index(characters, preselected_name);
    let enter_world_character_id = auto_enter_world
        .then(|| selected_idx.and_then(|idx| characters.get(idx).map(|ch| ch.character_id)))
        .flatten();
    let next_state = if enter_world_character_id.is_some() {
        None
    } else {
        Some(GameState::CharSelect)
    };
    LoginSuccessAction {
        selected_idx,
        enter_world_character_id,
        next_state,
    }
}

fn send_enter_world(
    character_id: u64,
    select_senders: &mut Query<&mut MessageSender<SelectCharacter>>,
) {
    let msg = SelectCharacter { character_id };
    for mut sender in select_senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
}

/// Handle CreateCharacterResponse: append new character to list.
pub fn receive_create_character_response(
    mut receivers: Query<&mut MessageReceiver<CreateCharacterResponse>>,
    mut char_list: ResMut<CharacterList>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                if let Some(ch) = resp.character {
                    info!("Created character '{}'", ch.name);
                    char_list.0.push(ch);
                }
            } else {
                let err = resp.error.unwrap_or_default();
                error!("Create character failed: {err}");
            }
        }
    }
}

/// Handle DeleteCharacterResponse: remove character from list.
pub fn receive_delete_character_response(
    mut receivers: Query<&mut MessageReceiver<DeleteCharacterResponse>>,
    mut char_list: ResMut<CharacterList>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                char_list.0.retain(|c| c.character_id != resp.character_id);
                info!("Deleted character {}", resp.character_id);
            } else {
                let err = resp.error.unwrap_or_default();
                error!("Delete character failed: {err}");
            }
        }
    }
}

/// Handle RegisterResponse: save token and transition on success.
pub fn receive_register_response(
    mut receivers: Query<&mut MessageReceiver<RegisterResponse>>,
    mut auth_token: ResMut<AuthToken>,
    mut auth_feedback: ResMut<AuthUiFeedback>,
    mut char_list: ResMut<CharacterList>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            handle_register_response(
                resp,
                &mut auth_token,
                &mut auth_feedback,
                &mut char_list,
                &mut next_state,
            );
        }
    }
}

fn handle_register_response(
    resp: RegisterResponse,
    auth_token: &mut AuthToken,
    auth_feedback: &mut AuthUiFeedback,
    char_list: &mut CharacterList,
    next_state: &mut NextState<GameState>,
) {
    if resp.success {
        save_auth_token(&resp.token);
        auth_token.0 = Some(resp.token);
        auth_feedback.0 = None;
        char_list.0.clear();
        info!("Registration success, transitioning to CharSelect");
        next_state.set(GameState::CharSelect);
    } else {
        let err = resp.error.unwrap_or_default();
        error!("Registration failed: {err}");
        auth_feedback.0 = Some(err);
        next_state.set(GameState::Login);
    }
}

fn user_facing_login_error(err: &str) -> &str {
    let normalized = err.trim().to_ascii_lowercase();
    if normalized.contains("invalid")
        || normalized.contains("incorrect")
        || normalized.contains("wrong password")
        || normalized.contains("bad password")
        || normalized.contains("credentials")
        || normalized.contains("password")
    {
        "Incorrect username or password"
    } else {
        "Login failed. Please try again."
    }
}

fn resolve_selected_char_index(
    characters: &[CharacterListEntry],
    preselected_name: Option<&str>,
) -> Option<usize> {
    preselected_name
        .and_then(|name| {
            characters
                .iter()
                .position(|ch| ch.name.eq_ignore_ascii_case(name))
        })
        .or_else(|| characters.first().map(|_| 0))
}

/// Handle EnterWorldResponse: store selected character info and transition to Loading.
pub fn receive_enter_world_response(
    mut receivers: Query<&mut MessageReceiver<EnterWorldResponse>>,
    mut selected: ResMut<SelectedCharacterId>,
    char_list: Res<CharacterList>,
    char_idx: Res<crate::char_select::SelectedCharIndex>,
    state: Res<State<GameState>>,
    mut reconnect: Option<ResMut<crate::networking::ReconnectState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                apply_enter_world(&mut selected, &char_list, &char_idx);
                if reconnect
                    .as_deref()
                    .is_some_and(|reconnect| reconnect.is_active())
                    && *state.get() == GameState::InWorld
                {
                    info!("Reconnect enter-world accepted, waiting for replicated world state");
                } else {
                    next_state.set(GameState::Loading);
                }
            } else {
                let err = resp.error.unwrap_or_default();
                error!("Enter world failed: {err}");
                if let Some(ref mut reconnect) = reconnect
                    && reconnect.is_active()
                {
                    reconnect.phase = crate::networking::ReconnectPhase::Inactive;
                    reconnect.terrain_refresh_seen = false;
                    next_state.set(GameState::CharSelect);
                }
            }
        }
    }
}

fn apply_enter_world(
    selected: &mut SelectedCharacterId,
    char_list: &CharacterList,
    char_idx: &crate::char_select::SelectedCharIndex,
) {
    if let Some(entry) = char_idx.0.and_then(|i| char_list.0.get(i)) {
        selected.character_id = Some(entry.character_id);
        selected.character_name = Some(entry.name.clone());
    }
    info!("Entering world as {:?}", selected.character_name);
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TEST_UUID: &str = "22222222-2222-2222-2222-222222222222";
    use bevy::ecs::system::RunSystemOnce;
    use lightyear::prelude::client::Client;

    fn run_login_response_for_test(
        resp: LoginResponse,
        chars: Vec<CharacterListEntry>,
        reconnect: crate::networking::ReconnectState,
        auto_enter: bool,
    ) -> (
        AuthUiFeedback,
        bool,
        bool,
        crate::networking::ReconnectState,
    ) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(AuthToken(None));
        app.init_resource::<AuthUiFeedback>();
        app.insert_resource(CharacterList(chars));
        app.init_resource::<crate::char_select::SelectedCharIndex>();
        app.init_resource::<NextState<GameState>>();
        app.insert_resource(reconnect);
        if auto_enter {
            app.insert_resource(crate::char_select::AutoEnterWorld);
        }

        let _ = app.world_mut().run_system_once(
            move |mut auth_token: ResMut<AuthToken>,
                  mut auth_feedback: ResMut<AuthUiFeedback>,
                  mut char_list: ResMut<CharacterList>,
                  mut selected_char_idx: ResMut<crate::char_select::SelectedCharIndex>,
                  mut next_state: ResMut<NextState<GameState>>,
                  mut select_senders: Query<&mut MessageSender<SelectCharacter>>,
                  mut reconnect: ResMut<crate::networking::ReconnectState>,
                  mut commands: Commands| {
                handle_login_response(
                    resp.clone(),
                    &mut auth_token,
                    &mut auth_feedback,
                    &mut char_list,
                    None,
                    None,
                    &mut selected_char_idx,
                    &mut select_senders,
                    &mut next_state,
                    Some(&mut reconnect),
                    &mut commands,
                );
            },
        );
        app.update();

        (
            app.world().resource::<AuthUiFeedback>().clone(),
            matches!(
                app.world().resource::<NextState<GameState>>(),
                NextState::Pending(GameState::Login)
            ),
            matches!(
                app.world().resource::<NextState<GameState>>(),
                NextState::Pending(GameState::CharSelect)
            ),
            *app.world().resource::<crate::networking::ReconnectState>(),
        )
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
        let chars = vec![
            CharacterListEntry {
                character_id: 1,
                name: "Elara".to_string(),
                level: 1,
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            CharacterListEntry {
                character_id: 2,
                name: "Borin".to_string(),
                level: 1,
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
        ];

        assert_eq!(resolve_selected_char_index(&chars, Some("borin")), Some(1));
    }

    #[test]
    fn resolve_selected_char_index_falls_back_to_first_character() {
        let chars = vec![CharacterListEntry {
            character_id: 7,
            name: "Elara".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
        }];

        assert_eq!(
            resolve_selected_char_index(&chars, Some("missing")),
            Some(0)
        );
        assert_eq!(resolve_selected_char_index(&chars, None), Some(0));
    }

    #[test]
    fn login_success_auto_enter_skips_charselect_when_character_exists() {
        let chars = vec![CharacterListEntry {
            character_id: 7,
            name: "Elara".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
        }];

        assert_eq!(
            decide_login_success_action(&chars, None, true),
            LoginSuccessAction {
                selected_idx: Some(0),
                enter_world_character_id: Some(7),
                next_state: None,
            }
        );
    }

    #[test]
    fn login_success_without_auto_enter_still_goes_to_charselect() {
        let chars = vec![CharacterListEntry {
            character_id: 7,
            name: "Elara".to_string(),
            level: 1,
            race: 1,
            class: 1,
            appearance: shared::components::CharacterAppearance::default(),
        }];

        assert_eq!(
            decide_login_success_action(&chars, None, false),
            LoginSuccessAction {
                selected_idx: Some(0),
                enter_world_character_id: None,
                next_state: Some(GameState::CharSelect),
            }
        );
    }

    #[test]
    fn login_failure_despawns_live_client() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_state(GameState::Connecting);
        app.insert_resource(AuthToken(None));
        app.init_resource::<AuthUiFeedback>();
        app.init_resource::<CharacterList>();
        app.init_resource::<crate::char_select::SelectedCharIndex>();
        app.init_resource::<NextState<GameState>>();
        app.init_resource::<crate::networking::ReconnectState>();

        let client = app.world_mut().spawn(Client::default()).id();

        let _ = app.world_mut().run_system_once(
            move |mut auth_token: ResMut<AuthToken>,
                  mut auth_feedback: ResMut<AuthUiFeedback>,
                  mut char_list: ResMut<CharacterList>,
                  mut selected_char_idx: ResMut<crate::char_select::SelectedCharIndex>,
                  mut next_state: ResMut<NextState<GameState>>,
                  mut select_senders: Query<&mut MessageSender<SelectCharacter>>,
                  mut reconnect: ResMut<crate::networking::ReconnectState>,
                  mut commands: Commands| {
                handle_login_response(
                    LoginResponse {
                        success: false,
                        token: String::new(),
                        characters: Vec::new(),
                        error: Some("Invalid password".to_string()),
                    },
                    &mut auth_token,
                    &mut auth_feedback,
                    &mut char_list,
                    None,
                    None,
                    &mut selected_char_idx,
                    &mut select_senders,
                    &mut next_state,
                    Some(&mut reconnect),
                    &mut commands,
                );
            },
        );
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
            decide_login_success_action(&[], None, true),
            LoginSuccessAction {
                selected_idx: None,
                enter_world_character_id: None,
                next_state: Some(GameState::CharSelect),
            }
        );
    }

    #[test]
    fn reconnect_login_failure_clears_reconnect_state() {
        let (feedback, goes_login, goes_charselect, reconnect) = run_login_response_for_test(
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
            feedback.0.as_deref(),
            Some("Incorrect username or password")
        );
        assert!(goes_login);
        assert!(!goes_charselect);
        assert_eq!(reconnect.phase, crate::networking::ReconnectPhase::Inactive);
    }

    #[test]
    fn reconnect_login_fallback_to_charselect_clears_reconnect_state() {
        let (_feedback, goes_login, goes_charselect, reconnect) = run_login_response_for_test(
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

        assert!(!goes_login);
        assert!(goes_charselect);
        assert_eq!(reconnect.phase, crate::networking::ReconnectPhase::Inactive);
    }
}
