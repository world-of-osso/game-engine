use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use game_engine::ui::automation::UiAutomationAction;
use game_engine::ui::frame::{Dimension, NineSlice, WidgetData};
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screen::Screen;
use game_engine::ui::screens::login_component::{SharedStatusText, login_screen};
use ui_toolkit::layout::{LayoutRect, recompute_layouts};

use crate::game_state::GameState;
use crate::networking;
use crate::ui_input::walk_up_for_onclick;

use super::super::helpers::topmost_frame_at;
use super::super::{
    LoginFocus, LoginStatus, LoginUi, apply_post_setup, run_login_automation_action, try_connect,
    view,
};

fn build_login_screen_for_test() -> (Screen, ui_toolkit::screen::SharedContext) {
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert::<SharedStatusText>(SharedStatusText::default());
    let screen = Screen::new(login_screen);
    (screen, shared)
}

pub(super) fn build_login_registry_with_real_layout() -> (FrameRegistry, LoginUi) {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut screen_res =
        view::build_login_screen(&LoginStatus::default(), "Development".to_string(), true);
    screen_res.screen.sync(&screen_res.shared, &mut reg);
    let login = resolve_login_ui(&reg);
    apply_post_setup(&mut reg, &login);
    recompute_layouts(&mut reg);
    (reg, login)
}

pub(super) fn layout_rect(reg: &FrameRegistry, id: u64) -> LayoutRect {
    reg.get(id)
        .and_then(|frame| frame.layout_rect.clone())
        .expect("layout_rect")
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

pub(super) fn login_fixture() -> (FrameRegistry, LoginUi) {
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

pub(super) fn set_editbox_text_for_test(reg: &mut FrameRegistry, id: u64, text: &str) {
    let Some(WidgetData::EditBox(eb)) = reg.get_mut(id).and_then(|f| f.widget_data.as_mut()) else {
        panic!("expected edit box");
    };
    eb.text = text.to_string();
    eb.cursor_position = eb.text.len();
}

pub(super) fn make_ui_state(reg: FrameRegistry) -> UiState {
    UiState {
        registry: reg,
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    }
}

pub(super) fn find_clicked_action(ui: &UiState, mx: f32, my: f32) -> Option<String> {
    let hit_id = topmost_frame_at(ui, mx, my)?;
    walk_up_for_onclick(&ui.registry, hit_id)
}

pub(super) fn make_world_with_commands() -> (World, SystemState<Commands<'static, 'static>>) {
    let mut world = World::new();
    let system_state = SystemState::new(&mut world);
    (world, system_state)
}

pub(super) fn editbox_nine_slice(reg: &FrameRegistry, id: u64) -> &NineSlice {
    reg.get(id)
        .and_then(|frame| frame.nine_slice.as_ref())
        .expect("expected editbox nine-slice")
}

pub(super) fn run_login_actions(
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

pub(super) fn login_submit_actions() -> [UiAutomationAction; 5] {
    [
        UiAutomationAction::ClickFrame("UsernameInput".to_string()),
        UiAutomationAction::TypeText("alice".to_string()),
        UiAutomationAction::ClickFrame("PasswordInput".to_string()),
        UiAutomationAction::TypeText("secret".to_string()),
        UiAutomationAction::ClickFrame("ConnectButton".to_string()),
    ]
}

pub(super) fn run_try_connect_with_credentials(
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

pub(super) fn make_login_app() -> App {
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

pub(super) fn count_login_status_frames(app: &App) -> usize {
    let ui = app.world().resource::<UiState>();
    ui.registry
        .frames_iter()
        .filter(|frame| frame.name.as_deref() == Some("LoginStatus"))
        .count()
}

pub(super) fn make_login_app_with_plugins() -> App {
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

pub(super) fn run_login_visuals_cycle(app: &mut App, status_text: &str) {
    app.world_mut().resource_mut::<LoginStatus>().0 = status_text.to_string();
    let _ = app
        .world_mut()
        .run_system_cached(super::super::login_update_visuals);
    app.update();
}

pub(super) fn collect_main_text_entities(app: &mut App, status_text_id: u64) -> Vec<String> {
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

pub(super) fn run_automation_action(
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
