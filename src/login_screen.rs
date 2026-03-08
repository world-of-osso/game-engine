use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use game_engine::ui::anchor::{Anchor, AnchorPoint};
use game_engine::ui::frame::{Frame, NineSlice, WidgetData, WidgetType};
use game_engine::ui::layout::resolve_frame_layout;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::{ButtonData, ButtonState as BtnState, CheckButtonData};
use game_engine::ui::widgets::edit_box::EditBoxData;
use game_engine::ui::widgets::font_string::{FontStringData, JustifyH};
use game_engine::ui::widgets::texture::{TextureData, TextureSource};

use crate::game_state::GameState;
use crate::networking;

const FADE_IN_DURATION: f32 = 0.75;

const TEX_RED_NORMAL: &str = "/home/osso/Projects/wow/Interface/BUTTONS/UI-Panel-Button-Up.blp";
const TEX_RED_PUSHED: &str = "/home/osso/Projects/wow/Interface/BUTTONS/UI-Panel-Button-Down.blp";
const TEX_RED_HL: &str = "/home/osso/Projects/wow/Interface/BUTTONS/UI-Panel-Button-Highlight.blp";
const TEX_DLG_NORMAL: &str = "/home/osso/Projects/wow/Interface/BUTTONS/UI-DialogBox-Button-Up.blp";
const TEX_DLG_PUSHED: &str =
    "/home/osso/Projects/wow/Interface/BUTTONS/UI-DialogBox-Button-Down.blp";
const TEX_DLG_HL: &str =
    "/home/osso/Projects/wow/Interface/BUTTONS/UI-DialogBox-Button-Highlight.blp";
const TEX_EDITBOX_BORDER: &str = "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border.blp";
const TEX_LOGIN_BACKGROUND: &str = "data/glues/login/UI_MainMenu_WarWithin_LowBandwidth.blp";
const TEX_GAME_LOGO: &str = "data/glues/common/Glues-WoW-TheWarWithinLogo.blp";
const TEX_BLIZZARD_LOGO: &str = "data/glues/mainmenu/Glues-BlizzardLogo.blp";

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

#[derive(Resource, Default)]
struct LoginFocus(Option<u64>);

#[derive(Resource, Default)]
struct LoginStatus(String);

#[derive(Resource, Default)]
struct LoginSaveAccount(bool);

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
                login_sync_root_size,
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
    let (sw, sh) = (reg.screen_width, reg.screen_height);
    let root = build_login_background(reg, sw, sh);
    build_login_titles(reg, root, sw, sh);
    let (server_input, username_input, password_input) = build_login_inputs(reg, root, sw, sh);
    let (
        connect_button,
        reconnect_button,
        create_account_button,
        menu_button,
        save_checkbox,
        exit_button,
        status_text,
    ) = build_login_buttons(reg, root, sw, sh, password_input);
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
    set_bg(reg, root, [0.01, 0.01, 0.01, 1.0]);
    set_strata(reg, root, FrameStrata::Background);
    let bg = create_texture(
        reg,
        "LoginBackground",
        Some(root),
        sw,
        sh,
        TEX_LOGIN_BACKGROUND,
    );
    set_layout(reg, bg, 0.0, 0.0, sw, sh);
    let overlay = create_frame(
        reg,
        "LoginBackgroundShade",
        Some(root),
        WidgetType::Frame,
        sw,
        sh,
    );
    set_layout(reg, overlay, 0.0, 0.0, sw, sh);
    set_bg(reg, overlay, [0.0, 0.0, 0.0, 0.22]);
    root
}

