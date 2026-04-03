use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use std::sync::Arc;

use game_engine::ui::automation::{UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::frame::{Dimension, NineSlice, WidgetData};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use ui_toolkit::screen::Screen;

use game_engine::ui::screens::login_component::{
    CONNECT_BUTTON, CREATE_ACCOUNT_BUTTON, EXIT_BUTTON, LOGIN_ROOT, LOGIN_STATUS, LoginAction,
    MENU_BUTTON, PASSWORD_INPUT, RECONNECT_BUTTON, SharedConnecting, SharedStatusText,
    USERNAME_INPUT, login_screen,
};
use game_engine::ui::widgets::button::ButtonState as BtnState;
use game_engine::ui::widgets::font_string::GameFont;
use game_engine::ui::widgets::texture::TextureSource;
use game_engine::ui_resource;

use crate::game_state::GameState;
use crate::networking;

mod connect;
pub mod helpers;

use connect::{prefill_offline_credentials, toggle_login_mode, try_reconnect};
pub(crate) use connect::{sync_button_states, try_connect};
use helpers::{
    editbox_backspace, editbox_cursor_end, editbox_cursor_home, editbox_delete,
    editbox_move_cursor, hit_frame, insert_char_into_editbox, insert_text_into_editbox,
    set_login_primary_button_textures,
};

const FADE_IN_DURATION: f32 = 0.75;
pub(crate) const DEFAULT_SERVER_ADDR: &str = crate::cli_args::DEFAULT_SERVER_ADDR;
const GLUE_EDITBOX_TEXT_COLOR: [f32; 4] = [1.0, 0.8, 0.2, 1.0];
const EDITBOX_BG: [f32; 4] = [0.22, 0.16, 0.11, 1.0];
const EDITBOX_BORDER: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BG: [f32; 4] = [0.32, 0.24, 0.16, 1.0];
const EDITBOX_FOCUSED_BORDER: [f32; 4] = [1.0, 0.78, 0.0, 1.0];
pub(crate) const STATUS_CONNECTING: &str = "Connecting...";
pub(crate) const STATUS_FILL_FIELDS: &str = "Please fill in all fields";
pub(crate) const STATUS_RECONNECT_UNAVAILABLE: &str = "No saved session to reconnect";

/// Grouped UI state params used by mouse/keyboard input systems.
#[derive(bevy::ecs::system::SystemParam)]
struct LoginUiParams<'w> {
    ui: ResMut<'w, UiState>,
    login_ui: Option<Res<'w, LoginUi>>,
    focus: ResMut<'w, LoginFocus>,
    pressed: ResMut<'w, LoginPressedButton>,
}

/// Grouped system params for login connect/automation actions.
#[derive(bevy::ecs::system::SystemParam)]
struct LoginConnectParams<'w, 's> {
    next_state: ResMut<'w, NextState<GameState>>,
    status: ResMut<'w, LoginStatus>,
    login_mode: ResMut<'w, networking::LoginMode>,
    auth_token: Res<'w, networking::AuthToken>,
    server_addr: Option<Res<'w, networking::ServerAddr>>,
    server_hostname: Option<Res<'w, networking::ServerHostname>>,
    commands: Commands<'w, 's>,
}

ui_resource! {
    pub(crate) LoginUi {
        root: LOGIN_ROOT,
        username_input: USERNAME_INPUT,
        password_input: PASSWORD_INPUT,
        connect_button: CONNECT_BUTTON,
        create_account_button: CREATE_ACCOUNT_BUTTON,
        menu_button: MENU_BUTTON,
        exit_button: EXIT_BUTTON,
        status_text: LOGIN_STATUS,
        reconnect_button?: RECONNECT_BUTTON,
    }
}

#[derive(Resource, Default)]
pub(crate) struct LoginFocus(pub(crate) Option<u64>);

/// Tracks which button is currently pressed (mouse-down) for visual feedback.
#[derive(Resource, Default)]
struct LoginPressedButton(Option<u64>);

#[derive(Resource, Default)]
struct LoginModifierState {
    ctrl: bool,
    super_key: bool,
}

#[derive(Resource, Default)]
pub(crate) struct LoginStatus(pub(crate) String);

#[derive(Resource)]
pub(crate) struct DevServer;

#[derive(Resource)]
struct LoginFadeIn(f32);

#[derive(Resource, Clone)]
struct LoginClipboard(Arc<dyn Fn() -> Result<String, String> + Send + Sync>);

