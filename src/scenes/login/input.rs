use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use game_engine::ui::frame::WidgetData;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::login_component::LoginAction;
use game_engine::ui::widgets::button::ButtonState as BtnState;

use super::helpers::{
    editbox_backspace, editbox_cursor_end, editbox_cursor_home, editbox_delete,
    editbox_move_cursor, hit_frame, insert_text_into_editbox,
};
use super::*;

/// Shared connect/action parameters passed to button click and key handlers.
struct ConnectParams<'a> {
    next_state: &'a mut NextState<GameState>,
    status: &'a mut LoginStatus,
    login_mode: &'a mut networking::LoginMode,
    auth_token: &'a networking::AuthToken,
    realm_selection: Option<&'a mut LoginRealmSelection>,
    server_addr: Option<std::net::SocketAddr>,
    server_hostname: Option<&'a str>,
}

pub(crate) fn selected_login_server(
    realm_selection: Option<&LoginRealmSelection>,
    server_addr: Option<std::net::SocketAddr>,
    server_hostname: Option<&str>,
) -> Result<(std::net::SocketAddr, String), String> {
    if let Some(selection) = realm_selection {
        return Ok((selection.server_addr()?, selection.server_hostname()));
    }
    let addr = server_addr.unwrap_or_else(connect::resolve_default_server);
    let hostname = server_hostname.unwrap_or(DEFAULT_SERVER_ADDR).to_string();
    Ok((addr, hostname))
}

pub(super) fn login_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut lp: LoginUiParams,
    mut cp: LoginConnectParams,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(login) = lp.login_ui.as_ref() else {
        return;
    };
    let cursor = windows.iter().next().and_then(|w| w.cursor_position());

    if buttons.just_pressed(MouseButton::Left)
        && let Some(cursor) = cursor
    {
        handle_mouse_press(&mut lp.ui, login, cursor, &mut lp.focus, &mut lp.pressed);
    }

    if buttons.just_released(MouseButton::Left) {
        handle_mouse_release(
            &mut lp.ui,
            &mut lp.focus,
            &mut lp.pressed,
            login,
            &mut cp,
            cursor,
            &mut exit,
        );
    }
}

fn handle_mouse_release(
    ui: &mut UiState,
    focus: &mut LoginFocus,
    pressed: &mut LoginPressedButton,
    login: &LoginUi,
    cp: &mut LoginConnectParams,
    cursor: Option<Vec2>,
    exit: &mut MessageWriter<AppExit>,
) {
    let released_id = pressed.0.take();
    if let Some(id) = released_id {
        reset_button_state(&mut ui.registry, id);
    }
    if let (Some(id), Some(cursor)) = (released_id, cursor)
        && hit_active_frame(ui, id, cursor.x, cursor.y)
    {
        let mut params = ConnectParams {
            next_state: &mut cp.next_state,
            status: &mut cp.status,
            login_mode: &mut cp.login_mode,
            auth_token: &cp.auth_token,
            realm_selection: cp.realm_selection.as_deref_mut(),
            server_addr: cp.server_addr.as_ref().map(|addr| addr.0),
            server_hostname: cp
                .server_hostname
                .as_ref()
                .map(|hostname| hostname.0.as_str()),
        };
        handle_button_click(
            ui,
            login,
            cursor,
            focus,
            &mut params,
            &mut cp.commands,
            Some(exit),
        );
    }
}

fn handle_mouse_press(
    ui: &mut UiState,
    login: &LoginUi,
    cursor: Vec2,
    focus: &mut LoginFocus,
    pressed: &mut LoginPressedButton,
) {
    let (cx, cy) = (cursor.x, cursor.y);
    for &id in &[login.username_input, login.password_input] {
        if hit_frame(ui, id, cx, cy) {
            ui.registry.click_frame(id);
            focus.0 = ui.registry.focused_frame;
            return;
        }
    }
    for id in button_ids(login) {
        if hit_active_frame(ui, id, cx, cy) {
            set_button_pushed(&mut ui.registry, id);
            pressed.0 = Some(id);
            return;
        }
    }
    focus.0 = None;
}

fn button_ids(login: &LoginUi) -> Vec<u64> {
    let mut ids = vec![
        login.realm_button,
        login.connect_button,
        login.create_account_button,
        login.menu_button,
        login.exit_button,
    ];
    if let Some(id) = login.reconnect_button {
        ids.push(id);
    }
    ids
}

fn reset_button_state(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.state = BtnState::Normal;
    }
}

