use crate::creature_display_data::{CreatureDisplay, query_display};
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
            scale_milli INTEGER NOT NULL
        );
        INSERT INTO creature_displays VALUES (42, 1234, 567, 890, 123, 1750);",
    )
    .unwrap();

    assert_eq!(
        query_display(&conn, 42).unwrap(),
        Some(CreatureDisplay {
            model_fdid: 1234,
            skin_fdids: [567, 890, 123],
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
