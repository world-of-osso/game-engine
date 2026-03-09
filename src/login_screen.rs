use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use game_engine::ui::anchor::{Anchor, AnchorPoint};
use game_engine::ui::automation::{UiAutomationAction, UiAutomationQueue};
use game_engine::ui::frame::{Frame, NineSlice, WidgetData, WidgetType};
use game_engine::ui::layout::{recompute_layouts, resolve_frame_layout};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::{ButtonData, ButtonState as BtnState};
use game_engine::ui::widgets::edit_box::EditBoxData;
use game_engine::ui::widgets::font_string::{FontStringData, JustifyH};
use game_engine::ui::widgets::texture::{TextureData, TextureSource};

use crate::game_state::GameState;
use crate::networking;

const FADE_IN_DURATION: f32 = 0.75;

const TEX_LOGIN_BACKGROUND: &str = "data/glues/login/UI_MainMenu_WarWithin_LowBandwidth.blp";
const TEX_GAME_LOGO: &str = "data/glues/common/world-of-osso-logo.png";
const TEX_BLIZZARD_LOGO: &str = "data/glues/mainmenu/Glues-BlizzardLogo.blp";
const FONT_GLUE_LABEL: &str = "/home/osso/Projects/wow/wow-ui-sim/fonts/FRIZQT__.TTF";
const FONT_GLUE_EDITBOX: &str = "/home/osso/Projects/wow/wow-ui-sim/fonts/ARIALN.ttf";
const LOGIN_BACKGROUND_SIZE: (f32, f32) = (2048.0, 1024.0);
const GLUE_BUTTON_SIZE: (f32, f32) = (200.0, 32.0);
const MAIN_LOGIN_BUTTON_SIZE: (f32, f32) = (250.0, 66.0);
const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:25565";
const GLUE_NORMAL_FONT_COLOR: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
const GLUE_EDITBOX_TEXT_COLOR: [f32; 4] = [1.0, 0.8, 0.2, 1.0];
const EDITBOX_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_BORDER: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const EDITBOX_FOCUSED_BORDER: [f32; 4] = [1.0, 0.92, 0.72, 1.0];
const STATUS_FILL_FIELDS: &str = "Please fill in all fields";
const STATUS_CONNECTING: &str = "Connecting...";
const STATUS_MENU_UNAVAILABLE: &str = "Menu is not implemented yet";
const STATUS_RECONNECT_UNAVAILABLE: &str = "No saved session to reconnect";
const LOGIN_BUTTON_GENERATED_REGULAR_UP_ATLAS: &str = "login-generated-regular-up";
const LOGIN_BUTTON_GENERATED_REGULAR_PRESSED_ATLAS: &str = "login-generated-regular-pressed";
const LOGIN_BUTTON_GENERATED_REGULAR_HIGHLIGHT_ATLAS: &str = "login-generated-regular-highlight";
const LOGIN_BUTTON_GENERATED_REGULAR_DISABLED_ATLAS: &str = "login-generated-regular-disabled";
const LOGIN_BUTTON_GENERATED_REGULAR_RAW: &str = "output/imagegen/button-dark-bronze-regular.ktx2";
const LOGIN_BUTTON_GENERATED_KNOTWORK: &str = "output/imagegen/button-carved-bronze-knotwork.ktx2";
const LOGIN_BUTTON_GENERATED_WALNUT: &str = "output/imagegen/button-walnut-bronze-framed.ktx2";
const LOGIN_BUTTON_ATLAS_UP: &str = "128-brownbutton-up";
const LOGIN_BUTTON_ATLAS_PRESSED: &str = "128-brownbutton-pressed";
const LOGIN_BUTTON_ATLAS_HIGHLIGHT: &str = "128-brownbutton-highlight";
const LOGIN_BUTTON_ATLAS_DISABLED: &str = "128-brownbutton-disable";

#[derive(Resource)]
struct LoginUi {
    root: u64,
    username_input: u64,
    password_input: u64,
    connect_button: u64,
    reconnect_button: u64,
    create_account_button: u64,
    menu_button: u64,
    exit_button: u64,
    status_text: u64,
}

#[derive(Resource, Default)]
struct LoginFocus(Option<u64>);