fn handle_button_click(
    ui: &mut UiState,
    login: &LoginUi,
    cursor: Vec2,
    focus: &mut LoginFocus,
    params: &mut ConnectParams<'_>,
    commands: &mut Commands,
    exit: Option<&mut MessageWriter<AppExit>>,
) {
    let clicked_id = button_ids(login)
        .into_iter()
        .find(|&id| hit_active_frame(ui, id, cursor.x, cursor.y));
    let action = clicked_id
        .and_then(|id| ui.registry.get(id))
        .and_then(|f| f.onclick.clone());
    dispatch_login_action(ui, login, focus, params, commands, exit, action.as_deref());
}

fn dispatch_login_action(
    ui: &mut UiState,
    login: &LoginUi,
    focus: &mut LoginFocus,
    params: &mut ConnectParams<'_>,
    commands: &mut Commands,
    exit: Option<&mut MessageWriter<AppExit>>,
    action: Option<&str>,
) {
    match action.and_then(LoginAction::parse) {
        Some(LoginAction::Connect) => dispatch_connect_action(ui, login, params, commands),
        Some(LoginAction::Reconnect) => dispatch_reconnect_action(params, commands),
        other => handle_local_login_action(ui, login, focus, params, commands, exit, other),
    }
}

fn dispatch_connect_action(
    ui: &mut UiState,
    login: &LoginUi,
    params: &mut ConnectParams<'_>,
    commands: &mut Commands,
) {
    match selected_login_server(
        params.realm_selection.as_deref(),
        params.server_addr,
        params.server_hostname,
    ) {
        Ok((server_addr, server_hostname)) => try_connect(
            &mut ui.registry,
            login,
            params.status,
            params.next_state,
            params.login_mode,
            Some(server_addr),
            Some(server_hostname.as_str()),
            commands,
        ),
        Err(err) => params.status.0 = err,
    }
}

fn dispatch_reconnect_action(params: &mut ConnectParams<'_>, commands: &mut Commands) {
    match selected_login_server(
        params.realm_selection.as_deref(),
        params.server_addr,
        params.server_hostname,
    ) {
        Ok((server_addr, server_hostname)) => try_reconnect(
            params.auth_token,
            params.status,
            params.next_state,
            params.login_mode,
            Some(server_addr),
            Some(server_hostname.as_str()),
            commands,
        ),
        Err(err) => params.status.0 = err,
    }
}

fn handle_local_login_action(
    ui: &mut UiState,
    login: &LoginUi,
    focus: &mut LoginFocus,
    params: &mut ConnectParams<'_>,
    commands: &mut Commands,
    exit: Option<&mut MessageWriter<AppExit>>,
    action: Option<LoginAction>,
) {
    match action {
        Some(LoginAction::CycleRealm) => {
            let Some(realm_selection) = params.realm_selection.as_deref_mut() else {
                params.status.0 = "Realm selection is unavailable".to_string();
                return;
            };
            realm_selection.cycle();
            if let Err(err) = apply_login_realm_resources(commands, realm_selection) {
                params.status.0 = err;
                return;
            }
            persist_login_realm_selection(realm_selection);
            params.status.0.clear();
        }
        Some(LoginAction::CreateAccount) => {
            toggle_login_mode(params.login_mode, &mut ui.registry, login);
            params.status.0.clear();
        }
        Some(LoginAction::Menu) => {
            crate::scenes::game_menu::open_game_menu(
                ui,
                commands,
                crate::game_state::GameState::Login,
            );
        }
        Some(LoginAction::Exit) => {
            if let Some(exit) = exit {
                exit.write(AppExit::Success);
            }
        }
        Some(LoginAction::Connect | LoginAction::Reconnect) => unreachable!(),
        None => focus.0 = None,
    }
}

fn set_button_pushed(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.state = BtnState::Pushed;
    }
}

pub(super) fn login_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    mut focus: ResMut<LoginFocus>,
    login_ui: Option<Res<LoginUi>>,
    clipboard: Res<LoginClipboard>,
    mut modifiers: ResMut<LoginModifierState>,
    mut cp: LoginConnectParams,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    for event in key_events.read() {
        update_login_modifiers(&mut modifiers, event);
        if event.state != ButtonState::Pressed {
            continue;
        }
        if handle_nav_key(event.key_code, &mut focus, login) {
            continue;
        }
        let Some(focused_id) = focus.0 else { continue };
        dispatch_login_key_event(
            event, &modifiers, &mut ui, focused_id, &clipboard, login, &mut cp,
        );
    }
}