fn build_login_titles(reg: &mut FrameRegistry, root: u64, sw: f32, sh: f32) {
    let logo = create_texture(
        reg,
        "LoginGameLogo",
        Some(root),
        256.0,
        128.0,
        TEX_GAME_LOGO,
    );
    set_strata(reg, logo, FrameStrata::Medium);
    set_layout(reg, logo, 3.0, 7.0, 256.0, 128.0);
    let title = create_frame(
        reg,
        "LoginTitle",
        Some(root),
        WidgetType::FontString,
        400.0,
        40.0,
    );
    set_strata(reg, title, FrameStrata::Medium);
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
    set_strata(reg, sub, FrameStrata::Medium);
    set_layout(reg, sub, (sw - 300.0) / 2.0, sh * 0.15 + 45.0, 300.0, 24.0);
    set_font_string(reg, sub, "Game Engine", 18.0, [0.7, 0.7, 0.8, 1.0]);
}

fn build_login_inputs(reg: &mut FrameRegistry, root: u64, _sw: f32, _sh: f32) -> (u64, u64, u64) {
    let panel_w = 320.0;
    let eb_h = 42.0;

    // ServerInput: CENTER on root, y=80 (shifted up from WoW's y=50 to fit 3 fields)
    let server =
        build_editbox_with_label(reg, root, panel_w, eb_h, "Server Address", "ServerInput");
    set_strata(reg, server, FrameStrata::Medium);
    set_anchor(
        reg,
        server,
        AnchorPoint::Center,
        Some(root),
        AnchorPoint::Center,
        0.0,
        80.0,
    );
    set_editbox_text(reg, server, "127.0.0.1:25565");

    // UsernameInput: TOP anchored to ServerInput BOTTOM, y=-30
    let username = build_editbox_with_label(reg, root, panel_w, eb_h, "Username", "UsernameInput");
    set_strata(reg, username, FrameStrata::Medium);
    set_anchor(
        reg,
        username,
        AnchorPoint::Top,
        Some(server),
        AnchorPoint::Bottom,
        0.0,
        -30.0,
    );

    // PasswordInput: TOP anchored to UsernameInput BOTTOM, y=-30
    let password = build_editbox_with_label(reg, root, panel_w, eb_h, "Password", "PasswordInput");
    set_strata(reg, password, FrameStrata::Medium);
    set_anchor(
        reg,
        password,
        AnchorPoint::Top,
        Some(username),
        AnchorPoint::Bottom,
        0.0,
        -30.0,
    );
    set_editbox_password(reg, password);

    (server, username, password)
}

fn build_login_buttons(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    sh: f32,
    password_input: u64,
) -> (u64, u64, u64, u64, u64, u64, u64) {
    let panel_w = 320.0;

    // LoginButton: TOP anchored to PasswordEditBox BOTTOM, y=-50
    let (connect, reconnect) = build_main_buttons(reg, root, password_input);
    // SaveCheckbox: TOP anchored to LoginButton BOTTOM, y=-10
    let save_checkbox = build_save_account_checkbox(reg, root, connect);
    // Bottom action cluster: anchor from the actual screen bottom so it stays visible
    // even when the fixed 1920x1080 registry is rendered into a smaller window.
    let exit = build_exit_button(reg, root, sw, sh);
    let create_account = build_action_button_anchored(
        reg,
        root,
        "CreateAccountButton",
        "Don't have an account? Register",
        AnchorPoint::Bottom,
        Some(root),
        AnchorPoint::Bottom,
        0.0,
        54.0,
    );
    let menu = build_action_button_anchored(
        reg,
        root,
        "MenuButton",
        "Menu",
        AnchorPoint::Bottom,
        Some(create_account),
        AnchorPoint::Top,
        0.0,
        10.0,
    );
    // Status text: TOP anchored to SaveCheckbox BOTTOM, y=-10
    let status = create_frame(
        reg,
        "LoginStatus",
        Some(root),
        WidgetType::FontString,
        panel_w,
        24.0,
    );
    set_strata(reg, status, FrameStrata::Medium);
    set_anchor(
        reg,
        status,
        AnchorPoint::Top,
        Some(save_checkbox),
        AnchorPoint::Bottom,
        0.0,
        -10.0,
    );
    set_font_string(reg, status, "", 13.0, [0.9, 0.5, 0.5, 1.0]);
    build_footer_text(reg, root, sw, sh);
    (
        connect,
        reconnect,
        create_account,
        menu,
        save_checkbox,
        exit,
        status,
    )
}

