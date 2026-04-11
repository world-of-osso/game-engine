use bevy::ecs::system::SystemState;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use game_engine::ui::frame::{Dimension, NineSlice, WidgetData};
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screen::Screen;
use game_engine::ui::screens::login_component::{
    CONNECT_BUTTON, CREATE_ACCOUNT_BUTTON, EXIT_BUTTON, LOGIN_ROOT, LOGIN_STATUS, MENU_BUTTON,
    PASSWORD_INPUT, REALM_BUTTON, SharedConnecting, SharedRealmSelectable, SharedRealmText,
    SharedStatusText, USERNAME_INPUT, login_screen,
};

use crate::game_state::GameState;
use crate::networking;

use super::helpers::get_editbox_text;
use super::{
    LoginFocus, LoginStatus, LoginUi, run_login_automation_action, sync_button_visibility,
    try_connect,
};

use game_engine::ui::automation::UiAutomationAction;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};

fn build_login_screen_for_test() -> (Screen, ui_toolkit::screen::SharedContext) {
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert::<SharedStatusText>(SharedStatusText::default());
    let screen = Screen::new(login_screen);
    (screen, shared)
}

fn build_login_registry_with_real_layout() -> (FrameRegistry, LoginUi) {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut screen_res =
        super::view::build_login_screen(&LoginStatus::default(), "Development".to_string(), true);
    screen_res.screen.sync(&screen_res.shared, &mut reg);
    let login = resolve_login_ui(&reg);
    super::apply_post_setup(&mut reg, &login);
    recompute_layouts(&mut reg);
    (reg, login)
}

fn layout_rect(reg: &FrameRegistry, id: u64) -> LayoutRect {
    reg.get(id)
        .and_then(|frame| frame.layout_rect.clone())
        .expect("layout_rect")
}

#[test]
fn build_login_screen_creates_all_critical_login_frames() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut screen_res =
        super::view::build_login_screen(&LoginStatus::default(), "Development".to_string(), true);
    screen_res.screen.sync(&screen_res.shared, &mut reg);

    for frame_name in [
        LOGIN_ROOT,
        USERNAME_INPUT,
        PASSWORD_INPUT,
        REALM_BUTTON,
        CONNECT_BUTTON,
        CREATE_ACCOUNT_BUTTON,
        MENU_BUTTON,
        EXIT_BUTTON,
        LOGIN_STATUS,
    ] {
        assert!(
            reg.get_by_name(frame_name.0).is_some(),
            "expected {} to exist after build_login_screen",
            frame_name.0
        );
    }
}

#[test]
fn login_form_is_vertically_centered_with_connect_button_near_screen_midpoint() {
    let (reg, login) = build_login_registry_with_real_layout();
    let rect = layout_rect(&reg, login.connect_button);
    let center_y = rect.y + rect.height * 0.5;

    assert!(
        (center_y - 540.0).abs() <= 10.0,
        "expected ConnectButton center Y near 540, got {center_y}"
    );
}

#[test]
fn login_form_is_horizontally_centered_with_inputs_near_screen_midpoint() {
    let (reg, login) = build_login_registry_with_real_layout();

    for (label, input_id) in [
        ("UsernameInput", login.username_input),
        ("PasswordInput", login.password_input),
    ] {
        let rect = layout_rect(&reg, input_id);
        let center_x = rect.x + rect.width * 0.5;

        assert!(
            (center_x - 960.0).abs() <= 10.0,
            "expected {label} center X near 960, got {center_x}"
        );
    }
}

#[test]
fn login_form_preserves_expected_vertical_ordering() {
    let (reg, login) = build_login_registry_with_real_layout();
    let username = layout_rect(&reg, login.username_input);
    let password = layout_rect(&reg, login.password_input);
    let realm = layout_rect(&reg, login.realm_button);
    let connect = layout_rect(&reg, login.connect_button);
    let create_account = layout_rect(&reg, login.create_account_button);

    assert!(
        username.y < password.y,
        "expected UsernameInput above PasswordInput, got {} >= {}",
        username.y,
        password.y
    );
    assert!(
        password.y < realm.y,
        "expected PasswordInput above RealmButton, got {} >= {}",
        password.y,
        realm.y
    );
    assert!(
        realm.y < connect.y,
        "expected RealmButton above ConnectButton, got {} >= {}",
        realm.y,
        connect.y
    );
    assert!(
        connect.y < create_account.y,
        "expected ConnectButton above CreateAccountButton, got {} >= {}",
        connect.y,
        create_account.y
    );
}

