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
fn journal_search_localized_display_quotes_are_not_export_escape_codes() {
    let _view = journal("flame");
    assert_eq!(
        game_engine_ui_model::auction_mail::text("ENCOUNTER_JOURNAL_SEARCH_RESULTS"),
        "Search Results for \"%s\"(%d)"
    );
    assert_eq!(
        game_engine_ui_model::auction_mail::text("ABANDON_QUEST_CONFIRM"),
        "Abandon \"%s\"?"
    );
    assert_eq!(
        game_engine_ui_model::auction_mail::text("SEARCH_RESULTS_SHOW_COUNT"),
        "Show All %d Results"
    );
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
        // EJ_GetSearchDisplay (Blizzard_EncounterJournal.lua2457–2471):
        // instance has no path; the following encounter result names its instance.
        for (name, expected) in [
            ("JournalSearchResult0Name", "Darkflame Cleft"),
            ("JournalSearchResult0Type", "Dungeon"),
            ("JournalSearchResult0Path", ""),
            ("JournalSearchResult2Name", "Flame Leviathan"),
            ("JournalSearchResult2Path", "Ulduar"),
        ] {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::FontString(text)) = &frame.widget_data else {
                panic!("result {name}")
            };
            assert_eq!(text.text, expected, "{name}");
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
    assert_eq!(
        view.search, "Paralyze",
        "Retail Close hides preview without clearing text"
    );
    view.search = "Molten Core".into();
    view.apply("journal:search-select:0").unwrap();
    assert_eq!(view.instance, Some(741));
    assert_eq!(view.tab, Tab::Raids);
    assert_eq!(view.encounter, None);
    assert_eq!(view.search, "Molten Core");
    view.search = "Lucifron".into();
    view.apply("journal:search-select:0").unwrap();
    assert_eq!(view.encounter, Some(1519));
    assert_eq!(view.search, "Lucifron");
    view.search = "flame".into();
    view.apply("journal:search-focus").unwrap();
    view.apply("journal:close").unwrap();
    assert!(view.search.is_empty());
    assert!(!view.visible);
}
