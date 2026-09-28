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
