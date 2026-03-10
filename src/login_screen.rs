use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use game_engine::ui::automation::{UiAutomationAction, UiAutomationQueue};
use game_engine::ui::dioxus_screen::DioxusScreen;
use game_engine::ui::frame::{NineSlice, WidgetData};
use game_engine::ui::layout::recompute_layouts;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::login_component::{
    CONNECT_BUTTON, CREATE_ACCOUNT_BUTTON, EXIT_BUTTON, LOGIN_ROOT, LOGIN_STATUS, MENU_BUTTON,
    PASSWORD_INPUT, RECONNECT_BUTTON, USERNAME_INPUT, login_screen,
};
use game_engine::ui::widgets::button::ButtonState as BtnState;
use game_engine::ui::widgets::font_string::GameFont;
use game_engine::ui::widgets::texture::TextureSource;
use game_engine::ui_resource;

use crate::game_state::GameState;
use crate::networking;

#[path = "login_screen_connect.rs"]
mod connect;
#[path = "login_screen_helpers.rs"]
mod helpers;

use connect::{prefill_offline_credentials, toggle_login_mode, try_reconnect};
pub(crate) use connect::{sync_button_states, try_connect};
use helpers::{
    editbox_backspace, editbox_cursor_end, editbox_cursor_home, editbox_delete,
    editbox_move_cursor, hit_frame, insert_char_into_editbox, select_all_editbox,
    set_button_hovered, set_login_primary_button_textures,
};

const FADE_IN_DURATION: f32 = 0.75;
pub(crate) const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:25565";
const GLUE_EDITBOX_TEXT_COLOR: [f32; 4] = [1.0, 0.8, 0.2, 1.0];
const EDITBOX_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_BORDER: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BG: [f32; 4] = [1.0, 0.95, 0.85, 1.0];
const EDITBOX_FOCUSED_BORDER: [f32; 4] = [1.0, 0.78, 0.0, 1.0];
pub(crate) const STATUS_CONNECTING: &str = "Connecting...";
pub(crate) const STATUS_FILL_FIELDS: &str = "Please fill in all fields";
const STATUS_MENU_UNAVAILABLE: &str = "Menu is not implemented yet";
pub(crate) const STATUS_RECONNECT_UNAVAILABLE: &str = "No saved session to reconnect";

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
pub(crate) struct LoginStatus(pub(crate) String);

#[derive(Resource)]
pub(crate) struct DevServer;

#[derive(Resource)]
struct LoginFadeIn(f32);

use game_engine::ui::screens::login_component::SharedStatusText;

struct LoginDioxusScreen {
    screen: DioxusScreen,
    shared_status: SharedStatusText,
}
// SAFETY: DioxusScreen contains Rc<Runtime> which is not Send/Sync, but login
// systems run exclusively on the main thread so this is safe.
unsafe impl Send for LoginDioxusScreen {}
unsafe impl Sync for LoginDioxusScreen {}

#[derive(Resource)]
struct LoginDioxusScreenRes(LoginDioxusScreen);

pub struct LoginScreenPlugin;

