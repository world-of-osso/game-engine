use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use game_engine::ui::anchor::{Anchor, AnchorPoint};
use game_engine::ui::frame::{Backdrop, Frame, WidgetData, WidgetType};
use game_engine::ui::layout::resolve_frame_layout;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::{ButtonData, CheckButtonData};
use game_engine::ui::widgets::edit_box::EditBoxData;
use game_engine::ui::widgets::font_string::{FontStringData, JustifyH};

use crate::game_state::GameState;
use crate::networking;

const BTN_COLOR_NORMAL: [f32; 4] = [0.15, 0.35, 0.6, 1.0];
const BTN_COLOR_HOVER: [f32; 4] = [0.25, 0.45, 0.7, 1.0];
const BTN_COLOR_PRESSED: [f32; 4] = [0.1, 0.25, 0.45, 1.0];
const FADE_IN_DURATION: f32 = 0.75;

/// Resource holding frame IDs for the login screen UI.
#[derive(Resource)]
struct LoginUi {
    root: u64,
    server_input: u64,
    username_input: u64,
    password_input: u64,
    connect_button: u64,
    reconnect_button: u64,
    create_account_button: u64,
    menu_button: u64,
    save_checkbox: u64,
    exit_button: u64,
    status_text: u64,
}

/// Tracks which editbox is focused within the login screen.
#[derive(Resource, Default)]
struct LoginFocus(Option<u64>);

/// Status message displayed during connection.
#[derive(Resource, Default)]
struct LoginStatus(String);

#[derive(Resource, Default)]
struct LoginSaveAccount(bool);

/// Fade-in timer for login screen appearance.
#[derive(Resource)]
struct LoginFadeIn(f32);

pub struct LoginScreenPlugin;

impl Plugin for LoginScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginFocus>();
        app.init_resource::<LoginStatus>();
        app.init_resource::<LoginSaveAccount>();
        app.add_systems(OnEnter(GameState::Login), build_login_ui);
        app.add_systems(OnExit(GameState::Login), teardown_login_ui);
        app.add_systems(
            Update,
            (
                login_mouse_input,
                login_keyboard_input,
                login_hover_visuals,
                login_update_visuals,
                login_fade_in,
            )
                .into_configs()
                .run_if(in_state(GameState::Login)),
        );
    }
}

fn build_login_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    server_addr: Option<Res<networking::ServerAddr>>,
) {
    let reg = &mut ui.registry;
    let sw = reg.screen_width;
    let sh = reg.screen_height;

    let root = build_login_background(reg, sw, sh);
    build_login_titles(reg, root, sw, sh);
    let (server_input, username_input, password_input) = build_login_inputs(reg, root, sw, sh);
    let buttons = build_login_buttons(reg, root, sw, sh);
    let (connect_button, reconnect_button, create_account_button, menu_button, save_checkbox, exit_button, status_text) = buttons;

    if let Some(addr) = server_addr {
        set_editbox_text(reg, server_input, &addr.0.to_string());
    }

    reg.set_alpha(root, 0.0);
    commands.insert_resource(LoginFadeIn(0.0));
    commands.insert_resource(LoginUi {
        root,
        server_input,
        username_input,
        password_input,
        connect_button,
        reconnect_button,
        create_account_button,
        menu_button,
        save_checkbox,
        exit_button,
        status_text,
    });
}

fn build_login_background(reg: &mut FrameRegistry, sw: f32, sh: f32) -> u64 {
    let root = create_frame(reg, "LoginRoot", None, WidgetType::Frame, sw, sh);
    set_layout(reg, root, 0.0, 0.0, sw, sh);
    set_bg(reg, root, [0.05, 0.05, 0.12, 1.0]);
    set_strata(reg, root, FrameStrata::Fullscreen);
    root
}

