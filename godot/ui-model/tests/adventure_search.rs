use game_engine_core::adventure_guide::Catalog;
use game_engine_ui_model::adventure_guide::{Page, Tab, View, adventure_guide_screen};
use std::sync::Arc;
use ui_toolkit::{
    atlas::ActiveSkin,
    frame::{Dimension, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn journal(query: &str) -> View {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    View {
        visible: true,
        search: query.into(),
        catalog: Arc::new(Catalog::load(&root.join("cache/adventure_guide.sqlite")).unwrap()),
        ..Default::default()
    }
}
fn render(view: &View, skin: ActiveSkin) -> FrameRegistry {
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx.insert(view.clone());
    let mut registry = FrameRegistry::new(1280., 720.);
    Screen::new(adventure_guide_screen).sync(&ctx, &mut registry);
    registry
}
fn present(registry: &FrameRegistry, name: &str) -> bool {
    registry
        .get_by_name(name)
        .is_some_and(|id| registry.get(id).unwrap().visible)
}
#[test]
fn journal_search_preview_keyboard_full_popup_and_close_preserve_query() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = journal("flame");
        view.tab = Tab::ItemSets;
        view.apply("journal:search-focus").unwrap();
        let registry = render(&view, skin);
        assert!(present(&registry, "EncounterJournalSearchPreview"));
        for index in 0..5 {
            let frame = registry
                .get(
                    registry
                        .get_by_name(&format!("JournalSearchPreview{index}"))
                        .unwrap(),
                )
                .unwrap();
            assert_eq!(frame.width, Dimension::Fixed(210.));
            assert_eq!(
                frame.height,
                Dimension::Fixed(if index == 0 { 24. } else { 27. })
            );
        }
        assert!(!present(&registry, "JournalSearchPreview5"));
        view.apply("journal:search-move:1").unwrap();
        assert!(present(
            &render(&view, skin),
            "JournalSearchPreview1Selected"
        ));
        view.apply("journal:search-move:-1").unwrap();
        view.apply("journal:search-move:-1").unwrap();
        assert!(present(&render(&view, skin), "JournalSearchAllSelected"));
        view.apply("journal:search-submit").unwrap();
        let registry = render(&view, skin);
        assert!(!present(&registry, "EncounterJournalSearchPreview"));
        let frame = registry
            .get(
                registry
                    .get_by_name("EncounterJournalSearchResults")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            (frame.width, frame.height),
            (Dimension::Fixed(600.), Dimension::Fixed(400.))
        );
        let frame = registry
            .get(registry.get_by_name("JournalSearchResult0").unwrap())
            .unwrap();
        assert_eq!(
            (frame.width, frame.height),
            (Dimension::Fixed(575.), Dimension::Fixed(49.))
        );
        for suffix in ["Name", "Type", "Path"] {
            let frame = registry
                .get(
                    registry
                        .get_by_name(&format!("JournalSearchResult0{suffix}"))
                        .unwrap(),
                )
                .unwrap();
            let Some(WidgetData::FontString(text)) = &frame.widget_data else {
                panic!("result {suffix}")
            };
            assert!(!text.text.is_empty());
        }
        view.apply("journal:search-close").unwrap();
        assert_eq!(view.search, "flame");
        assert!(!present(
            &render(&view, skin),
            "EncounterJournalSearchResults"
        ));
        view.apply("journal:search-focus").unwrap();
        view.apply("journal:search-blur").unwrap();
        assert!(!present(
            &render(&view, skin),
            "EncounterJournalSearchPreview"
        ));
        view.apply("journal:search-clear").unwrap();
        assert!(view.search.is_empty());
    }
}
#[test]
fn journal_search_minimum_empty_and_section_result_navigation() {
    let mut view = journal("fl");
    view.apply("journal:search-focus").unwrap();
    assert!(!present(
        &render(&view, ActiveSkin::Modern),
        "EncounterJournalSearchPreview"
    ));
    view.search = "not-a-real-journal-title-123".into();
    assert!(!present(
        &render(&view, ActiveSkin::Modern),
        "EncounterJournalSearchPreview"
    ));
    view.search = "Paralyze".into();
    view.page = Page::Loot;
    view.collapsed.insert(2405);
    view.apply("journal:search-select:0").unwrap();
    assert_eq!(view.encounter, Some(112));
    assert_eq!(view.instance, Some(67));
    assert_eq!(view.difficulty, 2);
    assert!(
        view.sections()
            .unwrap()
            .iter()
            .any(|(section, _)| section.id == 2410)
    );
    assert_eq!(view.page, Page::Overview);
    assert!(view.search.is_empty());
}
