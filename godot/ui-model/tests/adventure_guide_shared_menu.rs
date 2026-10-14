//! Untitled EJ menus retain source radio-row inset without an invented heading/divider.
use game_engine_ui_model::menu_primitives::{
    ContextMenu, ContextMenuItem, MenuElementKind, context_menu_with_rows, menu_element,
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::Dimension,
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widget_def::Element,
};
fn untitled_menu(_: &SharedContext) -> Element {
    let items = [
        ContextMenuItem {
            name: "MenuPaladin",
            label: "Paladin",
            action: "journal:class:2",
        },
        ContextMenuItem {
            name: "MenuWarrior",
            label: "Warrior",
            action: "journal:class:1",
        },
    ];
    let rows = items
        .iter()
        .enumerate()
        .flat_map(|(i, item)| {
            menu_element(
                *item,
                MenuElementKind::Radio { checked: i == 0 },
                true,
                (8., 8. + 20. * i as f32, 124., 20.),
            )
        })
        .collect();
    context_menu_with_rows(
        ContextMenu {
            frame_name: "JournalUntitledMenu",
            title_name: "JournalUnusedTitle",
            divider_name: "JournalUnusedDivider",
            hidden: false,
            title: "",
            width: 140.,
            x: 0.,
            y: 0.,
            items: &items,
        },
        rows,
    )
}
#[test]
fn adventure_guide_shared_untitled_menu_has_only_source_radio_rows_and_padding() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut registry = FrameRegistry::new(1920., 1080.);
        Screen::new(untitled_menu).sync(&SharedContext::new(), &mut registry);
        let menu = registry
            .get(registry.get_by_name("JournalUntitledMenu").unwrap())
            .unwrap();
        assert_eq!(menu.height, Dimension::Fixed(63.));
        assert!(registry.get_by_name("JournalUnusedTitle").is_none());
        assert!(registry.get_by_name("JournalUnusedDivider").is_none());
        let row = registry
            .get(registry.get_by_name("MenuPaladin").unwrap())
            .unwrap();
        assert_eq!(row.position.top, Val::Px(8.));
        assert_eq!(registry.click_frame(row.id), Some("journal:class:2".into()));
        assert!(registry.get_by_name("MenuPaladinTick").is_some());
    }
}
