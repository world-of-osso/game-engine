use crate::housing::{HousingAuthority, HousingDashboard, HousingSurface};
use game_engine_core::housing::{Category, Decor, HousingCatalog, Subcategory};
use std::sync::Arc;

#[test]
fn housing_navigation_requests_authored_neighborhoods_without_claiming_property() {
    let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
    assert_eq!(
        model.navigation_request("housing:finder").unwrap(),
        Some(shared::protocol::HousingCommand::ListPlots { neighborhood_id: 1 })
    );
    assert_eq!(model.surface, HousingSurface::Finder);
    assert_eq!(
        model.navigation_request("housing:neighborhood:2").unwrap(),
        Some(shared::protocol::HousingCommand::ListPlots { neighborhood_id: 2 })
    );
    assert_eq!(
        model.navigation_request("housing:visit").unwrap(),
        Some(shared::protocol::HousingCommand::EnterNeighborhood { neighborhood_id: 2 })
    );
    assert!(model.navigation_request("housing:neighborhood:99").is_err());
    assert_eq!(model.neighborhood, 2);
    assert_eq!(model.navigation_request("housing:catalog").unwrap(), None);
}

#[test]
fn housing_storage_stays_unknown_until_a_qualified_snapshot_and_clears_on_logout() {
    use shared::protocol::{HousingReply, HousingState, StoredDecor};
    let mut authority = HousingAuthority::default();
    assert_eq!(authority.quantity(1693), None);
    let state = HousingState {
        build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
        stored: vec![StoredDecor {
            decor_id: 1693,
            quantity: 1,
        }],
        ..Default::default()
    };
    authority
        .receive(HousingReply {
            request_id: 1,
            state,
            error: None,
        })
        .unwrap();
    assert_eq!(authority.quantity(1693), Some(1));
    assert_eq!(authority.quantity(80), Some(0));
    authority.clear();
    assert_eq!(authority.quantity(1693), None);
    let failed = HousingReply {
        request_id: 2,
        state: HousingState::default(),
        error: Some("Housing catalog unavailable.".into()),
    };
    assert!(authority.receive(failed).is_err());
    assert_eq!(authority.quantity(1693), None);
    assert_eq!(
        authority.error.as_deref(),
        Some("Housing catalog unavailable.")
    );
}

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
fn housing_all_category_is_the_unfiltered_retail_category() {
    let mut model = dashboard();
    model.set_category(Some(18));
    assert_eq!(model.matching().len(), 16);
    assert_eq!(model.selected().unwrap().id, 80);
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
