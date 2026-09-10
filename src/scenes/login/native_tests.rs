use super::*;
use bevy::ecs::system::RunSystemOnce;
use std::sync::Arc;

#[test]
fn setup_asset_failure_consumes_feedback_without_partial_view_or_camera_changes() {
    let mut world = World::new();
    world.init_resource::<Assets<Font>>();
    world.init_resource::<ui_toolkit::font_registry::FontRegistry>();
    world.insert_resource(LoginStatus("Previous status".into()));
    world.insert_resource(networking::AuthUiFeedback(Some("Session expired".into())));
    let addr = "192.0.2.17:5000".parse().unwrap();
    let hostname = "setup-preservation.invalid:5000";
    let selection = LoginRealmSelection::from_server(Some(addr), Some(hostname), true);
    world.insert_resource(selection.clone());
    world.insert_resource(networking::ServerAddr(addr));
    world.insert_resource(networking::ServerHostname(hostname.into()));
    let camera = world
        .spawn((
            UiCamera,
            Camera {
                order: 7,
                ..default()
            },
        ))
        .id();
    let unrelated = world.spawn(Name::new("OtherScreen")).id();

    world
        .run_system_once(setup)
        .expect("setup parameters are available");

    assert_eq!(world.resource::<networking::AuthUiFeedback>().0, None);
    let status = &world.resource::<LoginStatus>().0;
    assert!(
        status.contains("Unable to load native login texture"),
        "{status}"
    );
    assert!(status.contains("Common-Input-Border-TL.blp"), "{status}");
    assert!(!world.contains_resource::<LoginView>());
    assert!(!world.contains_resource::<LoginSession>());
    assert!(
        !world
            .query::<&Name>()
            .iter(&world)
            .any(|name| { matches!(name.as_str(), "NativeLoginCamera" | "LoginRoot") })
    );
    assert_eq!(world.get::<Camera>(camera).unwrap().order, 7);
    assert!(world.get_entity(unrelated).is_ok());
    assert_eq!(world.resource::<LoginRealmSelection>(), &selection);
    assert_eq!(world.resource::<networking::ServerAddr>().0, addr);
    assert_eq!(world.resource::<networking::ServerHostname>().0, hostname);
}

