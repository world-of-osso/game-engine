use bevy::prelude::*;
use std::sync::Arc;

use game_engine::ui::automation::{UiAutomationQueue, UiAutomationRunner};
use game_engine::ui::frame::{Dimension, NineSlice, WidgetData};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use ui_toolkit::screen::Screen;

use game_engine::ui::screens::login_component::{
    CONNECT_BUTTON, CREATE_ACCOUNT_BUTTON, EXIT_BUTTON, LOGIN_ROOT, LOGIN_STATUS, MENU_BUTTON,
    PASSWORD_INPUT, REALM_BUTTON, RECONNECT_BUTTON, SharedConnecting, SharedRealmSelectable,
    SharedRealmText, SharedStatusText, USERNAME_INPUT, login_screen,
};
use game_engine::ui::widgets::font_string::GameFont;
use game_engine::ui::widgets::texture::TextureSource;
use game_engine::ui_resource;

use crate::game_state::GameState;
use crate::networking;

mod connect;
pub mod helpers;
mod input;
mod view;

#[cfg(test)]
pub(crate) use view::apply_post_setup;

use connect::{prefill_offline_credentials, toggle_login_mode, try_reconnect};
pub(crate) use connect::{sync_button_visibility, try_connect};
use helpers::{hit_frame, set_login_primary_button_textures};
pub(crate) use input::{LoginKeyParams, handle_login_key, selected_login_server};
use input::{login_keyboard_input, login_mouse_input};

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

#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct LoginRealmSelectionLock(pub bool);

#[derive(Debug, Clone, PartialEq, Eq)]
enum LoginRealmChoice {
    Preset(crate::cli_args::RealmPreset),
    Custom {
        addr: std::net::SocketAddr,
        hostname: String,
    },
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoginRealmSelection {
    choice: LoginRealmChoice,
    locked: bool,
}

impl LoginRealmSelection {
    fn from_server(
        server_addr: Option<std::net::SocketAddr>,
        server_hostname: Option<&str>,
        locked: bool,
    ) -> Self {
        let choice = match (server_addr, server_hostname) {
            (_, Some(hostname)) => crate::cli_args::realm_preset_for_hostname(hostname)
                .map(LoginRealmChoice::Preset)
                .unwrap_or_else(|| {
                    let addr = server_addr.unwrap_or_else(connect::resolve_default_server);
                    LoginRealmChoice::Custom {
                        addr,
                        hostname: hostname.to_string(),
                    }
                }),
            (Some(addr), None) => LoginRealmChoice::Custom {
                addr,
                hostname: addr.to_string(),
            },
            (None, None) => LoginRealmChoice::Preset(crate::client_options::load_preferred_realm()),
        };
        Self { choice, locked }
    }

    fn button_text(&self) -> String {
        match &self.choice {
            LoginRealmChoice::Preset(preset) => preset.label().to_string(),
            LoginRealmChoice::Custom { hostname, .. } => hostname.clone(),
        }
    }

    fn is_selectable(&self) -> bool {
        !self.locked
    }

    fn server_addr(&self) -> Result<std::net::SocketAddr, String> {
        match &self.choice {
            LoginRealmChoice::Preset(preset) => preset.to_server_arg().map(|server| server.addr),
            LoginRealmChoice::Custom { addr, .. } => Ok(*addr),
        }
    }

    fn server_hostname(&self) -> String {
        match &self.choice {
            LoginRealmChoice::Preset(preset) => preset.hostname().to_string(),
            LoginRealmChoice::Custom { hostname, .. } => hostname.clone(),
        }
    }

    fn is_dev(&self) -> bool {
        matches!(
            self.choice,
            LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Dev)
        )
    }

    fn cycle(&mut self) {
        if self.locked {
            return;
        }
        self.choice = match self.choice {
            LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Dev) => {
                LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Prod)
            }
            LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Prod)
            | LoginRealmChoice::Custom { .. } => {
                LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Dev)
            }
        };
    }

    fn selected_preset(&self) -> Option<crate::cli_args::RealmPreset> {
        match self.choice {
            LoginRealmChoice::Preset(preset) => Some(preset),
            LoginRealmChoice::Custom { .. } => None,
        }
    }
}