impl Default for LoginClipboard {
    fn default() -> Self {
        Self(Arc::new(|| {
            let mut clipboard =
                arboard::Clipboard::new().map_err(|e| format!("clipboard init: {e}"))?;
            clipboard
                .get_text()
                .map_err(|e| format!("clipboard read: {e}"))
        }))
    }
}

impl LoginClipboard {
    fn read_text(&self) -> Result<String, String> {
        (self.0)()
    }
}

struct LoginScreenRes {
    screen: Screen,
    shared: ui_toolkit::screen::SharedContext,
}
// SAFETY: Screen contains non-Send/Sync types (Box<dyn Fn> + mpsc::Receiver + Any), but
// login systems run exclusively on the main thread so this is safe.
unsafe impl Send for LoginScreenRes {}
unsafe impl Sync for LoginScreenRes {}

#[derive(Resource)]
struct LoginScreenResWrap(LoginScreenRes);

pub struct LoginScreenPlugin;

impl Plugin for LoginScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginFocus>();
        app.init_resource::<LoginStatus>();
        app.init_resource::<LoginPressedButton>();
        app.init_resource::<LoginModifierState>();
        app.init_resource::<LoginClipboard>();
        app.add_systems(OnEnter(GameState::Login), build_login_ui);
        app.add_systems(OnExit(GameState::Login), teardown_login_ui);
        app.add_systems(
            Update,
            (
                login_sync_root_size,
                login_mouse_input,
                login_keyboard_input,
                login_run_automation,
                login_update_visuals,
                login_fade_in,
            )
                .into_configs()
                .run_if(in_state(GameState::Login)),
        );
    }
}

pub(crate) fn build_login_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut status: ResMut<LoginStatus>,
    mut auth_feedback: ResMut<networking::AuthUiFeedback>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    dev_server: Option<Res<DevServer>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    status.0 = auth_feedback.0.take().unwrap_or_default();

    let mut res = build_login_screen(&status);
    res.screen.sync(&res.shared, &mut ui.registry);

    let login = LoginUi::resolve(&ui.registry);
    apply_post_setup(&mut ui.registry, &login);

    if dev_server.is_some() {
        prefill_offline_credentials(&mut ui.registry, &login);
    }

    // Auto-focus: password if username is pre-filled, otherwise username
    let username_text = helpers::get_editbox_text(&ui.registry, login.username_input);
    let auto_focus = if username_text.is_empty() {
        login.username_input
    } else {
        login.password_input
    };
    commands.insert_resource(LoginFocus(Some(auto_focus)));

    ui.registry.set_alpha(login.root, 0.0);
    commands.insert_resource(LoginFadeIn(0.1));
    commands.insert_resource(LoginScreenResWrap(res));
    commands.insert_resource(login);
}

fn build_login_screen(status: &LoginStatus) -> LoginScreenRes {
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert::<SharedStatusText>(status.0.clone());
    shared.insert::<SharedConnecting>(false);
    let screen = Screen::new(login_screen);

    LoginScreenRes { screen, shared }
}

fn apply_post_setup(reg: &mut FrameRegistry, login: &LoginUi) {
    let (sw, sh) = (reg.screen_width, reg.screen_height);
    if let Some(frame) = reg.get_mut(login.root) {
        frame.width = Dimension::Fixed(sw);
        frame.height = Dimension::Fixed(sh);
    }
    set_editbox_backdrop(reg, login.username_input);
    set_editbox_backdrop(reg, login.password_input);
    set_login_primary_button_textures(reg, login.connect_button);
    if let Some(reconnect_button) = login.reconnect_button {
        set_login_primary_button_textures(reg, reconnect_button);
    }
}

fn teardown_login_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut screen: Option<ResMut<LoginScreenResWrap>>,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<LoginScreenResWrap>();
    commands.remove_resource::<LoginUi>();
    commands.remove_resource::<LoginFadeIn>();
    ui.focused_frame = None;
}

fn set_editbox_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.nine_slice = Some(NineSlice {
            edge_size: 8.0,
            part_textures: Some(common_input_border_part_textures()),
            bg_color: EDITBOX_BG,
            border_color: EDITBOX_BORDER,
            ..Default::default()
        });
        if let Some(WidgetData::EditBox(eb)) = &mut frame.widget_data {
            eb.text_insets = [12.0, 5.0, 8.0, 8.0];
            eb.font = GameFont::ArialNarrow;
            eb.text_color = GLUE_EDITBOX_TEXT_COLOR;
        }
    }
}

