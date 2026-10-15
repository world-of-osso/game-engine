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
            scale_milli INTEGER NOT NULL,
            source_product TEXT NOT NULL,
            pet_instance_scale_milli INTEGER NOT NULL,
            uses_display_geosets INTEGER NOT NULL,
            model_alpha INTEGER NOT NULL
        );
        INSERT INTO creature_displays VALUES (42, 1234, 567, 890, 123, 456, 1750, 'wow', 700, 0, 175);",
    )
    .unwrap();

    assert_eq!(
        query_display(&conn, 42).unwrap(),
        Some(CreatureDisplay {
            source_product: crate::asset_product::AssetProduct::Retail,
            model_fdid: 1234,
            model_alpha: 175,
            uses_display_geosets: false,
            skin_fdids: [567, 890, 123, 456],
            scale_milli: 1750,
            pet_instance_scale_milli: 700,
        })
    );
    assert_eq!(query_display(&conn, 43).unwrap(), None);
}

#[test]
fn ordinary_display_geoset_selection_uses_one_based_mesh_groups() {
    use crate::creature_display_data::display_geoset_visible;
    // Buzzard1105: CMD10253/FDID1661349, GeosetData343, source index0/value1.
    let parts = [0, 101, 102];
    let visible: Vec<_> = parts
        .into_iter()
        .filter(|part| display_geoset_visible(*part, true, &[(0, 1)]))
        .collect();
    assert_eq!(visible, [0, 101]);
    // Boar1967: CMD10605/FDID1733750, source index0/value2; group2 omitted.
    let visible: Vec<_> = [0, 101, 102, 103, 201]
        .into_iter()
        .filter(|part| display_geoset_visible(*part, true, &[(0, 2)]))
        .collect();
    assert_eq!(visible, [0, 102]);
    // A declared empty selection leaves the base, not every variant.
    assert!(!display_geoset_visible(101, true, &[]));
    assert!(display_geoset_visible(0, true, &[]));
    assert!(!display_geoset_visible(0, false, &[]));
    assert!(display_geoset_visible(901, true, &[]));
    assert!(!display_geoset_visible(901, true, &[(8, 2)]));
    assert!(display_geoset_visible(902, true, &[(8, 2)]));
    assert!(display_geoset_visible(100, true, &[(0, 0)]));
    assert!(!display_geoset_visible(101, true, &[(0, 0)]));
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
