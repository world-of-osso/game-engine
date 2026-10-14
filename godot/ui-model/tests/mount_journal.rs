use game_engine_ui_model::mount_journal::MountJournalView;
use shared::protocol::{MountJournalEntry, MountJournalUpdate};

fn mare() -> MountJournalEntry {
    MountJournalEntry {
        mount_id: 18,
        spell_id: 6648,
        teaching_items: vec![5655],
        name: "Chestnut Mare".into(),
        description: "Sturdy and steady; \"the horse that built Stormwind.\"".into(),
        source_text: r"|cFFFFD200Vendor: |rKatie Stokx|n|cFFFFD200Zone: |rStormwind City|n|cFFFFD200Cost: |r1|TINTERFACE\MONEYFRAME\UI-GOLDICON.BLP:0|t".into(),
        source_type: 2,
        icon_file_data_id: 132261,
        display_id: 2405,
        capability_flags: 29,
        journal_types: shared::protocol::mount_journal_types::GROUND,
        flags: 0,
        collected: true,
        favourite: false,
        usable: true,
        active: false,
    }
}
#[test]
fn mount_journal_snapshot_changes_rebuild_shared_screen_without_reopening() {
    use game_engine_ui_model::mount_journal::mount_journal_screen;
    use ui_toolkit::{
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut view = view();
    view.visible = true;
    let mut shared = SharedContext::new();
    shared.insert(view.clone());
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut screen = Screen::new(mount_journal_screen);
    screen.sync(&shared, &mut registry);
    for active in [true, false, true, false] {
        let mut mount = mare();
        mount.active = active;
        mount.favourite = true;
        view.receive(MountJournalUpdate {
            complete: false,
            mounts: vec![mount],
            error: None,
        });
        assert_eq!(
            view.catalog.len(),
            2,
            "Compact feedback must retain the other catalog rows"
        );
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        let button = registry
            .get(registry.get_by_name("MountJournalMountButton").unwrap())
            .unwrap();
        let Some(WidgetData::Button(button)) = &button.widget_data else {
            panic!("Mount action missing")
        };
        assert_eq!(button.text, if active { "Dismount" } else { "Mount" });
        assert!(registry.get_by_name("MountJournalRow18Favorite").is_some());
    }
}

#[test]
fn mount_display_follows_retail_inset_allpoints_in_both_skins() {
    use game_engine_ui_model::mount_journal::mount_journal_screen;
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        frame::Dimension,
        layout_values::Val,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = view();
        state.visible = true;
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(mount_journal_screen).sync(&shared, &mut registry);
        for (name, at, size) in [
            ("MountJournalRightInset", [284.0, 60.0], [413.0, 520.0]),
            ("MountJournalMountDisplay", [3.0, 3.0], [407.0, 514.0]),
            ("MountJournalModelScene", [0.0, 0.0], [407.0, 514.0]),
            ("MountJournalBackground", [0.0, 0.0], [407.0, 514.0]),
        ] {
            let frame = registry
                .get(registry.get_by_name(name).expect(name))
                .unwrap();
            assert_eq!(frame.width, Dimension::Fixed(size[0]));
            assert_eq!(frame.height, Dimension::Fixed(size[1]));
            assert_eq!(frame.position.left, Val::Px(at[0]));
            assert_eq!(frame.position.top, Val::Px(at[1]));
        }
        assert_eq!(
            registry
                .get(registry.get_by_name("MountJournalModelScene").unwrap())
                .unwrap()
                .parent_id,
            registry.get_by_name("MountJournalMountDisplay")
        );
        assert!(registry.get_by_name("MountJournalShadowTopLeft").is_some());
    }
}

#[test]
fn mount_context_menu_fits_its_rendered_labels_in_both_skins() {
    use game_engine_ui_model::mount_journal::mount_journal_screen;
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
        text_measure::measure_text,
        widgets::font_string::GameFont,
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = view();
        state.visible = true;
        state.context = Some((18, [300.0, 100.0]));
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(mount_journal_screen).sync(&shared, &mut registry);
        let menu = registry
            .get(registry.get_by_name("MountJournalContextMenu").unwrap())
            .unwrap();
        let label = measure_text("Set Favorite", GameFont::FrizQuadrata, 12.0)
            .unwrap()
            .0;
        assert_eq!(
            menu.width.value(),
            label.ceil() + 16.0,
            "Retail context menus fit content instead of reserving a brown push-button column"
        );
        for (name, action) in [
            ("MountJournalContextSummon", "mount:summon"),
            ("MountJournalFavoriteToggle", "mount:favourite"),
        ] {
            let row = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            assert_eq!(row.onclick.as_deref(), Some(action));
        }
    }
}