#[test]
fn exit_button_is_anchored_in_the_bottom_right_quadrant() {
    let (reg, login) = build_login_registry_with_real_layout();
    let rect = layout_rect(&reg, login.exit_button);
    let center_x = rect.x + rect.width * 0.5;
    let center_y = rect.y + rect.height * 0.5;

    assert!(
        center_x > 960.0,
        "expected ExitButton center X in right half, got {center_x}"
    );
    assert!(
        center_y > 540.0,
        "expected ExitButton center Y in bottom half, got {center_y}"
    );
}

#[test]
fn realm_selector_is_positioned_between_password_and_connect_button() {
    let (reg, login) = build_login_registry_with_real_layout();
    let password = layout_rect(&reg, login.password_input);
    let realm = layout_rect(&reg, login.realm_button);
    let connect = layout_rect(&reg, login.connect_button);

    assert!(
        password.y < realm.y,
        "expected RealmButton below PasswordInput, got {} <= {}",
        realm.y,
        password.y
    );
    assert!(
        realm.y < connect.y,
        "expected RealmButton above ConnectButton, got {} >= {}",
        realm.y,
        connect.y
    );
}

#[test]
fn status_text_is_positioned_below_connect_button() {
    let (reg, login) = build_login_registry_with_real_layout();
    let connect = layout_rect(&reg, login.connect_button);
    let status = layout_rect(&reg, login.status_text);

    assert!(
        connect.y < status.y,
        "expected LoginStatus below ConnectButton, got {} >= {}",
        connect.y,
        status.y
    );
}

fn resolve_login_ui(reg: &FrameRegistry) -> LoginUi {
    let root = reg.get_by_name("LoginRoot").expect("LoginRoot");
    let username_input = reg.get_by_name("UsernameInput").expect("UsernameInput");
    let password_input = reg.get_by_name("PasswordInput").expect("PasswordInput");
    let realm_button = reg.get_by_name("RealmButton").expect("RealmButton");
    let connect_button = reg.get_by_name("ConnectButton").expect("ConnectButton");
    let reconnect_button = reg.get_by_name("ReconnectButton");
    let create_account_button = reg
        .get_by_name("CreateAccountButton")
        .expect("CreateAccountButton");
    let menu_button = reg.get_by_name("MenuButton").expect("MenuButton");
    let exit_button = reg.get_by_name("ExitButton").expect("ExitButton");
    let status_text = reg.get_by_name("LoginStatus").expect("LoginStatus");
    LoginUi {
        root,
        username_input,
        password_input,
        realm_button,
        connect_button,
        reconnect_button,
        create_account_button,
        menu_button,
        exit_button,
        status_text,
    }
}

fn login_fixture() -> (FrameRegistry, LoginUi) {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let (mut screen, shared) = build_login_screen_for_test();
    screen.sync(&shared, &mut reg);

    let login = resolve_login_ui(&reg);

    inject_layout_rects(
        &mut reg,
        LoginLayoutIds {
            root: login.root,
            username_input: login.username_input,
            password_input: login.password_input,
            realm_button: login.realm_button,
            connect_button: login.connect_button,
            reconnect_button: login.reconnect_button,
            create_account_button: login.create_account_button,
            menu_button: login.menu_button,
            exit_button: login.exit_button,
            status_text: login.status_text,
        },
    );

    (reg, login)
}

struct LoginLayoutIds {
    root: u64,
    username_input: u64,
    password_input: u64,
    realm_button: u64,
    connect_button: u64,
    reconnect_button: Option<u64>,
    create_account_button: u64,
    menu_button: u64,
    exit_button: u64,
    status_text: u64,
}