#[derive(Resource, Default)]
struct LoginStatus(String);

#[derive(Resource)]
struct LoginFadeIn(f32);

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

fn build_login_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut status: ResMut<LoginStatus>,
    mut auth_feedback: ResMut<networking::AuthUiFeedback>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    status.0 = auth_feedback.0.take().unwrap_or_default();
    let reg = &mut ui.registry;
    let (sw, sh) = (reg.screen_width, reg.screen_height);
    let (root, ui_root) = build_login_background(reg, sw, sh);
    build_login_titles(reg, ui_root, sw, sh);
    let (username_input, password_input) = build_login_inputs(reg, ui_root, sw, sh);
    let (
        connect_button,
        reconnect_button,
        create_account_button,
        menu_button,
        exit_button,
        status_text,
    ) = build_login_buttons(reg, ui_root, sw, sh, password_input);
    reg.set_alpha(root, 0.0);
    commands.insert_resource(LoginFadeIn(0.0));
    commands.insert_resource(LoginUi {
        root,
        username_input,
        password_input,
        connect_button,
        reconnect_button,
        create_account_button,
        menu_button,
        exit_button,
        status_text,
    });
}

fn build_login_background(reg: &mut FrameRegistry, sw: f32, sh: f32) -> (u64, u64) {
    let root = create_frame(reg, "LoginRoot", None, WidgetType::Frame, sw, sh);
    set_layout(reg, root, 0.0, 0.0, sw, sh);
    set_bg(reg, root, [0.01, 0.01, 0.01, 1.0]);
    set_strata(reg, root, FrameStrata::Background);

    let black_bg = create_frame(
        reg,
        "BlackLoginBackground",
        Some(root),
        WidgetType::Frame,
        sw,
        sh,
    );
    set_layout(reg, black_bg, 0.0, 0.0, sw, sh);
    set_bg(reg, black_bg, [0.0, 0.0, 0.0, 1.0]);
    set_strata(reg, black_bg, FrameStrata::Background);

    let background_model = create_frame(
        reg,
        "LoginBackgroundModel",
        Some(root),
        WidgetType::Model,
        sw,
        sh,
    );
    set_layout(reg, background_model, 0.0, 0.0, sw, sh);
    set_strata(reg, background_model, FrameStrata::Background);
    let (bg_x, bg_y, bg_w, bg_h) =
        centered_cover_rect(sw, sh, LOGIN_BACKGROUND_SIZE.0, LOGIN_BACKGROUND_SIZE.1);

    let bg = create_texture(
        reg,
        "LoginBackground",
        Some(background_model),
        bg_w,
        bg_h,
        TEX_LOGIN_BACKGROUND,
    );
    set_layout(reg, bg, bg_x, bg_y, bg_w, bg_h);
    set_strata(reg, bg, FrameStrata::Background);
    let overlay = create_frame(
        reg,
        "LoginBackgroundShade",
        Some(background_model),
        WidgetType::Frame,
        sw,
        sh,
    );
    set_layout(reg, overlay, 0.0, 0.0, sw, sh);
    set_bg(reg, overlay, [0.0, 0.0, 0.0, 0.22]);
    set_strata(reg, overlay, FrameStrata::Background);

    let ui = create_frame(reg, "LoginUI", Some(root), WidgetType::Frame, sw, sh);
    set_layout(reg, ui, 0.0, 0.0, sw, sh);
    set_strata(reg, ui, FrameStrata::Medium);

    (root, ui)
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
    set_strata(reg, logo, FrameStrata::High);
    set_layout(reg, logo, 3.0, 7.0, 256.0, 128.0);
}

