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
    DEFAULT_SERVER_ADDR, LoginFocus, LoginKeyParams, LoginStatus, LoginUi, STATUS_CONNECTING,
    STATUS_FILL_FIELDS, STATUS_RECONNECT_UNAVAILABLE, handle_login_key,
};

pub(crate) fn resolve_default_server() -> std::net::SocketAddr {
    DEFAULT_SERVER_ADDR
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
        .unwrap_or_else(|| "127.0.0.1:5000".parse().unwrap())
}

pub fn prefill_offline_credentials(reg: &mut FrameRegistry, login: &LoginUi) {
    let creds = crate::client_options::load_login_credentials().unwrap_or(
        crate::client_options::LoginCredentials {
            username: "admin".to_string(),
            password: "admin".to_string(),
        },
    );
    set_editbox_text(reg, login.username_input, &creds.username);
    set_editbox_text(reg, login.password_input, &creds.password);
}

pub fn try_connect(
    reg: &FrameRegistry,
    login: &LoginUi,
    status: &mut LoginStatus,
    next_state: &mut NextState<GameState>,
    mode: &networking::LoginMode,
    server_addr: Option<std::net::SocketAddr>,
    server_hostname: Option<&str>,
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
    let resolved = server_addr.unwrap_or_else(resolve_default_server);
    commands.insert_resource(networking::ServerAddr(resolved));
    commands.insert_resource(networking::ServerHostname(
        server_hostname.unwrap_or(DEFAULT_SERVER_ADDR).to_string(),
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
    server_hostname: Option<&str>,
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
    let resolved = server_addr.unwrap_or_else(resolve_default_server);
    commands.insert_resource(networking::ServerAddr(resolved));
    commands.insert_resource(networking::ServerHostname(
        server_hostname.unwrap_or(DEFAULT_SERVER_ADDR).to_string(),
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

pub(crate) struct LoginAutomationContext<'a, 'w, 's> {
    pub(crate) ui: &'a mut UiState,
    pub(crate) login: &'a LoginUi,
    pub(crate) focus: &'a mut LoginFocus,
    pub(crate) next_state: &'a mut NextState<GameState>,
    pub(crate) status: &'a mut LoginStatus,
    pub(crate) login_mode: &'a mut networking::LoginMode,
    pub(crate) auth_token: &'a networking::AuthToken,
    pub(crate) server_addr: Option<std::net::SocketAddr>,
    pub(crate) server_hostname: Option<&'a str>,
    pub(crate) commands: &'a mut Commands<'w, 's>,
}

pub fn run_login_automation_action(
    ctx: LoginAutomationContext<'_, '_, '_>,
    action: &UiAutomationAction,
) -> Result<(), String> {
    match action {
        UiAutomationAction::ClickFrame(name) => click_login_frame(ctx, name),
        UiAutomationAction::TypeText(text) => type_login_automation_text(ctx, text),
        UiAutomationAction::PressKey(key) => press_login_automation_key(ctx, *key),
        _ => Ok(()),
    }
}

fn type_login_automation_text(
    ctx: LoginAutomationContext<'_, '_, '_>,
    text: &str,
) -> Result<(), String> {
    let LoginAutomationContext { ui, focus, .. } = ctx;
    let fid = focus
        .0
        .ok_or("automation type requires a focused edit box")?;
    for ch in text.chars() {
        insert_char_into_editbox(&mut ui.registry, fid, &ch.to_string());
    }
    Ok(())
}

fn press_login_automation_key(
    ctx: LoginAutomationContext<'_, '_, '_>,
    key: bevy::input::keyboard::KeyCode,
) -> Result<(), String> {
    let LoginAutomationContext {
        ui,
        login,
        focus,
        next_state,
        status,
        login_mode,
        server_addr,
        server_hostname,
        commands,
        ..
    } = ctx;
    let fid = focus
        .0
        .ok_or("automation key press requires a focused frame")?;
    let p = LoginKeyParams {
        login,
        status,
        next_state,
        mode: login_mode,
        server_addr,
        server_hostname,
    };
    handle_login_key(key, fid, ui, p, commands);
    Ok(())
}

fn click_login_frame(
    ctx: LoginAutomationContext<'_, '_, '_>,
    frame_name: &str,
) -> Result<(), String> {
    let LoginAutomationContext {
        ui,
        login,
        focus,
        next_state,
        status,
        login_mode,
        auth_token,
        server_addr,
        server_hostname,
        commands,
    } = ctx;
    let (frame_id, action) = resolve_clicked_login_frame(ui, focus, frame_name)?;
    dispatch_click(
        LoginAutomationContext {
            ui,
            login,
            focus,
            next_state,
            status,
            login_mode,
            auth_token,
            server_addr,
            server_hostname,
            commands,
        },
        action.as_deref(),
        frame_name,
        frame_id,
    )
}

fn resolve_clicked_login_frame(
    ui: &mut UiState,
    focus: &mut LoginFocus,
    frame_name: &str,
) -> Result<(u64, Option<String>), String> {
    recompute_layouts(&mut ui.registry);
    let frame_id = ui
        .registry
        .get_by_name(frame_name)
        .ok_or_else(|| format!("unknown login frame '{frame_name}'"))?;
    let action = ui.registry.click_frame(frame_id);
    focus.0 = ui.registry.focused_frame;
    Ok((frame_id, action))
}

fn dispatch_click(
    ctx: LoginAutomationContext<'_, '_, '_>,
    action: Option<&str>,
    frame_name: &str,
    frame_id: u64,
) -> Result<(), String> {
    let LoginAutomationContext {
        ui,
        login,
        focus: _,
        next_state,
        status,
        login_mode,
        auth_token,
        server_addr,
        server_hostname,
        commands,
    } = ctx;
    match action.and_then(LoginAction::parse) {
        Some(LoginAction::Connect) => try_connect(
            &ui.registry,
            login,
            status,
            next_state,
            login_mode,
            server_addr,
            server_hostname,
            commands,
        ),
        Some(LoginAction::Reconnect) => try_reconnect(
            auth_token,
            status,
            next_state,
            login_mode,
            server_addr,
            server_hostname,
            commands,
        ),
        Some(LoginAction::CreateAccount) => {
            toggle_login_mode(login_mode, &mut ui.registry, login);
            status.0.clear();
        }
        Some(LoginAction::Menu) => {
            crate::scenes::game_menu::open_game_menu(ui, commands, GameState::Login);
        }
        Some(LoginAction::Exit) => {}
        None if ui.registry.focused_frame == Some(frame_id) => {}
        _ => return Err(format!("login frame '{frame_name}' has no onclick action")),
    }
    Ok(())
}