fn inject_layout_rects(reg: &mut FrameRegistry, ids: LoginLayoutIds) {
    use game_engine::ui::layout::LayoutRect;
    let set_rect = |reg: &mut FrameRegistry, id: u64, x: f32, y: f32, w: f32, h: f32| {
        if let Some(f) = reg.get_mut(id) {
            f.layout_rect = Some(LayoutRect {
                x,
                y,
                width: w,
                height: h,
            });
            f.width = Dimension::Fixed(w);
            f.height = Dimension::Fixed(h);
        }
    };
    set_rect(reg, ids.root, 0.0, 0.0, 1920.0, 1080.0);
    set_rect(reg, ids.username_input, 800.0, 400.0, 320.0, 42.0);
    set_rect(reg, ids.password_input, 800.0, 460.0, 320.0, 42.0);
    set_rect(reg, ids.realm_button, 800.0, 532.0, 320.0, 42.0);
    set_rect(reg, ids.connect_button, 800.0, 596.0, 250.0, 66.0);
    if let Some(reconnect_button) = ids.reconnect_button {
        set_rect(reg, reconnect_button, 800.0, 596.0, 250.0, 66.0);
    }
    set_rect(reg, ids.create_account_button, 860.0, 630.0, 200.0, 32.0);
    set_rect(reg, ids.menu_button, 860.0, 672.0, 200.0, 32.0);
    set_rect(reg, ids.exit_button, 1700.0, 980.0, 200.0, 32.0);
    set_rect(reg, ids.status_text, 800.0, 620.0, 320.0, 24.0);
}

fn set_editbox_text_for_test(reg: &mut FrameRegistry, id: u64, text: &str) {
    let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) else {
        panic!("expected edit box");
    };
    eb.text = text.to_string();
    eb.cursor_position = eb.text.len();
}

fn make_ui_state(reg: FrameRegistry) -> UiState {
    UiState {
        registry: reg,
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    }
}

fn make_world_with_commands() -> (World, SystemState<Commands<'static, 'static>>) {
    let mut world = World::new();
    let system_state = SystemState::new(&mut world);
    (world, system_state)
}

#[test]
fn automation_click_focuses_username_editbox() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus::default();
    let mut next_state = NextState::<GameState>::default();
    let mut status = LoginStatus::default();
    let mut login_mode = networking::LoginMode::Login;
    let auth_token = networking::AuthToken(None);
    let (mut world, mut system_state) = make_world_with_commands();

    {
        let mut commands = system_state.get_mut(&mut world);
        run_login_automation_action(
            crate::scenes::login::connect::LoginAutomationContext {
                ui: &mut ui,
                login: &login,
                focus: &mut focus,
                next_state: &mut next_state,
                status: &mut status,
                login_mode: &mut login_mode,
                auth_token: &auth_token,
                realm_selection: None,
                server_addr: None,
                server_hostname: None,
                commands: &mut commands,
            },
            &UiAutomationAction::ClickFrame("UsernameInput".to_string()),
        )
        .expect("automation click should succeed");
    }

    assert_eq!(focus.0, Some(login.username_input));
    assert!(matches!(next_state, NextState::Unchanged));
}

fn editbox_nine_slice(reg: &FrameRegistry, id: u64) -> &NineSlice {
    reg.get(id)
        .and_then(|frame| frame.nine_slice.as_ref())
        .expect("expected editbox nine-slice")
}

#[test]
fn login_editboxes_use_a_darkened_fill_multiplier() {
    let (mut reg, login) = login_fixture();
    super::apply_post_setup(&mut reg, &login);

    let username = editbox_nine_slice(&reg, login.username_input);
    let password = editbox_nine_slice(&reg, login.password_input);

    assert_eq!(username.bg_color, password.bg_color);
    assert!(
        username.bg_color[..3].iter().all(|channel| *channel < 1.0),
        "login editbox fill should darken the backdrop art instead of multiplying it by pure white"
    );
}

