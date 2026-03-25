use bevy::prelude::*;

use game_engine::ui::automation::UiAutomationAction;
use game_engine::ui::layout::recompute_layouts;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::login_component::LoginAction;

use crate::game_state::GameState;
use crate::networking;

use game_engine::ui::frame::WidgetData;
use game_engine::ui::widgets::button::ButtonState as BtnState;

use std::net::ToSocketAddrs;

use super::helpers::{get_editbox_text, insert_char_into_editbox, set_editbox_text};
use super::{
    handle_login_key, LoginFocus, LoginKeyParams, LoginStatus, LoginUi,
    DEFAULT_SERVER_ADDR, STATUS_CONNECTING, STATUS_FILL_FIELDS, STATUS_RECONNECT_UNAVAILABLE,
};

pub(crate) fn resolve_default_server() -> std::net::SocketAddr {
    DEFAULT_SERVER_ADDR
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
        .unwrap_or_else(|| "127.0.0.1:5000".parse().unwrap())
}

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
    if status.0 == STATUS_CONNECTING {
        return;
    }
    let username = get_editbox_text(reg, login.username_input);
    let password = get_editbox_text(reg, login.password_input);
    if username.trim().is_empty() || password.trim().is_empty() {
        status.0 = STATUS_FILL_FIELDS.to_string();
        return;
    }
    commands.insert_resource(networking::ServerAddr(
        server_addr.unwrap_or_else(resolve_default_server),
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
        server_addr.unwrap_or_else(resolve_default_server),
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
    sync_button_states(
        reg,
        login,
        mode,
        &networking::AuthToken(None),
        &super::LoginStatus::default(),
    );
}

pub fn sync_button_states(
    reg: &mut FrameRegistry,
    login: &LoginUi,
    _mode: &networking::LoginMode,
    _auth_token: &networking::AuthToken,
    status: &LoginStatus,
) {
    reg.set_hidden(login.connect_button, false);
    if let Some(reconnect_button) = login.reconnect_button {
        reg.set_hidden(reconnect_button, true);
    }
    let connecting = status.0 == STATUS_CONNECTING;
    if let Some(WidgetData::Button(bd)) = reg
        .get_mut(login.connect_button)
        .and_then(|f| f.widget_data.as_mut())
    {
        if connecting {
            bd.state = BtnState::Disabled;
        } else if bd.state == BtnState::Disabled {
            bd.state = BtnState::Normal;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_login_automation_action(
    ui: &mut UiState,
    login: &LoginUi,
    focus: &mut LoginFocus,
    next_state: &mut NextState<GameState>,
    status: &mut LoginStatus,
    login_mode: &mut networking::LoginMode,
    auth_token: &networking::AuthToken,
    server_addr: Option<std::net::SocketAddr>,
    commands: &mut Commands,
    action: &UiAutomationAction,
) -> Result<(), String> {
    match action {
        UiAutomationAction::ClickFrame(name) => click_login_frame(
            ui, login, focus, next_state, status, login_mode, auth_token, server_addr, commands,
            name,
        ),
        UiAutomationAction::TypeText(text) => {
            let fid = focus.0.ok_or("automation type requires a focused edit box")?;
            for ch in text.chars() {
                insert_char_into_editbox(&mut ui.registry, fid, &ch.to_string());
            }
            Ok(())
        }
        UiAutomationAction::PressKey(key) => {
            let fid = focus.0.ok_or("automation key press requires a focused frame")?;
            let p = LoginKeyParams { login, status, next_state, mode: login_mode, server_addr };
            handle_login_key(*key, fid, ui, p, commands);
            Ok(())
        }
        _ => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
fn click_login_frame(
    ui: &mut UiState,
    login: &LoginUi,
    focus: &mut LoginFocus,
    next_state: &mut NextState<GameState>,
    status: &mut LoginStatus,
    login_mode: &mut networking::LoginMode,
    auth_token: &networking::AuthToken,
    server_addr: Option<std::net::SocketAddr>,
    commands: &mut Commands,
    frame_name: &str,
) -> Result<(), String> {
    recompute_layouts(&mut ui.registry);
    let frame_id = ui
        .registry
        .get_by_name(frame_name)
        .ok_or_else(|| format!("unknown login frame '{frame_name}'"))?;
    let action = ui.registry.click_frame(frame_id);
    focus.0 = ui.registry.focused_frame;
    dispatch_click(
        ui, login, next_state, status, login_mode, auth_token, server_addr, commands,
        action.as_deref(), frame_name, frame_id,
    )
}

#[allow(clippy::too_many_arguments)]
fn dispatch_click(
    ui: &mut UiState,
    login: &LoginUi,
    next_state: &mut NextState<GameState>,
    status: &mut LoginStatus,
    login_mode: &mut networking::LoginMode,
    auth_token: &networking::AuthToken,
    server_addr: Option<std::net::SocketAddr>,
    commands: &mut Commands,
    action: Option<&str>,
    frame_name: &str,
    frame_id: u64,
) -> Result<(), String> {
    match action.and_then(LoginAction::parse) {
        Some(LoginAction::Connect) => try_connect(
            &ui.registry, login, status, next_state, login_mode, server_addr, commands,
        ),
        Some(LoginAction::Reconnect) => try_reconnect(
            auth_token, status, next_state, login_mode, server_addr, commands,
        ),
        Some(LoginAction::CreateAccount) => {
            toggle_login_mode(login_mode, &mut ui.registry, login);
            status.0.clear();
        }
        Some(LoginAction::Menu) => {
            crate::game_menu_screen::open_game_menu(ui, commands, GameState::Login);
        }
        Some(LoginAction::Exit) => {}
        None if ui.registry.focused_frame == Some(frame_id) => {}
        _ => return Err(format!("login frame '{frame_name}' has no onclick action")),
    }
    Ok(())
}
