use game_engine_core::adventure_guide::Catalog;
use game_engine_ui_model::adventure_guide::adventure_guide_screen;
use game_engine_ui_model::adventure_guide::{Page, Tab, View};
use std::sync::Arc;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Dimension, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};
fn journal() -> View {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/cache/adventure_guide.sqlite");
    View {
        catalog: Arc::new(
            Catalog::load(&path).expect("Import pinned local journal cache before testing"),
        ),
        ..Default::default()
    }
}
#[test]
fn journal_shows_authoritative_keystone_only_for_its_dungeon() {
    use shared::protocol::{ChallengePhase, ChallengeRun, KeystoneInfo};
    let mut view = journal();
    let instance = view
        .catalog
        .instances
        .iter()
        .find(|i| i.map == 2515)
        .expect("Azure Vault Journal instance")
        .id;
    view.instance = Some(instance);
    view.challenge = Some(ChallengeRun {
        run_id: 7,
        map_id: 2515,
        instance_id: 3,
        name: "The Azure Vault".into(),
        key: KeystoneInfo {
            item_guid: 42,
            item_id: 180653,
            challenge_id: 401,
            level: 8,
            affixes: vec![],
        },
        phase: ChallengePhase::Running,
        countdown_ms: 0,
        elapsed_ms: 65000,
        time_limit_ms: 2250000,
        deaths: 2,
        time_lost_ms: 0,
        forces: 205,
        forces_required: 410,
        bosses: vec![],
        upgrade_levels: 0,
        resulting_level: None,
        rating: None,
    });
    let text = view.challenge_text().unwrap();
    assert!(text.contains("The Azure Vault +8"));
    assert!(text.contains("36:25"));
    assert!(text.contains("205 / 410"));
    assert!(!text.contains("Rating"));
    view.instance = Some(67);
    assert!(view.challenge_text().is_none());
}

#[test]
fn item_sets_render_retail_rows_and_filter_paladin_specialization() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut view = journal();
    view.visible = true;
    view.apply("journal:tab:sets").unwrap();
    view.apply("journal:class:2").unwrap();
    view.apply("journal:spec:70").unwrap();
    view.search = "Lightsworn Battlegear".into();
    let mut context = SharedContext::new();
    context.insert(ActiveSkin::Modern);
    context.insert(view.clone());
    let mut registry = FrameRegistry::new(1280., 720.);
    let mut screen = Screen::new(adventure_guide_screen);
    screen.sync(&context, &mut registry);
    assert!(
        registry.get_by_name("JournalSet900Level").is_some(),
        "Retail set row includes item level"
    );
    assert!(
        registry.get_by_name("JournalSet900Item51719Icon").is_some(),
        "Retail set row includes item icons"
    );
    assert!(registry.get_by_name("JournalClassFilter").is_some());
    view.apply("journal:class:1").unwrap();
    view.apply("journal:spec:71").unwrap();
    context.insert(view);
    screen.sync(&context, &mut registry);
    assert!(
        registry.get_by_name("JournalSet900").is_none(),
        "Warrior cannot browse Paladin-only sets"
    );
}

#[test]
fn pending_guide_shows_loading_without_catalog_choices() {
    let mut view = journal();
    view.visible = true;
    view.loading = true;
    let mut context = SharedContext::new();
    context.insert(ActiveSkin::Modern);
    context.insert(view);
    let mut registry = FrameRegistry::new(1280., 720.);
    Screen::new(adventure_guide_screen).sync(&context, &mut registry);
    let loading = registry
        .get(registry.get_by_name("EncounterJournalLoading").unwrap())
        .unwrap();
    match &loading.widget_data {
        Some(WidgetData::FontString(font)) => assert_eq!(font.text, "Loading Adventure Guide…"),
        other => panic!("Expected loading notice, got {other:?}"),
    }
    assert!(registry.get_by_name("JournalInstance67").is_none());
}

#[test]
fn robe_loot_uses_the_chest_filter() {
    let mut view = journal();
    view.select_encounter(96).unwrap();
    view.difficulty = 1;
    view.page = Page::Loot;
    assert!(view.loot().iter().any(|item| item.id == 6324));
    view.apply("journal:slot:5").unwrap();
    assert!(
        view.loot().iter().any(|item| item.id == 6324),
        "Robes are Chest loot, not a duplicate Chest category"
    );
}

#[test]
fn heroic_only_vanessa_does_not_offer_normal_difficulty() {
    let mut view = journal();
    view.difficulty = 1;
    view.select_encounter(95).unwrap();
    assert_eq!(view.difficulty, 2);
    assert!(view.apply("journal:difficulty:1").is_err());
}

