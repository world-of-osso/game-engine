use crate::housing::{CollectionFilter, HousingAuthority, HousingDashboard, HousingSurface};
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
fn housing_home_navigation_requests_house_identity_without_component_grants() {
    let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
    assert_eq!(
        model.navigation_request("housing:home:41").unwrap(),
        Some(shared::protocol::HousingCommand::EnterHouse { house_id: 41 })
    );
    assert_eq!(
        model.navigation_request("housing:leave").unwrap(),
        Some(shared::protocol::HousingCommand::LeaveHouse)
    );
    assert!(model.navigation_request("housing:home:invalid").is_err());
    assert_eq!(
        model.navigation_request("housing:access:41:on").unwrap(),
        Some(shared::protocol::HousingCommand::SetHouseAccessAnyone {
            house_id: 41,
            allowed: true
        })
    );
    assert_eq!(
        model.navigation_request("housing:access:41:off").unwrap(),
        Some(shared::protocol::HousingCommand::SetHouseAccessAnyone {
            house_id: 41,
            allowed: false
        })
    );
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
                sources: Vec::new(),
                dyeable: id == 81,
            })
            .collect(),
        categories: vec![
            Category {
                id: 1,
                name: "Furnishings".into(),
                order: 2,
                icon_element: 29134,
                icon: Some("category-icons_furnishings".into()),
            },
            Category {
                id: 2,
                name: "Structural".into(),
                order: 4,
                icon_element: 29188,
                icon: Some("category-icons_structural".into()),
            },
        ],
        subcategories: vec![
            Subcategory {
                id: 21,
                category_id: 1,
                name: "Fireplaces".into(),
                order: 0,
                icon_element: 30799,
                icon: Some("category-icons_misc".into()),
            },
            Subcategory {
                id: 1,
                category_id: 2,
                name: "Seating".into(),
                order: 1,
                icon_element: 29176,
                icon: Some("category-icons_seating".into()),
            },
        ],
        encrypted_records: 54,
        missing_tables: vec!["NeighborhoodInitiativeTask".into()],
        sources_imported: false,
    };
    HousingDashboard::new(Arc::new(catalog))
}

#[test]
fn housing_search_category_and_selection_follow_catalog_not_starting_quantity() {
    let mut model = dashboard();
    let none = HousingAuthority::default();
    assert_eq!(model.visible(&none).len(), 12);
    assert_eq!(model.selected(&none).unwrap().id, 80);
    model.set_search("ORNATE".into(), &none);
    assert_eq!(model.matching(&none).len(), 1);
    assert_eq!(model.visible(&none)[0].name, "Ornate Stonework Fireplace");
    model.set_category(Some(2), &none);
    assert!(model.visible(&none).is_empty());
    assert!(model.selected(&none).is_none());
    model.set_search(String::new(), &none);
    assert_eq!(model.matching(&none).len(), 15);
    assert!(!model.select(80, &none));
    assert!(model.select(83, &none));
    assert_eq!(model.selected(&none).unwrap().id, 83);
    model.set_subcategory(Some(21), &none);
    assert!(model.visible(&none).is_empty());
}

#[test]
fn housing_all_category_is_the_unfiltered_retail_category() {
    let mut model = dashboard();
    let none = HousingAuthority::default();
    model.set_category(Some(18), &none);
    assert_eq!(model.matching(&none).len(), 16);
    assert_eq!(model.selected(&none).unwrap().id, 80);
}

#[test]
fn housing_scroll_reaches_second_row_and_search_resets_scroll_without_stale_preview() {
    let mut model = dashboard();
    let none = HousingAuthority::default();
    assert_eq!(model.visible(&none)[0].id, 80);
    model.scroll(1, &none);
    assert_eq!(model.visible(&none)[0].id, 83);
    model.scroll(100, &none);
    assert_eq!(model.visible(&none).last().unwrap().id, 95);
    model.set_search("Ornate".into(), &none);
    assert_eq!(model.offset, 0);
    assert_eq!(model.selected(&none).unwrap().id, 80);
    model.set_search("No matching decor".into(), &none);
    assert!(model.selected(&none).is_none());
    model.scroll(-100, &none);
    assert_eq!(model.offset, 0);
}

