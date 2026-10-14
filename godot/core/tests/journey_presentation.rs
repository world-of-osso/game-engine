use game_engine_core::adventure_guide::read_journey_presentation;
use std::path::PathBuf;

#[test]
fn published_journeys_join_covenant_display_priority_to_texture_kit_prefix() {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let presentation = read_journey_presentation(&data).unwrap();
    for (covenant, kit, priority) in [
        (12, "Centaur", 3),
        (13, "Expedition", 4),
        (14, "Tuskarr", 2),
        (15, "Valdrakken", 1),
        (55, "labyrinth", 0),
    ] {
        let row = presentation.get(&covenant).unwrap();
        assert_eq!(row.texture_kit, kit);
        assert_eq!(row.priority, priority);
    }
    assert!(
        !presentation.contains_key(&2503),
        "Keys are Covenant IDs, not Faction IDs"
    );
}

#[test]
fn missing_authored_texture_kit_reports_the_covenant_and_kit() {
    let directory =
        std::env::temp_dir().join(format!("journey-presentation-{}", std::process::id()));
    let db2 = directory.join("db2/active");
    std::fs::create_dir_all(&db2).unwrap();
    std::fs::write(
        db2.join("UiCovenantDisplayInfo.csv"),
        "ID,CovenantID,UiTextureKitID,UiPriority\n7,12,5478,3\n",
    )
    .unwrap();
    std::fs::write(
        db2.join("UiTextureKit.csv"),
        "ID,KitPrefix\n5477,Expedition\n",
    )
    .unwrap();
    let error = read_journey_presentation(&directory).unwrap_err();
    std::fs::remove_dir_all(directory).unwrap();
    assert!(error.contains("12"), "{error}");
    assert!(error.contains("5478"), "{error}");
}