#[test]
fn collapsed_ability_parent_does_not_hide_overview_children() {
    let mut view = journal();
    view.select_encounter(2491).unwrap();
    let overview: Vec<_> = view.sections().unwrap().iter().map(|(s, _)| s.id).collect();
    assert!(
        overview.contains(&25040),
        "Kurog's Altar of Flame overview exists"
    );
    view.page = Page::Abilities;
    assert!(view.sections().unwrap().iter().any(|(s, _)| s.id == 25036));
    view.apply("journal:section:25036").unwrap();
    view.page = Page::Overview;
    let after: Vec<_> = view.sections().unwrap().iter().map(|(s, _)| s.id).collect();
    assert_eq!(after, overview);
}

#[test]
fn authentic_dungeon_sections_and_loot_are_selectable() {
    let mut view = journal();
    view.apply("journal:instance:67").unwrap();
    view.apply("journal:encounter:112").unwrap();
    assert_eq!(view.tab, Tab::Dungeons);
    assert_eq!(view.catalog.encounter(112).unwrap().name, "Ozruk");
    view.apply("journal:page:abilities").unwrap();
    assert_eq!(view.page, Page::Abilities);
    let sections = view.sections().unwrap();
    assert!(!sections.is_empty());
    assert!(sections.iter().any(|(s, _)| s.title.contains("Shatter")));
    let section = sections[0].0.id;
    view.apply(&format!("journal:section:{section}")).unwrap();
    assert!(view.collapsed.contains(&section));
    view.apply("journal:page:loot").unwrap();
    let loot = view.loot();
    assert!(!loot.is_empty());
    assert!(loot.iter().all(|i| !i.name.is_empty()));
    let slot = loot[0].slot;
    view.apply(&format!("journal:slot:{slot}")).unwrap();
    assert!(view.loot().iter().all(|i| i.slot == slot));
    view.apply("journal:slot:0").unwrap();
    view.apply("journal:class:2").unwrap();
    view.apply("journal:spec:70").unwrap();
    assert!(
        view.loot().iter().any(|i| i.id == 56342),
        "Retribution must include the authored two-handed sword"
    );
    assert!(
        view.loot().iter().all(|i| i.id != 56340),
        "Paladin filter excludes mail bracers"
    );
}
#[test]
fn both_skins_render_authored_ability_loot_and_raid_text() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = journal();
        view.select_encounter(112).unwrap();
        view.page = Page::Abilities;
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280., 720.);
        Screen::new(adventure_guide_screen).sync(&context, &mut registry);
        let root = registry
            .get(registry.get_by_name("EncounterJournal").unwrap())
            .unwrap();
        assert_eq!(root.width, Dimension::Fixed(800.));
        assert_eq!(root.height, Dimension::Fixed(496.));
        let title = registry
            .get(
                registry
                    .get_by_name("EncounterJournalEncounterTitle")
                    .unwrap(),
            )
            .unwrap();
        match &title.widget_data {
            Some(WidgetData::FontString(font)) => assert_eq!(font.text, "Ozruk"),
            other => panic!("Expected title, got {other:?}"),
        }
        let section = view.sections().unwrap()[0].0.id;
        assert!(
            registry
                .get_by_name(&format!("JournalSection{section}"))
                .is_some()
        );
        view.page = Page::Loot;
        let loot_id = view.loot()[0].id;
        context.insert(view.clone());
        Screen::new(adventure_guide_screen).sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name(&format!("JournalLoot{loot_id}Name"))
                .is_some()
        );
        view.select_encounter(1519).unwrap();
        context.insert(view);
        Screen::new(adventure_guide_screen).sync(&context, &mut registry);
        let title = registry
            .get(
                registry
                    .get_by_name("EncounterJournalEncounterTitle")
                    .unwrap(),
            )
            .unwrap();
        match &title.widget_data {
            Some(WidgetData::FontString(font)) => assert_eq!(font.text, "Lucifron"),
            other => panic!("Expected raid title, got {other:?}"),
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn raid_selection_search_and_back_preserve_authored_identity() {
    let mut view = journal();
    view.apply("journal:tab:raids").unwrap();
    view.apply("journal:tier:68").unwrap();
    view.search = "Molten".into();
    assert_eq!(
        view.instances().iter().map(|i| i.id).collect::<Vec<_>>(),
        vec![741]
    );
    view.apply("journal:encounter:1519").unwrap();
    assert_eq!(view.instance, Some(741));
    assert_eq!(
        view.difficulty, 9,
        "Molten Core uses its authored 40-player difficulty"
    );
    assert_eq!(view.catalog.display(1519), Some(13031));
    view.apply("journal:back").unwrap();
    assert_eq!(view.encounter, None);
    assert_eq!(view.search, "");
}