#[test]
fn automation_type_uses_login_editbox_code_path() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus(Some(login.username_input));
    let mut next_state = NextState::<GameState>::default();
    let mut status = LoginStatus::default();
    let mut login_mode = networking::LoginMode::Login;
    let auth_token = networking::AuthToken(None);
    let (mut world, mut system_state) = make_world_with_commands();

    {
        let mut commands = system_state.get_mut(&mut world);
        run_login_automation_action(
            crate::scenes::login::connect::LoginAutomationContext {
                ui: &mut ui,
                login: &login,
                focus: &mut focus,
                next_state: &mut next_state,
                status: &mut status,
                login_mode: &mut login_mode,
                auth_token: &auth_token,
                realm_selection: None,
                server_addr: None,
                server_hostname: None,
                commands: &mut commands,
            },
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
fn paste_shortcut_inserts_clipboard_text_into_focused_login_editbox() {
    let (mut reg, login) = login_fixture();
    set_editbox_text_for_test(&mut reg, login.username_input, "al");
    let mut ui = make_ui_state(reg);

    let handled = super::maybe_paste_into_login_editbox(
        &super::LoginModifierState {
            ctrl: true,
            super_key: false,
        },
        &KeyboardInput {
            key_code: KeyCode::KeyV,
            logical_key: Key::Character("v".into()),
            state: ButtonState::Pressed,
            text: Some("v".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
        &mut ui,
        login.username_input,
        &mut LoginStatus::default(),
        Ok("ice".to_string()),
    );

    assert!(handled);
    assert_eq!(
        get_editbox_text(&ui.registry, login.username_input),
        "alice"
    );
}

#[test]
fn paste_logical_key_inserts_clipboard_text_into_focused_login_editbox() {
    let (mut reg, login) = login_fixture();
    set_editbox_text_for_test(&mut reg, login.username_input, "al");
    let mut ui = make_ui_state(reg);

    let handled = super::maybe_paste_into_login_editbox(
        &super::LoginModifierState::default(),
        &KeyboardInput {
            key_code: KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified),
            logical_key: Key::Paste,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
        &mut ui,
        login.username_input,
        &mut LoginStatus::default(),
        Ok("ice".to_string()),
    );

    assert!(handled);
    assert_eq!(
        get_editbox_text(&ui.registry, login.username_input),
        "alice"
    );
}

#[test]
fn ctrl_v_without_text_payload_does_not_insert_literal_v() {
    let (mut reg, login) = login_fixture();
    set_editbox_text_for_test(&mut reg, login.username_input, "al");
    let mut ui = make_ui_state(reg);

    let inserted = super::maybe_insert_login_text(
        &KeyboardInput {
            key_code: KeyCode::KeyV,
            logical_key: Key::Character("v".into()),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        },
        &mut ui,
        login.username_input,
    );

    assert!(!inserted);
    assert_eq!(get_editbox_text(&ui.registry, login.username_input), "al");
}

fn run_login_actions(
    ui: &mut UiState,
    login: &LoginUi,
    focus: &mut LoginFocus,
    next_state: &mut NextState<GameState>,
    status: &mut LoginStatus,
    login_mode: &mut networking::LoginMode,
    auth_token: &networking::AuthToken,
    commands: &mut Commands,
    actions: &[UiAutomationAction],
) {
    for action in actions {
        run_login_automation_action(
            crate::scenes::login::connect::LoginAutomationContext {
                ui,
                login,
                focus,
                next_state,
                status,
                login_mode,
                auth_token,
                realm_selection: None,
                server_addr: None,
                server_hostname: None,
                commands,
            },
            action,
        )
        .expect("automation action should succeed");
    }
}

fn login_submit_actions() -> [UiAutomationAction; 5] {
    [
        UiAutomationAction::ClickFrame("UsernameInput".to_string()),
        UiAutomationAction::TypeText("alice".to_string()),
        UiAutomationAction::ClickFrame("PasswordInput".to_string()),
        UiAutomationAction::TypeText("secret".to_string()),
        UiAutomationAction::ClickFrame("ConnectButton".to_string()),
    ]
}

#[test]
fn automation_click_realm_button_cycles_selection_and_updates_server_resources() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus::default();
    let mut next_state = NextState::<GameState>::default();
    let mut status = LoginStatus::default();
    let mut login_mode = networking::LoginMode::Login;
    let auth_token = networking::AuthToken(None);
    let mut realm_selection = super::LoginRealmSelection::from_server(
        Some("127.0.0.1:5000".parse().unwrap()),
        Some("game.worldofosso.com:5000"),
        false,
    );
    let (mut world, mut system_state) = make_world_with_commands();

    {
        let mut commands = system_state.get_mut(&mut world);
        run_login_automation_action(
            crate::scenes::login::connect::LoginAutomationContext {
                ui: &mut ui,
                login: &login,
                focus: &mut focus,
                next_state: &mut next_state,
                status: &mut status,
                login_mode: &mut login_mode,
                auth_token: &auth_token,
                realm_selection: Some(&mut realm_selection),
                server_addr: None,
                server_hostname: None,
                commands: &mut commands,
            },
            &UiAutomationAction::ClickFrame("RealmButton".to_string()),
        )
        .expect("automation click should cycle realm");
    }
    system_state.apply(&mut world);

    assert_eq!(realm_selection.button_text(), "Development");
    assert_eq!(
        world.resource::<networking::ServerHostname>().0,
        "127.0.0.1:5000"
    );
    assert_eq!(
        world.resource::<networking::ServerAddr>().0,
        "127.0.0.1:5000".parse().unwrap()
    );
}

#[test]
fn automation_login_reaches_connecting_state() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus::default();
    let mut next_state = NextState::<GameState>::default();
    let mut status = LoginStatus::default();
    let mut login_mode = networking::LoginMode::Login;
    let auth_token = networking::AuthToken(None);
    let (mut world, mut system_state) = make_world_with_commands();

    {
        let mut commands = system_state.get_mut(&mut world);
        run_login_actions(
            &mut ui,
            &login,
            &mut focus,
            &mut next_state,
            &mut status,
            &mut login_mode,
            &auth_token,
            &mut commands,
            &login_submit_actions(),
        );
    }
    system_state.apply(&mut world);

    assert_eq!(status.0, super::STATUS_CONNECTING);
    assert!(matches!(
        next_state,
        NextState::Pending(GameState::Connecting)
    ));
    assert_eq!(world.resource::<networking::LoginUsername>().0, "alice");
    assert_eq!(world.resource::<networking::LoginPassword>().0, "secret");
}

#[test]
fn try_connect_requires_all_fields() {
    let (mut reg, login) = login_fixture();
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();
    let (mut world, mut system_state) = make_world_with_commands();

    {
        let mut commands = system_state.get_mut(&mut world);
        try_connect(
            &mut reg,
            &login,
            &mut status,
            &mut next_state,
            &networking::LoginMode::Login,
            None,
            None,
            &mut commands,
        );
    }
    system_state.apply(&mut world);

    assert_eq!(status.0, "Please fill in all fields");
    assert!(matches!(next_state, NextState::Unchanged));
    assert!(!world.contains_resource::<networking::ServerAddr>());
}

fn run_try_connect_with_credentials(
    reg: &mut FrameRegistry,
    login: &LoginUi,
    status: &mut LoginStatus,
    next_state: &mut NextState<GameState>,
    server_addr: Option<std::net::SocketAddr>,
    server_hostname: Option<&str>,
) -> World {
    let (mut world, mut system_state) = make_world_with_commands();
    {
        let mut commands = system_state.get_mut(&mut world);
        try_connect(
            reg,
            login,
            status,
            next_state,
            &networking::LoginMode::Login,
            server_addr,
            server_hostname,
            &mut commands,
        );
    }
    system_state.apply(&mut world);
    world
}

#[test]
fn try_connect_stores_credentials_and_enters_connecting_state() {
    let (mut reg, login) = login_fixture();
    set_editbox_text_for_test(&mut reg, login.username_input, "alice");
    set_editbox_text_for_test(&mut reg, login.password_input, "secret");
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();

    let world = run_try_connect_with_credentials(
        &mut reg,
        &login,
        &mut status,
        &mut next_state,
        None,
        None,
    );

    assert_eq!(status.0, "Connecting...");
    assert!(matches!(
        next_state,
        NextState::Pending(GameState::Connecting)
    ));
    assert_eq!(
        world.resource::<networking::ServerAddr>().0,
        super::connect::resolve_default_server()
    );
    assert_eq!(world.resource::<networking::LoginUsername>().0, "alice");
    assert_eq!(world.resource::<networking::LoginPassword>().0, "secret");
    assert!(matches!(
        *world.resource::<networking::LoginMode>(),
        networking::LoginMode::Login
    ));
}

#[test]
fn sync_button_visibility_keeps_login_button_visible() {
    let (mut reg, login) = login_fixture();

    sync_button_visibility(&mut reg, &login);
    assert!(
        reg.get(login.connect_button)
            .expect("connect button")
            .visible
    );
    assert!(login.reconnect_button.is_none());
}

#[test]
fn build_login_ui_shows_pending_auth_error_message() {
    let mut app = App::new();
    app.insert_resource(UiState {
        registry: FrameRegistry::new(0.0, 0.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.init_resource::<LoginStatus>();
    app.insert_resource(networking::AuthUiFeedback(Some(
        "Incorrect username or password".to_string(),
    )));

    let mut window = Window::default();
    window.resolution.set(1280.0, 720.0);
    app.world_mut().spawn((window, bevy::window::PrimaryWindow));

    let _ = app.world_mut().run_system_cached(super::build_login_ui);

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
    let explicit_addr = "127.0.0.1:5000"
        .parse()
        .expect("test server address should parse");

    let world = run_try_connect_with_credentials(
        &mut reg,
        &login,
        &mut status,
        &mut next_state,
        Some(explicit_addr),
        Some("127.0.0.1:5000"),
    );

    assert_eq!(world.resource::<networking::ServerAddr>().0, explicit_addr);
    assert_eq!(
        world.resource::<networking::ServerHostname>().0,
        "127.0.0.1:5000"
    );
}

fn make_login_app() -> App {
    let mut app = App::new();
    app.insert_resource(UiState {
        registry: FrameRegistry::new(0.0, 0.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.init_resource::<LoginStatus>();
    app.init_resource::<LoginFocus>();
    app.insert_resource(networking::AuthUiFeedback::default());
    app.insert_resource(networking::LoginMode::Login);
    app.insert_resource(networking::AuthToken(None));
    let mut window = Window::default();
    window.resolution.set(1280.0, 720.0);
    app.world_mut().spawn((window, bevy::window::PrimaryWindow));
    app
}

#[test]
fn login_update_visuals_updates_login_status_fontstring() {
    let mut app = make_login_app();
    let _ = app.world_mut().run_system_cached(super::build_login_ui);
    app.world_mut().resource_mut::<LoginStatus>().0 = "Server unavailable".to_string();
    let _ = app
        .world_mut()
        .run_system_cached(super::login_update_visuals);

    let ui = app.world().resource::<UiState>();
    let login = app.world().resource::<LoginUi>();
    let Some(WidgetData::FontString(fs)) = ui
        .registry
        .get(login.status_text)
        .and_then(|frame| frame.widget_data.as_ref())
    else {
        panic!("expected LoginStatus to be a font string");
    };

    assert_eq!(fs.text, "Server unavailable");
}

fn count_login_status_frames(app: &App) -> usize {
    let ui = app.world().resource::<UiState>();
    ui.registry
        .frames_iter()
        .filter(|frame| frame.name.as_deref() == Some("LoginStatus"))
        .count()
}

#[test]
fn login_update_visuals_does_not_duplicate_login_status_frames() {
    let mut app = make_login_app();
    let _ = app.world_mut().run_system_cached(super::build_login_ui);

    app.world_mut().resource_mut::<LoginStatus>().0 = "First error".to_string();
    let _ = app
        .world_mut()
        .run_system_cached(super::login_update_visuals);
    app.world_mut().resource_mut::<LoginStatus>().0 = "Second error".to_string();
    let _ = app
        .world_mut()
        .run_system_cached(super::login_update_visuals);

    let count = count_login_status_frames(&app);
    assert_eq!(
        count, 1,
        "expected exactly one LoginStatus frame, found {count}"
    );
}

#[test]
fn login_button_is_visible_and_enabled() {
    let (reg, login) = login_fixture();
    let frame = reg.get(login.connect_button).expect("connect button");
    assert!(frame.visible, "Login button should be visible");
    assert!(!frame.hidden, "Login button should not be hidden");
    let Some(WidgetData::Button(bd)) = &frame.widget_data else {
        panic!("expected button widget data");
    };
    assert_eq!(
        bd.state,
        game_engine::ui::widgets::button::ButtonState::Normal,
        "Login button should be enabled (Normal state), not {:?}",
        bd.state,
    );
}

#[test]
fn login_button_stays_enabled_after_screen_sync() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert::<SharedStatusText>(SharedStatusText(String::new()));
    shared.insert::<SharedConnecting>(SharedConnecting(false));
    shared.insert::<SharedRealmText>(SharedRealmText("Development".to_string()));
    shared.insert::<SharedRealmSelectable>(SharedRealmSelectable(true));
    let mut screen = Screen::new(login_screen);
    screen.sync(&shared, &mut reg);

    let connect_id = reg.get_by_name("ConnectButton").expect("ConnectButton");
    let frame = reg.get(connect_id).expect("connect button frame");

    let Some(WidgetData::Button(bd)) = &frame.widget_data else {
        panic!("expected button widget data");
    };
    assert_eq!(
        bd.state,
        game_engine::ui::widgets::button::ButtonState::Normal,
        "Login button should be Normal after initial sync, got {:?}",
        bd.state,
    );

    // Second sync should also keep it Normal
    screen.sync(&shared, &mut reg);
    let frame = reg.get(connect_id).expect("connect button frame");
    let Some(WidgetData::Button(bd)) = &frame.widget_data else {
        panic!("expected button widget data");
    };
    assert_eq!(
        bd.state,
        game_engine::ui::widgets::button::ButtonState::Normal,
        "Login button should remain Normal after re-sync",
    );
}

#[test]
fn try_connect_disables_button_while_connecting() {
    let (mut reg, login) = login_fixture();
    set_editbox_text_for_test(&mut reg, login.username_input, "testuser");
    set_editbox_text_for_test(&mut reg, login.password_input, "testpass");
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();

    let world = run_try_connect_with_credentials(
        &mut reg,
        &login,
        &mut status,
        &mut next_state,
        Some("127.0.0.1:5000".parse().unwrap()),
        Some("127.0.0.1:5000"),
    );

    // Status should show connecting
    assert_eq!(status.0, "Connecting...");

    // Button should be disabled after connect
    let Some(WidgetData::Button(bd)) = reg
        .get(login.connect_button)
        .and_then(|f| f.widget_data.as_ref())
    else {
        panic!("expected button widget data");
    };
    assert_eq!(
        bd.state,
        game_engine::ui::widgets::button::ButtonState::Disabled,
        "Login button should be disabled while connecting",
    );

    // Game state should transition
    assert!(matches!(
        next_state,
        NextState::Pending(GameState::Connecting)
    ));

    // Credentials should be stored
    assert_eq!(world.resource::<networking::LoginUsername>().0, "testuser");
    assert_eq!(world.resource::<networking::LoginPassword>().0, "testpass");
}

fn make_login_app_with_plugins() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(Assets::<bevy::text::Font>::default());
    app.add_plugins(game_engine::ui::plugin::UiPlugin);
    app.init_resource::<LoginStatus>();
    app.init_resource::<LoginFocus>();
    app.insert_resource(networking::AuthUiFeedback::default());
    app.insert_resource(networking::LoginMode::Login);
    app.insert_resource(networking::AuthToken(None));
    let mut window = Window::default();
    window.resolution.set(1280.0, 720.0);
    app.world_mut().spawn((window, bevy::window::PrimaryWindow));
    app
}

fn run_login_visuals_cycle(app: &mut App, status_text: &str) {
    app.world_mut().resource_mut::<LoginStatus>().0 = status_text.to_string();
    let _ = app
        .world_mut()
        .run_system_cached(super::login_update_visuals);
    app.update();
}

fn collect_main_text_entities(app: &mut App, status_text_id: u64) -> Vec<String> {
    let mut q = app.world_mut().query::<(
        &game_engine::ui::render::UiText,
        &Text2d,
        Option<&game_engine::ui::render_text_fx::UiTextShadow>,
        Option<&game_engine::ui::render_text_fx::UiTextOutline>,
    )>();
    q.iter(app.world())
        .filter(|(ui_text, _, shadow, outline)| {
            ui_text.0 == status_text_id && shadow.is_none() && outline.is_none()
        })
        .map(|(_, text, _, _)| format!("{text:?}"))
        .collect()
}

#[test]
fn rendered_login_status_text_replaces_previous_text() {
    let mut app = make_login_app_with_plugins();

    let _ = app.world_mut().run_system_cached(super::build_login_ui);
    app.update();

    run_login_visuals_cycle(&mut app, "First error");
    run_login_visuals_cycle(&mut app, "Second error");

    let status_text_id = app.world().resource::<LoginUi>().status_text;
    let rendered = collect_main_text_entities(&mut app, status_text_id);

    assert_eq!(
        rendered.len(),
        1,
        "expected one main rendered text entity, got {rendered:?}"
    );
    assert!(
        rendered[0].contains("Second error"),
        "unexpected text payload: {}",
        rendered[0]
    );
    assert!(
        !rendered[0].contains("First errorSecond error"),
        "text was appended instead of replaced: {}",
        rendered[0]
    );
}

// --- Login workflow integration tests ---

fn run_automation_action(
    ui: &mut UiState,
    login: &LoginUi,
    focus: &mut LoginFocus,
    status: &mut LoginStatus,
    next_state: &mut NextState<GameState>,
    action: &UiAutomationAction,
) {
    let mut login_mode = networking::LoginMode::Login;
    let auth_token = networking::AuthToken(None);
    let (mut world, mut system_state) = make_world_with_commands();
    {
        let mut commands = system_state.get_mut(&mut world);
        run_login_automation_action(
            crate::scenes::login::connect::LoginAutomationContext {
                ui,
                login,
                focus,
                next_state,
                status,
                login_mode: &mut login_mode,
                auth_token: &auth_token,
                realm_selection: None,
                server_addr: None,
                server_hostname: None,
                commands: &mut commands,
            },
            action,
        )
        .expect("automation action should succeed");
    }
    system_state.apply(&mut world);
}

#[test]
fn login_workflow_type_credentials_and_connect() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus::default();
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();

    // 1. Click username field to focus it
    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::ClickFrame("UsernameInput".to_string()),
    );
    assert_eq!(focus.0, Some(login.username_input));

    // 2. Type username
    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::TypeText("testuser".to_string()),
    );
    assert_eq!(
        get_editbox_text(&ui.registry, login.username_input),
        "testuser"
    );

    // 3. Tab to password field
    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::PressKey(KeyCode::Tab),
    );
    assert_eq!(focus.0, Some(login.password_input));

    // 4. Type password
    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::TypeText("secret123".to_string()),
    );
    assert_eq!(
        get_editbox_text(&ui.registry, login.password_input),
        "secret123"
    );

    // 5. Press Enter to connect (should fail with no server, but transitions state)
    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::PressKey(KeyCode::Enter),
    );

    // Without a server address, connect should show error or transition
    assert!(
        !status.0.is_empty(),
        "status should have feedback after pressing Enter with credentials",
    );
}

#[test]
fn login_workflow_empty_fields_shows_error() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus::default();
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();

    // Click connect without entering credentials
    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::ClickFrame("ConnectButton".to_string()),
    );

    assert_eq!(status.0, "Please fill in all fields");
    assert!(matches!(next_state, NextState::Unchanged));
}

#[test]
fn login_workflow_tab_cycles_through_fields() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus::default();
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();

    // Click username
    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::ClickFrame("UsernameInput".to_string()),
    );
    assert_eq!(
        focus.0,
        Some(login.username_input),
        "focus should be on username (id={})",
        login.username_input
    );

    // Tab → password
    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::PressKey(KeyCode::Tab),
    );
    assert_eq!(
        focus.0,
        Some(login.password_input),
        "Tab should move focus to password (id={}), got {:?}. username_id={}",
        login.password_input,
        focus.0,
        login.username_input
    );

    // Tab → back to username
    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::PressKey(KeyCode::Tab),
    );
    assert_eq!(
        focus.0,
        Some(login.username_input),
        "Tab again should cycle back to username"
    );
}