fn apply_login_realm_resources(
    commands: &mut Commands,
    selection: &LoginRealmSelection,
) -> Result<(), String> {
    let addr = selection.server_addr()?;
    let hostname = selection.server_hostname();
    commands.insert_resource(networking::ServerAddr(addr));
    commands.insert_resource(networking::ServerHostname(hostname.clone()));
    commands.insert_resource(networking::AuthToken(networking::load_auth_token(Some(
        hostname.as_str(),
    ))));
    Ok(())
}

#[cfg(not(test))]
fn persist_login_realm_selection(selection: &LoginRealmSelection) {
    if let Some(preset) = selection.selected_preset()
        && let Err(err) = crate::client_options::save_preferred_realm(preset)
    {
        warn!("Failed to save preferred realm '{}': {err}", preset.alias());
    }
}

#[cfg(test)]
fn persist_login_realm_selection(_selection: &LoginRealmSelection) {}

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
    realm_selection: Option<ResMut<'w, LoginRealmSelection>>,
    server_addr: Option<Res<'w, networking::ServerAddr>>,
    server_hostname: Option<Res<'w, networking::ServerHostname>>,
    commands: Commands<'w, 's>,
}

ui_resource! {
    pub(crate) LoginUi {
        root: LOGIN_ROOT,
        username_input: USERNAME_INPUT,
        password_input: PASSWORD_INPUT,
        realm_button: REALM_BUTTON,
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
    realm_selection: Option<Res<LoginRealmSelection>>,
    realm_lock: Option<Res<LoginRealmSelectionLock>>,
    server_addr: Option<Res<networking::ServerAddr>>,
    server_hostname: Option<Res<networking::ServerHostname>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    dev_server: Option<Res<DevServer>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    status.0 = auth_feedback.0.take().unwrap_or_default();

    let selection = realm_selection.as_deref().cloned().unwrap_or_else(|| {
        LoginRealmSelection::from_server(
            server_addr.as_ref().map(|addr| addr.0),
            server_hostname.as_ref().map(|hostname| hostname.0.as_str()),
            realm_lock.as_deref().is_some_and(|lock| lock.0),
        )
    });
    if let Err(err) = apply_login_realm_resources(&mut commands, &selection) {
        status.0 = err;
    }
    commands.insert_resource(selection.clone());

    let mut res =
        view::build_login_screen(&status, selection.button_text(), selection.is_selectable());
    res.screen.sync(&res.shared, &mut ui.registry);

    let login = LoginUi::resolve(&ui.registry);
    view::apply_post_setup(&mut ui.registry, &login);

    if dev_server.is_some() || selection.is_dev() {
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
        crate::scenes::login::connect::LoginAutomationContext {
            ui: &mut ui,
            login,
            focus: &mut focus,
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
            commands: &mut cp.commands,
        },
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
    let fields = [
        login.username_input,
        login.password_input,
    ];
    let idx = current
        .and_then(|id| fields.iter().position(|&f| f == id))
        .map(|i| (i + 1) % fields.len())
        .unwrap_or(0);
    fields[idx]
}

#[cfg(test)]
fn maybe_paste_into_login_editbox(
    modifiers: &LoginModifierState,
    event: &bevy::input::keyboard::KeyboardInput,
    ui: &mut UiState,
    focused_id: u64,
    status: &mut LoginStatus,
    clipboard_text: Result<String, String>,
) -> bool {
    input::maybe_paste_into_login_editbox(modifiers, event, ui, focused_id, status, clipboard_text)
}

#[cfg(test)]
fn maybe_insert_login_text(
    event: &bevy::input::keyboard::KeyboardInput,
    ui: &mut UiState,
    focused_id: u64,
) -> bool {
    input::maybe_insert_login_text(event, ui, focused_id)
}

fn login_update_visuals(
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    mut screen_res: Option<ResMut<LoginScreenResWrap>>,
    status: Res<LoginStatus>,
    realm_selection: Option<Res<LoginRealmSelection>>,
    focus: Res<LoginFocus>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    let realm_text = realm_selection
        .as_deref()
        .map(LoginRealmSelection::button_text)
        .unwrap_or_else(|| {
            crate::client_options::load_preferred_realm()
                .label()
                .to_string()
        });
    let realm_selectable = realm_selection
        .as_deref()
        .is_none_or(LoginRealmSelection::is_selectable);
    ui.focused_frame = focus.0;
    sync_button_visibility(&mut ui.registry, login);
    view::sync_login_status(
        &mut ui.registry,
        screen_res.as_mut(),
        &status,
        realm_text,
        realm_selectable,
    );
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
