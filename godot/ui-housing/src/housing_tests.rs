use crate::housing::{CollectionFilter, HousingAuthority, HousingDashboard, HousingSurface};
use game_engine_core::housing::{Category, Decor, HousingCatalog, Subcategory};
use std::sync::Arc;

#[test]
fn housing_storage_menu_hover_delay_cancels_and_preserves_retained_levels() {
    use crate::housing_storage::StorageMenuTarget::{RootRow, Sort};
    let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
    model
        .storage_filter_action("housing:storage:filters")
        .unwrap();
    model.advance_storage_menu_hover(Some(Sort), 0.32);
    assert!(!model.storage_sort_open);
    model.advance_storage_menu_hover(None, 1.0);
    assert!(!model.storage_sort_open);
    model.advance_storage_menu_hover(Some(Sort), 0.2);
    assert!(!model.storage_sort_open);
    model.advance_storage_menu_hover(Some(Sort), 0.14);
    assert!(model.storage_sort_open);
    model.advance_storage_menu_hover(None, 1.0);
    assert!(model.storage_filters_open && model.storage_sort_open);
    model.advance_storage_menu_hover(Some(RootRow), 0.2);
    assert!(model.storage_sort_open);
    model.advance_storage_menu_hover(Some(RootRow), 0.14);
    assert!(model.storage_filters_open && !model.storage_sort_open);
    model.close_storage_menus();
    model
        .storage_filter_action("housing:storage:filters")
        .unwrap();
    model.advance_storage_menu_hover(Some(Sort), 0.1);
    assert!(!model.storage_sort_open);
}

#[test]
fn housing_storage_menu_checkbox_refresh_and_radio_close_are_independent() {
    let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
    for action in [
        "housing:storage:filters",
        "housing:storage:sort",
        "housing:storage:sort",
    ] {
        model.storage_filter_action(action).unwrap();
    }
    assert!(model.storage_filters_open && model.storage_sort_open);
    model
        .storage_filter_action("housing:storage:filter:dyeable")
        .unwrap();
    assert!(model.storage_filters_open && !model.storage_sort_open && model.storage_dyeable_only);
    model.storage_filter_action("housing:storage:sort").unwrap();
    model
        .storage_filter_action("housing:storage:sort:alphabetical")
        .unwrap();
    assert!(!model.storage_filters_open && !model.storage_sort_open && model.storage_alphabetical);
    assert!(model.storage_dyeable_only);
    model
        .storage_filter_action("housing:storage:filters")
        .unwrap();
    assert!(model.storage_filters_open && !model.storage_sort_open);
}

#[test]
fn housing_storage_transitions_use_source_order_durations_and_out_exponential_offsets() {
    use crate::housing_storage::DrawerPart;
    let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
    model.storage_panel.resize([725.0, 730.0]).unwrap();
    assert_eq!(
        model.storage_drawer_presentation(),
        (DrawerPart::Panel, 0.0)
    );
    model.start_storage_transition(true);
    assert_eq!(
        model.storage_drawer_presentation(),
        (DrawerPart::Panel, 0.0)
    );
    model.advance_storage_transition(0.1).unwrap();
    let (part, left) = model.storage_drawer_presentation();
    assert_eq!(part, DrawerPart::Panel);
    assert!((left + 725.0 * 0.96875).abs() < 0.001);
    model.advance_storage_transition(0.1).unwrap();
    let (part, left) = model.storage_drawer_presentation();
    assert_eq!(part, DrawerPart::Handle);
    assert!((left + 64.0 * 0.67).abs() < 0.001);
    model.advance_storage_transition(0.075).unwrap();
    let (part, left) = model.storage_drawer_presentation();
    assert_eq!(part, DrawerPart::Handle);
    assert!((left + 64.0 * 0.67 / 32.0).abs() < 0.001);
    model.advance_storage_transition(0.075).unwrap();
    assert_eq!(
        model.storage_drawer_presentation(),
        (DrawerPart::Handle, 0.0)
    );
    assert!(model.storage_panel.collapsed);
    model.start_storage_transition(false);
    assert_eq!(
        model.storage_drawer_presentation(),
        (DrawerPart::Handle, 0.0)
    );
    model.advance_storage_transition(0.15).unwrap();
    let (part, left) = model.storage_drawer_presentation();
    assert_eq!(part, DrawerPart::Panel);
    assert!((left + 725.0).abs() < 0.001);
    model.advance_storage_transition(0.1).unwrap();
    let (part, left) = model.storage_drawer_presentation();
    assert_eq!(part, DrawerPart::Panel);
    assert!((left + 725.0 / 32.0).abs() < 0.001);
    model.advance_storage_transition(0.1).unwrap();
    assert_eq!(
        model.storage_drawer_presentation(),
        (DrawerPart::Panel, 0.0)
    );
    assert!(!model.storage_panel.collapsed);
    assert!(model.advance_storage_transition(f32::NAN).is_err());
    assert!(model.advance_storage_transition(-0.1).is_err());
    model.storage_panel.collapsed = true; // Cold mount has no transient timer.
    assert_eq!(
        model.storage_drawer_presentation(),
        (DrawerPart::Handle, 0.0)
    );
}