fn build_login_titles(reg: &mut FrameRegistry, root: u64, sw: f32, sh: f32) {
    let title = create_frame(
        reg,
        "LoginTitle",
        Some(root),
        WidgetType::FontString,
        400.0,
        40.0,
    );
    set_layout(reg, title, (sw - 400.0) / 2.0, sh * 0.15, 400.0, 40.0);
    set_font_string(reg, title, "World of Osso", 28.0, [1.0, 0.82, 0.0, 1.0]);

    let sub = create_frame(
        reg,
        "LoginSubtitle",
        Some(root),
        WidgetType::FontString,
        300.0,
        24.0,
    );
    set_layout(reg, sub, (sw - 300.0) / 2.0, sh * 0.15 + 45.0, 300.0, 24.0);
    set_font_string(reg, sub, "Game Engine", 18.0, [0.7, 0.7, 0.8, 1.0]);
}

fn build_login_inputs(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    sh: f32,
) -> (u64, u64, u64) {
    let panel_w = 320.0;
    let panel_x = (sw - panel_w) / 2.0;
    let mut y = sh * 0.35;

    let server = build_labeled_editbox(reg, root, panel_x, &mut y, panel_w, "Server Address", "ServerInput");
    set_editbox_text(reg, server, "127.0.0.1:25565");
    let username = build_labeled_editbox(reg, root, panel_x, &mut y, panel_w, "Username", "UsernameInput");
    let password = build_labeled_editbox(reg, root, panel_x, &mut y, panel_w, "Password", "PasswordInput");
    set_editbox_password(reg, password);
    (server, username, password)
}

fn build_login_buttons(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    sh: f32,
) -> (u64, u64, u64, u64, u64, u64, u64) {
    let panel_w = 320.0;
    let panel_x = (sw - panel_w) / 2.0;
    let mut y = sh * 0.35 + 3.0 * 72.0 + 8.0;

    let connect = create_button(reg, "ConnectButton", Some(root), 250.0, 66.0, "Login");
    set_layout(reg, connect, (sw - 250.0) / 2.0, y, 250.0, 66.0);
    set_bg(reg, connect, BTN_COLOR_NORMAL);

    let reconnect = create_button(reg, "ReconnectButton", Some(root), 250.0, 66.0, "Reconnect");
    set_layout(reg, reconnect, (sw - 250.0) / 2.0, y, 250.0, 66.0);
    set_bg(reg, reconnect, [0.15, 0.4, 0.2, 1.0]);
    hide_frame(reg, reconnect);

    let controls_y = y + 74.0;
    let save_checkbox = build_save_account_checkbox(reg, root, sw, controls_y - 36.0);
    let create_account = build_action_button(
        reg,
        root,
        "CreateAccountButton",
        "Don't have an account? Register",
        sw,
        controls_y,
    );
    let menu = build_action_button(reg, root, "MenuButton", "Menu", sw, controls_y + 38.0);
    y = controls_y + 76.0;

    let status = create_frame(reg, "LoginStatus", Some(root), WidgetType::FontString, panel_w, 24.0);
    set_layout(reg, status, panel_x, y, panel_w, 24.0);
    set_font_string(reg, status, "", 13.0, [0.9, 0.5, 0.5, 1.0]);

    let exit = create_button(reg, "ExitButton", Some(root), 80.0, 28.0, "Quit");
    set_layout(reg, exit, sw - 90.0, sh - 44.0, 80.0, 28.0);
    set_bg(reg, exit, [0.3, 0.1, 0.1, 1.0]);

    build_footer_text(reg, root, sw, sh);
    (connect, reconnect, create_account, menu, save_checkbox, exit, status)
}

fn build_action_button(
    reg: &mut FrameRegistry,
    root: u64,
    name: &str,
    text: &str,
    sw: f32,
    y: f32,
) -> u64 {
    let btn = create_button(reg, name, Some(root), 200.0, 30.0, text);
    set_layout(reg, btn, (sw - 200.0) / 2.0, y, 200.0, 30.0);
    set_bg(reg, btn, [0.12, 0.12, 0.2, 1.0]);
    btn
}