fn build_login_inputs(reg: &mut FrameRegistry, root: u64, _sw: f32, _sh: f32) -> (u64, u64) {
    let panel_w = 320.0;
    let eb_h = 42.0;

    // UsernameInput: CENTER on root, y=50.
    let username = build_editbox_with_label(reg, root, panel_w, eb_h, "Username", "UsernameInput");
    set_strata(reg, username, FrameStrata::Medium);
    set_anchor(
        reg,
        username,
        AnchorPoint::Center,
        Some(root),
        AnchorPoint::Center,
        0.0,
        50.0,
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

    (username, password)
}

fn build_login_buttons(
    reg: &mut FrameRegistry,
    root: u64,
    sw: f32,
    sh: f32,
    password_input: u64,
) -> (u64, u64, u64, u64, u64, u64) {
    let panel_w = 320.0;

    let (connect, reconnect) = build_main_buttons(reg, root, password_input);
    let exit = build_exit_button(reg, root, sw, sh);
    let create_account = build_action_button_anchored(
        reg,
        root,
        "CreateAccountButton",
        "Create Account",
        AnchorPoint::Bottom,
        Some(exit),
        AnchorPoint::Top,
        0.0,
        10.0,
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
        Some(connect),
        AnchorPoint::Bottom,
        0.0,
        -20.0,
    );
    set_font_string(reg, status, "", 13.0, [0.9, 0.5, 0.5, 1.0]);
    build_footer_text(reg, root, sw, sh);
    (connect, reconnect, create_account, menu, exit, status)
}

fn build_main_buttons(reg: &mut FrameRegistry, root: u64, password_input: u64) -> (u64, u64) {
    let connect = create_button(
        reg,
        "ConnectButton",
        Some(root),
        MAIN_LOGIN_BUTTON_SIZE.0,
        MAIN_LOGIN_BUTTON_SIZE.1,
        "Login",
    );
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
    set_login_primary_button_textures(reg, connect);
    set_button_font_size(reg, connect, 16.0);
    let reconnect = create_button(
        reg,
        "ReconnectButton",
        Some(root),
        MAIN_LOGIN_BUTTON_SIZE.0,
        MAIN_LOGIN_BUTTON_SIZE.1,
        "Reconnect",
    );
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
    set_login_primary_button_textures(reg, reconnect);
    set_button_font_size(reg, reconnect, 16.0);
    hide_frame(reg, reconnect);
    (connect, reconnect)
}

fn build_exit_button(reg: &mut FrameRegistry, root: u64, _sw: f32, _sh: f32) -> u64 {
    let exit = create_button(
        reg,
        "ExitButton",
        Some(root),
        GLUE_BUTTON_SIZE.0,
        GLUE_BUTTON_SIZE.1,
        "Quit",
    );
    set_strata(reg, exit, FrameStrata::Medium);
    set_button_font_size(reg, exit, 12.0);
    set_anchor(
        reg,
        exit,
        AnchorPoint::BottomRight,
        Some(root),
        AnchorPoint::BottomRight,
        -24.0,
        56.0,
    );
    set_button_atlases(
        reg,
        exit,
        LOGIN_BUTTON_ATLAS_UP,
        LOGIN_BUTTON_ATLAS_PRESSED,
        LOGIN_BUTTON_ATLAS_HIGHLIGHT,
        LOGIN_BUTTON_ATLAS_DISABLED,
    );
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
    let btn = create_button(
        reg,
        name,
        Some(root),
        GLUE_BUTTON_SIZE.0,
        GLUE_BUTTON_SIZE.1,
        text,
    );
    set_strata(reg, btn, FrameStrata::Medium);
    set_button_font_size(reg, btn, 12.0);
    set_anchor(reg, btn, point, relative_to, relative_point, x_off, y_off);
    set_button_atlases(
        reg,
        btn,
        LOGIN_BUTTON_ATLAS_UP,
        LOGIN_BUTTON_ATLAS_PRESSED,
        LOGIN_BUTTON_ATLAS_HIGHLIGHT,
        LOGIN_BUTTON_ATLAS_DISABLED,
    );
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
    let label_font_size = 18.0_f32;
    let label = create_frame(
        reg,
        &format!("{name}Label"),
        Some(input),
        WidgetType::FontString,
        w,
        label_font_size,
    );
    set_anchor(
        reg,
        label,
        AnchorPoint::Bottom,
        Some(input),
        AnchorPoint::Top,
        0.0,
        0.0,
    );
    set_font_string_with_font(
        reg,
        label,
        label_text,
        FONT_GLUE_LABEL,
        18.0,
        GLUE_NORMAL_FONT_COLOR,
    );
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
    set_strata(reg, logo, FrameStrata::High);
    set_anchor(
        reg,
        logo,
        AnchorPoint::Bottom,
        Some(root),
        AnchorPoint::Bottom,
        0.0,
        40.0,
    );
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
            // Match AccountLogin.xml text insets for the glue edit boxes.
            eb.text_insets = [12.0, 5.0, 0.0, 5.0];
            eb.font = FONT_GLUE_EDITBOX.to_string();
            eb.font_size = 16.0;
            eb.text_color = GLUE_EDITBOX_TEXT_COLOR;
        }
    }
}

