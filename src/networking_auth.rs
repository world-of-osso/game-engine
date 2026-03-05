use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AuthChannel, CharacterListEntry, CreateCharacterResponse, DeleteCharacterResponse,
    EnterWorldResponse, LoginRequest, LoginResponse,
};

use crate::game_state::GameState;

/// Persisted auth token for reconnection.
#[derive(Resource)]
pub struct AuthToken(pub Option<String>);

/// Character list populated by LoginResponse.
#[derive(Resource, Default)]
pub struct CharacterList(pub Vec<CharacterListEntry>);

/// Entity bits of the selected character, set on EnterWorldResponse.
#[derive(Resource, Default)]
pub struct SelectedCharacterId(pub Option<u64>);

/// Username captured from the login screen.
#[derive(Resource, Default)]
pub struct LoginUsername(pub String);

const AUTH_TOKEN_PATH: &str = "data/auth_token";

pub fn load_auth_token() -> Option<String> {
    std::fs::read_to_string(AUTH_TOKEN_PATH).ok().filter(|s| !s.trim().is_empty())
}

fn save_auth_token(token: &str) {
    if let Err(e) = std::fs::write(AUTH_TOKEN_PATH, token) {
        warn!("Failed to save auth token: {e}");
    }
}

/// Send LoginRequest when connection is established.
pub fn send_login_request(
    auth_token: Res<AuthToken>,
    username: Res<LoginUsername>,
    mut senders: Query<&mut MessageSender<LoginRequest>>,
) {
    let request = LoginRequest {
        token: auth_token.0.clone(),
        username: username.0.clone(),
    };
    for mut sender in senders.iter_mut() {
        sender.send::<AuthChannel>(request.clone());
    }
    info!("Sent LoginRequest for '{}'", username.0);
}

/// Handle LoginResponse: save token, populate character list, transition state.
pub fn receive_login_response(
    mut receivers: Query<&mut MessageReceiver<LoginResponse>>,
    mut auth_token: ResMut<AuthToken>,
    mut char_list: ResMut<CharacterList>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            handle_login_response(resp, &mut auth_token, &mut char_list, &mut next_state);
        }
    }
}

fn handle_login_response(
    resp: LoginResponse,
    auth_token: &mut AuthToken,
    char_list: &mut CharacterList,
    next_state: &mut NextState<GameState>,
) {
    if resp.success {
        save_auth_token(&resp.token);
        auth_token.0 = Some(resp.token);
        char_list.0 = resp.characters;
        info!("Login success, {} characters", char_list.0.len());
        next_state.set(GameState::CharSelect);
    } else {
        let err = resp.error.unwrap_or_default();
        error!("Login failed: {err}");
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

/// Handle EnterWorldResponse: store player entity bits and transition to Loading.
pub fn receive_enter_world_response(
    mut receivers: Query<&mut MessageReceiver<EnterWorldResponse>>,
    mut selected: ResMut<SelectedCharacterId>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for mut receiver in receivers.iter_mut() {
        for resp in receiver.receive() {
            if resp.success {
                selected.0 = resp.player_entity;
                info!("Entering world, player entity: {:?}", resp.player_entity);
                next_state.set(GameState::Loading);
            } else {
                let err = resp.error.unwrap_or_default();
                error!("Enter world failed: {err}");
            }
        }
    }
}