#[test]
fn mount_filter_menu_fits_owner_and_keeps_source_spacer_in_both_skins() {
    use game_engine_ui_model::mount_journal::mount_journal_screen;
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        frame::Dimension,
        layout_values::Val,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
        text_measure::measure_text,
        widgets::font_string::GameFont,
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = view();
        state.visible = true;
        state.filters_open = true;
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(mount_journal_screen).sync(&shared, &mut registry);
        let frame = |name| {
            registry
                .get(registry.get_by_name(name).expect(name))
                .unwrap()
        };
        let widest = measure_text("Not Collected", GameFont::FrizQuadrata, 12.0)
            .unwrap()
            .0
            + 19.0;
        assert_eq!(
            frame("MountJournalFilterMenu").width,
            Dimension::Fixed((widest.ceil() + 16.0).max(90.0))
        );
        assert_eq!(frame("MountJournalFilter3").position.top, Val::Px(78.0));
        assert_eq!(
            frame("MountJournalFilterMenu").height,
            Dimension::Fixed(153.0)
        );
    }
}

fn view() -> MountJournalView {
    let mut view = MountJournalView::default();
    let mut horse = mare();
    horse.mount_id = 6;
    horse.spell_id = 458;
    horse.name = "Brown Horse".into();
    horse.display_id = 2404;
    horse.description =
        "A favorite among Stormwind's guards thanks to its patience and stamina.".into();
    horse.collected = false;
    horse.usable = false;
    view.receive(MountJournalUpdate {
        complete: true,
        mounts: vec![mare(), horse],
        error: None,
    });
    view
}
#[test]
fn mount_journal_known_item_feedback_tracks_authoritative_ownership_and_source() {
    use game_engine_ui_model::tooltip_presentation::TooltipPresentation;
    use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
    let mut view = view();
    assert_eq!(
        view.item_label(Retail, 5655, "Chestnut Mare Bridle"),
        "Chestnut Mare Bridle (Already known)"
    );
    assert_eq!(
        view.item_label(Forever70205, 5655, "Different item"),
        "Different item"
    );
    assert_eq!(
        view.item_tooltip(Retail, 5655, TooltipPresentation::default())
            .lines[0]
            .left_text,
        "Already known"
    );
    let mut mare = mare();
    mare.collected = false;
    view.receive(MountJournalUpdate {
        complete: false,
        mounts: vec![mare.clone()],
        error: None,
    });
    assert!(
        view.item_tooltip(Retail, 5655, TooltipPresentation::default())
            .lines
            .is_empty()
    );
    mare.collected = true;
    mare.usable = false;
    view.receive(MountJournalUpdate {
        complete: false,
        mounts: vec![mare],
        error: None,
    });
    assert!(
        view.item_is_known(Retail, 5655),
        "Known is account ownership, not riding usability"
    );
    assert!(!view.item_is_known(Retail, 25));
}

