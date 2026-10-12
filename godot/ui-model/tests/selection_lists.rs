use game_engine_ui_model::battleground::{PvpView, SPECIFIC_LIST, pvp_screen};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn selection_lists_both_skins_keep_selection_through_scroll_and_refresh_without_push_button_art() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = PvpView {
            visible: true,
            ..Default::default()
        };
        view.action("pvp:select:699");
        let mut shared = SharedContext::new();
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(pvp_screen);
        screen.sync(&shared, &mut registry);
        let id = registry
            .get_by_name("HonorFrameSpecificBattlegroundButton699")
            .unwrap();
        let Some(WidgetData::Button(button)) = &registry.get(id).unwrap().widget_data else {
            panic!("selectable row")
        };
        assert!(
            !button.use_default_skin,
            "selection rows must not receive push-button art"
        );
        assert!(
            button.normal_texture.is_none(),
            "idle rows are flat on the panel"
        );
        assert!(button.highlight_texture.is_some(), "hover remains visible");
        let selected = registry
            .get_by_name("HonorFrameSpecificBattlegroundButton699Selected")
            .unwrap();
        assert!(registry.get(selected).unwrap().visible);
        assert!(registry.scroll_lists.scroll_to(SPECIFIC_LIST, 80));
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        assert!(
            registry
                .get(
                    registry
                        .get_by_name("HonorFrameSpecificBattlegroundButton699Selected")
                        .unwrap()
                )
                .unwrap()
                .visible
        );
        assert_eq!(view.selected_battleground, 699);
        view.action("pvp:select:1014");
        shared.insert(view);
        screen.sync(&shared, &mut registry);
        assert!(
            !registry
                .get(
                    registry
                        .get_by_name("HonorFrameSpecificBattlegroundButton699Selected")
                        .unwrap()
                )
                .unwrap()
                .visible
        );
        let frame = registry
            .get(registry.get_by_name("HonorFrameQueueButton").unwrap())
            .unwrap();
        let Some(WidgetData::Button(button)) = &frame.widget_data else {
            panic!("action button")
        };
        assert!(button.use_default_skin);
    }
    set_thread_skin(ActiveSkin::Modern);
}
