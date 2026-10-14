use game_engine_ui_model::{
    cooldown_assist::CooldownViewerView, cooldown_settings_component::CooldownSettingsView,
    cooldown_viewer_component::cooldown_viewer_screen,
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn retail_cooldown_settings_shell_and_side_tabs_use_source_rects_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(CooldownViewerView {
            settings: CooldownSettingsView {
                open: true,
                ..Default::default()
            },
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(cooldown_viewer_screen).sync(&shared, &mut registry);
        let shell = registry
            .get(registry.get_by_name("CooldownViewerSettings").unwrap())
            .unwrap();
        assert_eq!((shell.width.value(), shell.height.value()), (399.0, 609.0));
        assert_eq!(
            (shell.position.left, shell.position.top),
            (Val::Px(16.0), Val::Px(116.0))
        );
        for (index, name) in ["Spells", "Auras", "GroupBuffs"].iter().enumerate() {
            let tab = registry
                .get(
                    registry
                        .get_by_name(&format!("CooldownSettings{name}Tab"))
                        .unwrap(),
                )
                .unwrap();
            assert_eq!((tab.width.value(), tab.height.value()), (43.0, 55.0));
            assert_eq!(
                (tab.position.left, tab.position.top),
                (Val::Px(399.0), Val::Px(28.0 + index as f32 * 58.0))
            );
        }
        let layout = registry
            .get(
                registry
                    .get_by_name("CooldownSettingsLayoutDropdown")
                    .expect("Retail layout dropdown"),
            )
            .unwrap();
        assert_eq!((layout.width.value(), layout.height.value()), (120.0, 25.0));
        assert_eq!(
            (layout.position.left, layout.position.top),
            (Val::Px(5.0), Val::Px(581.0))
        );
        let undo = registry
            .get(
                registry
                    .get_by_name("CooldownSettingsUndo")
                    .expect("Retail UndoButton"),
            )
            .unwrap();
        assert_eq!((undo.width.value(), undo.height.value()), (164.0, 22.0));
        assert_eq!(
            (undo.position.left, undo.position.top),
            (Val::Px(232.0), Val::Px(583.0))
        );
        assert!(registry.get_by_name("CooldownSettingsPrevious").is_none());
        assert!(registry.get_by_name("CooldownSettingsNext").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}
