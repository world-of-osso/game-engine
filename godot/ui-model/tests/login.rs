use game_engine_ui_model::LoginModel;
use game_engine_ui_model::login::{
    CONNECT_BUTTON, CREATE_ACCOUNT_BUTTON, EXIT_BUTTON, LOGIN_ROOT, LOGIN_STATUS, LoginAction,
    MENU_BUTTON, PASSWORD_INPUT, REALM_BUTTON, SharedConnecting, SharedStatusText, USERNAME_INPUT,
};
use ui_toolkit::frame::{Dimension, WidgetData, WidgetType};
use ui_toolkit::layout_values::{PositionType, Val};
use ui_toolkit::widgets::button::ButtonState;
use ui_toolkit::widgets::texture::TextureSource;

#[test]
fn authored_login_registry_retains_frames_layout_and_resources() {
    let mut model = LoginModel::new(1920.0, 1080.0);
    assert_eq!(model.credentials(), None);
    model.sync();
    assert_eq!(model.credentials(), Some((String::new(), String::new())));
    let registry = &model.registry;
    assert_eq!(
        (registry.screen_width, registry.screen_height),
        (1920.0, 1080.0)
    );

    let root = registry
        .get(registry.get_by_name(LOGIN_ROOT.0).unwrap())
        .unwrap();
    assert_eq!(root.position_type, PositionType::Absolute);
    assert_eq!(
        (root.width, root.height),
        (Dimension::Auto, Dimension::Auto)
    );
    assert_eq!(root.position.left, Val::Px(0.0));

    let form = registry
        .get(registry.get_by_name("LoginInputContainer").unwrap())
        .unwrap();
    // The form is centred on the screen.
    assert_eq!(form.position.left, Val::Percent(50.0));
    assert_eq!(form.position.top, Val::Percent(50.0));
    assert_eq!(form.translation.x, Val::Percent(-50.0));

    let username = registry
        .get(registry.get_by_name(USERNAME_INPUT.0).unwrap())
        .unwrap();
    let password = registry
        .get(registry.get_by_name(PASSWORD_INPUT.0).unwrap())
        .unwrap();
    assert_eq!(username.parent_id, Some(form.id));
    assert_eq!(password.parent_id, Some(form.id));
    assert_eq!(
        (username.height, password.height),
        (Dimension::Fixed(42.0), Dimension::Fixed(42.0))
    );
    // The password box sits below the username box, clear of it.
    let (Val::Px(user_top), Val::Px(pass_top)) = (username.position.top, password.position.top)
    else {
        panic!("inputs are not placed in px")
    };
    assert!(
        pass_top >= user_top + 42.0,
        "{pass_top} overlaps {user_top}"
    );
    assert!(matches!(password.widget_data, Some(WidgetData::EditBox(ref data)) if data.password));

    let bg = registry
        .get(registry.get_by_name("LoginBackground").unwrap())
        .unwrap();
    assert!(
        matches!(bg.widget_data, Some(WidgetData::Texture(ref data)) if matches!(&data.source, TextureSource::File(path) if !path.is_empty()))
    );
    let logo = registry
        .get(registry.get_by_name("LoginGameLogo").unwrap())
        .unwrap();
    assert_eq!(
        (logo.width, logo.height),
        (Dimension::Fixed(384.0), Dimension::Fixed(256.0))
    );
    assert!(
        matches!(logo.widget_data, Some(WidgetData::Texture(ref data)) if matches!(&data.source, TextureSource::File(path) if !path.is_empty()))
    );
    let connect = registry
        .get(registry.get_by_name(CONNECT_BUTTON.0).unwrap())
        .unwrap();
    assert_eq!(connect.widget_type, WidgetType::Button);
    assert_eq!(connect.onclick.as_deref(), Some("connect"));
    assert_eq!(
        (connect.width, connect.height),
        (Dimension::Fixed(250.0), Dimension::Fixed(66.0))
    );
    assert!(
        matches!(connect.widget_data, Some(WidgetData::Button(ref data)) if data.text == "Login" && data.enabled)
    );
    let realm = registry
        .get(registry.get_by_name(REALM_BUTTON.0).unwrap())
        .unwrap();
    assert!(realm.hidden);
    for (name, action) in [
        (CONNECT_BUTTON, LoginAction::Connect),
        (REALM_BUTTON, LoginAction::CycleRealm),
        (CREATE_ACCOUNT_BUTTON, LoginAction::CreateAccount),
        (MENU_BUTTON, LoginAction::Menu),
        (EXIT_BUTTON, LoginAction::Exit),
    ] {
        let frame = registry.get(registry.get_by_name(name.0).unwrap()).unwrap();
        assert_eq!(
            frame.onclick.as_deref().and_then(LoginAction::parse),
            Some(action)
        );
    }
    let version = registry
        .get(registry.get_by_name("VersionText").unwrap())
        .unwrap();
    assert!(
        matches!(version.widget_data, Some(WidgetData::FontString(ref data)) if data.text == "game-engine v0.1.0")
    );
}

#[test]
fn status_rebuild_updates_button_and_text_without_discarding_credentials() {
    let mut model = LoginModel::new(1920.0, 1080.0);
    model.sync();
    for (name, text) in [(USERNAME_INPUT, "alice"), (PASSWORD_INPUT, "secret")] {
        let id = model.registry.get_by_name(name.0).unwrap();
        let frame = model.registry.get_mut(id).unwrap();
        let Some(WidgetData::EditBox(data)) = &mut frame.widget_data else {
            panic!("missing editbox")
        };
        data.insert_at_cursor(text);
    }
    assert_eq!(
        model.credentials().unwrap(),
        ("alice".into(), "secret".into())
    );
    let username_id = model.registry.get_by_name(USERNAME_INPUT.0).unwrap();
    model
        .shared
        .insert(SharedStatusText("Connecting...".into()));
    model.shared.insert(SharedConnecting(true));
    model.sync();
    assert_eq!(
        model.registry.get_by_name(USERNAME_INPUT.0),
        Some(username_id)
    );
    assert_eq!(
        model.credentials().unwrap(),
        ("alice".into(), "secret".into())
    );
    let connect = model
        .registry
        .get(model.registry.get_by_name(CONNECT_BUTTON.0).unwrap())
        .unwrap();
    assert!(
        matches!(connect.widget_data, Some(WidgetData::Button(ref data)) if data.state == ButtonState::Disabled)
    );
    let status = model
        .registry
        .get(model.registry.get_by_name(LOGIN_STATUS.0).unwrap())
        .unwrap();
    assert!(
        matches!(status.widget_data, Some(WidgetData::FontString(ref data)) if data.text == "Connecting...")
    );
    model
        .shared
        .insert(SharedStatusText("Connection failed".into()));
    model.shared.insert(SharedConnecting(false));
    model.sync();
    assert_eq!(
        model.credentials().unwrap(),
        ("alice".into(), "secret".into())
    );
    let connect = model
        .registry
        .get(model.registry.get_by_name(CONNECT_BUTTON.0).unwrap())
        .unwrap();
    assert!(
        matches!(connect.widget_data, Some(WidgetData::Button(ref data)) if data.state == ButtonState::Normal)
    );
}