fn common_input_border_part_textures() -> [TextureSource; 9] {
    let base = "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-";
    [
        TextureSource::File(format!("{base}TL.blp")),
        TextureSource::File(format!("{base}T.blp")),
        TextureSource::File(format!("{base}TR.blp")),
        TextureSource::File(format!("{base}L.blp")),
        TextureSource::File(format!("{base}M.blp")),
        TextureSource::File(format!("{base}R.blp")),
        TextureSource::File(format!("{base}BL.blp")),
        TextureSource::File(format!("{base}B.blp")),
        TextureSource::File(format!("{base}BR.blp")),
    ]
}

fn sync_editbox_focus_visual(reg: &mut FrameRegistry, id: u64, focused: bool) {
    let Some(frame) = reg.get_mut(id) else { return };
    let Some(nine_slice) = frame.nine_slice.as_mut() else {
        return;
    };
    if focused {
        nine_slice.bg_color = EDITBOX_FOCUSED_BG;
        nine_slice.border_color = EDITBOX_FOCUSED_BORDER;
    } else {
        nine_slice.bg_color = EDITBOX_BG;
        nine_slice.border_color = EDITBOX_BORDER;
    }
}

fn hit_active_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry
        .get(frame_id)
        .is_some_and(|frame| frame.visible && !frame.hidden)
        && hit_frame(ui, frame_id, mx, my)
}

/// Shared connect/action parameters passed to button click and key handlers.
struct ConnectParams<'a> {
    next_state: &'a mut NextState<GameState>,
    status: &'a mut LoginStatus,
    login_mode: &'a mut networking::LoginMode,
    auth_token: &'a networking::AuthToken,
    server_addr: Option<std::net::SocketAddr>,
    server_hostname: Option<&'a str>,
}

fn login_sync_root_size(mut ui: ResMut<UiState>, login_ui: Option<Res<LoginUi>>) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    let sw = ui.registry.screen_width;
    let sh = ui.registry.screen_height;
    if let Some(root) = ui.registry.get_mut(login.root)
        && ((root.width.value() - sw).abs() > 0.5 || (root.height.value() - sh).abs() > 0.5)
    {
        root.width = Dimension::Fixed(sw);
        root.height = Dimension::Fixed(sh);
        if let Some(rect) = &mut root.layout_rect {
            rect.width = sw;
            rect.height = sh;
        }
    }
}

fn login_mouse_input(
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
        let released_id = lp.pressed.0.take();
        if let Some(id) = released_id {
            reset_button_state(&mut lp.ui.registry, id);
        }
        if let (Some(id), Some(cursor)) = (released_id, cursor)
            && hit_active_frame(&lp.ui, id, cursor.x, cursor.y)
        {
            let mut params = ConnectParams {
                next_state: &mut cp.next_state,
                status: &mut cp.status,
                login_mode: &mut cp.login_mode,
                auth_token: &cp.auth_token,
                server_addr: cp.server_addr.as_ref().map(|addr| addr.0),
                server_hostname: cp
                    .server_hostname
                    .as_ref()
                    .map(|hostname| hostname.0.as_str()),
            };
            handle_button_click(
                &mut lp.ui,
                login,
                cursor,
                &mut lp.focus,
                &mut params,
                &mut cp.commands,
                Some(&mut exit),
            );
        }
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
    let button_ids = button_ids(login);
    for id in button_ids {
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
        Some(LoginAction::Connect) => try_connect(
            &ui.registry,
            login,
            params.status,
            params.next_state,
            params.login_mode,
            params.server_addr,
            params.server_hostname,
            commands,
        ),
        Some(LoginAction::Reconnect) => try_reconnect(
            params.auth_token,
            params.status,
            params.next_state,
            params.login_mode,
            params.server_addr,
            params.server_hostname,
            commands,
        ),
        Some(LoginAction::CreateAccount) => {
            toggle_login_mode(params.login_mode, &mut ui.registry, login);
            params.status.0.clear();
        }
        Some(LoginAction::Menu) => {
            crate::game_menu_screen::open_game_menu(
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
        None => {
            focus.0 = None;
        }
    }
}

fn set_button_pushed(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.state = BtnState::Pushed;
    }
}

fn login_keyboard_input(
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
        server_addr: cp.server_addr.as_ref().map(|addr| addr.0),
        server_hostname: cp
            .server_hostname
            .as_ref()
            .map(|hostname| hostname.0.as_str()),
    };
    handle_login_key(event.key_code, focused_id, ui, key_params, &mut cp.commands);
}

fn maybe_paste_into_login_editbox(
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
    }
    true
}

