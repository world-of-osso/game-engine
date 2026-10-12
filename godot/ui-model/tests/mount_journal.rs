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
        capability_flags: 1,
        collected: true,
        favourite: false,
        usable: true,
        active: false,
    }
}
fn view() -> MountJournalView {
    let mut view = MountJournalView::default();
    let mut horse = mare();
    horse.mount_id = 6;
    horse.spell_id = 458;
    horse.name = "Brown Horse".into();
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