fn build_main_buttons(reg: &mut FrameRegistry, root: u64, password_input: u64) -> (u64, u64) {
    let connect = create_button(reg, "ConnectButton", Some(root), 250.0, 66.0, "Login");
    set_strata(reg, connect, FrameStrata::Medium);
    set_anchor(
        reg,
        connect,
        AnchorPoint::Top,
        Some(password_input),
        AnchorPoint::Bottom,
        0.0,
        -50.0,
    );
    set_button_textures(reg, connect, TEX_RED_NORMAL, TEX_RED_PUSHED, TEX_RED_HL);
    let reconnect = create_button(reg, "ReconnectButton", Some(root), 250.0, 66.0, "Reconnect");
    set_strata(reg, reconnect, FrameStrata::Medium);
    set_anchor(
        reg,
        reconnect,
        AnchorPoint::Top,
        Some(password_input),
        AnchorPoint::Bottom,
        0.0,
        -50.0,
    );
    set_button_textures(reg, reconnect, TEX_RED_NORMAL, TEX_RED_PUSHED, TEX_RED_HL);
    hide_frame(reg, reconnect);
    (connect, reconnect)
}

fn build_exit_button(reg: &mut FrameRegistry, root: u64, _sw: f32, _sh: f32) -> u64 {
    let exit = create_button(reg, "ExitButton", Some(root), 80.0, 28.0, "Quit");
    set_strata(reg, exit, FrameStrata::Medium);
    set_anchor(
        reg,
        exit,
        AnchorPoint::BottomRight,
        Some(root),
        AnchorPoint::BottomRight,
        -10.0,
        16.0,
    );
    set_button_textures(reg, exit, TEX_DLG_NORMAL, TEX_DLG_PUSHED, TEX_DLG_HL);
    exit
}

fn build_action_button_anchored(
    reg: &mut FrameRegistry,
    root: u64,
    name: &str,
    text: &str,
    point: AnchorPoint,
    relative_to: Option<u64>,
    relative_point: AnchorPoint,
    x_off: f32,
    y_off: f32,
) -> u64 {
    let btn = create_button(reg, name, Some(root), 250.0, 30.0, text);
    set_strata(reg, btn, FrameStrata::Medium);
    set_anchor(reg, btn, point, relative_to, relative_point, x_off, y_off);
    set_button_textures(reg, btn, TEX_DLG_NORMAL, TEX_DLG_PUSHED, TEX_DLG_HL);
    btn
}

fn build_save_account_checkbox(reg: &mut FrameRegistry, root: u64, login_button: u64) -> u64 {
    let btn = create_check_button(reg, "SaveAccountCheckbox", Some(root), 200.0, 30.0, false);
    set_strata(reg, btn, FrameStrata::Medium);
    set_anchor(
        reg,
        btn,
        AnchorPoint::Top,
        Some(login_button),
        AnchorPoint::Bottom,
        0.0,
        -10.0,
    );
    set_bg(reg, btn, [0.12, 0.12, 0.2, 1.0]);
    btn
}

/// Create an editbox with its label as a child (WoW style: label BOTTOM→editbox TOP, y=-23).
fn build_editbox_with_label(
    reg: &mut FrameRegistry,
    root: u64,
    w: f32,
    h: f32,
    label_text: &str,
    name: &str,
) -> u64 {
    let input = create_editbox(reg, name, Some(root), w, h);
    set_editbox_backdrop(reg, input);
    // Label as child of editbox, BOTTOM anchored to editbox TOP with y=-23
    let label = create_frame(
        reg,
        &format!("{name}Label"),
        Some(input),
        WidgetType::FontString,
        w,
        20.0,
    );
    set_anchor(
        reg,
        label,
        AnchorPoint::Bottom,
        Some(input),
        AnchorPoint::Top,
        0.0,
        23.0,
    );
    set_font_string(reg, label, label_text, 14.0, [0.8, 0.8, 0.9, 1.0]);
    input
}

