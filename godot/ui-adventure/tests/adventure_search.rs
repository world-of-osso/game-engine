use game_engine_core::adventure_guide::Catalog;
use game_engine_ui_model::adventure_guide::{Page, Tab, View, adventure_guide_screen};
use std::sync::Arc;
use ui_toolkit::{
    atlas::ActiveSkin,
    frame::{Dimension, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::texture::TextureSource,
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
fn journal_global_search_preserves_the_authored_instance_grid_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = journal("");
        view.tier = view.catalog.instance(67).unwrap().tiers[0];
        let expected: Vec<_> = view
            .instances()
            .iter()
            .map(|instance| instance.id)
            .collect();
        assert!(expected.contains(&67), "Authentic Stonecore grid fixture");
        for query in ["flame", "Paralyze"] {
            view.search = query.into();
            for action in [
                "journal:search-focus",
                "journal:search-full",
                "journal:search-close",
            ] {
                view.apply(action).unwrap();
                let actual: Vec<_> = view
                    .instances()
                    .iter()
                    .map(|instance| instance.id)
                    .collect();
                assert_eq!(
                    actual, expected,
                    "Global search must not filter the instance grid"
                );
                assert!(present(&render(&view, skin), "JournalInstance67"));
            }
        }
    }
}

#[test]
fn journal_search_item_children_resolve_the_shared_owner_in_preview_and_full_results() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = journal("Buzzer Blade");
        game_engine_ui_model::item_catalog::wait_for_item_catalog();
        view.apply("journal:search-focus").unwrap();
        for (action, row) in [
            ("journal:search-focus", "JournalSearchPreview0"),
            ("journal:search-full", "JournalSearchResult0"),
        ] {
            view.apply(action).unwrap();
            let registry = render(&view, skin);
            let hit = registry.get_by_name(&format!("{row}Icon")).unwrap();
            let (owner, binding) = ui_toolkit::tooltip::tooltip_owner(&registry, hit)
                .expect("Actual item-search child must resolve a live tooltip owner");
            assert_eq!(owner.name.as_deref(), Some(row));
            assert_eq!(binding.provider.as_deref(), Some("journal_loot"));
            assert_eq!(binding.anchor, ui_toolkit::tooltip::TooltipAnchor::Right);
            assert_eq!(binding.offset, [0., 0.]);
            assert_eq!(view.search_item(row).unwrap().id, 2169);
        }
    }
}

#[test]
fn journal_search_item_results_preserve_source_icon_quality_parent_and_difficulty() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = journal("Buzzer Blade");
        game_engine_ui_model::item_catalog::wait_for_item_catalog();
        view.apply("journal:search-focus").unwrap();
        let registry = render(&view, skin);
        let frame = registry
            .get(
                registry
                    .get_by_name("JournalSearchPreview0Name")
                    .expect("Real Journal loot must produce its search preview"),
            )
            .unwrap();
        let Some(WidgetData::FontString(name)) = &frame.widget_data else {
            panic!("Item result title");
        };
        assert_eq!(name.text, "Buzzer Blade");
        assert_eq!(name.color, [0., 0.44, 0.87, 1.]);
        assert_eq!(
            view.search_item("JournalSearchPreview0")
                .map(|item| item.id),
            Some(2169)
        );
        let icon = registry
            .get(registry.get_by_name("JournalSearchPreview0Icon").unwrap())
            .unwrap();
        assert!(
            matches!(&icon.widget_data, Some(WidgetData::Texture(texture))
            if texture.source == TextureSource::FileDataId(135641))
        );
        view.apply("journal:search-full").unwrap();
        assert_eq!(
            view.search_item("JournalSearchResult0").map(|item| item.id),
            Some(2169)
        );
        assert!(view.search_item("JournalSearchPreview0").is_none());
        let registry = render(&view, skin);
        for (name, expected) in [
            (
                "JournalSearchResult0Type",
                game_engine_ui_model::auction_mail::text("ENCOUNTER_JOURNAL_ITEM"),
            ),
            ("JournalSearchResult0Path", "Deadmines > Glubtok"),
        ] {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            assert!(
                matches!(&frame.widget_data, Some(WidgetData::FontString(text)) if text.text == expected)
            );
        }
        for (index, encounter, difficulty, item) in [(0, 89, 1, 2169), (1, 2624, 2, 65163)] {
            let mut selection = journal("Buzzer Blade");
            selection.difficulty = 24;
            selection
                .apply(&format!("journal:search-select:{index}"))
                .unwrap();
            assert_eq!(selection.instance, Some(63));
            assert_eq!(selection.encounter, Some(encounter));
            assert_eq!(selection.difficulty, difficulty);
            assert_eq!(selection.page, Page::Loot);
            assert_eq!(selection.search, "Buzzer Blade");
            assert!(selection.loot().iter().any(|drop| drop.id == item));
            assert!(selection.search_item("JournalSearchPreview0").is_none());
        }
    }
}

#[test]
fn journal_search_item_result_resolves_the_real_zero_raw_icon_appearance() {
    let mut view = journal("Onyx-Imbued Breeches");
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    assert_eq!(view.catalog.items[&174139].icon, 0);
    view.apply("journal:search-focus").unwrap();
    let registry = render(&view, ActiveSkin::Modern);
    let icon = registry
        .get(
            registry
                .get_by_name("JournalSearchPreview0Icon")
                .expect("Real zero-raw-icon item search result is required"),
        )
        .unwrap();
    assert!(
        matches!(&icon.widget_data, Some(WidgetData::Texture(texture))
        if texture.source == TextureSource::FileDataId(2978247))
    );
}

#[test]
fn journal_search_creature_results_open_the_authored_model_choice() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut view = journal("Lumbering Oaf");
        view.apply("journal:search-focus").unwrap();
        let registry = render(&view, skin);
        assert!(
            present(&registry, "JournalSearchPreview0"),
            "Authored creature result missing"
        );
        view.apply("journal:search-full").unwrap();
        let registry = render(&view, skin);
        for (name, expected) in [
            ("JournalSearchResult0Name", "Lumbering Oaf"),
            (
                "JournalSearchResult0Type",
                game_engine_ui_model::auction_mail::text("ENCOUNTER_JOURNAL_ENCOUNTER"),
            ),
            ("JournalSearchResult0Path", "Deadmines > Helix Gearbreaker"),
        ] {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            assert!(
                matches!(&frame.widget_data, Some(WidgetData::FontString(text)) if text.text == expected)
            );
        }
        view.apply("journal:search-select:0").unwrap();
        assert_eq!(view.instance, Some(63));
        assert_eq!(view.encounter, Some(90));
        assert_eq!(view.difficulty, 1);
        assert_eq!(view.page, Page::Model);
        assert_eq!(view.creature, Some(630));
        assert_eq!(view.model_creature().unwrap().display, 33003);
        assert_eq!(view.search, "Lumbering Oaf");
        let registry = render(&view, skin);
        assert!(present(&registry, "JournalCreature630"));
        let mut heroic = journal("Lumbering Oaf");
        heroic.apply("journal:search-select:1").unwrap();
        assert_eq!(heroic.encounter, Some(2623));
        assert_eq!(heroic.difficulty, 2);
        assert_eq!(heroic.page, Page::Model);
        assert_eq!(heroic.creature, Some(5689));
        assert_eq!(heroic.model_creature().unwrap().display, 33003);
    }
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
