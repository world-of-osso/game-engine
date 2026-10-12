use crate::npc_appearance_data::{AuthoredNpcAppearance, query_authored_npc_appearance};
use rusqlite::Connection;

fn fixture() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE display_coverage (
            display_id INTEGER PRIMARY KEY, requires_appearance INTEGER NOT NULL
        );
        INSERT INTO display_coverage VALUES (19177, 1), (19178, 1), (99, 0), (42, 1);
        CREATE TABLE appearances (
            display_id INTEGER PRIMARY KEY, race INTEGER, sex INTEGER,
            class INTEGER, baked_texture_fdid INTEGER
        );
        CREATE TABLE choices (display_id INTEGER, choice_id INTEGER);
        CREATE TABLE geosets (
            display_id INTEGER, geoset_index INTEGER, geoset_value INTEGER
        );
        INSERT INTO appearances VALUES (19177, 1, 0, 1, 1050256);
        INSERT INTO appearances VALUES (19178, 1, 1, 8, 0);
        INSERT INTO choices VALUES (19177, 9001), (19178, 65537), (19177, 300);
        INSERT INTO geosets VALUES (19177, 21, 2), (19178, 4, 3), (19177, 7, 1);",
        )
        .unwrap();
    connection
}

#[test]
fn reads_distinct_appearances_with_full_choice_ids_and_sorted_geosets() {
    let connection = fixture();
    assert_eq!(
        query_authored_npc_appearance(&connection, 19177).unwrap(),
        Some(AuthoredNpcAppearance {
            race: 1,
            sex: 0,
            class: 1,
            baked_texture_fdid: Some(1050256),
            choice_ids: vec![300, 9001],
            geosets: vec![(7, 1), (21, 2)],
        }),
    );
    assert_eq!(
        query_authored_npc_appearance(&connection, 19178).unwrap(),
        Some(AuthoredNpcAppearance {
            race: 1,
            sex: 1,
            class: 8,
            baked_texture_fdid: None,
            choice_ids: vec![65537],
            geosets: vec![(4, 3)],
        }),
    );
}