fn update_login_modifiers(modifiers: &mut LoginModifierState, event: &KeyboardInput) {
    let pressed = event.state == ButtonState::Pressed;
    match event.key_code {
        KeyCode::ControlLeft | KeyCode::ControlRight => modifiers.ctrl = pressed,
        KeyCode::SuperLeft | KeyCode::SuperRight | KeyCode::Meta => modifiers.super_key = pressed,
        _ => {}
    }
    match event.logical_key {
        Key::Control => modifiers.ctrl = pressed,
        Key::Super | Key::Meta => modifiers.super_key = pressed,
        _ => {}
    }
}

fn dispatch_login_key_event(
    event: &KeyboardInput,
    modifiers: &LoginModifierState,
    ui: &mut UiState,
    focused_id: u64,
    clipboard: &LoginClipboard,
    login: &LoginUi,
    cp: &mut LoginConnectParams,
) {
    if maybe_paste_into_login_editbox(
        modifiers,
        event,
        ui,
        focused_id,
        &mut cp.status,
        clipboard.read_text(),
    ) {
        return;
    }
    if maybe_insert_login_text(event, ui, focused_id) {
        return;
    }
    let key_params = LoginKeyParams {
        login,
        status: &mut cp.status,
        next_state: &mut cp.next_state,
        mode: &cp.login_mode,
        realm_selection: cp.realm_selection.as_deref(),
        server_addr: cp.server_addr.as_ref().map(|addr| addr.0),
        server_hostname: cp
            .server_hostname
            .as_ref()
            .map(|hostname| hostname.0.as_str()),
    };
    handle_login_key(event.key_code, focused_id, ui, key_params, &mut cp.commands);
}

pub(crate) fn maybe_paste_into_login_editbox(
    modifiers: &LoginModifierState,
    event: &KeyboardInput,
    ui: &mut UiState,
    focused_id: u64,
    status: &mut LoginStatus,
    clipboard_text: Result<String, String>,
) -> bool {
    if !is_paste_shortcut(modifiers, event) {
        return false;
    }
    match clipboard_text {
        Ok(text) => {
            insert_text_into_editbox(&mut ui.registry, focused_id, &text);
        }
        Err(err) => status.0 = err,
    };
    true
}

pub(crate) fn maybe_insert_login_text(
    event: &KeyboardInput,
    ui: &mut UiState,
    focused_id: u64,
) -> bool {
    let Some(text) = &event.text else {
        return false;
    };
    insert_text_into_editbox(&mut ui.registry, focused_id, text.as_str())
}

fn is_paste_shortcut(modifiers: &LoginModifierState, event: &KeyboardInput) -> bool {
    matches!(event.logical_key, Key::Paste)
        || (event.key_code == KeyCode::KeyV && (modifiers.ctrl || modifiers.super_key))
}

pub(crate) struct LoginKeyParams<'a> {
    pub(crate) login: &'a LoginUi,
    pub(crate) status: &'a mut LoginStatus,
    pub(crate) next_state: &'a mut NextState<GameState>,
    pub(crate) mode: &'a networking::LoginMode,
    pub(crate) realm_selection: Option<&'a LoginRealmSelection>,
    pub(crate) server_addr: Option<std::net::SocketAddr>,
    pub(crate) server_hostname: Option<&'a str>,
}

pub(crate) fn handle_login_key(
    key: KeyCode,
    focused_id: u64,
    ui: &mut UiState,
    p: LoginKeyParams<'_>,
    commands: &mut Commands,
) {
    let LoginKeyParams {
        login,
        status,
        next_state,
        mode,
        realm_selection,
        server_addr,
        server_hostname,
    } = p;
    match key {
        KeyCode::Backspace => editbox_backspace(&mut ui.registry, focused_id),
        KeyCode::Delete => editbox_delete(&mut ui.registry, focused_id),
        KeyCode::ArrowLeft => editbox_move_cursor(&mut ui.registry, focused_id, -1_i32),
        KeyCode::ArrowRight => editbox_move_cursor(&mut ui.registry, focused_id, 1_i32),
        KeyCode::Home => editbox_cursor_home(&mut ui.registry, focused_id),
        KeyCode::End => editbox_cursor_end(&mut ui.registry, focused_id),
        KeyCode::Enter => {
            match selected_login_server(realm_selection, server_addr, server_hostname) {
                Ok((server_addr, server_hostname)) => try_connect(
                    &mut ui.registry,
                    login,
                    status,
                    next_state,
                    mode,
                    Some(server_addr),
                    Some(server_hostname.as_str()),
                    commands,
                ),
                Err(err) => *status = LoginStatus(err),
            }
        }
        _ => {}
    }
}
