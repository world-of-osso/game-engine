use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use game_engine::ui::anchor::{Anchor, AnchorPoint};
use game_engine::ui::frame::{Frame, WidgetData, WidgetType};
use game_engine::ui::layout::resolve_frame_layout;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::ButtonData;
use game_engine::ui::widgets::edit_box::EditBoxData;
use game_engine::ui::widgets::font_string::{FontStringData, JustifyH};

use crate::game_state::GameState;
use crate::networking;

/// Resource holding frame IDs for the login screen UI.
#[derive(Resource)]
struct LoginUi {
    root: u64,
    server_input: u64,
    username_input: u64,
    password_input: u64,
    connect_button: u64,
    register_button: u64,
    status_text: u64,
}

/// Tracks which editbox is focused within the login screen.
#[derive(Resource, Default)]
struct LoginFocus(Option<u64>);

/// Status message displayed during connection.
#[derive(Resource, Default)]
struct LoginStatus(String);

pub struct LoginScreenPlugin;

impl Plugin for LoginScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginFocus>();
        app.init_resource::<LoginStatus>();
        app.add_systems(OnEnter(GameState::Login), build_login_ui);
        app.add_systems(OnExit(GameState::Login), teardown_login_ui);
        app.add_systems(
            Update,
            (
                login_mouse_input,
                login_keyboard_input,
                login_update_visuals,
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
    let (
        server_input,
        username_input,
        password_input,
        connect_button,
        register_button,
        status_text,
    ) = build_login_form(reg, root, sw, sh);

    // Pre-fill from --server CLI arg if provided.
    if let Some(addr) = server_addr {
        set_editbox_text(reg, server_input, &addr.0.to_string());
    }

    commands.insert_resource(LoginUi {
        root,
        server_input,
        username_input,
        password_input,
        connect_button,
        register_button,
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
    set_font_string(reg, title, "World of Warcraft", 28.0, [1.0, 0.82, 0.0, 1.0]);

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

fn build_login_form(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    sh: f32,
) -> (u64, u64, u64, u64, u64, u64) {
    let panel_w = 340.0;
    let panel_x = (sw - panel_w) / 2.0;
    let mut y = sh * 0.35;

    let server_input = build_labeled_editbox(
        reg,
        root,
        panel_x,
        &mut y,
        panel_w,
        "Server Address",
        "ServerInput",
    );
    set_editbox_text(reg, server_input, "127.0.0.1:25565");
    let username_input = build_labeled_editbox(
        reg,
        root,
        panel_x,
        &mut y,
        panel_w,
        "Username",
        "UsernameInput",
    );
    let password_input = build_labeled_editbox(
        reg,
        root,
        panel_x,
        &mut y,
        panel_w,
        "Password",
        "PasswordInput",
    );
    y += 8.0;

    let connect_button = create_button(reg, "ConnectButton", Some(root), 200.0, 36.0, "Login");
    set_layout(reg, connect_button, (sw - 200.0) / 2.0, y, 200.0, 36.0);
    set_bg(reg, connect_button, [0.15, 0.35, 0.6, 1.0]);
    y += 44.0;

    let register_button = build_mode_toggle(reg, root, sw, y, panel_w);
    y += 32.0;

    let status_text = create_frame(
        reg,
        "LoginStatus",
        Some(root),
        WidgetType::FontString,
        panel_w,
        24.0,
    );
    set_layout(reg, status_text, panel_x, y, panel_w, 24.0);
    set_font_string(reg, status_text, "", 13.0, [0.9, 0.5, 0.5, 1.0]);

    (
        server_input,
        username_input,
        password_input,
        connect_button,
        register_button,
        status_text,
    )
}

fn build_mode_toggle(reg: &mut FrameRegistry, root: u64, sw: f32, y: f32, panel_w: f32) -> u64 {
    let btn = create_frame(
        reg,
        "RegisterToggle",
        Some(root),
        WidgetType::FontString,
        panel_w,
        20.0,
    );
    set_layout(reg, btn, (sw - panel_w) / 2.0, y, panel_w, 20.0);
    set_font_string(
        reg,
        btn,
        "Don't have an account? Register",
        12.0,
        [0.5, 0.7, 1.0, 1.0],
    );
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

    let input = create_editbox(reg, name, Some(root), panel_w, 32.0);
    set_layout(reg, input, panel_x, *y, panel_w, 32.0);
    set_bg(reg, input, [0.12, 0.12, 0.2, 1.0]);
    *y += 48.0;

    input
}

fn teardown_login_ui(
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    mut commands: Commands,
) {
    if let Some(login) = login_ui {
        remove_frame_tree(&mut ui.registry, login.root);
        commands.remove_resource::<LoginUi>();
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
    mut commands: Commands,
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
    } else if hit_frame(&ui, login.username_input, cx, cy) {
        focus.0 = Some(login.username_input);
    } else if hit_frame(&ui, login.password_input, cx, cy) {
        focus.0 = Some(login.password_input);
    } else if hit_frame(&ui, login.connect_button, cx, cy) {
        try_connect(
            &ui.registry,
            login,
            &mut status,
            &mut next_state,
            &*login_mode,
            &mut commands,
        );
    } else if hit_frame(&ui, login.register_button, cx, cy) {
        toggle_login_mode(&mut login_mode, &mut ui.registry, login);
    } else {
        focus.0 = None;
    }
}

fn login_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    focus: Res<LoginFocus>,
    login_ui: Option<Res<LoginUi>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut status: ResMut<LoginStatus>,
    login_mode: Res<networking::LoginMode>,
    mut commands: Commands,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    let Some(focused_id) = focus.0 else { return };

    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
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
                &mut commands,
            );
        }
    }
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
                [0.18, 0.18, 0.3, 1.0]
            } else {
                [0.12, 0.12, 0.2, 1.0]
            });
        }
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
    if let Some(WidgetData::FontString(fs)) = reg
        .get_mut(login.register_button)
        .and_then(|f| f.widget_data.as_mut())
    {
        fs.text = toggle_text.to_string();
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
