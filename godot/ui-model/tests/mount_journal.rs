use game_engine_ui_model::mount_journal::MountJournalView;
use shared::protocol::{MountJournalEntry, MountJournalUpdate};

fn mare() -> MountJournalEntry {
    MountJournalEntry {
        mount_id: 18,
        spell_id: 6648,
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
            mounts: vec![mount],
            error: None,
        });
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
        mounts: vec![mare(), horse],
        error: None,
    });
    view
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
        mounts: vec![selected],
        error: Some("InCombat".into()),
    });
    assert_eq!(view.selected, Some(18));
    assert!(view.selected_mount().unwrap().active);
    assert!(view.selected_mount().unwrap().favourite);
    assert_eq!(view.error, "InCombat");
}