fn build_footer_text(reg: &mut FrameRegistry, root: u64, _sw: f32, _sh: f32) {
    let version = create_frame(
        reg,
        "VersionText",
        Some(root),
        WidgetType::FontString,
        200.0,
        16.0,
    );
    set_strata(reg, version, FrameStrata::Medium);
    set_anchor(
        reg,
        version,
        AnchorPoint::BottomLeft,
        Some(root),
        AnchorPoint::BottomLeft,
        10.0,
        8.0,
    );
    set_font_string_left(
        reg,
        version,
        "game-engine v0.1.0",
        11.0,
        [0.7, 0.7, 0.75, 1.0],
    );
    let disclaimer = create_frame(
        reg,
        "DisclaimerText",
        Some(root),
        WidgetType::FontString,
        400.0,
        16.0,
    );
    set_strata(reg, disclaimer, FrameStrata::Medium);
    set_anchor(
        reg,
        disclaimer,
        AnchorPoint::Bottom,
        Some(root),
        AnchorPoint::Bottom,
        0.0,
        8.0,
    );
    set_font_string(
        reg,
        disclaimer,
        "© 2025 World of Osso. All rights reserved.",
        11.0,
        [0.65, 0.65, 0.7, 1.0],
    );
    let logo = create_texture(
        reg,
        "BlizzardLogo",
        Some(root),
        100.0,
        100.0,
        TEX_BLIZZARD_LOGO,
    );
    set_strata(reg, logo, FrameStrata::Medium);
    set_anchor(
        reg,
        logo,
        AnchorPoint::Bottom,
        Some(root),
        AnchorPoint::Bottom,
        0.0,
        8.0,
    );
}

fn set_editbox_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.nine_slice = Some(NineSlice {
            edge_size: 12.0,
            texture: Some(TextureSource::File(TEX_EDITBOX_BORDER.to_string())),
            bg_color: [0.06, 0.06, 0.10, 0.9],
            border_color: [0.3, 0.25, 0.15, 1.0],
        });
        if let Some(WidgetData::EditBox(eb)) = &mut frame.widget_data {
            eb.text_insets = [12.0, 5.0, 0.0, 5.0];
        }
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
    let Some(cursor) = windows.iter().next().and_then(|w| w.cursor_position()) else {
        return;
    };
    handle_mouse_click(
        &mut ui,
        login,
        cursor,
        &mut focus,
        &mut next_state,
        &mut status,
        &mut login_mode,
        &mut save_account,
        &mut commands,
        &mut exit,
    );
}

fn handle_mouse_click(
    ui: &mut UiState,
    login: &LoginUi,
    cursor: Vec2,
    focus: &mut LoginFocus,
    next_state: &mut NextState<GameState>,
    status: &mut LoginStatus,
    login_mode: &mut networking::LoginMode,
    save_account: &mut LoginSaveAccount,
    commands: &mut Commands,
    exit: &mut MessageWriter<AppExit>,
) {
    let (cx, cy) = (cursor.x, cursor.y);
    if hit_frame(ui, login.server_input, cx, cy) {
        focus.0 = Some(login.server_input);
        select_all_editbox(&mut ui.registry, login.server_input);
    } else if hit_frame(ui, login.username_input, cx, cy) {
        focus.0 = Some(login.username_input);
        select_all_editbox(&mut ui.registry, login.username_input);
    } else if hit_frame(ui, login.password_input, cx, cy) {
        focus.0 = Some(login.password_input);
        select_all_editbox(&mut ui.registry, login.password_input);
    } else {
        handle_button_click(
            ui,
            login,
            cx,
            cy,
            focus,
            next_state,
            status,
            login_mode,
            save_account,
            commands,
            exit,
        );
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
    save_account: &mut LoginSaveAccount,
    commands: &mut Commands,
    exit: &mut MessageWriter<AppExit>,
) {
    if hit_frame(ui, login.connect_button, cx, cy) {
        if let Some(WidgetData::Button(bd)) = ui
            .registry
            .get_mut(login.connect_button)
            .and_then(|f| f.widget_data.as_mut())
        {
            bd.state = BtnState::Pushed;
        }
        try_connect(
            &ui.registry,
            login,
            status,
            next_state,
            &*login_mode,
            commands,
        );
    } else if hit_frame(ui, login.save_checkbox, cx, cy) {
        toggle_save_account(save_account, &mut ui.registry, login.save_checkbox);
    } else if hit_frame(ui, login.create_account_button, cx, cy) {
        toggle_login_mode(login_mode, &mut ui.registry, login);
    } else if hit_frame(ui, login.exit_button, cx, cy) {
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
                &mut commands,
            );
        }
    }
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
    let fields = [
        login.server_input,
        login.username_input,
        login.password_input,
    ];
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
    update_button_hover(&mut ui.registry, login.connect_button, cursor);
    update_button_hover(&mut ui.registry, login.reconnect_button, cursor);
    update_button_hover(&mut ui.registry, login.create_account_button, cursor);
    update_button_hover(&mut ui.registry, login.menu_button, cursor);
    update_button_hover(&mut ui.registry, login.exit_button, cursor);
}