#[test]
fn housing_storage_filters_use_authentic_dye_slots_and_independent_alphabetical_results() {
    use shared::protocol::{HousingState, StoredDecor};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap();
    let mut model = HousingDashboard::new(Arc::new(catalog));
    let authority = HousingAuthority {
        state: Some(HousingState {
            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
            stored: [1693, 374, 2330]
                .into_iter()
                .map(|decor_id| StoredDecor {
                    decor_id,
                    quantity: 1,
                })
                .collect(),
            ..Default::default()
        }),
        error: None,
    };
    let original = authority.clone();
    let ids = |model: &HousingDashboard| {
        model
            .stored_entries(&authority)
            .iter()
            .map(|(_, decor)| decor.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&model), vec![1693, 374, 2330]);
    assert!(
        model
            .storage_filter_action("housing:storage:sort:alphabetical")
            .unwrap()
    );
    assert_eq!(ids(&model), vec![2330, 1693, 374]);
    model
        .storage_filter_action("housing:storage:filter:dyeable")
        .unwrap();
    assert_eq!(ids(&model), vec![374]);
    assert!(!model.dyeable_only);
    model.set_storage_search("Tesla".into());
    assert!(ids(&model).is_empty());
    model
        .storage_filter_action("housing:storage:filter:reset")
        .unwrap();
    assert_eq!(model.storage_search, "Tesla");
    assert_eq!(ids(&model), vec![2330]);
    model.set_storage_search(String::new());
    assert_eq!(ids(&model), vec![2330, 1693, 374]); // Reset does not change the source sort selection.
    model
        .storage_filter_action("housing:storage:filter:dyeable")
        .unwrap();
    model
        .editor
        .set_mode(crate::housing_editor::EditorMode::Cleanup);
    model
        .editor
        .set_mode(crate::housing_editor::EditorMode::Basic);
    assert_eq!(ids(&model), vec![374]);
    assert!(
        model
            .storage_filter_action("housing:storage:sort:date-added")
            .is_err()
    );
    assert!(
        model
            .storage_filter_action("housing:storage:filter:collected")
            .is_err()
    );
    assert!(
        !model
            .storage_filter_action("housing:filter:dyeable")
            .unwrap()
    );
    assert!(
        model
            .stored_entries(&HousingAuthority::default())
            .is_empty()
    );
    assert_eq!(authority, original);
}

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
    assert!(
        model
            .settings_request(&HousingAuthority::default(), "housing:settings:open:41")
            .is_err()
    );
}

#[test]
fn housing_storage_same_focus_is_noop_all_keeps_query_and_changed_specific_focus_clears() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap();
    let mut model = HousingDashboard::new(Arc::new(catalog));
    let authority = owning(&[(1693, 1), (2330, 1)], &[]);
    let original = authority.clone();
    model.set_search("Catalog independent".into(), &authority);
    model.set_storage_category(Some(2), &authority).unwrap();
    model.set_storage_search("zz".into());
    model.storage_search_focused = true;
    model.set_storage_subcategory(None, &authority).unwrap();
    assert_eq!(
        model.storage_search, "zz",
        "same child-All focus must not invoke clear callback"
    );
    assert!(model.storage_search_focused);
    model.set_storage_category(Some(2), &authority).unwrap();
    assert_eq!(model.storage_search, "zz");
    model.set_storage_category(None, &authority).unwrap();
    assert_eq!(model.storage_search, "zz");
    assert!(model.storage_search_focused);
    model.set_storage_category(Some(1), &authority).unwrap();
    assert!(model.storage_search.is_empty());
    assert!(!model.storage_search_focused);
    assert_eq!(model.search, "Catalog independent");
    assert_eq!(authority, original);
}

