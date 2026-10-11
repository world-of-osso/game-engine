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
            json!({"ID":"80", "Name_lang":"Ornate Stonework Fireplace", "ItemID":"235994", "ModelType":"1", "ModelFileDataID":"6033663", "ThumbnailFileDataID":"7415905", "StartingQuantity":"1", "OrderIndex":"1"}),
        ),
        (
            "HouseDecor",
            81,
            json!({"ID":"81", "Name_lang":"Tusked Fireplace", "ItemID":"245398", "ModelType":"1", "ModelFileDataID":"6033664", "ThumbnailFileDataID":"7415913", "StartingQuantity":"1", "OrderIndex":"2"}),
        ),
        (
            "DecorCategory",
            1,
            json!({"ID":"1", "Name_lang":"Furnishings", "OrderIndex":"2"}),
        ),
        (
            "DecorSubcategory",
            21,
            json!({"ID":"21", "Name_lang":"Fireplaces", "DecorCategoryID":"1", "OrderIndex":"0"}),
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
