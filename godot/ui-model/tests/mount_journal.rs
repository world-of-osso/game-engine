use game_engine_ui_model::mount_journal::MountJournalView;
use shared::protocol::{MountJournalEntry, MountJournalUpdate};

fn mare() -> MountJournalEntry {
    MountJournalEntry {
        mount_id: 18,
        spell_id: 6648,
        name: "Chestnut Mare".into(),
        description: "Sturdy and steady; \"the horse that built Stormwind.\"".into(),
        source_text: "Vendor: Katie Stokx".into(),
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