fn owning(stored: &[(u32, u32)], placed: &[(u32, u32)]) -> HousingAuthority {
    use shared::protocol::{HousingReply, HousingState, StoredDecor};
    let decor = |rows: &[(u32, u32)]| {
        rows.iter()
            .map(|&(decor_id, quantity)| StoredDecor { decor_id, quantity })
            .collect()
    };
    let mut authority = HousingAuthority::default();
    authority
        .receive(HousingReply {
            request_id: 1,
            state: HousingState {
                build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                stored: decor(stored),
                placed: decor(placed),
                ..Default::default()
            },
            error: None,
        })
        .unwrap();
    authority
}

#[test]
fn housing_collection_filter_counts_placed_and_stored_decor_as_owned() {
    let mut model = dashboard();
    // Decor 80 stored, 83 only placed in a house, the rest never acquired.
    let authority = owning(&[(80, 1)], &[(83, 2)]);
    assert_eq!(authority.owned(80), Some(1));
    assert_eq!(authority.owned(83), Some(2));
    assert_eq!(authority.owned(84), Some(0));
    model.set_collection(CollectionFilter::Collected, &authority);
    let ids = |model: &HousingDashboard, authority| {
        model
            .matching(authority)
            .iter()
            .map(|decor| decor.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&model, &authority), vec![80, 83]);
    assert_eq!(model.selected(&authority).unwrap().id, 80);
    model.set_collection(CollectionFilter::Uncollected, &authority);
    assert_eq!(model.matching(&authority).len(), 14);
    assert!(!model.select(83, &authority));
    // Ownership the server has not confirmed is neither collected nor uncollected.
    let unknown = HousingAuthority::default();
    assert!(model.matching(&unknown).is_empty());
    model.set_collection(CollectionFilter::All, &unknown);
    assert_eq!(model.matching(&unknown).len(), 16);
    model.toggle_dyeable(&unknown);
    assert_eq!(ids(&model, &unknown), vec![81]);
}

#[test]
fn housing_category_with_several_subcategories_expands_like_retail() {
    let mut model = dashboard();
    let none = HousingAuthority::default();
    assert!(model.expanded_category().is_none());
    model.set_category(Some(1), &none);
    // Furnishings has one subcategory here: Retail keeps the top-level column.
    assert!(model.expanded_category().is_none());
    let mut catalog = (*model.catalog).clone();
    catalog.subcategories[1].category_id = 1;
    let mut model = HousingDashboard::new(Arc::new(catalog));
    model.set_category(Some(1), &none);
    assert_eq!(model.expanded_category().unwrap().name, "Furnishings");
    model.set_category(Some(crate::housing::FEATURED_CATEGORY), &none);
    assert!(model.expanded_category().is_none());
}

#[test]
fn housing_source_lines_name_vendors_only_through_the_creature_query() {
    use game_engine_core::housing::DecorSource;
    let mut decor = dashboard().catalog.decor[0].clone();
    // Local Retail CollectableSource rows: Pawdo (252312) in Stormwind City and
    // Orgrimmar; an unnamed Dornogal vendor; Mythic Chrome King Gallywix.
    decor.sources = vec![
        DecorSource::Vendor {
            creature_id: 252312,
            area: Some("Stormwind City".into()),
        },
        DecorSource::Vendor {
            creature_id: 252312,
            area: Some("Orgrimmar".into()),
        },
        DecorSource::Vendor {
            creature_id: 255222,
            area: Some("Dornogal".into()),
        },
        DecorSource::Encounter {
            encounter: "Chrome King Gallywix".into(),
            instance: "Liberation of Undermine".into(),
            difficulty: Some("Mythic".into()),
        },
    ];
    let mut names = std::collections::HashMap::new();
    assert_eq!(
        crate::housing::source_lines(&decor, &names),
        vec!["Boss Drop: Chrome King Gallywix in Liberation of Undermine (Mythic)"]
    );
    names.insert(252312, "Second Chair Pawdo".to_string());
    assert_eq!(
        crate::housing::source_lines(&decor, &names),
        vec![
            "Vendor: Second Chair Pawdo in Stormwind City",
            "Vendor: Second Chair Pawdo in Orgrimmar",
            "Boss Drop: Chrome King Gallywix in Liberation of Undermine (Mythic)",
        ]
    );
}