fn update_button_hover(reg: &mut FrameRegistry, id: u64, cursor: Option<Vec2>) {
    let hovered = cursor.is_some_and(|c| {
        reg.get(id)
            .and_then(|f| f.layout_rect.as_ref())
            .is_some_and(|r| {
                c.x >= r.x && c.x <= r.x + r.width && c.y >= r.y && c.y <= r.y + r.height
            })
    });
    set_button_hovered(reg, id, hovered);
}

fn login_update_visuals(
    mut ui: ResMut<UiState>,
    login_ui: Option<Res<LoginUi>>,
    status: Res<LoginStatus>,
    save_account: Res<LoginSaveAccount>,
    login_mode: Res<networking::LoginMode>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    sync_button_states(&mut ui.registry, login, &*login_mode);
    sync_status_text(&mut ui.registry, login.status_text, &status.0);
    sync_checkbox_text(&mut ui.registry, login.save_checkbox, save_account.0);
}

fn sync_button_states(reg: &mut FrameRegistry, login: &LoginUi, mode: &networking::LoginMode) {
    if let Some(WidgetData::Button(btn)) = reg
        .get_mut(login.connect_button)
        .and_then(|f| f.widget_data.as_mut())
    {
        btn.text = match mode {
            networking::LoginMode::Login => "Login".to_string(),
            networking::LoginMode::Register => "Create Account".to_string(),
        };
    }
    if let Some(WidgetData::Button(btn)) = reg
        .get_mut(login.create_account_button)
        .and_then(|f| f.widget_data.as_mut())
    {
        btn.text = match mode {
            networking::LoginMode::Login => "Don't have an account? Register".to_string(),
            networking::LoginMode::Register => "Already have an account? Login".to_string(),
        };
    }
}

fn sync_status_text(reg: &mut FrameRegistry, id: u64, text: &str) {
    if let Some(WidgetData::FontString(fs)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        fs.text = text.to_string();
    }
}

