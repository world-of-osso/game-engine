use game_engine_ui_model::GameMenuModel;
use game_engine_ui_model::game_menu_main::{
    ACTION_ADDONS, ACTION_EXIT, ACTION_LOGOUT, ACTION_OPTIONS, ACTION_RESUME, ACTION_SUPPORT,
    GAME_MENU_ROOT,
};
use ui_toolkit::frame::{Dimension, WidgetData};

fn action(model: &GameMenuModel, name: &str) -> Option<String> {
    let id = model.registry.get_by_name(name)?;
    model.registry.get(id)?.onclick.clone()
}

#[test]
fn logged_in_menu_preserves_authored_actions_and_labels() {
    let mut model = GameMenuModel::new(1920.0, 1080.0, true);
    model.sync();

    for (name, expected_action, expected_label) in [
        ("MenuBtnOptions", ACTION_OPTIONS, "Options"),
        ("MenuBtnSupport", ACTION_SUPPORT, "Support"),
        ("MenuBtnAddons", ACTION_ADDONS, "AddOns"),
        ("MenuBtnLogout", ACTION_LOGOUT, "Log Out"),
        ("MenuBtnExit", ACTION_EXIT, "Exit Game"),
        ("MenuBtnResume", ACTION_RESUME, "Return to Game"),
    ] {
        assert_eq!(action(&model, name).as_deref(), Some(expected_action));
        let frame = model
            .registry
            .get(model.registry.get_by_name(name).unwrap())
            .unwrap();
        assert!(
            matches!(&frame.widget_data, Some(WidgetData::Button(data)) if data.text == expected_label)
        );
    }
    assert_eq!(model.registry.get_by_name("OptionsRoot"), None);
    for name in ["GameMenuPanel", "GameMenuTitleFrame"] {
        let frame = model
            .registry
            .get(model.registry.get_by_name(name).unwrap())
            .unwrap();
        let border = frame
            .nine_slice
            .as_ref()
            .expect("authored panel style resolved");
        assert_eq!(border.edge_size, 8.0);
        assert_eq!(border.uv_edge_size, Some(8.0));
        assert!(border.texture.is_some(), "{name}: border texture resolved");
    }
}

#[test]
fn logged_out_menu_has_five_actions_and_full_screen_blocker() {
    let mut model = GameMenuModel::new(1280.0, 720.0, false);
    model.sync();

    assert!(model.registry.get_by_name("MenuBtnLogout").is_none());
    for name in [
        "MenuBtnOptions",
        "MenuBtnSupport",
        "MenuBtnAddons",
        "MenuBtnExit",
        "MenuBtnResume",
    ] {
        assert!(action(&model, name).is_some(), "{name} missing");
    }
    assert_eq!(action(&model, "MenuBtnExit").as_deref(), Some(ACTION_EXIT));
    assert_eq!(
        action(&model, "MenuBtnResume").as_deref(),
        Some(ACTION_RESUME)
    );
    let root = model
        .registry
        .get(model.registry.get_by_name(GAME_MENU_ROOT.0).unwrap())
        .unwrap();
    assert!(root.mouse_enabled);
    // The blocker dims the world behind it without hiding it.
    assert!(
        root.background_color
            .is_some_and(|color| color[3] > 0.0 && color[3] < 1.0)
    );
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(1280.0), Dimension::Fixed(720.0))
    );
    assert!(model.registry.get_by_name("OptionsRoot").is_none());

    model.registry.screen_width = 1600.0;
    model.registry.screen_height = 900.0;
    model.sync();
    let root = model
        .registry
        .get(model.registry.get_by_name(GAME_MENU_ROOT.0).unwrap())
        .unwrap();
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(1600.0), Dimension::Fixed(900.0))
    );
}
