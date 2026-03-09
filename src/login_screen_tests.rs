use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use game_engine::ui::dioxus_screen::DioxusScreen;
use game_engine::ui::frame::WidgetData;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::login_component::login_screen;

use crate::networking;
use crate::game_state::GameState;

use super::{
    LoginFocus, LoginStatus, LoginUi,
    run_login_automation_action, sync_button_states, try_connect,
};
use super::helpers::get_editbox_text;

use game_engine::ui::automation::UiAutomationAction;

fn login_fixture() -> (FrameRegistry, LoginUi) {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut screen = DioxusScreen::new(login_screen);
    screen.sync(&mut reg);

    let root = reg.get_by_name("LoginRoot").expect("LoginRoot");
    let username_input = reg.get_by_name("UsernameInput").expect("UsernameInput");
    let password_input = reg.get_by_name("PasswordInput").expect("PasswordInput");
    let connect_button = reg.get_by_name("ConnectButton").expect("ConnectButton");
    let reconnect_button = reg.get_by_name("ReconnectButton").expect("ReconnectButton");
    let create_account_button = reg
        .get_by_name("CreateAccountButton")
        .expect("CreateAccountButton");
    let menu_button = reg.get_by_name("MenuButton").expect("MenuButton");
    let exit_button = reg.get_by_name("ExitButton").expect("ExitButton");
    let status_text = reg.get_by_name("LoginStatus").expect("LoginStatus");

    // Inject layout rects manually so hit-testing works in unit tests.
    inject_layout_rects(&mut reg, root, username_input, password_input,
        connect_button, reconnect_button, create_account_button, menu_button,
        exit_button, status_text);

    (reg, LoginUi {
        root,
        username_input,
        password_input,
        connect_button,
        reconnect_button,
        create_account_button,
        menu_button,
        exit_button,
        status_text,
    })
}

#[allow(clippy::too_many_arguments)]
fn inject_layout_rects(
    reg: &mut FrameRegistry,
    root: u64,
    username_input: u64,
    password_input: u64,
    connect_button: u64,
    reconnect_button: u64,
    create_account_button: u64,
    menu_button: u64,
    exit_button: u64,
    status_text: u64,
) {
    use game_engine::ui::layout::LayoutRect;
    let set_rect = |reg: &mut FrameRegistry, id: u64, x: f32, y: f32, w: f32, h: f32| {
        if let Some(f) = reg.get_mut(id) {
            f.layout_rect = Some(LayoutRect { x, y, width: w, height: h });
            f.width = w;
            f.height = h;
        }
    };
    set_rect(reg, root, 0.0, 0.0, 1920.0, 1080.0);
    set_rect(reg, username_input, 800.0, 400.0, 320.0, 42.0);
    set_rect(reg, password_input, 800.0, 460.0, 320.0, 42.0);
    set_rect(reg, connect_button, 800.0, 522.0, 250.0, 66.0);
    set_rect(reg, reconnect_button, 800.0, 522.0, 250.0, 66.0);
    set_rect(reg, create_account_button, 860.0, 630.0, 200.0, 32.0);
    set_rect(reg, menu_button, 860.0, 672.0, 200.0, 32.0);
    set_rect(reg, exit_button, 1700.0, 980.0, 200.0, 32.0);
    set_rect(reg, status_text, 800.0, 620.0, 320.0, 24.0);
}

fn set_editbox_text_for_test(reg: &mut FrameRegistry, id: u64, text: &str) {
    let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut())
    else {
        panic!("expected edit box");
    };
    eb.text = text.to_string();
    eb.cursor_position = eb.text.len();
}

fn make_ui_state(reg: FrameRegistry) -> UiState {
    UiState {
        registry: reg,
        event_bus: game_engine::ui::event::EventBus::new(),
        wasm_host: game_engine::ui::wasm_host::WasmHost::new(),
        focused_frame: None,
    }
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
    let mut world = World::new();
    let mut system_state: SystemState<Commands> = SystemState::new(&mut world);

    {
        let mut commands = system_state.get_mut(&mut world);
        run_login_automation_action(
            &mut ui, &login, &mut focus, &mut next_state, &mut status,
            &mut login_mode, &auth_token, None, &mut commands,
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
    let mut ui = make_ui_state(reg);
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
            &mut ui, &login, &mut focus, &mut next_state, &mut status,
            &mut login_mode, &auth_token, None, &mut commands,
            &UiAutomationAction::TypeText("alice".to_string()),
        )
        .expect("automation typing should succeed");
    }

    assert_eq!(get_editbox_text(&ui.registry, login.username_input), "alice");
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
                &mut ui, &login, &mut focus, &mut next_state, &mut status,
                &mut login_mode, &auth_token, None, &mut commands, &action,
            )
            .expect("automation action should succeed");
        }
    }
    system_state.apply(&mut world);

    assert_eq!(status.0, super::STATUS_CONNECTING);
    assert!(matches!(next_state, NextState::Pending(GameState::Connecting)));
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
        try_connect(&reg, &login, &mut status, &mut next_state,
            &networking::LoginMode::Login, None, &mut commands);
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
        try_connect(&reg, &login, &mut status, &mut next_state,
            &networking::LoginMode::Login, None, &mut commands);
    }
    system_state.apply(&mut world);

    assert_eq!(status.0, "Connecting...");
    assert!(matches!(next_state, NextState::Pending(GameState::Connecting)));
    assert_eq!(
        world.resource::<networking::ServerAddr>().0,
        super::DEFAULT_SERVER_ADDR.parse().unwrap()
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

    sync_button_states(&mut reg, &login, &networking::LoginMode::Login,
        &networking::AuthToken(Some("saved-token".to_string())));
    assert!(reg.get(login.connect_button).expect("connect button").visible);
    assert!(!reg.get(login.reconnect_button).expect("reconnect button").visible);

    sync_button_states(&mut reg, &login, &networking::LoginMode::Register,
        &networking::AuthToken(Some("saved-token".to_string())));
    assert!(reg.get(login.connect_button).expect("connect button").visible);
    assert!(!reg.get(login.reconnect_button).expect("reconnect button").visible);
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

    let _ = app.world_mut().run_system_cached(super::build_login_ui);

    assert_eq!(
        app.world().resource::<LoginStatus>().0,
        "Incorrect username or password"
    );
    assert!(app.world().resource::<networking::AuthUiFeedback>().0.is_none());
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
    let explicit_addr = "127.0.0.1:5000".parse().expect("test server address should parse");

    {
        let mut commands = system_state.get_mut(&mut world);
        try_connect(&reg, &login, &mut status, &mut next_state,
            &networking::LoginMode::Login, Some(explicit_addr), &mut commands);
    }
    system_state.apply(&mut world);

    assert_eq!(world.resource::<networking::ServerAddr>().0, explicit_addr);
}