#[test]
fn mount_journal_collected_details_show_already_known() {
    use game_engine_ui_model::mount_journal::mount_journal_screen;
    use ui_toolkit::{
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut view = view();
    view.visible = true;
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(mount_journal_screen).sync(&shared, &mut registry);
    let id = registry
        .get_by_name("MountJournalKnown")
        .expect("Collected mount known status");
    let Some(WidgetData::FontString(text)) = &registry.get(id).unwrap().widget_data else {
        panic!("Known text missing")
    };
    assert_eq!(text.text, "Already known");
}

#[test]
fn mount_journal_source_renders_real_db2_markup_as_readable_vendor_zone_and_cost() {
    use game_engine_ui_model::mount_journal::mount_journal_screen;
    use ui_toolkit::{
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut view = view();
    view.visible = true;
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(mount_journal_screen).sync(&shared, &mut registry);
    let frame = registry
        .get(registry.get_by_name("MountJournalSource").unwrap())
        .unwrap();
    let Some(WidgetData::FontString(text)) = &frame.widget_data else {
        panic!("Source label missing");
    };
    assert_eq!(
        text.text,
        "Vendor: Katie Stokx\nZone: Stormwind City\nCost: 1g"
    );
}

#[test]
fn mount_journal_search_source_type_and_collected_filters_reselect() {
    let mut view = view();
    assert_eq!(view.selected, Some(18));
    view.search = "BROWN".into();
    view.reselect();
    assert_eq!(view.rows()[0].mount_id, 6);
    assert_eq!(view.selected, Some(6));
    view.apply_filter("mount:uncollected");
    assert!(view.rows().is_empty());
    assert_eq!(view.selected, None);
    view.search.clear();
    view.reselect();
    assert_eq!(view.rows()[0].mount_id, 18);
    view.apply_filter("mount:source:2");
    assert!(view.rows().is_empty());
    view.apply_filter("mount:source:2");
    assert_eq!(view.rows().len(), 1);
    view.apply_filter("mount:type:1");
    assert!(view.rows().is_empty());
    view.apply_filter("mount:type:1");
    assert_eq!(view.rows().len(), 1);
}
#[test]
fn mount_search_keeps_query_when_filter_rebuilds_and_renders_search_template() {
    use game_engine_ui_model::mount_journal::mount_journal_screen;
    use ui_toolkit::{
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut view = view();
    view.visible = true;
    view.search = "Brown".into();
    view.apply_filter("mount:filters");
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(mount_journal_screen).sync(&shared, &mut registry);
    let search = registry
        .get(registry.get_by_name("MountJournalSearchBox").unwrap())
        .unwrap();
    let Some(WidgetData::EditBox(edit)) = &search.widget_data else {
        panic!("search editbox")
    };
    assert_eq!(edit.text, "Brown");
    assert!(registry.get_by_name("MountJournalSearchLeft").is_some());
    let clear = registry
        .get(registry.get_by_name("MountJournalSearchClear").unwrap())
        .unwrap();
    assert_eq!(clear.onclick.as_deref(), Some("mount:search-clear"));
    let title = registry
        .get(registry.get_by_name("MountJournalFilter3").unwrap())
        .unwrap();
    assert_eq!(title.position.top, ui_toolkit::layout_values::Val::Px(78.0));
}

#[test]
fn mount_filters_reset_and_source_bulk_controls_reselect_real_rows() {
    let mut view = view();
    assert!(view.apply_filter("mount:sources:none"));
    assert!(view.rows().is_empty());
    assert_eq!(view.selected, None);
    assert!(view.apply_filter("mount:sources:all"));
    assert_eq!(view.rows().len(), 2);
    assert!(view.apply_filter("mount:unusable"));
    assert!(view.rows().iter().all(|mount| mount.usable));
    assert!(view.apply_filter("mount:reset"));
    assert_eq!(view.rows().len(), 2);
}

#[test]
fn mount_journal_hidden_until_collected_and_server_only_rows_do_not_leak_into_list() {
    let mut view = view();
    view.catalog.get_mut(&6).unwrap().flags = 0x40;
    assert_eq!(
        view.rows()
            .iter()
            .map(|mount| mount.mount_id)
            .collect::<Vec<_>>(),
        vec![18]
    );
    view.catalog.get_mut(&6).unwrap().collected = true;
    assert_eq!(view.rows().len(), 2);
    view.catalog.get_mut(&6).unwrap().flags = 0x1;
    assert_eq!(view.rows().len(), 1);
}

#[test]
fn mount_journal_ground_filter_does_not_include_flying_mount_ground_capability() {
    use shared::protocol::mount_journal_types as types;
    let mut view = view();
    let mut gryphon = mare();
    gryphon.mount_id = 129;
    gryphon.spell_id = 32235;
    gryphon.name = "Golden Gryphon".into();
    gryphon.display_id = 17697;
    gryphon.icon_file_data_id = 294468;
    gryphon.flags = 132;
    gryphon.capability_flags = 95;
    gryphon.journal_types = types::FLYING | types::SKYRIDING;
    view.catalog.insert(129, gryphon);
    view.excluded_types = [
        types::FLYING,
        types::AQUATIC,
        types::SKYRIDING,
        types::RIDE_ALONG,
    ]
    .into();
    assert!(view.rows().iter().all(|mount| mount.mount_id != 129));
    assert!(view.rows().iter().any(|mount| mount.mount_id == 18));
}

#[test]
fn mount_journal_authoritative_favourite_and_active_refresh_preserve_selection() {
    let mut view = view();
    assert!(!view.selected_mount().unwrap().favourite);
    let mut selected = mare();
    selected.favourite = true;
    selected.active = true;
    view.receive(MountJournalUpdate {
        complete: false,
        mounts: vec![selected],
        error: Some("InCombat".into()),
    });
    assert_eq!(view.selected, Some(18));
    assert!(view.selected_mount().unwrap().active);
    assert!(view.selected_mount().unwrap().favourite);
    assert_eq!(view.error, "InCombat");
}
