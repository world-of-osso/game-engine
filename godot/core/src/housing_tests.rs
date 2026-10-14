use crate::housing::{HousingCatalog, load_connection};
use rusqlite::Connection;
use serde_json::json;

fn catalog_connection() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE housing_catalog_build (build_key TEXT, build TEXT);
         INSERT INTO housing_catalog_build VALUES ('dcfc90fffd79ba00406ae46f5f657592', '12.1.0.69933');
         CREATE TABLE housing_catalog_source (build_key TEXT, table_name TEXT, status TEXT, encrypted_records_dropped INTEGER);
         INSERT INTO housing_catalog_source VALUES ('dcfc90fffd79ba00406ae46f5f657592', 'HouseDecor', 'exported', 20);
         CREATE TABLE housing_catalog_record (build_key TEXT, table_name TEXT, record_id INTEGER, fields_json TEXT);",
    ).unwrap();
    let key = "dcfc90fffd79ba00406ae46f5f657592";
    for (table, id, fields) in [
        (
            "HouseDecor",
            80,
            json!({"ID":"80", "Name_lang":"Ornate Stonework Fireplace", "ItemID":"235994", "ModelType":"1", "ModelFileDataID":"6033663", "ThumbnailFileDataID":"7415905", "StartingQuantity":"1", "OrderIndex":"1", "InitialRotation_0":"0.0", "InitialRotation_1":"0.0", "InitialRotation_2":"0.0", "InitialScale":"1.0"}),
        ),
        (
            "HouseDecor",
            81,
            json!({"ID":"81", "Name_lang":"Tusked Fireplace", "ItemID":"245398", "ModelType":"1", "ModelFileDataID":"6033664", "ThumbnailFileDataID":"7415913", "StartingQuantity":"1", "OrderIndex":"2", "InitialRotation_0":"0.0", "InitialRotation_1":"0.0", "InitialRotation_2":"0.0", "InitialScale":"1.0"}),
        ),
        (
            "DecorCategory",
            1,
            json!({"ID":"1", "Name_lang":"Furnishings", "OrderIndex":"2", "UiTextureAtlasElementID":"29134"}),
        ),
        (
            "DecorSubcategory",
            21,
            json!({"ID":"21", "Name_lang":"Fireplaces", "DecorCategoryID":"1", "OrderIndex":"0", "UiTextureAtlasElementID":"33476"}),
        ),
        (
            "DecorXDecorSubcategory",
            29,
            json!({"ID":"29", "HouseDecorID":"80", "DecorSubcategoryID":"21"}),
        ),
        (
            "DecorXDecorSubcategory",
            45,
            json!({"ID":"45", "HouseDecorID":"81", "DecorSubcategoryID":"21"}),
        ),
    ] {
        db.execute(
            "INSERT INTO housing_catalog_record VALUES (?, ?, ?, ?)",
            rusqlite::params![key, table, id, fields.to_string()],
        )
        .unwrap();
    }
    db
}

#[test]
fn housing_reads_qualified_names_categories_and_authored_models_without_ownership() {
    let db = catalog_connection();
    let catalog: HousingCatalog =
        load_connection(&db, "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592").unwrap();
    assert_eq!(catalog.decor.len(), 2);
    let fireplace = &catalog.decor[0];
    assert_eq!(fireplace.id, 80);
    assert_eq!(fireplace.name, "Ornate Stonework Fireplace");
    assert_eq!(fireplace.model_fdid, 6033663);
    assert_eq!(fireplace.thumbnail_fdid, 7415905);
    assert_eq!(fireplace.subcategories, vec![21]);
    assert_eq!(fireplace.initial_rotation, [0.0; 3]);
    assert_eq!(fireplace.initial_scale, 1.0);
    assert_eq!(catalog.categories[0].name, "Furnishings");
    assert_eq!(catalog.subcategories[0].name, "Fireplaces");
    assert_eq!(catalog.encrypted_records, 20);
}

#[test]
fn housing_refuses_different_build_and_malformed_authoritative_catalog() {
    let db = catalog_connection();
    assert!(load_connection(&db, "12.1.0.70338", "dcfc90fffd79ba00406ae46f5f657592").is_err());
    db.execute("UPDATE housing_catalog_record SET fields_json=? WHERE table_name='HouseDecor' AND record_id=80", [json!({"ID":"80", "Name_lang":"Fireplace", "ModelFileDataID":"not an ID"}).to_string()]).unwrap();
    assert!(load_connection(&db, "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592").is_err());
}