fn build_save_account_checkbox(reg: &mut FrameRegistry, root: u64, sw: f32, y: f32) -> u64 {
    let btn = create_check_button(reg, "SaveAccountCheckbox", Some(root), 200.0, 30.0, false);
    set_layout(reg, btn, (sw - 200.0) / 2.0, y, 200.0, 30.0);
    set_bg(reg, btn, [0.12, 0.12, 0.2, 1.0]);
    btn
}

fn build_labeled_editbox(
    reg: &mut FrameRegistry,
    root: u64,
    panel_x: f32,
    y: &mut f32,
    panel_w: f32,
    label_text: &str,
    name: &str,
) -> u64 {
    let label = create_frame(
        reg,
        &format!("{name}Label"),
        Some(root),
        WidgetType::FontString,
        panel_w,
        20.0,
    );
    set_layout(reg, label, panel_x, *y, panel_w, 20.0);
    set_font_string_left(reg, label, label_text, 13.0, [0.8, 0.8, 0.9, 1.0]);
    *y += 24.0;

    let input = create_editbox(reg, name, Some(root), panel_w, 40.0);
    set_layout(reg, input, panel_x, *y, panel_w, 40.0);
    set_editbox_backdrop(reg, input);
    *y += 48.0;

    input
}


fn build_footer_text(reg: &mut FrameRegistry, root: u64, sw: f32, sh: f32) {
    let version = create_frame(reg, "VersionText", Some(root), WidgetType::FontString, 200.0, 16.0);
    set_layout(reg, version, 10.0, sh - 20.0, 200.0, 16.0);
    set_font_string_left(reg, version, "game-engine v0.1.0", 11.0, [0.5, 0.5, 0.5, 1.0]);

    let disclaimer = create_frame(reg, "DisclaimerText", Some(root), WidgetType::FontString, 400.0, 16.0);
    set_layout(reg, disclaimer, (sw - 400.0) / 2.0, sh - 20.0, 400.0, 16.0);
    set_font_string(reg, disclaimer, "© 2025 World of Osso. All rights reserved.", 11.0, [0.4, 0.4, 0.4, 1.0]);

    let logo = create_frame(reg, "BlizzardLogo", Some(root), WidgetType::FontString, 100.0, 20.0);
    set_layout(reg, logo, (sw - 100.0) / 2.0, sh - 40.0, 100.0, 20.0);
    set_font_string(reg, logo, "BLIZZARD", 14.0, [0.8, 0.6, 0.0, 1.0]);
}

fn set_editbox_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.backdrop = Some(Backdrop {
            bg_color: Some([0.06, 0.06, 0.10, 0.9]),
            border_color: Some([0.3, 0.25, 0.15, 1.0]),
            edge_size: 1.0,
            insets: [0.0; 4],
        });
    }
}

fn hide_frame(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.visible = false;
        frame.shown = false;
    }
}

fn select_all_editbox(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = eb.text.len();
    }
}

