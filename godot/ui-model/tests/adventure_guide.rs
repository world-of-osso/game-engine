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
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    let path = root.join("cache/adventure_guide.sqlite");
    View {
        catalog: Arc::new(
            Catalog::load(&path).expect("Import pinned local journal cache before testing"),
        ),
        ..Default::default()
    }
}
#[test]
fn journal_monthly_and_suggested_tabs_open_server_loading_panes() {
    let mut view = journal();
    view.visible = true;
    for tab in ["monthly", "suggested"] {
        view.apply(&format!("journal:tab:{tab}"))
            .expect("server-backed tab");
        let mut shared = SharedContext::new();
        shared.insert(ActiveSkin::Modern);
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1280., 720.);
        let mut screen = Screen::new(adventure_guide_screen);
        screen.sync(&shared, &mut registry);
        let name = if tab == "monthly" {
            "JournalMonthlyLoading"
        } else {
            "JournalSuggestedLoading"
        };
        assert!(registry.get_by_name(name).is_some(), "{name}");
    }
}

#[test]
fn runtime_encounter_link_uses_authoritative_timewalking_difficulty() {
    let mut view = journal();
    view.difficulty = 2;
    view.select_dungeon_encounter(1058, Some(24)).unwrap();
    assert_eq!(view.encounter, Some(112));
    assert_eq!(view.instance, Some(67));
    assert_eq!(view.difficulty, 24);
    assert_eq!(view.loot().len(), 8);
    let before = view.clone();
    assert!(view.select_dungeon_encounter(1058, Some(999)).is_err());
    assert_eq!(
        view, before,
        "Unsupported realm difficulty cannot change selection"
    );
    assert!(view.select_dungeon_encounter(u32::MAX, Some(24)).is_err());
    assert_eq!(view, before);
}

#[test]
fn travelers_log_navigation_opens_closed_or_already_visible_guide() {
    let mut view = journal();
    view.search = "Lucifron".into();
    view.select_encounter(1519).unwrap();
    for visible in [false, true] {
        view.visible = visible;
        view.apply("journal:tab:monthly").unwrap();
        assert!(view.visible, "Log is navigation, not a visibility toggle");
        assert_eq!(view.tab, Tab::Monthly);
        assert!(view.instance.is_none());
        assert!(view.encounter.is_none());
        assert!(view.search.is_empty());
    }
}

#[test]
fn encounter_link_replaces_search_and_open_navigation_menus() {
    let mut view = journal();
    view.visible = true;
    view.tab = Tab::Monthly;
    view.search = "Lucifron".into();
    view.tier_open = true;
    view.difficulty_open = true;
    view.filter_menu = Some(game_engine_ui_model::adventure_guide::LootFilter::Class);
    view.collapsed.insert(2405);
    view.select_encounter(112).unwrap();
    assert_eq!(view.encounter, Some(112));
    assert_eq!(view.instance, Some(67));
    assert_eq!(view.tab, Tab::Dungeons);
    assert!(view.visible);
    assert!(
        view.search.is_empty(),
        "Old search must not cover linked Ozruk"
    );
    assert!(!view.tier_open);
    assert!(!view.difficulty_open);
    assert!(view.filter_menu.is_none());
    assert!(view.collapsed.is_empty());
}

#[test]
fn encounter_link_dismisses_overflow_and_nested_class_menu_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = journal();
        view.select_encounter(2491).unwrap();
        view.page = Page::Loot;
        view.apply("journal:filter:class").unwrap();
        view.apply("journal:class-menu").unwrap();
        view.apply("journal:nav-overflow").unwrap();
        view.select_dungeon_encounter(1058, Some(24)).unwrap();
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1920., 1080.);
        let mut screen = Screen::new(adventure_guide_screen);
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("JournalBoss112").is_some());
        assert!(
            registry
                .get_by_name("EncounterJournalNavBarOverflowMenu")
                .is_none(),
            "Linked Ozruk must not paint the previous encounter's overflow"
        );
        assert!(!view.nav_overflow_open);
        assert!(!view.class_menu_open);
        view.apply("journal:filter:class").unwrap();
        shared.insert(view);
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("EncounterJournalFilterMenu").is_some());
        assert!(
            registry.get_by_name("EncounterJournalClassMenu").is_none(),
            "Opening the new encounter's filter must not reopen the old submenu"
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn encounter_link_reopens_the_class_filter_without_the_previous_submenu() {
    let mut view = journal();
    view.select_encounter(2491).unwrap();
    view.page = Page::Loot;
    view.apply("journal:filter:class").unwrap();
    view.apply("journal:class-menu").unwrap();
    view.select_dungeon_encounter(1058, Some(24)).unwrap();
    assert!(
        !view.class_menu_open,
        "Encounter navigation must discard the previous nested menu state"
    );
    view.apply("journal:filter:class").unwrap();
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920., 1080.);
    let mut screen = Screen::new(adventure_guide_screen);
    screen.sync(&shared, &mut registry);
    assert!(registry.get_by_name("EncounterJournalFilterMenu").is_some());
    assert!(
        registry.get_by_name("EncounterJournalClassMenu").is_none(),
        "Linked encounter's filter must not paint the previous nested class submenu"
    );
}