#[test]
fn housing_storage_short_utf8_search_keeps_category_focus_until_three_characters() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap();
    let mut model = HousingDashboard::new(Arc::new(catalog));
    let authority = owning(&[(1693, 1), (2330, 1)], &[]);
    let original = authority.clone();
    model.set_search("Tesla".into(), &authority);
    model.storage_alphabetical = true;
    let ids = |model: &HousingDashboard| {
        model
            .stored_entries(&authority)
            .iter()
            .map(|(_, decor)| decor.id)
            .collect::<Vec<_>>()
    };
    for short in ["z", "zz", "éé", "🦀🦀"] {
        model.set_storage_category(Some(1), &authority).unwrap();
        model.set_storage_subcategory(Some(6), &authority).unwrap();
        model.set_storage_search(short.into());
        assert_eq!(model.storage_search, short);
        assert_eq!(
            model.storage_category,
            Some(1),
            "short UTF8 query must retain category: {short}"
        );
        assert_eq!(model.storage_subcategory, Some(6));
        assert_eq!(ids(&model), vec![1693]);
    }
    model.set_storage_search("zzz".into());
    assert_eq!(model.storage_category, None);
    assert_eq!(model.storage_subcategory, None);
    assert!(ids(&model).is_empty());
    model.set_storage_search("zz".into());
    assert_eq!(model.storage_category, None);
    assert_eq!(ids(&model), vec![2330, 1693]);
    model.set_storage_search("tEs".into());
    assert_eq!(ids(&model), vec![2330]);
    assert_eq!(model.search, "Tesla");
    assert!(model.storage_alphabetical);
    assert_eq!(authority, original);
}

#[test]
fn housing_storage_categories_use_real_ownership_and_keep_search_focus_independent() {
    use shared::protocol::{HousingState, StoredDecor};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap();
    let mut model = HousingDashboard::new(Arc::new(catalog));
    let authority = HousingAuthority {
        state: Some(HousingState {
            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
            stored: vec![
                StoredDecor {
                    decor_id: 1693,
                    quantity: 1,
                },
                StoredDecor {
                    decor_id: 2330,
                    quantity: 1,
                },
            ],
            placed: vec![StoredDecor {
                decor_id: 80,
                quantity: 1,
            }],
            ..Default::default()
        }),
        error: None,
    };
    let categories: Vec<_> = model
        .storage_categories(&authority)
        .iter()
        .map(|category| category.id)
        .collect();
    assert_eq!(categories, vec![18, 1, 2, 4]);
    let children: Vec<_> = model
        .storage_subcategories(&authority, 1)
        .iter()
        .map(|subcategory| subcategory.id)
        .collect();
    assert_eq!(children, vec![6, 7]); // Fireplace80 also belongs to Misc Furnishings7, even when only placed.
    let ids = |model: &HousingDashboard| {
        model
            .stored_entries(&authority)
            .iter()
            .map(|(_, decor)| decor.id)
            .collect::<Vec<_>>()
    };
    assert!(
        model
            .storage_focus_action("housing:storage:category:1", &authority)
            .unwrap()
    );
    assert_eq!(ids(&model), vec![1693]);
    model.set_storage_subcategory(Some(6), &authority).unwrap();
    assert_eq!(model.storage_subcategory, Some(6));
    assert!(model.set_storage_subcategory(Some(10), &authority).is_err());
    model.set_storage_search("Tesla".into());
    assert_eq!(model.storage_category, None);
    assert_eq!(model.storage_subcategory, None);
    assert_eq!(ids(&model), vec![2330]);
    model.set_storage_category(Some(2), &authority).unwrap();
    assert!(model.storage_search.is_empty());
    assert_eq!(ids(&model), vec![2330]);
    model
        .editor
        .set_mode(crate::housing_editor::EditorMode::Cleanup);
    model
        .editor
        .set_mode(crate::housing_editor::EditorMode::Basic);
    assert_eq!(model.storage_category, Some(2));
    model.set_storage_category(Some(4), &authority).unwrap();
    assert!(ids(&model).is_empty()); // Owned/placed fireplace is not stored.
    assert!(model.set_storage_category(Some(17), &authority).is_err());
    assert!(model.set_storage_category(Some(6), &authority).is_err());
    assert_eq!(model.storage_category, Some(4));
    model.set_storage_category(Some(18), &authority).unwrap();
    assert_eq!(ids(&model), vec![1693, 2330]);
    assert!(
        model
            .storage_focus_action("housing:storage:category:back", &authority)
            .unwrap()
    );
    assert!(
        !model
            .storage_focus_action("housing:category:1", &authority)
            .unwrap()
    );
    assert!(
        model
            .storage_focus_action("housing:storage:category:invalid", &authority)
            .is_err()
    );
    assert_eq!(model.category, None);
    assert!(model.search.is_empty());
    assert!(
        model
            .stored_entries(&HousingAuthority::default())
            .is_empty()
    );
}