fn teardown_login_ui(
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    mut commands: Commands,
) {
    if let Some(login) = login_ui {
        remove_frame_tree(&mut ui.registry, login.root);
        commands.remove_resource::<LoginUi>();
        commands.remove_resource::<LoginFadeIn>();
    }
    ui.focused_frame = None;
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
    mut save_account: ResMut<LoginSaveAccount>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let (cx, cy) = (cursor.x, cursor.y);
    if hit_frame(&ui, login.server_input, cx, cy) {
        focus.0 = Some(login.server_input);
        select_all_editbox(&mut ui.registry, login.server_input);
    } else if hit_frame(&ui, login.username_input, cx, cy) {
        focus.0 = Some(login.username_input);
        select_all_editbox(&mut ui.registry, login.username_input);
    } else if hit_frame(&ui, login.password_input, cx, cy) {
        focus.0 = Some(login.password_input);
        select_all_editbox(&mut ui.registry, login.password_input);
    } else if hit_frame(&ui, login.connect_button, cx, cy) {
        set_bg(&mut ui.registry, login.connect_button, BTN_COLOR_PRESSED);
        try_connect(
            &ui.registry,
            login,
            &mut status,
            &mut next_state,
            &*login_mode,
            &mut commands,
        );
    } else if hit_frame(&ui, login.save_checkbox, cx, cy) {
        toggle_save_account(&mut save_account, &mut ui.registry, login.save_checkbox);
    } else if hit_frame(&ui, login.create_account_button, cx, cy) {
        toggle_login_mode(&mut login_mode, &mut ui.registry, login);
    } else if hit_frame(&ui, login.menu_button, cx, cy) {
    } else if hit_frame(&ui, login.exit_button, cx, cy) {
        exit.write(AppExit::Success);
    } else {
        focus.0 = None;
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
    mut commands: Commands,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };

    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Tab => {
                focus.0 = Some(cycle_focus(focus.0, login));
                continue;
            }
            KeyCode::Escape => {
                focus.0 = None;
                continue;
            }
            _ => {}
        }
        let Some(focused_id) = focus.0 else { continue };
        if let Key::Character(ch) = &event.logical_key {
            insert_char_into_editbox(&mut ui.registry, focused_id, ch.as_str());
        } else {
            handle_login_key(
                event.key_code, focused_id, &mut ui, login,
                &mut status, &mut next_state, &*login_mode, &mut commands,
            );
        }
    }
}

fn cycle_focus(current: Option<u64>, login: &LoginUi) -> u64 {
    let fields = [login.server_input, login.username_input, login.password_input];
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
    commands: &mut Commands,
) {
    match key {
        KeyCode::Backspace => editbox_backspace(&mut ui.registry, focused_id),
        KeyCode::Delete => editbox_delete(&mut ui.registry, focused_id),
        KeyCode::ArrowLeft => editbox_move_cursor(&mut ui.registry, focused_id, -1),
        KeyCode::ArrowRight => editbox_move_cursor(&mut ui.registry, focused_id, 1),
        KeyCode::Home => editbox_cursor_home(&mut ui.registry, focused_id),
        KeyCode::End => editbox_cursor_end(&mut ui.registry, focused_id),
        KeyCode::Enter => try_connect(&ui.registry, login, status, next_state, mode, commands),
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
    let hovering = cursor.is_some_and(|c| hit_frame(&ui, login.connect_button, c.x, c.y));
    if let Some(frame) = ui.registry.get_mut(login.connect_button) {
        frame.background_color = Some(if hovering { BTN_COLOR_HOVER } else { BTN_COLOR_NORMAL });
    }
}

fn login_update_visuals(
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    focus: Res<LoginFocus>,
    status: Res<LoginStatus>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };

    if let Some(frame) = ui.registry.get_mut(login.status_text) {
        if let Some(WidgetData::FontString(fs)) = &mut frame.widget_data {
            fs.text.clone_from(&status.0);
        }
    }

    for &id in &[
        login.server_input,
        login.username_input,
        login.password_input,
    ] {
        let is_focused = focus.0 == Some(id);
        if let Some(frame) = ui.registry.get_mut(id) {
            frame.background_color = Some(if is_focused {
                [0.2, 0.2, 0.35, 1.0]
            } else {
                [0.12, 0.12, 0.2, 1.0]
            });
        }
    }
}

fn login_fade_in(
    time: Res<Time>,
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    fade: Option<ResMut<LoginFadeIn>>,
    mut commands: Commands,
) {
    let (Some(login), Some(mut fade)) = (login_ui.as_ref(), fade) else {
        return;
    };
    fade.0 += time.delta_secs();
    let alpha = (fade.0 / FADE_IN_DURATION).min(1.0);
    ui.registry.set_alpha(login.root, alpha);
    if alpha >= 1.0 {
        commands.remove_resource::<LoginFadeIn>();
    }
}

// --- Connection logic ---