#[test]
fn encounter_overview_role_change_expands_the_new_role_without_resetting_abilities() {
    let mut view = journal();
    view.role = 2;
    view.select_encounter(2491).unwrap();
    assert!(
        !view.collapsed.contains(&25033),
        "Kurog Damage Dealers initially expanded"
    );
    assert!(
        view.collapsed.contains(&25035),
        "Kurog Tanks initially collapsed"
    );
    view.collapsed.insert(25036);
    view.apply("journal:role:1").unwrap();
    assert!(
        !view.collapsed.contains(&25035),
        "New Tank overview must expand"
    );
    assert!(
        view.collapsed.contains(&25033),
        "Previous Damage overview must collapse"
    );
    assert!(
        view.collapsed.contains(&25034),
        "Healer overview remains collapsed"
    );
    assert!(
        view.collapsed.contains(&25036),
        "Independent ability disclosure is preserved"
    );
    view.apply("journal:section:25033").unwrap();
    view.apply("journal:role:1").unwrap();
    assert!(
        !view.collapsed.contains(&25033),
        "Unchanged role must preserve manual disclosure"
    );
}

#[test]
fn encounter_search_section_selection_preserves_real_loot_with_its_retained_query() {
    let mut view = journal();
    view.select_dungeon_encounter(1058, Some(2)).unwrap();
    let expected: Vec<_> = view.loot().iter().map(|item| item.id).collect();
    assert!(
        !expected.is_empty(),
        "Authentic Heroic Ozruk loot is required"
    );
    view.update_search("Paralyze".into());
    view.apply("journal:search-select:0").unwrap();
    assert_eq!(view.encounter, Some(112));
    assert_eq!(view.difficulty, 2);
    assert_eq!(view.search, "Paralyze");
    view.apply("journal:page:loot").unwrap();
    let actual: Vec<_> = view.loot().iter().map(|item| item.id).collect();
    assert_eq!(
        actual, expected,
        "The retained section-title search must not hide this boss's authored drops"
    );
}

#[test]
fn owned_keystones_browse_authentic_dungeons_without_active_run() {
    use shared::protocol::KeystoneInfo;
    let mut view = journal();
    view.keys = vec![KeystoneInfo {
        item_guid: 42,
        item_id: 180653,
        challenge_id: 401,
        level: 8,
        affixes: vec![],
    }];
    view.owned_keys_only = true;
    view.tier = 68;
    let maps: Vec<_> = view
        .instances()
        .iter()
        .map(|instance| instance.map)
        .collect();
    assert_eq!(
        maps,
        vec![2515],
        "Owned Azure key selects its authored dungeon across expansion filters"
    );
    assert!(view.challenge.is_none());
    view.keys.clear();
    assert!(
        view.instances().is_empty(),
        "No active run or manufactured key substitutes for an empty authoritative bag"
    );
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
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    let mut view = journal();
    view.visible = true;
    view.player_spec = Some(70);
    view.apply("journal:tab:sets").unwrap();
    assert_eq!(view.class_spec_filters(), (2, 70));
    view.apply("journal:class:2").unwrap();
    view.apply("journal:spec:70").unwrap();
    let mut catalog = (*view.catalog).clone();
    catalog.sets.retain(|set| set.id == 900);
    view.catalog = Arc::new(catalog);
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
    let level = registry
        .get(registry.get_by_name("JournalSet900Level").unwrap())
        .unwrap();
    match &level.widget_data {
        Some(WidgetData::FontString(font)) => assert_eq!(
            font.text, "Item Level 15",
            "Set level uses the same authentic Retail squish as its item tooltip"
        ),
        other => panic!("Expected item level, got {other:?}"),
    }
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
fn set_name_takes_quality_of_last_sorted_member_like_retail_configure_order() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut view = journal();
    view.visible = true;
    view.apply("journal:tab:sets").unwrap();
    view.apply("journal:class:3").unwrap();
    view.apply("journal:spec:253").unwrap();
    let mut catalog = (*view.catalog).clone();
    catalog.sets.retain(|set| set.id == 483);
    view.catalog = Arc::new(catalog);
    let mut context = SharedContext::new();
    context.insert(ActiveSkin::Modern);
    context.insert(view);
    let mut registry = FrameRegistry::new(1280., 720.);
    let mut screen = Screen::new(adventure_guide_screen);
    screen.sync(&context, &mut registry);
    let name = registry
        .get(
            registry
                .get_by_name("JournalSet483")
                .expect("Defiler's Determination row"),
        )
        .unwrap();
    // Epic Pauldrons20158 is listed first; rare Greaves20154 is configured last.
    match &name.widget_data {
        Some(WidgetData::FontString(font)) => assert_eq!(
            font.color,
            [0.0, 0.44, 0.87, 1.0],
            "Retail recolors the set name per configured button; the last sorted member wins"
        ),
        other => panic!("Expected set name, got {other:?}"),
    }
}