fn maybe_insert_login_text(event: &KeyboardInput, ui: &mut UiState, focused_id: u64) -> bool {
    let Some(text) = &event.text else {
        return false;
    };
    insert_char_into_editbox(&mut ui.registry, focused_id, text.as_str());
    true
}

fn is_paste_shortcut(modifiers: &LoginModifierState, event: &KeyboardInput) -> bool {
    matches!(event.logical_key, Key::Paste)
        || (event.key_code == KeyCode::KeyV && (modifiers.ctrl || modifiers.super_key))
}

fn login_run_automation(
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    mut focus: ResMut<LoginFocus>,
    mut cp: LoginConnectParams,
    mut queue: ResMut<UiAutomationQueue>,
    mut runner: ResMut<UiAutomationRunner>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    let Some(action) = queue.peek().cloned() else {
        return;
    };
    if !action.is_input_action() {
        return;
    }
    let result = run_login_automation_action(
        &mut ui,
        login,
        &mut focus,
        &mut cp.next_state,
        &mut cp.status,
        &mut cp.login_mode,
        &cp.auth_token,
        cp.server_addr.as_ref().map(|addr| addr.0),
        cp.server_hostname
            .as_ref()
            .map(|hostname| hostname.0.as_str()),
        &mut cp.commands,
        &action,
    );
    queue.pop();
    if let Err(err) = result {
        runner.last_error = Some(err.clone());
        cp.status.0 = err;
    }
}

pub(crate) use connect::run_login_automation_action;

fn handle_nav_key(key: KeyCode, focus: &mut LoginFocus, login: &LoginUi) -> bool {
    match key {
        KeyCode::Tab => {
            focus.0 = Some(cycle_focus(focus.0, login));
            true
        }
        KeyCode::Escape => {
            focus.0 = None;
            true
        }
        _ => false,
    }
}

fn cycle_focus(current: Option<u64>, login: &LoginUi) -> u64 {
    let fields = [login.username_input, login.password_input];
    let idx = current
        .and_then(|id| fields.iter().position(|&f| f == id))
        .map(|i| (i + 1) % fields.len())
        .unwrap_or(0);
    fields[idx]
}

pub(crate) struct LoginKeyParams<'a> {
    pub(crate) login: &'a LoginUi,
    pub(crate) status: &'a mut LoginStatus,
    pub(crate) next_state: &'a mut NextState<GameState>,
    pub(crate) mode: &'a networking::LoginMode,
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
        KeyCode::Enter => try_connect(
            &ui.registry,
            login,
            status,
            next_state,
            mode,
            server_addr,
            server_hostname,
            commands,
        ),
        _ => {}
    }
}

fn login_update_visuals(
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    mut screen_res: Option<ResMut<LoginScreenResWrap>>,
    status: Res<LoginStatus>,
    focus: Res<LoginFocus>,
    login_mode: Res<networking::LoginMode>,
    auth_token: Res<networking::AuthToken>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    ui.focused_frame = focus.0;
    sync_button_states(&mut ui.registry, login, &login_mode, &auth_token, &status);
    sync_login_status(&mut ui.registry, screen_res.as_mut(), &status);
    sync_editbox_focus_visual(
        &mut ui.registry,
        login.username_input,
        focus.0 == Some(login.username_input),
    );
    sync_editbox_focus_visual(
        &mut ui.registry,
        login.password_input,
        focus.0 == Some(login.password_input),
    );
}

fn sync_login_status(
    reg: &mut FrameRegistry,
    screen_res: Option<&mut ResMut<LoginScreenResWrap>>,
    status: &LoginStatus,
) {
    let Some(res) = screen_res else { return };
    let inner = &mut res.0;
    let connecting = status.0 == STATUS_CONNECTING;
    inner.shared.insert::<SharedStatusText>(status.0.clone());
    inner.shared.insert::<SharedConnecting>(connecting);
    inner.screen.sync(&inner.shared, reg);
}

fn login_fade_in(
    time: Res<Time>,
    mut fade: Option<ResMut<LoginFadeIn>>,
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
) {
    let (Some(fade), Some(login)) = (fade.as_mut(), login_ui.as_ref()) else {
        return;
    };
    fade.0 = (fade.0 + time.delta_secs()).min(FADE_IN_DURATION);
    let alpha = if FADE_IN_DURATION <= 0.0 {
        1.0
    } else {
        fade.0 / FADE_IN_DURATION
    };
    ui.registry.set_alpha(login.root, alpha);
}

#[cfg(test)]
#[path = "../../../tests/unit/login_screen_tests.rs"]
mod tests;