impl Plugin for LoginScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginFocus>();
        app.init_resource::<LoginStatus>();
        app.init_resource::<LoginPressedButton>();
        app.add_systems(OnEnter(GameState::Login), build_login_ui);
        app.add_systems(OnExit(GameState::Login), teardown_login_ui);
        app.add_systems(
            Update,
            (
                login_sync_root_size,
                login_mouse_input,
                login_keyboard_input,
                login_run_automation,
                login_hover_visuals,
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

    let mut screen = build_login_dioxus_screen(&status);
    screen.screen.sync(&mut ui.registry);

    let login = LoginUi::resolve(&ui.registry);
    apply_post_setup(&mut ui.registry, &login);

    if dev_server.is_some() {
        prefill_offline_credentials(&mut ui.registry, &login);
    }

    ui.registry.set_alpha(login.root, 0.0);
    commands.insert_resource(LoginFadeIn(0.1));
    commands.insert_resource(LoginDioxusScreenRes(screen));
    commands.insert_resource(login);
}

fn build_login_dioxus_screen(status: &LoginStatus) -> LoginDioxusScreen {
    let screen = DioxusScreen::new(login_screen);
    let shared_status: SharedStatusText =
        std::rc::Rc::new(std::cell::RefCell::new(status.0.clone()));
    screen.provide_root_context(shared_status.clone());
    LoginDioxusScreen {
        screen,
        shared_status,
    }
}

fn apply_post_setup(reg: &mut FrameRegistry, login: &LoginUi) {
    let (sw, sh) = (reg.screen_width, reg.screen_height);
    if let Some(frame) = reg.get_mut(login.root) {
        frame.width = sw;
        frame.height = sh;
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
    mut screen: Option<ResMut<LoginDioxusScreenRes>>,
) {
    if let Some(screen) = screen.as_mut() {
        screen.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<LoginDioxusScreenRes>();
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
        .is_some_and(|frame| frame.visible && frame.shown)
        && hit_frame(ui, frame_id, mx, my)
}

fn login_sync_root_size(mut ui: ResMut<UiState>, login_ui: Option<Res<LoginUi>>) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    let sw = ui.registry.screen_width;
    let sh = ui.registry.screen_height;
    if let Some(root) = ui.registry.get_mut(login.root) {
        if (root.width - sw).abs() > 0.5 || (root.height - sh).abs() > 0.5 {
            root.width = sw;
            root.height = sh;
            if let Some(rect) = &mut root.layout_rect {
                rect.width = sw;
                rect.height = sh;
            }
        }
    }
}

fn login_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    mut focus: ResMut<LoginFocus>,
    mut next_state: ResMut<NextState<GameState>>,
    mut status: ResMut<LoginStatus>,
    mut login_mode: ResMut<networking::LoginMode>,
    auth_token: Res<networking::AuthToken>,
    server_addr: Option<Res<networking::ServerAddr>>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
    mut pressed: ResMut<LoginPressedButton>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    let cursor = windows.iter().next().and_then(|w| w.cursor_position());

    if buttons.just_pressed(MouseButton::Left) {
        if let Some(cursor) = cursor {
            handle_mouse_press(&mut ui, login, cursor, &mut focus, &mut pressed);
        }
    }

    if buttons.just_released(MouseButton::Left) {
        let released_id = pressed.0.take();
        if let Some(id) = released_id {
            reset_button_state(&mut ui.registry, id);
        }
        if let (Some(id), Some(cursor)) = (released_id, cursor) {
            if hit_active_frame(&ui, id, cursor.x, cursor.y) {
                handle_button_click(
                    &mut ui,
                    login,
                    cursor.x,
                    cursor.y,
                    &mut focus,
                    &mut next_state,
                    &mut status,
                    &mut login_mode,
                    &auth_token,
                    server_addr.as_ref().map(|addr| addr.0),
                    &mut commands,
                    Some(&mut exit),
                );
            }
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
    if hit_frame(ui, login.username_input, cx, cy) {
        focus.0 = Some(login.username_input);
        select_all_editbox(&mut ui.registry, login.username_input);
        return;
    }
    if hit_frame(ui, login.password_input, cx, cy) {
        focus.0 = Some(login.password_input);
        select_all_editbox(&mut ui.registry, login.password_input);
        return;
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

#[allow(clippy::too_many_arguments)]
fn handle_button_click(
    ui: &mut UiState,
    login: &LoginUi,
    cx: f32,
    cy: f32,
    focus: &mut LoginFocus,
    next_state: &mut NextState<GameState>,
    status: &mut LoginStatus,
    login_mode: &mut networking::LoginMode,
    auth_token: &networking::AuthToken,
    server_addr: Option<std::net::SocketAddr>,
    commands: &mut Commands,
    exit: Option<&mut MessageWriter<AppExit>>,
) {
    if hit_active_frame(ui, login.connect_button, cx, cy) {
        try_connect(
            &ui.registry,
            login,
            status,
            next_state,
            &*login_mode,
            server_addr,
            commands,
        );
    } else if login
        .reconnect_button
        .is_some_and(|id| hit_active_frame(ui, id, cx, cy))
    {
        try_reconnect(
            auth_token,
            status,
            next_state,
            login_mode,
            server_addr,
            commands,
        );
    } else if hit_active_frame(ui, login.create_account_button, cx, cy) {
        toggle_login_mode(login_mode, &mut ui.registry, login);
        status.0.clear();
    } else if hit_active_frame(ui, login.menu_button, cx, cy) {
        status.0 = STATUS_MENU_UNAVAILABLE.to_string();
    } else if hit_active_frame(ui, login.exit_button, cx, cy) {
        if let Some(exit) = exit {
            exit.write(AppExit::Success);
        }
    } else {
        focus.0 = None;
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
    mut next_state: ResMut<NextState<GameState>>,
    mut status: ResMut<LoginStatus>,
    login_mode: Res<networking::LoginMode>,
    server_addr: Option<Res<networking::ServerAddr>>,
    mut commands: Commands,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if handle_nav_key(event.key_code, &mut focus, login) {
            continue;
        }
        let Some(focused_id) = focus.0 else { continue };
        if let Key::Character(ch) = &event.logical_key {
            insert_char_into_editbox(&mut ui.registry, focused_id, ch.as_str());
        } else {
            handle_login_key(
                event.key_code,
                focused_id,
                &mut ui,
                login,
                &mut status,
                &mut next_state,
                &*login_mode,
                server_addr.as_ref().map(|addr| addr.0),
                &mut commands,
            );
        }
    }
}

fn login_run_automation(
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    mut focus: ResMut<LoginFocus>,
    mut next_state: ResMut<NextState<GameState>>,
    mut status: ResMut<LoginStatus>,
    mut login_mode: ResMut<networking::LoginMode>,
    auth_token: Res<networking::AuthToken>,
    server_addr: Option<Res<networking::ServerAddr>>,
    mut queue: ResMut<UiAutomationQueue>,
    mut commands: Commands,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    let Some(action) = queue.pop() else { return };
    if let Err(err) = run_login_automation_action(
        &mut ui,
        login,
        &mut focus,
        &mut next_state,
        &mut status,
        &mut login_mode,
        &auth_token,
        server_addr.as_ref().map(|addr| addr.0),
        &mut commands,
        &action,
    ) {
        status.0 = err;
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_login_automation_action(
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
        UiAutomationAction::ClickFrame(name) => {
            click_login_frame(
                ui,
                login,
                focus,
                next_state,
                status,
                login_mode,
                auth_token,
                server_addr,
                commands,
                name,
            )?;
        }
        UiAutomationAction::TypeText(text) => {
            let focused_id = focus
                .0
                .ok_or("automation type requires a focused edit box")?;
            for ch in text.chars() {
                insert_char_into_editbox(&mut ui.registry, focused_id, &ch.to_string());
            }
        }
        UiAutomationAction::PressKey(key) => {
            let focused_id = focus
                .0
                .ok_or("automation key press requires a focused frame")?;
            handle_login_key(
                *key,
                focused_id,
                ui,
                login,
                status,
                next_state,
                &*login_mode,
                server_addr,
                commands,
            );
        }
        UiAutomationAction::WaitForState(_, _)
        | UiAutomationAction::WaitForFrame(_, _)
        | UiAutomationAction::DumpTree
        | UiAutomationAction::DumpUiTree => {}
    }
    Ok(())
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
    let rect = ui
        .registry
        .get(frame_id)
        .and_then(|frame| frame.layout_rect.as_ref())
        .cloned()
        .ok_or_else(|| format!("login frame '{frame_name}' has no layout rect"))?;
    let cursor = Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let mut pressed = LoginPressedButton::default();
    handle_mouse_press(ui, login, cursor, focus, &mut pressed);
    if let Some(id) = pressed.0.take() {
        reset_button_state(&mut ui.registry, id);
        let (cx, cy) = (cursor.x, cursor.y);
        handle_button_click(
            ui,
            login,
            cx,
            cy,
            focus,
            next_state,
            status,
            login_mode,
            auth_token,
            server_addr,
            commands,
            None,
        );
    }
    Ok(())
}

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

fn handle_login_key(
    key: KeyCode,
    focused_id: u64,
    ui: &mut UiState,
    login: &LoginUi,
    status: &mut LoginStatus,
    next_state: &mut NextState<GameState>,
    mode: &networking::LoginMode,
    server_addr: Option<std::net::SocketAddr>,
    commands: &mut Commands,
) {
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
            commands,
        ),
        _ => {}
    }
}

fn login_hover_visuals(
    windows: Query<&Window>,
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    let cursor = windows.iter().next().and_then(|w| w.cursor_position());
    let mut button_ids = vec![
        login.connect_button,
        login.create_account_button,
        login.menu_button,
        login.exit_button,
    ];
    if let Some(reconnect_button) = login.reconnect_button {
        button_ids.push(reconnect_button);
    }
    for id in button_ids {
        let hovered = cursor.is_some_and(|c| {
            ui.registry
                .get(id)
                .and_then(|f| f.layout_rect.as_ref())
                .is_some_and(|r| {
                    c.x >= r.x && c.x <= r.x + r.width && c.y >= r.y && c.y <= r.y + r.height
                })
        });
        set_button_hovered(&mut ui.registry, id, hovered);
    }
}

fn login_update_visuals(
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    mut screen_res: Option<ResMut<LoginDioxusScreenRes>>,
    status: Res<LoginStatus>,
    focus: Res<LoginFocus>,
    login_mode: Res<networking::LoginMode>,
    auth_token: Res<networking::AuthToken>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    ui.focused_frame = focus.0;
    sync_button_states(&mut ui.registry, login, &*login_mode, &auth_token);
    sync_dioxus_status(&mut ui.registry, screen_res.as_mut(), &status);
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

fn sync_dioxus_status(
    reg: &mut FrameRegistry,
    screen_res: Option<&mut ResMut<LoginDioxusScreenRes>>,
    status: &LoginStatus,
) {
    let Some(dioxus) = screen_res else { return };
    {
        let mut shared = dioxus.0.shared_status.borrow_mut();
        if *shared != status.0 {
            *shared = status.0.clone();
            drop(shared);
            dioxus.0.screen.mark_dirty_root();
        }
    }
    dioxus.0.screen.sync(reg);
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
#[path = "login_screen_tests.rs"]
mod tests;
