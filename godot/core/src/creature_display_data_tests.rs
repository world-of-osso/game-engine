use crate::creature_display_data::{CreatureDisplay, query_display, query_preferred_skins};
use rusqlite::Connection;

#[test]
fn query_display_reads_model_all_skins_and_scale() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE creature_displays (
            display_id INTEGER PRIMARY KEY,
            model_fdid INTEGER NOT NULL,
            skin_fdid_0 INTEGER NOT NULL,
            skin_fdid_1 INTEGER NOT NULL,
            skin_fdid_2 INTEGER NOT NULL,
            skin_fdid_3 INTEGER NOT NULL,
            scale_milli INTEGER NOT NULL
        );
        INSERT INTO creature_displays VALUES (42, 1234, 567, 890, 123, 456, 1750);",
    )
    .unwrap();

    assert_eq!(
        query_display(&conn, 42).unwrap(),
        Some(CreatureDisplay {
            model_fdid: 1234,
            skin_fdids: [567, 890, 123, 456],
            scale_milli: 1750,
        })
    );
    assert_eq!(query_display(&conn, 43).unwrap(), None);
}

#[test]
fn query_display_reports_missing_table() {
    let conn = Connection::open_in_memory().unwrap();
    assert!(matches!(
        query_display(&conn, 42),
        Err(rusqlite::Error::SqliteFailure(_, _))
    ));
}

#[test]
fn query_preferred_skins_reads_the_model_row() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE preferred_skins (
            model_fdid INTEGER PRIMARY KEY,
            skin_fdid_0 INTEGER NOT NULL,
            skin_fdid_1 INTEGER NOT NULL,
            skin_fdid_2 INTEGER NOT NULL,
            skin_fdid_3 INTEGER NOT NULL
        );
        INSERT INTO preferred_skins VALUES (126487, 126494, 126495, 0, 4240636);",
    )
    .unwrap();

    assert_eq!(
        query_preferred_skins(&conn, 126487).unwrap(),
        Some([126494, 126495, 0, 4240636])
    );
    assert_eq!(query_preferred_skins(&conn, 1).unwrap(), None);
}