#[test]
fn set_tooltip_uses_authentic_members_thresholds_and_inactive_legacy_flag() {
    use game_engine_ui_model::{
        adventure_guide_tooltip::set_lines,
        bag_data::{InventorySlot, InventoryState},
    };
    use shared::protocol::EquipmentSlot;
    let mut view = journal();
    view.spec = 70;
    let set = view.catalog.sets.iter().find(|set| set.id == 900).unwrap();
    let mut inventory = InventoryState::default();
    let missing_descriptions = std::collections::BTreeMap::new();
    let lines = set_lines(&view, set, &inventory, &missing_descriptions);
    assert!(
        lines
            .iter()
            .any(|line| line.left_text == "Lightsworn Battlegear (0/5)")
    );
    assert!(
        lines
            .iter()
            .any(|line| line.left_text == "Legacy Set: Bonus is inactive")
    );
    assert!(
        lines
            .iter()
            .any(|line| line.left_text.contains("(2) Set:") && line.left_text.contains("70765"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.left_text.contains("(4) Set:") && line.left_text.contains("70762"))
    );
    for (slot, id) in [
        (EquipmentSlot::Chest, 51717),
        (EquipmentSlot::Hands, 51718),
        (EquipmentSlot::Head, 51719),
        (EquipmentSlot::Legs, 51720),
        (EquipmentSlot::Shoulder, 51721),
    ] {
        inventory.equipment.insert(
            slot,
            InventorySlot {
                item_id: id,
                count: 1,
                definition_source: shared::item_data::ItemDefinitionSource::Retail,
                ..Default::default()
            },
        );
    }
    let lines = set_lines(&view, set, &inventory, &missing_descriptions);
    assert!(
        lines
            .iter()
            .any(|line| line.left_text == "Lightsworn Battlegear (5/5)")
    );
    assert!(
        lines
            .iter()
            .filter(|line| line.left_text.contains(" Set:"))
            .all(|line| line.left_color == [0.5, 0.5, 0.5, 1.])
    );
    assert!(
        lines.iter().any(
            |line| line.left_text == "Lightsworn Helmet" && line.left_color == [1., 1., 1., 1.]
        )
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
    // No type3 overview root: Retail maps the first tab to ability details.
    view.apply("journal:page:overview").unwrap();
    assert_eq!(view.page, Page::Overview);
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
        assert_eq!(view.page, Page::Overview); // Stonecore's first tab contains details.
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280., 720.);
        let mut screen = Screen::new(adventure_guide_screen);
        screen.sync(&context, &mut registry);
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
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name(&format!("JournalLoot{loot_id}Name"))
                .is_some()
        );
        view.select_encounter(1519).unwrap();
        assert_eq!(view.page, Page::Loot); // Preserve Loot; root0 has no Overview.
        context.insert(view);
        screen.sync(&context, &mut registry);
        let title = registry
            .get(registry.get_by_name("JournalBoss1519Name").unwrap())
            .unwrap();
        match &title.widget_data {
            Some(WidgetData::FontString(font)) => assert_eq!(font.text, "Lucifron"),
            other => panic!("Expected raid title, got {other:?}"),
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn wailing_caverns_boss_link_selects_the_dungeon_tab() {
    let mut view = journal();
    view.select_encounter(475).unwrap();
    assert_eq!(view.catalog.encounter(475).unwrap().name, "Lord Cobrahn");
    assert_eq!(
        view.tab,
        Tab::Dungeons,
        "Map43 is an authored dungeon even though JournalInstance.Flags is zero"
    );
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