fn try_connect(
    reg: &FrameRegistry,
    login: &LoginUi,
    status: &mut LoginStatus,
    next_state: &mut NextState<GameState>,
    mode: &networking::LoginMode,
    commands: &mut Commands,
) {
    let server_text = get_editbox_text(reg, login.server_input);
    if server_text.is_empty() {
        status.0 = "Enter a server address".to_string();
        return;
    }
    let Ok(addr) = server_text.parse::<std::net::SocketAddr>() else {
        status.0 = format!("Invalid address: {server_text}");
        return;
    };
    let username = get_editbox_text(reg, login.username_input);
    let password = get_editbox_text(reg, login.password_input);
    status.0 = format!("Connecting to {addr}...");
    commands.insert_resource(networking::ServerAddr(addr));
    commands.insert_resource(networking::LoginUsername(username));
    commands.insert_resource(networking::LoginPassword(password));
    commands.insert_resource(*mode);
    next_state.set(GameState::Connecting);
}

fn toggle_login_mode(mode: &mut networking::LoginMode, reg: &mut FrameRegistry, login: &LoginUi) {
    *mode = match *mode {
        networking::LoginMode::Login => networking::LoginMode::Register,
        networking::LoginMode::Register => networking::LoginMode::Login,
    };
    update_mode_labels(reg, login, *mode);
}

fn update_mode_labels(reg: &mut FrameRegistry, login: &LoginUi, mode: networking::LoginMode) {
    let (btn_text, toggle_text) = match mode {
        networking::LoginMode::Login => ("Login", "Don't have an account? Register"),
        networking::LoginMode::Register => ("Register", "Already have an account? Login"),
    };
    if let Some(WidgetData::Button(bd)) = reg
        .get_mut(login.connect_button)
        .and_then(|f| f.widget_data.as_mut())
    {
        bd.text = btn_text.to_string();
    }
    if let Some(WidgetData::Button(bd)) = reg
        .get_mut(login.create_account_button)
        .and_then(|f| f.widget_data.as_mut())
    {
        bd.text = toggle_text.to_string();
    }
}

// --- EditBox manipulation ---

fn editbox_backspace(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb.cursor_position > 0 {
            eb.cursor_position -= 1;
            eb.text.remove(eb.cursor_position);
        }
    }
}

fn editbox_delete(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb.cursor_position < eb.text.len() {
            eb.text.remove(eb.cursor_position);
        }
    }
}

fn editbox_move_cursor(reg: &mut FrameRegistry, id: u64, delta: i32) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if delta < 0 {
            eb.cursor_position = eb.cursor_position.saturating_sub((-delta) as usize);
        } else {
            eb.cursor_position = (eb.cursor_position + delta as usize).min(eb.text.len());
        }
    }
}

fn editbox_cursor_home(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = 0;
    }
}

fn editbox_cursor_end(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.cursor_position = eb.text.len();
    }
}

fn insert_char_into_editbox(reg: &mut FrameRegistry, id: u64, ch: &str) {
    if !ch.chars().all(|c| !c.is_control()) {
        return;
    }
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        if eb
            .max_letters
            .is_some_and(|max| eb.text.len() >= max as usize)
        {
            return;
        }
        eb.text.insert_str(eb.cursor_position, ch);
        eb.cursor_position += ch.len();
    }
}

fn set_editbox_password(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.password = true;
    }
}

// --- Frame creation helpers ---

fn create_frame(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    wt: WidgetType,
    w: f32,
    h: f32,
) -> u64 {
    let id = reg.next_id();
    let mut frame = Frame::new(id, Some(name.to_string()), wt);
    frame.parent_id = parent;
    frame.width = w;
    frame.height = h;
    frame.mouse_enabled = true;
    reg.insert_frame(frame);
    id
}

fn create_editbox(reg: &mut FrameRegistry, name: &str, parent: Option<u64>, w: f32, h: f32) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::EditBox, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::EditBox(EditBoxData::default()));
    }
    id
}

