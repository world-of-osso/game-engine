use bevy::prelude::*;

use game_engine::ui::registry::FrameRegistry;

use crate::game_state::GameState;
use crate::networking;

use super::helpers::{get_editbox_text, set_editbox_text};
use super::{
    DEFAULT_SERVER_ADDR, LoginStatus, LoginUi, STATUS_CONNECTING, STATUS_FILL_FIELDS,
    STATUS_RECONNECT_UNAVAILABLE,
};

pub fn prefill_offline_credentials(reg: &mut FrameRegistry, login: &LoginUi) {
    set_editbox_text(reg, login.username_input, "admin");
    set_editbox_text(reg, login.password_input, "admin");
}

pub fn try_connect(
    reg: &FrameRegistry,
    login: &LoginUi,
    status: &mut LoginStatus,
    next_state: &mut NextState<GameState>,
    mode: &networking::LoginMode,
    server_addr: Option<std::net::SocketAddr>,
    commands: &mut Commands,
) {
    let username = get_editbox_text(reg, login.username_input);
    let password = get_editbox_text(reg, login.password_input);
    if username.trim().is_empty() || password.trim().is_empty() {
        status.0 = STATUS_FILL_FIELDS.to_string();
        return;
    }
    commands.insert_resource(networking::ServerAddr(
        server_addr.unwrap_or_else(|| DEFAULT_SERVER_ADDR.parse().unwrap()),
    ));
    commands.insert_resource(networking::LoginUsername(username));
    commands.insert_resource(networking::LoginPassword(password));
    commands.insert_resource(*mode);
    status.0 = STATUS_CONNECTING.to_string();
    next_state.set(GameState::Connecting);
}

pub fn try_reconnect(
    auth_token: &networking::AuthToken,
    status: &mut LoginStatus,
    next_state: &mut NextState<GameState>,
    login_mode: &mut networking::LoginMode,
    server_addr: Option<std::net::SocketAddr>,
    commands: &mut Commands,
) {
    if auth_token
        .0
        .as_deref()
        .is_none_or(|token| token.trim().is_empty())
    {
        status.0 = STATUS_RECONNECT_UNAVAILABLE.to_string();
        return;
    }
    *login_mode = networking::LoginMode::Login;
    commands.insert_resource(networking::ServerAddr(
        server_addr.unwrap_or_else(|| DEFAULT_SERVER_ADDR.parse().unwrap()),
    ));
    commands.insert_resource(networking::LoginUsername(String::new()));
    commands.insert_resource(networking::LoginPassword(String::new()));
    commands.insert_resource(networking::LoginMode::Login);
    status.0 = STATUS_CONNECTING.to_string();
    next_state.set(GameState::Connecting);
}

pub fn toggle_login_mode(
    mode: &mut networking::LoginMode,
    reg: &mut FrameRegistry,
    login: &LoginUi,
) {
    *mode = match mode {
        networking::LoginMode::Login => networking::LoginMode::Register,
        networking::LoginMode::Register => networking::LoginMode::Login,
    };
    sync_button_states(reg, login, mode, &networking::AuthToken(None));
}

pub fn sync_button_states(
    reg: &mut FrameRegistry,
    login: &LoginUi,
    _mode: &networking::LoginMode,
    _auth_token: &networking::AuthToken,
) {
    reg.set_hidden(login.connect_button, false);
    if let Some(reconnect_button) = login.reconnect_button {
        reg.set_hidden(reconnect_button, true);
    }
}
