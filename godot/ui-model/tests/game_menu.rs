use game_engine_ui_model::GameMenuModel;
use game_engine_ui_model::game_menu_main::{
    ACTION_ADDONS, ACTION_EXIT, ACTION_LOGOUT, ACTION_OPTIONS, ACTION_RESUME, ACTION_SUPPORT,
    GAME_MENU_ROOT,
};
use ui_toolkit::frame::{Dimension, WidgetData};

fn load_fonts_and_atlases() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn action(model: &GameMenuModel, name: &str) -> Option<String> {
    let id = model.registry.get_by_name(name)?;
    model.registry.get(id)?.onclick.clone()
}

#[test]
fn logged_in_menu_preserves_authored_actions_and_labels() {
    load_fonts_and_atlases();
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
    // MainMenuFrameTemplates.xml:23-33: 260 wide, padding48/34, spacing0.
    let panel = model
        .registry
        .get(model.registry.get_by_name("GameMenuPanel").unwrap())
        .unwrap();
    assert_eq!(
        (panel.width, panel.height),
        (Dimension::Fixed(260.0), Dimension::Fixed(358.0))
    );
    let contents = model
        .registry
        .get(model.registry.get_by_name("GameMenuContents").unwrap())
        .unwrap();
    assert_eq!(contents.flex_layout.as_ref().unwrap().gap, 0.0);
    assert_eq!(
        contents.margin.top,
        ui_toolkit::layout_values::Val::Px(48.0)
    );
    assert_eq!(
        contents.margin.left,
        ui_toolkit::layout_values::Val::Px(28.0)
    );
    assert!(panel.background_color.is_none());
}

#[test]
fn logged_out_menu_has_five_actions_and_full_screen_blocker() {
    load_fonts_and_atlases();
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