fn checkbox_text(checked: bool) -> &'static str {
    if checked {
        "[x] Save account name"
    } else {
        "[ ] Save account name"
    }
}

fn toggle_save_account(save: &mut LoginSaveAccount, reg: &mut FrameRegistry, id: u64) {
    save.0 = !save.0;
    set_button_text(reg, id, checkbox_text(save.0));
}

fn set_button_text(reg: &mut FrameRegistry, id: u64, text: &str) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.text = text.to_string();
    }
}

fn create_button(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
    text: &str,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::Button, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::Button(ButtonData {
            text: text.to_string(),
            ..Default::default()
        }));
    }
    id
}

fn create_check_button(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
    checked: bool,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::CheckButton, w, h);
    if let Some(frame) = reg.get_mut(id) {
        let mut data = CheckButtonData::default();
        data.checked = checked;
        data.button.text = checkbox_text(checked).to_string();
        frame.widget_data = Some(WidgetData::Button(data.button));
    }
    id
}

fn set_layout(reg: &mut FrameRegistry, id: u64, x: f32, y: f32, w: f32, h: f32) {
    let (relative_to, x_offset, y_offset) = reg
        .get(id)
        .and_then(|frame| frame.parent_id)
        .and_then(|parent_id| {
            reg.get(parent_id)
                .and_then(|parent| parent.layout_rect.as_ref())
                .map(|rect| (Some(parent_id), x - rect.x, y - rect.y))
        })
        .unwrap_or((None, x, y));

    if let Some(frame) = reg.get_mut(id) {
        frame.width = w;
        frame.height = h;
        frame.layout_rect = None;
    }

    reg.clear_all_points(id);
    reg.set_point(
        id,
        Anchor {
            point: AnchorPoint::TopLeft,
            relative_to,
            relative_point: AnchorPoint::TopLeft,
            x_offset,
            y_offset: -y_offset,
        },
    )
    .expect("screen layout helper must create a valid anchor");

    if let Some(layout_rect) = resolve_frame_layout(reg, id)
        && let Some(frame) = reg.get_mut(id)
    {
        frame.layout_rect = Some(layout_rect);
    }
}

fn set_bg(reg: &mut FrameRegistry, id: u64, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.background_color = Some(color);
    }
}

fn set_strata(reg: &mut FrameRegistry, id: u64, strata: FrameStrata) {
    if let Some(frame) = reg.get_mut(id) {
        frame.strata = strata;
    }
}

fn set_font_string(reg: &mut FrameRegistry, id: u64, text: &str, size: f32, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::FontString(FontStringData {
            text: text.to_string(),
            font_size: size,
            color,
            justify_h: JustifyH::Center,
            ..Default::default()
        }));
    }
}

fn set_font_string_left(reg: &mut FrameRegistry, id: u64, text: &str, size: f32, color: [f32; 4]) {
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::FontString(FontStringData {
            text: text.to_string(),
            font_size: size,
            color,
            justify_h: JustifyH::Left,
            ..Default::default()
        }));
    }
}

fn set_editbox_text(reg: &mut FrameRegistry, id: u64, text: &str) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.text = text.to_string();
        eb.cursor_position = text.len();
    }
}

fn get_editbox_text(reg: &FrameRegistry, id: u64) -> String {
    reg.get(id)
        .and_then(|f| match &f.widget_data {
            Some(WidgetData::EditBox(eb)) => Some(eb.text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn hit_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry.get(frame_id).is_some_and(|f| {
        f.layout_rect
            .as_ref()
            .is_some_and(|r| mx >= r.x && mx <= r.x + r.width && my >= r.y && my <= r.y + r.height)
    })
}

fn remove_frame_tree(reg: &mut FrameRegistry, id: u64) {
    let children = reg.get(id).map(|f| f.children.clone()).unwrap_or_default();
    for child in children {
        remove_frame_tree(reg, child);
    }
    reg.remove_frame(id);
}