fn insert(db: &Connection, table: &str, id: u32, fields: serde_json::Value) {
    db.execute(
        "INSERT INTO housing_catalog_record VALUES ('dcfc90fffd79ba00406ae46f5f657592', ?, ?, ?)",
        rusqlite::params![table, id, fields.to_string()],
    )
    .unwrap();
}

fn import_sources(db: &Connection) {
    for table in [
        "CollectableSourceInfo",
        "CollectableSourceVendorSparse",
        "CollectableSourceEncounterSparse",
        "AreaTable",
        "JournalEncounter",
        "JournalInstance",
        "Difficulty",
    ] {
        db.execute(
            "INSERT INTO housing_catalog_source VALUES ('dcfc90fffd79ba00406ae46f5f657592', ?, 'exported', 0)",
            [table],
        )
        .unwrap();
    }
    // Local Retail 12.1.0.69933 rows: Pawdo sells decor 80 in Stormwind and
    // Orgrimmar (the same pair twice through a second source-info row) and decor 81
    // drops from Mythic Gallywix.
    insert(
        db,
        "CollectableSourceInfo",
        175962,
        json!({"ID":"175962", "SourceTypeEnum":"3", "HouseDecorID":"80", "Description":"House Decor:: (80)", "ItemModifiedAppearanceID":"0"}),
    );
    insert(
        db,
        "CollectableSourceInfo",
        175963,
        json!({"ID":"175963", "SourceTypeEnum":"3", "HouseDecorID":"80", "Description":"House Decor:: (80)", "ItemModifiedAppearanceID":"0"}),
    );
    insert(
        db,
        "CollectableSourceInfo",
        175628,
        json!({"ID":"175628", "SourceTypeEnum":"1", "HouseDecorID":"81", "Description":"House Decor: (81)", "ItemModifiedAppearanceID":"0"}),
    );
    for (id, info, area) in [
        (239602, 175962, 1519),
        (239603, 175962, 1637),
        (239604, 175963, 1519),
    ] {
        insert(
            db,
            "CollectableSourceVendorSparse",
            id,
            json!({"ID": id.to_string(), "CreatureID":"252312", "AreaTableID": area.to_string(), "VendorMapID":"0", "CollectableSourceInfoID": info.to_string()}),
        );
    }
    insert(
        db,
        "CollectableSourceEncounterSparse",
        2507,
        json!({"ID":"2507", "JournalEncounterID":"2646", "DifficultyID":"16", "CollectableSourceInfoID":"175628"}),
    );
    insert(
        db,
        "AreaTable",
        1519,
        json!({"ID":"1519", "AreaName_lang":"Stormwind City"}),
    );
    insert(
        db,
        "AreaTable",
        1637,
        json!({"ID":"1637", "AreaName_lang":"Orgrimmar"}),
    );
    insert(
        db,
        "JournalEncounter",
        2646,
        json!({"ID":"2646", "Name_lang":"Chrome King Gallywix", "JournalInstanceID":"1296"}),
    );
    insert(
        db,
        "JournalInstance",
        1296,
        json!({"ID":"1296", "Name_lang":"Liberation of Undermine"}),
    );
    insert(
        db,
        "Difficulty",
        16,
        json!({"ID":"16", "Name_lang":"Mythic"}),
    );
}

#[test]
fn housing_joins_authored_collection_sources_without_inventing_vendor_names() {
    use crate::housing::DecorSource;
    let db = catalog_connection();
    let catalog = load_connection(&db, "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592").unwrap();
    assert!(!catalog.sources_imported);
    assert!(catalog.decor.iter().all(|decor| decor.sources.is_empty()));
    import_sources(&db);
    // Display order differs from record order: joins must follow decor identity.
    db.execute(
        "UPDATE housing_catalog_record SET fields_json=? WHERE table_name='HouseDecor' AND record_id=80",
        [json!({"ID":"80", "Name_lang":"Ornate Stonework Fireplace", "ItemID":"235994", "ModelType":"1", "ModelFileDataID":"6033663", "ThumbnailFileDataID":"7415905", "StartingQuantity":"1", "OrderIndex":"3", "InitialRotation_0":"0.0", "InitialRotation_1":"0.0", "InitialRotation_2":"0.0", "InitialScale":"1.0"}).to_string()],
    )
    .unwrap();
    let catalog = load_connection(&db, "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592").unwrap();
    let decor = |id| catalog.decor.iter().find(|decor| decor.id == id).unwrap();
    assert_eq!(catalog.decor[0].id, 81);
    assert!(catalog.sources_imported);
    let vendor = |area: &str| DecorSource::Vendor {
        creature_id: 252312,
        area: Some(area.into()),
    };
    assert_eq!(
        decor(80).sources,
        vec![vendor("Stormwind City"), vendor("Orgrimmar")]
    );
    assert_eq!(
        decor(81).sources,
        vec![DecorSource::Encounter {
            encounter: "Chrome King Gallywix".into(),
            instance: "Liberation of Undermine".into(),
            difficulty: Some("Mythic".into()),
        }]
    );
    // A source naming an area the catalog lacks is malformed, not an unnamed zone.
    db.execute(
        "DELETE FROM housing_catalog_record WHERE table_name='AreaTable' AND record_id=1637",
        [],
    )
    .unwrap();
    assert!(load_connection(&db, "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592").is_err());
}

