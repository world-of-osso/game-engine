use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AuthChannel, CharacterListEntry, CreateCharacterResponse, DeleteCharacterResponse,
    EnterWorldResponse, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse,
    SelectCharacter,
};

use crate::game_state::GameState;

/// Persisted auth token for reconnection.
#[derive(Resource)]
pub struct AuthToken(pub Option<String>);

/// Pending auth feedback to surface when the login screen is shown again.
#[derive(Resource, Default)]
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
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum LoginMode {
    #[default]
    Login,
    Register,
}

const AUTH_TOKEN_PATH: &str = "data/auth_token";

pub fn load_auth_token() -> Option<String> {
    std::fs::read_to_string(AUTH_TOKEN_PATH)
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn save_auth_token(token: &str) {
    let _ = std::fs::create_dir_all("data");
    if let Err(e) = std::fs::write(AUTH_TOKEN_PATH, token) {
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
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
    }
    info!("Sent LoginRequest for '{}'", username.0);
}

fn build_login_request(
    auth_token: &AuthToken,
    username: &LoginUsername,
    password: &LoginPassword,
) -> LoginRequest {
    LoginRequest {
        token: auth_token.0.clone(),
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
    commands: &mut Commands,
) {
    if resp.success {
        save_auth_token(&resp.token);
        auth_token.0 = Some(resp.token);
        auth_feedback.0 = None;
        char_list.0 = resp.characters;
        info!("Login success, {} characters", char_list.0.len());
        let selected_idx =
            resolve_selected_char_index(&char_list.0, preselected.map(|name| name.0.as_str()));
        selected_char_idx.0 = selected_idx;
        if auto_enter_world.is_some()
            && try_auto_enter_world(selected_idx, &char_list.0, select_senders)
        {
            commands.remove_resource::<crate::char_select::AutoEnterWorld>();
        } else {
            next_state.set(GameState::CharSelect);
        }
    } else {
        let err = resp.error.unwrap_or_default();
        error!("Login failed: {err}");
        auth_feedback.0 = Some(user_facing_login_error(&err).to_string());
        next_state.set(GameState::Login);
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

fn try_auto_enter_world(
    selected_idx: Option<usize>,
    characters: &[CharacterListEntry],
    select_senders: &mut Query<&mut MessageSender<SelectCharacter>>,
) -> bool {
    let Some(character) = selected_idx.and_then(|idx| characters.get(idx)) else {
        return false;
    };
    let msg = SelectCharacter {
        character_id: character.character_id,
    };
    for mut sender in select_senders.iter_mut() {
        sender.send::<AuthChannel>(msg.clone());
    }
    info!("Auto-enter requested for '{}'", character.name);
    true
}

/// Handle EnterWorldResponse: store selected character info and transition to Loading.
pub fn receive_enter_world_response(
    mut receivers: Query<&mut MessageReceiver<EnterWorldResponse>>,
    mut selected: ResMut<SelectedCharacterId>,
    char_list: Res<CharacterList>,
    char_idx: Res<crate::char_select::SelectedCharIndex>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                apply_enter_world(&mut selected, &char_list, &char_idx);
                next_state.set(GameState::Loading);
            } else {
                let err = resp.error.unwrap_or_default();
                error!("Enter world failed: {err}");
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

    #[test]
    fn build_login_request_keeps_credentials_for_password_login() {
        let request = build_login_request(
            &AuthToken(Some("saved-token".to_string())),
            &LoginUsername("alice".to_string()),
            &LoginPassword("secret".to_string()),
        );

        assert_eq!(request.token.as_deref(), Some("saved-token"));
        assert_eq!(request.username, "alice");
        assert_eq!(request.password, "secret");
    }

    #[test]
    fn build_login_request_allows_token_only_login() {
        let request = build_login_request(
            &AuthToken(Some("saved-token".to_string())),
            &LoginUsername(String::new()),
            &LoginPassword(String::new()),
        );

        assert_eq!(request.token.as_deref(), Some("saved-token"));
        assert!(request.username.is_empty());
        assert!(request.password.is_empty());
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
}