#[test]
fn housing_owned_storage_search_uses_positive_confirmed_counts_and_keeps_catalog_search_separate() {
    use shared::protocol::{HousingState, StoredDecor};
    let mut catalog = HousingCatalog::default();
    for (id, name, item_id, model_fdid, thumbnail_fdid) in [
        (1693, "Small Val'sharah Bookcase", 245259, 1096761, 7421877),
        (2330, "Gnomish Tesla Coil", 246487, 6699745, 7422633),
        (80, "Ornate Stonework Fireplace", 235994, 6033663, 7415905),
    ] {
        catalog.decor.push(Decor {
            id,
            name: name.into(),
            item_id,
            model_type: 1,
            model_fdid,
            initial_rotation: [0.0; 3],
            initial_scale: 1.0,
            thumbnail_fdid,
            order: 0,
            subcategories: vec![],
            sources: vec![],
            dyeable: false,
        });
    }
    let mut model = HousingDashboard::new(Arc::new(catalog));
    model.set_search("Fireplace".into(), &HousingAuthority::default());
    let unknown = HousingAuthority::default();
    assert!(model.stored_entries(&unknown).is_empty());
    let authority = HousingAuthority {
        state: Some(HousingState {
            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
            stored: vec![
                StoredDecor {
                    decor_id: 1693,
                    quantity: 1,
                },
                StoredDecor {
                    decor_id: 2330,
                    quantity: 2,
                },
                StoredDecor {
                    decor_id: 80,
                    quantity: 0,
                },
            ],
            placed: vec![StoredDecor {
                decor_id: 80,
                quantity: 1,
            }],
            ..Default::default()
        }),
        error: None,
    };
    let ids = |model: &HousingDashboard| {
        model
            .stored_entries(&authority)
            .iter()
            .map(|(_, decor)| decor.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&model), vec![1693, 2330]);
    model.storage_search = "  tEsLa  ".into();
    assert_eq!(ids(&model), vec![2330]);
    model
        .editor
        .set_mode(crate::housing_editor::EditorMode::Cleanup);
    model
        .editor
        .set_mode(crate::housing_editor::EditorMode::Basic);
    assert_eq!(model.storage_search, "  tEsLa  ");
    assert_eq!(model.search, "Fireplace");
    model.storage_search = "Fireplace".into();
    assert!(ids(&model).is_empty());
    model.storage_search.clear();
    assert_eq!(ids(&model), vec![1693, 2330]);
    assert_eq!(authority.quantity(2330), Some(2));
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
                initial_rotation: [0.0; 3],
                initial_scale: 1.0,
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
        neighborhoods: Vec::new(),
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
    assert_eq!(model.search, "");
    assert_eq!(model.visible(&none).len(), 12);
    assert_eq!(model.selected(&none).unwrap().id, 81);
    assert_eq!(model.matching(&none).len(), 15);
    assert!(!model.select(80, &none));
    assert!(model.select(83, &none));
    assert_eq!(model.selected(&none).unwrap().id, 83);
    model.set_subcategory(Some(21), &none);
    assert!(model.visible(&none).is_empty());
}