fn common_input_border_part_textures() -> [TextureSource; 9] {
    [
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-TL.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-T.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-TR.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-L.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-M.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-R.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-BL.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-B.blp".to_string(),
        ),
        TextureSource::File(
            "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-BR.blp".to_string(),
        ),
    ]
}

fn sync_editbox_focus_visual(reg: &mut FrameRegistry, id: u64, focused: bool) {
    let Some(frame) = reg.get_mut(id) else {
        return;
    };
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
    auth_token: Res<networking::AuthToken>,
    server_addr: Option<Res<networking::ServerAddr>>,
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
        &auth_token,
        server_addr.as_ref().map(|addr| addr.0),
        &mut commands,
        Some(&mut exit),
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
    auth_token: &networking::AuthToken,
    server_addr: Option<std::net::SocketAddr>,
    commands: &mut Commands,
    exit: Option<&mut MessageWriter<AppExit>>,
) {
    let (cx, cy) = (cursor.x, cursor.y);
    if hit_frame(ui, login.username_input, cx, cy) {
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
            auth_token,
            server_addr,
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
    auth_token: &networking::AuthToken,
    server_addr: Option<std::net::SocketAddr>,
    commands: &mut Commands,
    exit: Option<&mut MessageWriter<AppExit>>,
) {
    if hit_active_frame(ui, login.connect_button, cx, cy) {
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
            server_addr,
            commands,
        );
    } else if hit_active_frame(ui, login.reconnect_button, cx, cy) {
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
    let Some(action) = queue.pop() else {
        return;
    };
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
fn run_login_automation_action(
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
        UiAutomationAction::ClickFrame(frame_name) => {
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
                frame_name,
            )?;
        }
        UiAutomationAction::TypeText(text) => {
            let Some(focused_id) = focus.0 else {
                return Err("automation type requires a focused edit box".to_string());
            };
            for ch in text.chars() {
                insert_char_into_editbox(&mut ui.registry, focused_id, &ch.to_string());
            }
        }
        UiAutomationAction::PressKey(key) => {
            let Some(focused_id) = focus.0 else {
                return Err("automation key press requires a focused frame".to_string());
            };
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
    let _ = login;
    let Some(frame_id) = ui.registry.get_by_name(frame_name) else {
        return Err(format!("unknown login frame '{frame_name}'"));
    };
    let Some(rect) = ui
        .registry
        .get(frame_id)
        .and_then(|frame| frame.layout_rect.as_ref())
        .cloned()
    else {
        return Err(format!("login frame '{frame_name}' has no layout rect"));
    };
    handle_mouse_click(
        ui,
        login,
        Vec2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0),
        focus,
        next_state,
        status,
        login_mode,
        auth_token,
        server_addr,
        commands,
        None,
    );
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
        KeyCode::ArrowLeft => editbox_move_cursor(&mut ui.registry, focused_id, -1),
        KeyCode::ArrowRight => editbox_move_cursor(&mut ui.registry, focused_id, 1),
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
    focus: Res<LoginFocus>,
    login_mode: Res<networking::LoginMode>,
    auth_token: Res<networking::AuthToken>,
) {
    let Some(login) = login_ui.as_ref() else {
        return;
    };
    ui.focused_frame = focus.0;
    sync_button_states(&mut ui.registry, login, &*login_mode, &auth_token);
    sync_status_text(&mut ui.registry, login.status_text, &status.0);
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

fn sync_button_states(
    reg: &mut FrameRegistry,
    login: &LoginUi,
    mode: &networking::LoginMode,
    _auth_token: &networking::AuthToken,
) {
    reg.set_shown(login.connect_button, true);
    reg.set_shown(login.reconnect_button, false);
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
            networking::LoginMode::Login => "Create Account".to_string(),
            networking::LoginMode::Register => "Back to Login".to_string(),
        };
    }
}

