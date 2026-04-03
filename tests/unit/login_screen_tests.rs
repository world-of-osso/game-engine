use bevy::ecs::system::SystemState;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use game_engine::ui::frame::{Dimension, NineSlice, WidgetData};
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screen::Screen;
use game_engine::ui::screens::login_component::{SharedStatusText, login_screen};

use crate::game_state::GameState;
use crate::networking;

use super::helpers::get_editbox_text;
use super::{
    LoginFocus, LoginStatus, LoginUi, run_login_automation_action, sync_button_states, try_connect,
};

use game_engine::ui::automation::UiAutomationAction;

fn build_login_screen_for_test() -> (Screen, ui_toolkit::screen::SharedContext) {
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert::<SharedStatusText>(Default::default());
    let screen = Screen::new(login_screen);
    (screen, shared)
}

fn resolve_login_ui(reg: &FrameRegistry) -> LoginUi {
    let root = reg.get_by_name("LoginRoot").expect("LoginRoot");
    let username_input = reg.get_by_name("UsernameInput").expect("UsernameInput");
    let password_input = reg.get_by_name("PasswordInput").expect("PasswordInput");
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
    set_rect(reg, ids.connect_button, 800.0, 522.0, 250.0, 66.0);
    if let Some(reconnect_button) = ids.reconnect_button {
        set_rect(reg, reconnect_button, 800.0, 522.0, 250.0, 66.0);
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
    let (reg, login) = login_fixture();
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();
    let (mut world, mut system_state) = make_world_with_commands();

    {
        let mut commands = system_state.get_mut(&mut world);
        try_connect(
            &reg,
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
    reg: &FrameRegistry,
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

    let world =
        run_try_connect_with_credentials(&reg, &login, &mut status, &mut next_state, None, None);

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
fn sync_button_states_keeps_login_button_visible_even_with_saved_token() {
    let (mut reg, login) = login_fixture();

    sync_button_states(
        &mut reg,
        &login,
        &networking::LoginMode::Login,
        &networking::AuthToken(Some("saved-token".to_string())),
        &LoginStatus::default(),
    );
    assert!(
        reg.get(login.connect_button)
            .expect("connect button")
            .visible
    );
    assert!(login.reconnect_button.is_none());

    sync_button_states(
        &mut reg,
        &login,
        &networking::LoginMode::Register,
        &networking::AuthToken(Some("saved-token".to_string())),
        &LoginStatus::default(),
    );
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
        &reg,
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