#[test]
fn housing_catalog_search_uses_utf8_threshold_and_source_focus_callbacks() {
    let mut model = dashboard();
    let authority = HousingAuthority::default();
    model.set_storage_search("Storage independent".into());
    model.set_category(Some(2), &authority);
    for raw in ["z", "zz", "éé"] {
        model.set_search(raw.into(), &authority);
        assert_eq!(model.category, Some(2));
        assert_eq!(model.matching(&authority).len(), 15);
    }
    model.set_search("Ornate".into(), &authority);
    assert_eq!(model.category, None);
    assert_eq!(model.matching(&authority).len(), 1);
    model.set_category(Some(18), &authority);
    assert_eq!(model.search, "Ornate"); // All is a no-op, not ClearSearchText.
    model.set_category(Some(2), &authority);
    assert_eq!(model.search, "");
    assert_eq!(model.matching(&authority).len(), 15);
    model.set_search("zz".into(), &authority);
    model.set_category(Some(2), &authority);
    assert_eq!(model.search, "zz"); // Already focused: source early return.
    model.set_search("ééé".into(), &authority);
    assert_eq!(model.category, None);
    assert!(model.matching(&authority).is_empty());
    model.set_search(String::new(), &authority);
    assert_eq!(model.matching(&authority).len(), 16);
    assert_eq!(model.storage_search, "Storage independent");
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

#[test]
fn housing_storage_handle_tooltip_names_the_source_house_chest_without_policy_lines() {
    let tooltip = crate::housing::storage_handle_tooltip();
    assert_eq!(tooltip.content.title, "House Chest");
    assert_eq!(
        tooltip.content.title_color,
        game_engine_ui_model::tooltip_presentation::TOOLTIP_WHITE
    );
    assert!(tooltip.content.lines.is_empty());
}

#[test]
fn housing_entry_tooltip_lists_categories_and_owned_breakdown() {
    let model = dashboard();
    let decor = &model.catalog.decor[0];
    let lines = |authority: &HousingAuthority| {
        crate::housing::decor_entry_tooltip(decor, &model.catalog, authority)
            .content
            .lines
            .iter()
            .map(|line| line.left_text.clone())
            .collect::<Vec<_>>()
    };
    let tooltip = crate::housing::decor_entry_tooltip(decor, &model.catalog, &owning(&[], &[]));
    assert_eq!(tooltip.content.title, "Ornate Stonework Fireplace");
    // Never owned: no owned line, as Retail skips a zero total.
    assert_eq!(lines(&owning(&[], &[])), vec!["Furnishings - Fireplaces"]);
    assert_eq!(
        lines(&owning(&[(80, 1)], &[(80, 2)])),
        vec![
            "Furnishings - Fireplaces",
            "Total Owned: 3 (Placed: 2, Storage: 1)"
        ]
    );
    assert_eq!(
        lines(&HousingAuthority::default()),
        vec!["Furnishings - Fireplaces"]
    );
}

#[test]
fn bulletin_board_sorts_like_retail_roster_columns() {
    use crate::housing::{BulletinBoard, RosterColumn};
    use shared::protocol::{NeighborhoodResident, NeighborhoodRoster};
    let resident = |name: &str, online: bool, plot_index: u32| NeighborhoodResident {
        name: name.into(),
        online,
        plot_index,
    };
    let mut board = BulletinBoard::new(NeighborhoodRoster {
        neighborhood_id: 2,
        residents: vec![
            resident("Ashara", false, 7),
            resident("Brann", true, 3),
            resident("Cyra", false, 1),
            resident("Doran", true, 12),
        ],
    });
    let names = |board: &BulletinBoard| {
        board
            .residents
            .iter()
            .map(|resident| resident.name.clone())
            .collect::<Vec<_>>()
    };
    // Name is the initial column: the first click reverses the server's order.
    board.sort_by(RosterColumn::Name);
    assert_eq!(names(&board), ["Doran", "Cyra", "Brann", "Ashara"]);
    board.sort_by(RosterColumn::Name);
    assert_eq!(names(&board), ["Ashara", "Brann", "Cyra", "Doran"]);
    board.sort_by(RosterColumn::Status);
    assert_eq!(names(&board), ["Brann", "Doran", "Ashara", "Cyra"]);
    board.sort_by(RosterColumn::Status);
    assert_eq!(names(&board), ["Ashara", "Cyra", "Brann", "Doran"]);
    board.sort_by(RosterColumn::Plot);
    assert_eq!(names(&board), ["Cyra", "Brann", "Ashara", "Doran"]);
    let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
    assert!(
        model
            .navigation_request("housing:roster-sort:plot")
            .is_err()
    );
    model.bulletin_board = Some(board);
    assert!(
        model
            .navigation_request("housing:roster-sort:rank")
            .is_err()
    );
}
