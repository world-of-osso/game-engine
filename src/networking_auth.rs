use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AuthChannel, CharacterListEntry, CharacterListUpdate, CreateCharacterResponse,
    DeleteCharacterResponse, EnterWorldResponse, LoginRequest, LoginResponse, RegisterRequest,
    RegisterResponse, SelectCharacter,
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

const AUTH_TOKEN_FILE: &str = "auth_token";
const AUTH_TOKEN_DIR: &str = "data";
const TEST_PLACEHOLDER_UUID: &str = "11111111-1111-1111-1111-111111111111";

/// Token file path keyed by server address.
/// `*.worldofosso.com` → `data/auth_token` (shared), others → `data/auth_token.<host>_<port>`.
fn auth_token_path(server: Option<&str>) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(AUTH_TOKEN_DIR);
    match server {
        Some(addr) if !is_worldofosso_host(addr) => {
            let sanitized = addr.replace(':', "_").replace('/', "_");
            base.join(format!("{AUTH_TOKEN_FILE}.{sanitized}"))
        }
        _ => base.join(AUTH_TOKEN_FILE),
    }
}

fn is_worldofosso_host(addr: &str) -> bool {
    let host = addr.split(':').next().unwrap_or(addr);
    host.eq_ignore_ascii_case("worldofosso.com")
        || host.to_ascii_lowercase().ends_with(".worldofosso.com")
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
            auth_token_path(None).display()
        );
        return None;
    }

    if token == TEST_PLACEHOLDER_UUID {
        warn!(
            "Ignoring cached auth token from {}: found test sentinel UUID",
            auth_token_path(None).display()
        );
        return None;
    }

    Some(token.to_string())
}

pub fn load_auth_token(server: Option<&str>) -> Option<String> {
    std::fs::read_to_string(auth_token_path(server))
        .ok()
        .and_then(|token| normalize_auth_token(&token))
}

fn save_auth_token(token: &str, server: Option<&str>) {
    let path = auth_token_path(server);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, token) {
        warn!("Failed to save auth token to {}: {e}", path.display());
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
    if request.token.is_some() && username.0.trim().is_empty() && password.0.trim().is_empty() {
        info!("Auth flow: attempting token login with saved session");
    } else if !username.0.trim().is_empty() && !password.0.trim().is_empty() {
        info!(
            "Auth flow: attempting credential login for '{}'",
            username.0
        );
    }
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
    startup_screen_target: Option<Res<crate::game_state::StartupScreenTarget>>,
    mut selected_char_idx: ResMut<crate::char_select::SelectedCharIndex>,
    mut select_senders: Query<&mut MessageSender<SelectCharacter>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut reconnect: Option<ResMut<crate::networking::ReconnectState>>,
    server_hostname: Option<Res<crate::networking::ServerHostname>>,
    mut commands: Commands,
) {
    let server = server_hostname.as_ref().map(|h| h.0.as_str());
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            handle_login_response(
                resp,
                &mut auth_token,
                &mut auth_feedback,
                &mut char_list,
                auto_enter_world.as_ref(),
                preselected.as_ref(),
                startup_screen_target.as_ref(),
                &mut selected_char_idx,
                &mut select_senders,
                &mut next_state,
                reconnect.as_deref_mut(),
                server,
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
    startup_screen_target: Option<&Res<crate::game_state::StartupScreenTarget>>,
    selected_char_idx: &mut crate::char_select::SelectedCharIndex,
    select_senders: &mut Query<&mut MessageSender<SelectCharacter>>,
    next_state: &mut NextState<GameState>,
    reconnect: Option<&mut crate::networking::ReconnectState>,
    server: Option<&str>,
    commands: &mut Commands,
) {
    if resp.success {
        save_auth_token(&resp.token, server);
        info!(
            "Auth flow: saved new auth token to {}",
            auth_token_path(server).display()
        );
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
            startup_screen_target.map(|target| target.0),
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
    startup_screen_target: Option<GameState>,
) -> LoginSuccessAction {
    let selected_idx = resolve_selected_char_index(characters, preselected_name);
    let enter_world_character_id = auto_enter_world
        .then(|| selected_idx.and_then(|idx| characters.get(idx).map(|ch| ch.character_id)))
        .flatten();
    let next_state = if enter_world_character_id.is_some() {
        Some(GameState::Loading)
    } else {
        Some(startup_screen_target.unwrap_or(GameState::CharSelect))
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

pub fn receive_character_list_update(
    mut receivers: Query<&mut MessageReceiver<CharacterListUpdate>>,
    mut char_list: ResMut<CharacterList>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            if let Some(existing) = char_list
                .0
                .iter_mut()
                .find(|entry| entry.character_id == update.character.character_id)
            {
                *existing = update.character.clone();
            } else {
                char_list.0.push(update.character.clone());
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
    server_hostname: Option<Res<crate::networking::ServerHostname>>,
) {
    let server = server_hostname.as_ref().map(|h| h.0.as_str());
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            handle_register_response(
                resp,
                &mut auth_token,
                &mut auth_feedback,
                &mut char_list,
                &mut next_state,
                server,
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
    server: Option<&str>,
) {
    if resp.success {
        save_auth_token(&resp.token, server);
        info!(
            "Auth flow: saved new auth token to {}",
            auth_token_path(server).display()
        );
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
                }
                next_state.set(GameState::CharSelect);
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
#[path = "networking_auth_tests.rs"]
mod tests;