fn sync_checkbox_text(reg: &mut FrameRegistry, id: u64, checked: bool) {
    if let Some(WidgetData::Button(btn)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        btn.text = checkbox_text(checked).to_string();
    }
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

fn try_connect(
    reg: &FrameRegistry,
    login: &LoginUi,
    status: &mut LoginStatus,
    next_state: &mut NextState<GameState>,
    mode: &networking::LoginMode,
    commands: &mut Commands,
) {
    let server = get_editbox_text(reg, login.server_input);
    let username = get_editbox_text(reg, login.username_input);
    let password = get_editbox_text(reg, login.password_input);
    if server.trim().is_empty() || username.trim().is_empty() || password.trim().is_empty() {
        status.0 = "Please fill in all fields".to_string();
        return;
    }
    commands.insert_resource(networking::ServerAddr(
        server
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:25565".parse().unwrap()),
    ));
    commands.insert_resource(mode.clone());
    status.0 = "Connecting...".to_string();
    next_state.set(GameState::CharSelect);
}

fn toggle_login_mode(mode: &mut networking::LoginMode, reg: &mut FrameRegistry, login: &LoginUi) {
    *mode = match mode {
        networking::LoginMode::Login => networking::LoginMode::Register,
        networking::LoginMode::Register => networking::LoginMode::Login,
    };
    sync_button_states(reg, login, mode);
}

fn toggle_save_account(save_account: &mut LoginSaveAccount, reg: &mut FrameRegistry, id: u64) {
    save_account.0 = !save_account.0;
    sync_checkbox_text(reg, id, save_account.0);
}

fn insert_char_into_editbox(reg: &mut FrameRegistry, id: u64, s: &str) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.text.insert_str(eb.cursor_position, s);
        eb.cursor_position += s.len();
    }
}

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

fn editbox_move_cursor(reg: &mut FrameRegistry, id: u64, delta: isize) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        let pos = eb.cursor_position as isize + delta;
        eb.cursor_position = pos.clamp(0, eb.text.len() as isize) as usize;
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

fn set_editbox_password(reg: &mut FrameRegistry, id: u64) {
    if let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        eb.password = true;
    }
}

fn checkbox_text(checked: bool) -> &'static str {
    if checked {
        "[x] Save Account Name"
    } else {
        "[ ] Save Account Name"
    }
}

fn remove_frame_tree(reg: &mut FrameRegistry, id: u64) {
    let children = reg.get(id).map(|f| f.children.clone()).unwrap_or_default();
    for child in children {
        remove_frame_tree(reg, child);
    }
    reg.remove_frame(id);
}

fn create_frame(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    widget: WidgetType,
    w: f32,
    h: f32,
) -> u64 {
    let id = reg.next_id();
    let mut frame = Frame::new(id, Some(name.to_string()), widget);
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

fn create_texture(
    reg: &mut FrameRegistry,
    name: &str,
    parent: Option<u64>,
    w: f32,
    h: f32,
    path: &str,
) -> u64 {
    let id = create_frame(reg, name, parent, WidgetType::Texture, w, h);
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::Texture(TextureData {
            source: TextureSource::File(path.to_string()),
            ..Default::default()
        }));
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

fn set_anchor(
    reg: &mut FrameRegistry,
    id: u64,
    point: AnchorPoint,
    relative_to: Option<u64>,
    relative_point: AnchorPoint,
    x_offset: f32,
    y_offset: f32,
) {
    reg.clear_all_points(id);
    reg.set_point(
        id,
        Anchor {
            point,
            relative_to,
            relative_point,
            x_offset,
            y_offset,
        },
    )
    .expect("anchor must be valid");
}

fn set_layout_anchor(
    reg: &mut FrameRegistry,
    id: u64,
    relative_to: Option<u64>,
    x_offset: f32,
    y_offset: f32,
) {
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
    set_layout_anchor(reg, id, relative_to, x_offset, y_offset);
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

fn set_button_textures(
    reg: &mut FrameRegistry,
    id: u64,
    normal: &str,
    pushed: &str,
    highlight: &str,
) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.normal_texture = Some(TextureSource::File(normal.to_string()));
        bd.pushed_texture = Some(TextureSource::File(pushed.to_string()));
        bd.highlight_texture = Some(TextureSource::File(highlight.to_string()));
    }
}

fn set_button_hovered(reg: &mut FrameRegistry, id: u64, hovered: bool) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.hovered = hovered;
    }
}

fn hit_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry.get(frame_id).is_some_and(|f| {
        f.layout_rect
            .as_ref()
            .is_some_and(|r| mx >= r.x && mx <= r.x + r.width && my >= r.y && my <= r.y + r.height)
    })
}