/// Real prepared catalogs and source-authenticated assets, not synthetic display substitutions.
#[test]
#[ignore = "requires NPC_APPEARANCE_ACCEPTANCE_DATA pointing at a prepared local data root"]
fn full_world_named_bosses_resolve_profiles_displays_and_required_asset_receipts() {
    use crate::asset_product::AssetProduct;
    use crate::creature_display_data::query_display;
    use crate::model_asset_index::ModelAssetIndex;
    use rusqlite::OpenFlags;
    use std::path::PathBuf;

    let data = PathBuf::from(std::env::var("NPC_APPEARANCE_ACCEPTANCE_DATA").unwrap());
    let appearances = Connection::open_with_flags(
        data.join("cache/npc_appearance.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let cobrahn = query_authored_npc_appearance(&appearances, 4213)
        .unwrap()
        .unwrap();
    assert_eq!((cobrahn.race, cobrahn.sex, cobrahn.class), (4, 0, 0));
    assert_eq!(cobrahn.baked_texture_fdid, Some(1060054));
    assert_eq!(
        cobrahn.choice_ids,
        vec![
            702, 723, 740, 754, 763, 769, 782, 789, 7610, 8018, 8040, 8062, 8084, 8094, 8107, 8129,
            8166,
        ]
    );
    for display in [4203, 8570, 22132] {
        assert_eq!(
            query_authored_npc_appearance(&appearances, display).unwrap(),
            None
        );
    }
    let displays = Connection::open_with_flags(
        data.join("cache/creature_display.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let skum = query_display(&displays, 4203).unwrap().unwrap();
    assert_eq!(skum.model_fdid, 1625963);
    assert_eq!(skum.skin_fdids, [1626025, 0, 0, 0]);
    assert_eq!(skum.source_product, AssetProduct::Retail);
    assert_eq!(
        query_display(&displays, 4213).unwrap().unwrap().scale_milli,
        1200
    );
    assert_eq!(
        query_display(&displays, 8570).unwrap().unwrap().model_fdid,
        123465
    );
    let assets = ModelAssetIndex::load(&data).unwrap();
    for (display, fdid, kind) in [
        (4213, 974343, "m2"),
        (4213, 1060054, "blp"),
        (4213, 1029851, "blp"),
        (4203, 1625963, "m2"),
        (4203, 1626025, "blp"),
        (8570, 123465, "m2"),
        (13031, 147868, "blp"),
        (22132, 125876, "m2"),
    ] {
        let receipt = assets
            .receipt(AssetProduct::Retail, fdid, kind)
            .unwrap_or_else(|error| panic!("display {display}: {error}"));
        let bytes = std::fs::read(data.join(&receipt.path)).unwrap();
        assert_eq!(
            bytes.len() as u64,
            receipt.bytes,
            "display {display} asset {fdid}"
        );
    }
}

#[test]
fn ordinary_display_in_coverage_has_no_appearance() {
    assert_eq!(query_authored_npc_appearance(&fixture(), 99).unwrap(), None);
}

#[test]
fn required_display_without_profile_is_an_error() {
    let error = query_authored_npc_appearance(&fixture(), 42).unwrap_err();
    assert!(error.contains("42"), "{error}");
    assert!(error.contains("required appearance"), "{error}");
}

#[test]
fn display_outside_coverage_is_an_error() {
    let error = query_authored_npc_appearance(&fixture(), 999).unwrap_err();
    assert!(error.contains("999"), "{error}");
    assert!(error.contains("coverage"), "{error}");
}

#[test]
fn appearance_without_choices_or_geosets_retains_empty_lists() {
    let connection = fixture();
    connection
        .execute_batch("INSERT INTO appearances VALUES (42, 2, 0, 3, 4294967295);")
        .unwrap();
    let appearance = query_authored_npc_appearance(&connection, 42)
        .unwrap()
        .unwrap();
    assert_eq!(appearance.baked_texture_fdid, Some(u32::MAX));
    assert!(appearance.choice_ids.is_empty());
    assert!(appearance.geosets.is_empty());
}

#[test]
fn missing_schema_returns_error_with_display_context() {
    let connection = Connection::open_in_memory().unwrap();
    let error = query_authored_npc_appearance(&connection, 19177).unwrap_err();
    assert!(error.contains("19177"), "{error}");
    for table in ["display_coverage", "choices", "geosets"] {
        let connection = fixture();
        connection
            .execute_batch(&format!("DROP TABLE {table};"))
            .unwrap();
        let error = query_authored_npc_appearance(&connection, 19177).unwrap_err();
        assert!(error.contains("19177"), "{error}");
    }
}

#[test]
fn invalid_numeric_values_return_errors_without_partial_appearances() {
    for update in [
        "UPDATE appearances SET race = -1 WHERE display_id = 19177",
        "UPDATE appearances SET sex = 256 WHERE display_id = 19177",
        "UPDATE appearances SET class = 256 WHERE display_id = 19177",
        "UPDATE appearances SET baked_texture_fdid = -1 WHERE display_id = 19177",
        "UPDATE appearances SET baked_texture_fdid = 4294967296 WHERE display_id = 19177",
        "UPDATE choices SET choice_id = -1 WHERE display_id = 19177",
        "UPDATE choices SET choice_id = 4294967296 WHERE display_id = 19177",
        "UPDATE geosets SET geoset_index = 65536 WHERE display_id = 19177",
        "UPDATE geosets SET geoset_value = -1 WHERE display_id = 19177",
        "UPDATE appearances SET race = 'invalid' WHERE display_id = 19177",
    ] {
        let connection = fixture();
        connection.execute_batch(update).unwrap();
        let error = query_authored_npc_appearance(&connection, 19177).unwrap_err();
        assert!(error.contains("19177"), "{update}: {error}");
    }
}