fn sync_status_text(reg: &mut FrameRegistry, id: u64, text: &str) {
    if let Some(WidgetData::FontString(fs)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        fs.text = text.to_string();
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
    commands.insert_resource(mode.clone());
    status.0 = STATUS_CONNECTING.to_string();
    next_state.set(GameState::Connecting);
}

fn try_reconnect(
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

fn toggle_login_mode(mode: &mut networking::LoginMode, reg: &mut FrameRegistry, login: &LoginUi) {
    *mode = match mode {
        networking::LoginMode::Login => networking::LoginMode::Register,
        networking::LoginMode::Register => networking::LoginMode::Login,
    };
    sync_button_states(reg, login, mode, &networking::AuthToken(None));
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
    frame.raise_order = id as i32;
    if let Some(parent_id) = parent
        && let Some(parent_frame) = reg.get(parent_id)
    {
        frame.frame_level = parent_frame.frame_level + 1;
        frame.visible = parent_frame.visible && frame.shown;
        frame.effective_alpha = parent_frame.effective_alpha * frame.alpha;
        frame.effective_scale = parent_frame.effective_scale * frame.scale;
    }
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

fn centered_cover_rect(sw: f32, sh: f32, tex_w: f32, tex_h: f32) -> (f32, f32, f32, f32) {
    let scale = (sw / tex_w).max(sh / tex_h);
    let w = tex_w * scale;
    let h = tex_h * scale;
    ((sw - w) * 0.5, (sh - h) * 0.5, w, h)
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

fn set_font_string_with_font(
    reg: &mut FrameRegistry,
    id: u64,
    text: &str,
    font: &str,
    size: f32,
    color: [f32; 4],
) {
    if let Some(frame) = reg.get_mut(id) {
        frame.widget_data = Some(WidgetData::FontString(FontStringData {
            text: text.to_string(),
            font: font.to_string(),
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

fn set_button_font_size(reg: &mut FrameRegistry, id: u64, font_size: f32) {
    if let Some(WidgetData::Button(button)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        button.font_size = font_size;
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

fn set_button_atlases(
    reg: &mut FrameRegistry,
    id: u64,
    normal: &str,
    pushed: &str,
    highlight: &str,
    disabled: &str,
) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.normal_texture = Some(TextureSource::Atlas(normal.to_string()));
        bd.pushed_texture = Some(TextureSource::Atlas(pushed.to_string()));
        bd.highlight_texture = Some(TextureSource::Atlas(highlight.to_string()));
        bd.disabled_texture = Some(TextureSource::Atlas(disabled.to_string()));
    }
}

fn set_login_primary_button_textures(reg: &mut FrameRegistry, id: u64) {
    match selected_generated_login_button_path() {
        Some(path) => set_button_files(reg, id, path, path, path, path),
        None => set_button_atlases(
            reg,
            id,
            LOGIN_BUTTON_GENERATED_REGULAR_UP_ATLAS,
            LOGIN_BUTTON_GENERATED_REGULAR_PRESSED_ATLAS,
            LOGIN_BUTTON_GENERATED_REGULAR_HIGHLIGHT_ATLAS,
            LOGIN_BUTTON_GENERATED_REGULAR_DISABLED_ATLAS,
        ),
    }
}

fn selected_generated_login_button_path() -> Option<&'static str> {
    match std::env::var("LOGIN_BUTTON_VARIANT").ok().as_deref() {
        Some("regular") => Some(LOGIN_BUTTON_GENERATED_REGULAR_RAW),
        Some("knotwork") => Some(LOGIN_BUTTON_GENERATED_KNOTWORK),
        Some("walnut") => Some(LOGIN_BUTTON_GENERATED_WALNUT),
        _ => None,
    }
}

fn set_button_files(
    reg: &mut FrameRegistry,
    id: u64,
    normal: &str,
    pushed: &str,
    highlight: &str,
    disabled: &str,
) {
    if let Some(WidgetData::Button(bd)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) {
        bd.normal_texture = Some(TextureSource::File(normal.to_string()));
        bd.pushed_texture = Some(TextureSource::File(pushed.to_string()));
        bd.highlight_texture = Some(TextureSource::File(highlight.to_string()));
        bd.disabled_texture = Some(TextureSource::File(disabled.to_string()));
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

fn hit_active_frame(ui: &UiState, frame_id: u64, mx: f32, my: f32) -> bool {
    ui.registry
        .get(frame_id)
        .is_some_and(|frame| frame.visible && frame.shown)
        && hit_frame(ui, frame_id, mx, my)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    fn login_fixture() -> (FrameRegistry, LoginUi) {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let root = create_frame(
            &mut reg,
            "LoginRoot",
            None,
            WidgetType::Frame,
            1920.0,
            1080.0,
        );
        let username_input = create_editbox(&mut reg, "UsernameInput", Some(root), 320.0, 42.0);
        let password_input = create_editbox(&mut reg, "PasswordInput", Some(root), 320.0, 42.0);
        let connect_button = create_button(
            &mut reg,
            "ConnectButton",
            Some(root),
            MAIN_LOGIN_BUTTON_SIZE.0,
            MAIN_LOGIN_BUTTON_SIZE.1,
            "Login",
        );
        let reconnect_button = create_button(
            &mut reg,
            "ReconnectButton",
            Some(root),
            MAIN_LOGIN_BUTTON_SIZE.0,
            MAIN_LOGIN_BUTTON_SIZE.1,
            "Reconnect",
        );
        let create_account_button = create_button(
            &mut reg,
            "CreateAccountButton",
            Some(root),
            200.0,
            32.0,
            "Create Account",
        );
        let menu_button = create_button(&mut reg, "MenuButton", Some(root), 200.0, 32.0, "Menu");
        let exit_button = create_button(&mut reg, "ExitButton", Some(root), 200.0, 32.0, "Quit");
        let status_text = create_frame(
            &mut reg,
            "LoginStatus",
            Some(root),
            WidgetType::FontString,
            320.0,
            24.0,
        );
        set_layout(&mut reg, root, 0.0, 0.0, 1920.0, 1080.0);
        set_layout(&mut reg, username_input, 800.0, 400.0, 320.0, 42.0);
        set_layout(&mut reg, password_input, 800.0, 460.0, 320.0, 42.0);
        set_layout(
            &mut reg,
            connect_button,
            800.0,
            522.0,
            MAIN_LOGIN_BUTTON_SIZE.0,
            MAIN_LOGIN_BUTTON_SIZE.1,
        );
        set_layout(
            &mut reg,
            reconnect_button,
            800.0,
            522.0,
            MAIN_LOGIN_BUTTON_SIZE.0,
            MAIN_LOGIN_BUTTON_SIZE.1,
        );
        set_layout(&mut reg, create_account_button, 860.0, 630.0, 200.0, 32.0);
        set_layout(&mut reg, menu_button, 860.0, 672.0, 200.0, 32.0);
        set_layout(&mut reg, exit_button, 1700.0, 980.0, 200.0, 32.0);
        set_layout(&mut reg, status_text, 800.0, 620.0, 320.0, 24.0);
        (
            reg,
            LoginUi {
                root,
                username_input,
                password_input,
                connect_button,
                reconnect_button,
                create_account_button,
                menu_button,
                exit_button,
                status_text,
            },
        )
    }

    fn set_editbox_text_for_test(reg: &mut FrameRegistry, id: u64, text: &str) {
        let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut())
        else {
            panic!("expected edit box");
        };
        eb.text = text.to_string();
        eb.cursor_position = eb.text.len();
    }

    #[test]
    fn automation_click_focuses_username_editbox() {
        let (reg, login) = login_fixture();
        let mut ui = UiState {
            registry: reg,
            event_bus: game_engine::ui::event::EventBus::new(),
            wasm_host: game_engine::ui::wasm_host::WasmHost::new(),
            focused_frame: None,
        };
        let mut focus = LoginFocus::default();
        let mut next_state = NextState::<GameState>::default();
        let mut status = LoginStatus::default();
        let mut login_mode = networking::LoginMode::Login;
        let auth_token = networking::AuthToken(None);
        let mut world = World::new();
        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);

        {
            let mut commands = system_state.get_mut(&mut world);
            run_login_automation_action(
                &mut ui,
                &login,
                &mut focus,
                &mut next_state,
                &mut status,
                &mut login_mode,
                &auth_token,
                None,
                &mut commands,
                &UiAutomationAction::ClickFrame("UsernameInput".to_string()),
            )
            .expect("automation click should succeed");
        }

        assert_eq!(focus.0, Some(login.username_input));
        assert!(matches!(next_state, NextState::Unchanged));
    }

    #[test]
    fn automation_type_uses_login_editbox_code_path() {
        let (reg, login) = login_fixture();
        let mut ui = UiState {
            registry: reg,
            event_bus: game_engine::ui::event::EventBus::new(),
            wasm_host: game_engine::ui::wasm_host::WasmHost::new(),
            focused_frame: None,
        };
        let mut focus = LoginFocus(Some(login.username_input));
        let mut next_state = NextState::<GameState>::default();
        let mut status = LoginStatus::default();
        let mut login_mode = networking::LoginMode::Login;
        let auth_token = networking::AuthToken(None);
        let mut world = World::new();
        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);

        {
            let mut commands = system_state.get_mut(&mut world);
            run_login_automation_action(
                &mut ui,
                &login,
                &mut focus,
                &mut next_state,
                &mut status,
                &mut login_mode,
                &auth_token,
                None,
                &mut commands,
                &UiAutomationAction::TypeText("alice".to_string()),
            )
            .expect("automation typing should succeed");
        }

        assert_eq!(
            get_editbox_text(&ui.registry, login.username_input),
            "alice"
        );
    }

    #[test]
    fn automation_login_reaches_connecting_state() {
        let (reg, login) = login_fixture();
        let mut ui = UiState {
            registry: reg,
            event_bus: game_engine::ui::event::EventBus::new(),
            wasm_host: game_engine::ui::wasm_host::WasmHost::new(),
            focused_frame: None,
        };
        let mut focus = LoginFocus::default();
        let mut next_state = NextState::<GameState>::default();
        let mut status = LoginStatus::default();
        let mut login_mode = networking::LoginMode::Login;
        let auth_token = networking::AuthToken(None);
        let mut world = World::new();
        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);

        {
            let mut commands = system_state.get_mut(&mut world);
            let actions = [
                UiAutomationAction::ClickFrame("UsernameInput".to_string()),
                UiAutomationAction::TypeText("alice".to_string()),
                UiAutomationAction::ClickFrame("PasswordInput".to_string()),
                UiAutomationAction::TypeText("secret".to_string()),
                UiAutomationAction::ClickFrame("ConnectButton".to_string()),
            ];
            for action in actions {
                run_login_automation_action(
                    &mut ui,
                    &login,
                    &mut focus,
                    &mut next_state,
                    &mut status,
                    &mut login_mode,
                    &auth_token,
                    None,
                    &mut commands,
                    &action,
                )
                .expect("automation action should succeed");
            }
        }
        system_state.apply(&mut world);

        assert_eq!(status.0, STATUS_CONNECTING);
        assert!(matches!(
            next_state,
            NextState::Pending(GameState::Connecting)
        ));
        assert_eq!(world.resource::<networking::LoginUsername>().0, "alice");
        assert_eq!(world.resource::<networking::LoginPassword>().0, "secret");
    }

    #[test]
    fn try_connect_requires_all_fields() {
        let (reg, login) = login_fixture();
        let mut status = LoginStatus::default();
        let mut next_state = NextState::<GameState>::default();
        let mut world = World::new();
        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);

        {
            let mut commands = system_state.get_mut(&mut world);
            try_connect(
                &reg,
                &login,
                &mut status,
                &mut next_state,
                &networking::LoginMode::Login,
                None,
                &mut commands,
            );
        }
        system_state.apply(&mut world);

        assert_eq!(status.0, "Please fill in all fields");
        assert!(matches!(next_state, NextState::Unchanged));
        assert!(!world.contains_resource::<networking::ServerAddr>());
    }

    #[test]
    fn try_connect_stores_credentials_and_enters_connecting_state() {
        let (mut reg, login) = login_fixture();
        set_editbox_text_for_test(&mut reg, login.username_input, "alice");
        set_editbox_text_for_test(&mut reg, login.password_input, "secret");
        let mut status = LoginStatus::default();
        let mut next_state = NextState::<GameState>::default();
        let mut world = World::new();
        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);

        {
            let mut commands = system_state.get_mut(&mut world);
            try_connect(
                &reg,
                &login,
                &mut status,
                &mut next_state,
                &networking::LoginMode::Login,
                None,
                &mut commands,
            );
        }
        system_state.apply(&mut world);

        assert_eq!(status.0, "Connecting...");
        assert!(matches!(
            next_state,
            NextState::Pending(GameState::Connecting)
        ));
        assert_eq!(
            world.resource::<networking::ServerAddr>().0,
            DEFAULT_SERVER_ADDR.parse().unwrap()
        );
        assert_eq!(world.resource::<networking::LoginUsername>().0, "alice");
        assert_eq!(world.resource::<networking::LoginPassword>().0, "secret");
        assert!(matches!(
            *world.resource::<networking::LoginMode>(),
            networking::LoginMode::Login
        ));
    }

    #[test]
    fn sync_button_states_keeps_login_button_visible_even_with_saved_token() {
        let (mut reg, login) = login_fixture();

        sync_button_states(
            &mut reg,
            &login,
            &networking::LoginMode::Login,
            &networking::AuthToken(Some("saved-token".to_string())),
        );
        assert!(
            reg.get(login.connect_button)
                .expect("connect button")
                .visible
        );
        assert!(
            !reg.get(login.reconnect_button)
                .expect("reconnect button")
                .visible
        );

        sync_button_states(
            &mut reg,
            &login,
            &networking::LoginMode::Register,
            &networking::AuthToken(Some("saved-token".to_string())),
        );
        assert!(
            reg.get(login.connect_button)
                .expect("connect button")
                .visible
        );
        assert!(
            !reg.get(login.reconnect_button)
                .expect("reconnect button")
                .visible
        );
    }

    #[test]
    fn build_login_ui_shows_pending_auth_error_message() {
        let mut app = App::new();
        app.insert_resource(UiState {
            registry: FrameRegistry::new(0.0, 0.0),
            event_bus: game_engine::ui::event::EventBus::new(),
            wasm_host: game_engine::ui::wasm_host::WasmHost::new(),
            focused_frame: None,
        });
        app.init_resource::<LoginStatus>();
        app.insert_resource(networking::AuthUiFeedback(Some(
            "Incorrect username or password".to_string(),
        )));

        let mut window = Window::default();
        window.resolution.set(1280.0, 720.0);
        app.world_mut().spawn((window, bevy::window::PrimaryWindow));

        let _ = app.world_mut().run_system_cached(build_login_ui);

        assert_eq!(
            app.world().resource::<LoginStatus>().0,
            "Incorrect username or password"
        );
        assert!(
            app.world()
                .resource::<networking::AuthUiFeedback>()
                .0
                .is_none()
        );
    }

    #[test]
    fn try_connect_preserves_explicit_server_address() {
        let (mut reg, login) = login_fixture();
        set_editbox_text_for_test(&mut reg, login.username_input, "alice");
        set_editbox_text_for_test(&mut reg, login.password_input, "secret");
        let mut status = LoginStatus::default();
        let mut next_state = NextState::<GameState>::default();
        let mut world = World::new();
        let mut system_state: SystemState<Commands> = SystemState::new(&mut world);
        let explicit_addr = "127.0.0.1:5000"
            .parse()
            .expect("test server address should parse");

        {
            let mut commands = system_state.get_mut(&mut world);
            try_connect(
                &reg,
                &login,
                &mut status,
                &mut next_state,
                &networking::LoginMode::Login,
                Some(explicit_addr),
                &mut commands,
            );
        }
        system_state.apply(&mut world);

        assert_eq!(world.resource::<networking::ServerAddr>().0, explicit_addr);
    }
}