fn fixture() -> (World, LoginSession) {
    let mut world = World::new();
    world.insert_resource(LoginStatus::default());
    world.insert_resource(networking::LoginMode::Login);
    world.insert_resource(networking::AuthToken(None));
    world.insert_resource(networking::ServerAddr("127.0.0.1:5000".parse().unwrap()));
    world.insert_resource(networking::ServerHostname("local.example:5000".into()));
    world.insert_resource(NextState::<GameState>::default());
    world.insert_resource(ButtonInput::<MouseButton>::default());
    world.insert_resource(Messages::<KeyboardInput>::default());
    world.insert_resource(UiAutomationQueue::default());
    world.insert_resource(UiAutomationRunner::default());
    world.insert_resource(Time::<()>::default());
    world.insert_resource(LoginClipboard(Arc::new(|| Ok("ice".into()))));
    world.insert_resource(UiState {
        registry: game_engine::ui::registry::FrameRegistry::new(1280.0, 720.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    spawn_control(
        &mut world,
        "UsernameInput",
        LoginControl::Field(LoginFieldId::Username),
    );
    spawn_control(
        &mut world,
        "PasswordInput",
        LoginControl::Field(LoginFieldId::Password),
    );
    spawn_control(
        &mut world,
        "ConnectButton",
        LoginControl::Action(LoginAction::Connect),
    );
    (world, LoginSession::default())
}

fn spawn_control(world: &mut World, name: &str, control: LoginControl) -> Entity {
    world
        .spawn((
            Name::new(name.to_owned()),
            Node::default(),
            control,
            Interaction::None,
        ))
        .id()
}

fn find_control(world: &mut World, name: &str) -> Entity {
    world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, candidate)| candidate.as_str() == name)
        .unwrap()
        .0
}

fn credentials(session: &mut LoginSession) {
    session.form.username.set_text("alice");
    session.form.password.set_text("sëcret");
}

fn click(world: &mut World, session: &mut LoginSession, name: &str) -> Result<(), String> {
    automate(world, session, &UiAutomationAction::ClickFrame(name.into()))
}

fn key(code: KeyCode, logical: Key, text: Option<&str>) -> KeyboardInput {
    KeyboardInput {
        key_code: code,
        logical_key: logical,
        state: ButtonState::Pressed,
        text: text.map(Into::into),
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

fn assert_not_submitted(world: &World) {
    assert!(matches!(
        world.resource::<NextState<GameState>>(),
        NextState::Unchanged
    ));
    assert!(!world.contains_resource::<networking::LoginUsername>());
    assert!(!world.contains_resource::<networking::LoginPassword>());
}

fn assert_submitted(world: &World, username: &str, password: &str) {
    assert_eq!(world.resource::<LoginStatus>().0, STATUS_CONNECTING);
    assert_eq!(world.resource::<networking::LoginUsername>().0, username);
    assert_eq!(world.resource::<networking::LoginPassword>().0, password);
    assert!(matches!(
        world.resource::<NextState<GameState>>(),
        NextState::Pending(GameState::Connecting)
    ));
}

#[test]
fn automation_workflow_focuses_types_tabs_and_submits_unmasked_credentials() {
    let (mut world, mut session) = fixture();
    click(&mut world, &mut session, "UsernameInput").unwrap();
    assert_eq!(session.focus, Some(LoginFieldId::Username));
    automate(
        &mut world,
        &mut session,
        &UiAutomationAction::TypeText("alice".into()),
    )
    .unwrap();
    automate(
        &mut world,
        &mut session,
        &UiAutomationAction::PressKey(KeyCode::Tab),
    )
    .unwrap();
    assert_eq!(session.focus, Some(LoginFieldId::Password));
    automate(
        &mut world,
        &mut session,
        &UiAutomationAction::TypeText("sëcret".into()),
    )
    .unwrap();
    assert_eq!(session.form.password.display_text(), "*******");
    automate(
        &mut world,
        &mut session,
        &UiAutomationAction::PressKey(KeyCode::Enter),
    )
    .unwrap();
    assert_submitted(&world, "alice", "sëcret");
}

#[test]
fn empty_or_whitespace_credentials_leave_authentication_untouched() {
    for (username, password) in [("", ""), ("alice", "  \t"), (" \n", "secret")] {
        let (mut world, mut session) = fixture();
        session.form.username.set_text(username);
        session.form.password.set_text(password);
        click(&mut world, &mut session, "ConnectButton").unwrap();
        assert_eq!(world.resource::<LoginStatus>().0, STATUS_FILL_FIELDS);
        assert_not_submitted(&world);
    }
}

#[test]
fn connect_preserves_explicit_server_and_registration_mode() {
    let (mut world, mut session) = fixture();
    credentials(&mut session);
    let addr = "192.0.2.14:6123".parse().unwrap();
    world.insert_resource(networking::ServerAddr(addr));
    world.insert_resource(networking::ServerHostname("custom.example:6123".into()));
    world.insert_resource(networking::LoginMode::Register);
    click(&mut world, &mut session, "ConnectButton").unwrap();
    assert_submitted(&world, "alice", "sëcret");
    assert_eq!(world.resource::<networking::ServerAddr>().0, addr);
    assert_eq!(
        world.resource::<networking::ServerHostname>().0,
        "custom.example:6123"
    );
    assert!(matches!(
        world.resource::<networking::LoginMode>(),
        networking::LoginMode::Register
    ));
}

#[test]
fn selected_custom_realm_overrides_stale_server_resources() {
    let (mut world, mut session) = fixture();
    credentials(&mut session);
    let addr = "192.0.2.19:7000".parse().unwrap();
    world.insert_resource(LoginRealmSelection::from_server(
        Some(addr),
        Some("realm.example:7000"),
        true,
    ));
    dispatch_action(&mut world, &mut session, LoginAction::Connect);
    assert_eq!(world.resource::<networking::ServerAddr>().0, addr);
    assert_eq!(
        world.resource::<networking::ServerHostname>().0,
        "realm.example:7000"
    );
    assert_submitted(&world, "alice", "sëcret");
}

#[test]
fn repeated_connect_does_not_replace_submitted_credentials() {
    let (mut world, mut session) = fixture();
    credentials(&mut session);
    click(&mut world, &mut session, "ConnectButton").unwrap();
    session.form.username.set_text("replacement");
    session.form.password.set_text("replacement");
    click(&mut world, &mut session, "ConnectButton").unwrap();
    dispatch_action(&mut world, &mut session, LoginAction::Connect);
    assert_submitted(&world, "alice", "sëcret");
}

#[test]
fn reconnect_without_a_nonempty_token_reports_error_without_transition() {
    for token in [None, Some(""), Some(" \t")] {
        let (mut world, mut session) = fixture();
        world.insert_resource(networking::AuthToken(token.map(str::to_owned)));
        dispatch_action(&mut world, &mut session, LoginAction::Reconnect);
        assert_eq!(
            world.resource::<LoginStatus>().0,
            STATUS_RECONNECT_UNAVAILABLE
        );
        assert_not_submitted(&world);
    }
}

#[test]
fn reconnect_uses_saved_token_mode_and_empty_credential_payload() {
    let (mut world, mut session) = fixture();
    credentials(&mut session);
    world.insert_resource(networking::AuthToken(Some("saved-token".into())));
    world.insert_resource(networking::LoginMode::Register);
    dispatch_action(&mut world, &mut session, LoginAction::Reconnect);
    assert_submitted(&world, "", "");
    assert!(matches!(
        world.resource::<networking::LoginMode>(),
        networking::LoginMode::Login
    ));
    assert_eq!(
        world.resource::<networking::AuthToken>().0.as_deref(),
        Some("saved-token")
    );
    assert_eq!(
        world.resource::<networking::ServerHostname>().0,
        "local.example:5000"
    );
}

#[test]
fn tab_cycles_and_escape_clears_focus_without_submitting() {
    let (mut world, mut session) = fixture();
    for expected in [
        LoginFieldId::Username,
        LoginFieldId::Password,
        LoginFieldId::Username,
    ] {
        press_key(&mut world, &mut session, KeyCode::Tab);
        assert_eq!(session.focus, Some(expected));
    }
    press_key(&mut world, &mut session, KeyCode::Escape);
    assert_eq!(session.focus, None);
    assert_not_submitted(&world);
}

#[test]
fn automation_requires_focus_for_typing_and_editing_but_tab_recovers_it() {
    let (mut world, mut session) = fixture();
    assert!(
        automate(
            &mut world,
            &mut session,
            &UiAutomationAction::TypeText("x".into())
        )
        .is_err()
    );
    assert!(
        automate(
            &mut world,
            &mut session,
            &UiAutomationAction::PressKey(KeyCode::Backspace)
        )
        .is_err()
    );
    automate(
        &mut world,
        &mut session,
        &UiAutomationAction::PressKey(KeyCode::Tab),
    )
    .unwrap();
    automate(
        &mut world,
        &mut session,
        &UiAutomationAction::TypeText("x".into()),
    )
    .unwrap();
    assert_eq!(session.form.username.text, "x");
    assert_eq!(session.form.password.text, "");
}

#[test]
fn refocusing_an_input_moves_cursor_to_end_before_typing() {
    let (mut world, mut session) = fixture();
    session.form.username.set_text("alice");
    session.form.username.home();
    click(&mut world, &mut session, "UsernameInput").unwrap();
    automate(
        &mut world,
        &mut session,
        &UiAutomationAction::TypeText("!".into()),
    )
    .unwrap();
    assert_eq!(session.form.username.text, "alice!");
}

#[test]
fn clipboard_shortcuts_replace_no_literal_v_and_share_insertion_behavior() {
    for (ctrl, super_key, logical) in [
        (true, false, Key::Character("v".into())),
        (false, true, Key::Character("v".into())),
        (false, false, Key::Paste),
    ] {
        let (mut world, mut session) = fixture();
        session.focus = Some(LoginFieldId::Username);
        session.form.username.set_text("al");
        session.modifiers.ctrl = ctrl;
        session.modifiers.super_key = super_key;
        process_key_event(
            &mut world,
            &mut session,
            &key(KeyCode::KeyV, logical, Some("v")),
        );
        assert_eq!(session.form.username.text, "alice");
        assert_eq!(session.form.username.cursor_position, 5);
    }
}

#[test]
fn clipboard_failure_preserves_text_and_exposes_error() {
    let (mut world, mut session) = fixture();
    session.focus = Some(LoginFieldId::Password);
    session.form.password.set_text("keep");
    world.insert_resource(LoginClipboard(Arc::new(|| {
        Err("clipboard read: unavailable".into())
    })));
    process_key_event(
        &mut world,
        &mut session,
        &key(KeyCode::KeyV, Key::Paste, Some("v")),
    );
    assert_eq!(session.form.password.text, "keep");
    assert_eq!(
        world.resource::<LoginStatus>().0,
        "clipboard read: unavailable"
    );
}

#[test]
fn missing_text_payload_does_not_insert_logical_character() {
    let (mut world, mut session) = fixture();
    session.focus = Some(LoginFieldId::Username);
    session.form.username.set_text("al");
    process_key_event(
        &mut world,
        &mut session,
        &key(KeyCode::KeyV, Key::Character("v".into()), None),
    );
    assert_eq!(session.form.username.text, "al");
}

#[test]
fn keyboard_unicode_edits_preserve_boundaries_and_filter_controls() {
    let (mut world, mut session) = fixture();
    session.focus = Some(LoginFieldId::Username);
    process_key_event(
        &mut world,
        &mut session,
        &key(
            KeyCode::KeyA,
            Key::Character("Aé猫Z".into()),
            Some("Aé\n猫\tZ"),
        ),
    );
    assert_eq!(session.form.username.text, "Aé猫Z");
    press_key(&mut world, &mut session, KeyCode::ArrowLeft);
    press_key(&mut world, &mut session, KeyCode::Backspace);
    assert_eq!(session.form.username.text, "AéZ");
    assert_eq!(session.form.username.cursor_position, 3);
    press_key(&mut world, &mut session, KeyCode::Delete);
    assert_eq!(session.form.username.text, "Aé");
    press_key(&mut world, &mut session, KeyCode::Home);
    assert_eq!(session.form.username.cursor_position, 0);
    press_key(&mut world, &mut session, KeyCode::ArrowRight);
    assert_eq!(session.form.username.cursor_position, 1);
    press_key(&mut world, &mut session, KeyCode::End);
    assert_eq!(session.form.username.cursor_position, 3);
}

#[test]
fn typing_and_paste_obey_field_limits() {
    let (mut world, mut session) = fixture();
    session.focus = Some(LoginFieldId::Username);
    session.form.username.max_letters = Some(3);
    session.form.username.max_bytes = Some(4);
    world.insert_resource(LoginClipboard(Arc::new(|| Ok("é猫AB".into()))));
    process_key_event(
        &mut world,
        &mut session,
        &key(KeyCode::KeyV, Key::Paste, None),
    );
    assert_eq!(session.form.username.text, "é");
    automate(
        &mut world,
        &mut session,
        &UiAutomationAction::TypeText("ABZ".into()),
    )
    .unwrap();
    assert_eq!(session.form.username.text, "éAB");
}

#[test]
fn unknown_controls_report_errors_without_submitting() {
    let (mut world, mut session) = fixture();
    assert!(click(&mut world, &mut session, "MissingButton").is_err());
    assert_not_submitted(&world);
}

#[test]
fn pointer_release_on_the_pressed_button_submits() {
    let (mut world, mut session) = fixture();
    credentials(&mut session);
    let button = find_control(&mut world, "ConnectButton");
    world.entity_mut(button).insert(Interaction::Pressed);
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    process_pointer(&mut world, &mut session);
    assert_not_submitted(&world);
    world.resource_mut::<ButtonInput<MouseButton>>().clear();
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    world.entity_mut(button).insert(Interaction::Hovered);
    process_pointer(&mut world, &mut session);
    assert_submitted(&world, "alice", "sëcret");
}

#[test]
fn dragging_away_then_releasing_does_not_submit_or_replay_press() {
    let (mut world, mut session) = fixture();
    credentials(&mut session);
    let button = find_control(&mut world, "ConnectButton");
    world.entity_mut(button).insert(Interaction::Pressed);
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    process_pointer(&mut world, &mut session);
    world.entity_mut(button).insert(Interaction::None);
    world.resource_mut::<ButtonInput<MouseButton>>().clear();
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    process_pointer(&mut world, &mut session);
    assert_not_submitted(&world);
    world.resource_mut::<ButtonInput<MouseButton>>().clear();
    world.entity_mut(button).insert(Interaction::Hovered);
    process_pointer(&mut world, &mut session);
    assert_not_submitted(&world);
}

#[test]
fn pointer_focus_and_background_click_clear_focus() {
    let (mut world, mut session) = fixture();
    session.form.password.set_text("secret");
    session.form.password.home();
    let password = find_control(&mut world, "PasswordInput");
    world.entity_mut(password).insert(Interaction::Pressed);
    world
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    process_pointer(&mut world, &mut session);
    assert_eq!(session.focus, Some(LoginFieldId::Password));
    process_key_event(
        &mut world,
        &mut session,
        &key(KeyCode::Digit1, Key::Character("!".into()), Some("!")),
    );
    assert_eq!(session.form.password.text, "secret!");
    world.entity_mut(password).insert(Interaction::None);
    process_pointer(&mut world, &mut session);
    assert_eq!(session.focus, None);
}

#[test]
fn realm_dispatch_cycles_to_development_and_locked_selection_stays_custom() {
    let (mut world, mut session) = fixture();
    world.insert_resource(LoginRealmSelection::from_server(
        Some("127.0.0.1:5000".parse().unwrap()),
        Some("game.worldofosso.com:5000"),
        false,
    ));
    let realm = spawn_control(
        &mut world,
        "RealmButton",
        LoginControl::Action(LoginAction::CycleRealm),
    );
    world.get_mut::<Node>(realm).unwrap().display = Display::None;
    click(&mut world, &mut session, "RealmButton").unwrap();
    world.flush();
    assert_eq!(
        world.resource::<LoginRealmSelection>().server_hostname(),
        "127.0.0.1:5000"
    );
    assert_eq!(
        world.resource::<networking::ServerHostname>().0,
        "127.0.0.1:5000"
    );
    let custom = "192.0.2.8:6000".parse().unwrap();
    world.insert_resource(LoginRealmSelection::from_server(
        Some(custom),
        Some("locked.example:6000"),
        true,
    ));
    dispatch_action(&mut world, &mut session, LoginAction::CycleRealm);
    world.flush();
    assert_eq!(world.resource::<networking::ServerAddr>().0, custom);
    assert_eq!(
        world.resource::<networking::ServerHostname>().0,
        "locked.example:6000"
    );
}

#[test]
fn registration_action_toggles_mode_and_clears_feedback() {
    let (mut world, mut session) = fixture();
    world.resource_mut::<LoginStatus>().0 = "old error".into();
    dispatch_action(&mut world, &mut session, LoginAction::CreateAccount);
    assert!(matches!(
        world.resource::<networking::LoginMode>(),
        networking::LoginMode::Register
    ));
    assert_eq!(world.resource::<LoginStatus>().0, "");
    dispatch_action(&mut world, &mut session, LoginAction::CreateAccount);
    assert!(matches!(
        world.resource::<networking::LoginMode>(),
        networking::LoginMode::Login
    ));
}

fn insert_view(world: &mut World) -> LoginView {
    let root = world.spawn((Name::new("LoginRoot"), Node::default())).id();
    let camera = world.spawn(Camera::default()).id();
    let username_input = find_control(world, "UsernameInput");
    let password_input = find_control(world, "PasswordInput");
    let connect_button = find_control(world, "ConnectButton");
    let realm_button = spawn_control(
        world,
        "RealmButton",
        LoginControl::Action(LoginAction::CycleRealm),
    );
    let create_account_button = spawn_control(
        world,
        "CreateAccountButton",
        LoginControl::Action(LoginAction::CreateAccount),
    );
    let menu_button = spawn_control(world, "MenuButton", LoginControl::Action(LoginAction::Menu));
    let exit_button = spawn_control(world, "ExitButton", LoginControl::Action(LoginAction::Exit));
    let username_text = world.spawn((Text::new(""), ChildOf(username_input))).id();
    let password_text = world.spawn((Text::new(""), ChildOf(password_input))).id();
    let status_text = world.spawn((Text::new(""), ChildOf(root))).id();
    for entity in [
        username_input,
        password_input,
        connect_button,
        realm_button,
        create_account_button,
        menu_button,
        exit_button,
    ] {
        world.entity_mut(entity).insert(ChildOf(root));
    }
    let view = LoginView {
        root,
        camera,
        username_input,
        password_input,
        username_text,
        password_text,
        connect_button,
        realm_button,
        create_account_button,
        menu_button,
        exit_button,
        status_text,
    };
    world.insert_resource(view.clone());
    view
}

#[test]
fn modal_drains_keyboard_and_cancels_press_without_replaying_after_close() {
    let (mut world, mut session) = fixture();
    insert_view(&mut world);
    session.focus = Some(LoginFieldId::Username);
    session.pressed = Some(find_control(&mut world, "ConnectButton"));
    world.insert_resource(session);
    world.insert_resource(crate::scenes::game_menu::UiModalOpen);
    world.resource_mut::<Messages<KeyboardInput>>().write(key(
        KeyCode::KeyA,
        Key::Character("a".into()),
        Some("a"),
    ));
    update(&mut world);
    assert_eq!(world.resource::<LoginSession>().form.username.text, "");
    assert_eq!(world.resource::<LoginSession>().pressed, None);
    world.remove_resource::<crate::scenes::game_menu::UiModalOpen>();
    update(&mut world);
    assert_eq!(world.resource::<LoginSession>().form.username.text, "");
    world.resource_mut::<Messages<KeyboardInput>>().write(key(
        KeyCode::KeyB,
        Key::Character("b".into()),
        Some("b"),
    ));
    update(&mut world);
    assert_eq!(world.resource::<LoginSession>().form.username.text, "b");
}

#[test]
fn cleanup_removes_only_login_subtree_and_restores_legacy_camera_order() {
    let (mut world, mut session) = fixture();
    let view = insert_view(&mut world);
    let legacy_camera = world
        .spawn(Camera {
            order: 2,
            ..default()
        })
        .id();
    let unrelated = world.spawn(Name::new("OtherScreen")).id();
    session.camera_orders.push((legacy_camera, 1));
    world.insert_resource(session);
    cleanup(&mut world);
    for entity in [
        view.root,
        view.camera,
        view.username_input,
        view.password_input,
        view.username_text,
        view.password_text,
        view.connect_button,
        view.status_text,
    ] {
        assert!(world.get_entity(entity).is_err());
    }
    assert_eq!(world.get::<Camera>(legacy_camera).unwrap().order, 1);
    assert!(world.get_entity(unrelated).is_ok());
    assert!(!world.contains_resource::<LoginSession>());
    assert!(!world.contains_resource::<LoginView>());
}
