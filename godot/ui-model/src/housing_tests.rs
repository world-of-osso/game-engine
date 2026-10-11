use crate::housing::HousingDashboard;
use game_engine_core::housing::{Category, Decor, HousingCatalog, Subcategory};
use std::sync::Arc;

fn dashboard() -> HousingDashboard {
    let catalog = HousingCatalog {
        decor: (80..96)
            .map(|id| Decor {
                id,
                name: if id == 80 {
                    "Ornate Stonework Fireplace".into()
                } else {
                    format!("Tusked Fireplace {id}")
                },
                item_id: 235994,
                model_type: 1,
                model_fdid: 6033663,
                thumbnail_fdid: 7415905,
                order: id as i32,
                subcategories: if id == 80 { vec![21] } else { vec![1] },
            })
            .collect(),
        categories: vec![
            Category {
                id: 1,
                name: "Furnishings".into(),
                order: 2,
            },
            Category {
                id: 2,
                name: "Structural".into(),
                order: 4,
            },
        ],
        subcategories: vec![
            Subcategory {
                id: 21,
                category_id: 1,
                name: "Fireplaces".into(),
                order: 0,
            },
            Subcategory {
                id: 1,
                category_id: 2,
                name: "Seating".into(),
                order: 1,
            },
        ],
        encrypted_records: 54,
        missing_tables: vec!["NeighborhoodInitiativeTask".into()],
    };
    HousingDashboard::new(Arc::new(catalog))
}

#[test]
fn housing_search_category_and_selection_follow_catalog_not_starting_quantity() {
    let mut model = dashboard();
    assert_eq!(model.visible().len(), 12);
    assert_eq!(model.selected().unwrap().id, 80);
    model.set_search("ORNATE".into());
    assert_eq!(model.matching().len(), 1);
    assert_eq!(model.visible()[0].name, "Ornate Stonework Fireplace");
    model.set_category(Some(2));
    assert!(model.visible().is_empty());
    assert!(model.selected().is_none());
    model.set_search(String::new());
    assert_eq!(model.matching().len(), 15);
    assert!(!model.select(80));
    assert!(model.select(83));
    assert_eq!(model.selected().unwrap().id, 83);
    model.set_subcategory(Some(21));
    assert!(model.visible().is_empty());
}

#[test]
fn housing_scroll_reaches_second_row_and_search_resets_scroll_without_stale_preview() {
    let mut model = dashboard();
    assert_eq!(model.visible()[0].id, 80);
    model.scroll(1);
    assert_eq!(model.visible()[0].id, 83);
    model.scroll(100);
    assert_eq!(model.visible().last().unwrap().id, 95);
    model.set_search("Ornate".into());
    assert_eq!(model.offset, 0);
    assert_eq!(model.selected().unwrap().id, 80);
    model.set_search("No matching decor".into());
    assert!(model.selected().is_none());
    model.scroll(-100);
    assert_eq!(model.offset, 0);
}