#[test]
fn housing_names_category_icons_by_retail_atlas_key_and_marks_dyeable_decor() {
    let db = catalog_connection();
    insert(
        &db,
        "DecorDyeSlot",
        28,
        json!({"ID":"28", "HouseDecorID":"81", "Channel":"0", "DyeColorCategoryID":"2", "OrderIndex":"0"}),
    );
    let mut catalog =
        load_connection(&db, "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592").unwrap();
    assert_eq!(
        catalog
            .decor
            .iter()
            .map(|decor| (decor.id, decor.dyeable))
            .collect::<Vec<_>>(),
        vec![(80, false), (81, true)]
    );
    let elements =
        std::env::temp_dir().join(format!("housing-atlas-elements-{}.csv", std::process::id()));
    // Local Retail 12.1.0.69933 rows: the data names one state of each icon.
    std::fs::write(
        &elements,
        "Name,ID,Flags\ncategory-icons_furnishings_active,29134,0\ncategory-icons_vines_inactive,33476,0\n",
    )
    .unwrap();
    catalog.resolve_icons(&elements).unwrap();
    assert_eq!(
        catalog.categories[0].icon.as_deref(),
        Some("category-icons_furnishings")
    );
    assert_eq!(
        catalog.subcategories[0].icon.as_deref(),
        Some("category-icons_vines")
    );
    std::fs::write(
        &elements,
        "Name,ID,Flags\ncategory-icons_furnishings_active,29134,0\n",
    )
    .unwrap();
    assert!(catalog.resolve_icons(&elements).is_err());
    std::fs::remove_file(&elements).unwrap();
}

#[test]
fn housing_names_neighborhood_art_by_its_authored_texture_kit() {
    let db = catalog_connection();
    // NeighborhoodMap 1 Founder's Point (map 2735) and 7 Home Interior (no kit).
    insert(
        &db,
        "NeighborhoodMap",
        1,
        json!({"ID":"1", "MapID":"2735", "UiTextureKitID":"5721"}),
    );
    insert(
        &db,
        "NeighborhoodMap",
        7,
        json!({"ID":"7", "MapID":"2783", "UiTextureKitID":"0"}),
    );
    let mut catalog =
        load_connection(&db, "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592").unwrap();
    assert_eq!(catalog.neighborhood_texture(1), None);
    let kits =
        std::env::temp_dir().join(format!("housing-texture-kits-{}.csv", std::process::id()));
    std::fs::write(&kits, "ID,KitPrefix\n5720,durotar\n5721,elwynn\n").unwrap();
    catalog.resolve_texture_kits(&kits).unwrap();
    assert_eq!(catalog.neighborhood_texture(1), Some("elwynn"));
    assert_eq!(catalog.neighborhood_texture(7), None);
    assert_eq!(catalog.neighborhood_texture(2), None);
    std::fs::write(&kits, "ID,KitPrefix\n5720,durotar\n").unwrap();
    assert!(
        catalog
            .resolve_texture_kits(&kits)
            .unwrap_err()
            .contains("5721")
    );
    std::fs::remove_file(&kits).unwrap();
}

#[test]
fn db2_header_housing_rejects_invalid_canonical_atlas_id() {
    let db = catalog_connection();
    let mut catalog =
        load_connection(&db, "12.1.0.69933", "dcfc90fffd79ba00406ae46f5f657592").unwrap();
    let fixture = crate::db2_test_support::ExportFixture::copy("UiTextureAtlasElement", Some("ID"));
    let error = catalog.resolve_icons(&fixture.0).unwrap_err();
    assert!(error.contains("UiTextureAtlasElement.csv:2:"), "{error}");
}
