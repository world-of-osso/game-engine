use game_engine_core::housing::{Decor, HousingCatalog};
use game_engine_ui_model::housing::{DashboardTab, HousingDashboard};
use game_engine_ui_model::housing_component::{HousingView, housing_screen};
use std::collections::BTreeSet;
use std::sync::Arc;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn neighborhood_finder_and_authenticated_cornerstone_controls_match_in_both_skins() {
    use game_engine_ui_model::housing::HousingSurface;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        model.surface = HousingSurface::Finder;
        let plot = shared::protocol::HousingPlot {
            plot_id: 417,
            neighborhood_id: 1,
            plot_index: 0,
            name: String::new(),
            cornerstone: [2942.744, -214.26563, 113.11285],
            teleport: [2939.3438, -212.2554, 113.321],
            cost: 10_000_000,
            house_id: None,
            owner_character_id: None,
        };
        let mut authority = game_engine_ui_model::housing::HousingAuthority::default();
        authority
            .receive(shared::protocol::HousingReply {
                request_id: 1,
                state: shared::protocol::HousingState {
                    build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                    plots: vec![plot],
                    ..Default::default()
                },
                error: None,
            })
            .unwrap();
        let mut view = HousingView {
            model,
            authority,
            viewport: [1280.0, 720.0],
            money: Some(20_000_000),
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        let finder = registry
            .get(registry.get_by_name("HouseFinderFrame").unwrap())
            .unwrap();
        assert_eq!(finder.width, Dimension::Fixed(954.0));
        assert_eq!(finder.height, Dimension::Fixed(490.0));
        for (name, id) in [
            ("HouseFinderNeighborhood1", 1),
            ("HouseFinderNeighborhood2", 2),
        ] {
            let button = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            assert_eq!(
                button.onclick.as_deref(),
                Some(format!("housing:neighborhood:{id}").as_str())
            );
        }
        view.model.surface = HousingSurface::Cornerstone(417);
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        let frame = registry
            .get(
                registry
                    .get_by_name("HousingCornerstonePurchaseFrame")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(frame.width, Dimension::Fixed(340.0));
        assert_eq!(frame.height, Dimension::Fixed(320.0));
        let buy = registry
            .get(registry.get_by_name("HousingCornerstoneBuyButton").unwrap())
            .unwrap();
        assert_eq!(buy.onclick.as_deref(), Some("housing:purchase"));
        let cost = registry
            .get(registry.get_by_name("HousingCornerstonePrice").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(text)) = &cost.widget_data else {
            panic!("missing authenticated price")
        };
        assert!(text.text.contains("1000"));
        view.money = Some(0);
        context.insert(view.clone());
        let mut poor_registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut poor_registry);
        let poor_buy = poor_registry
            .get(
                poor_registry
                    .get_by_name("HousingCornerstoneBuyButton")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(&poor_buy.widget_data,Some(WidgetData::Button(data)) if data.state == ui_toolkit::widgets::button::ButtonState::Disabled)
        );
        view.money = Some(20_000_000);
        view.model.pending = true;
        context.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        let button = registry
            .get(registry.get_by_name("HousingCornerstoneBuyButton").unwrap())
            .unwrap();
        assert!(
            matches!(&button.widget_data,Some(WidgetData::Button(data)) if data.state == ui_toolkit::widgets::button::ButtonState::Disabled)
        );
    }
}

#[test]
fn neighborhood_bulletin_board_lists_and_sorts_the_servers_residents_in_both_skins() {
    use game_engine_ui_model::housing::{BulletinBoard, HousingSurface};
    use shared::protocol::{NeighborhoodResident, NeighborhoodRoster};
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        model.surface = HousingSurface::BulletinBoard;
        // The server sends the roster alphabetically (SetAlphabeticalSortedMemberList).
        model.bulletin_board = Some(BulletinBoard::new(NeighborhoodRoster {
            neighborhood_id: 1,
            residents: vec![
                NeighborhoodResident {
                    name: "Houseproof".into(),
                    online: false,
                    plot_index: 1,
                },
                NeighborhoodResident {
                    name: "Housevisitor".into(),
                    online: true,
                    plot_index: 0,
                },
            ],
        }));
        let mut view = HousingView {
            model,
            authority: Default::default(),
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        let render = |view: &HousingView| {
            let mut context = SharedContext::new();
            context.insert(view.clone());
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            Screen::new(housing_screen).sync(&context, &mut registry);
            registry
        };
        let text = |registry: &FrameRegistry, name: &str| {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::FontString(data)) = &frame.widget_data else {
                panic!("{name} is not a fontstring")
            };
            (data.text.clone(), data.color)
        };
        let registry = render(&view);
        let frame = registry
            .get(registry.get_by_name("HousingBulletinBoardFrame").unwrap())
            .unwrap();
        assert_eq!(frame.width, Dimension::Fixed(600.0));
        assert_eq!(frame.height, Dimension::Fixed(400.0));
        assert_eq!(
            text(&registry, "HousingBulletinBoardNeighborhoodName").0,
            "Founder's Point"
        );
        assert_eq!(
            text(&registry, "HousingBulletinBoardRosterTabLabel").0,
            "Neighborhood Roster"
        );
        for column in ["Name", "Status", "Plot"] {
            assert_eq!(
                text(&registry, &format!("NeighborhoodRosterColumn{column}")).0,
                column
            );
        }
        let disabled = [0.5, 0.5, 0.5, 1.0];
        let highlight = [1.0, 1.0, 1.0, 1.0];
        let rows = |registry: &FrameRegistry| {
            (1..=2)
                .map(|row| {
                    ["Name", "Status", "Plot"]
                        .map(|cell| text(registry, &format!("NeighborhoodRosterEntry{row}{cell}")))
                })
                .collect::<Vec<_>>()
        };
        let houseproof = [
            ("Houseproof".to_string(), disabled),
            ("Offline".to_string(), disabled),
            ("#1".to_string(), disabled),
        ];
        let housevisitor = [
            ("Housevisitor".to_string(), highlight),
            ("Online".to_string(), highlight),
            ("#0".to_string(), highlight),
        ];
        assert_eq!(
            rows(&registry),
            vec![houseproof.clone(), housevisitor.clone()]
        );
        assert!(
            registry
                .get_by_name("NeighborhoodRosterEntry3Name")
                .is_none()
        );
        // Public neighborhoods have no managers: Retail hides Invite Resident.
        assert!(
            registry
                .get_by_name("NeighborhoodRosterInviteResidentButton")
                .is_none()
        );
        let plot_header = registry
            .get(
                registry
                    .get_by_name("NeighborhoodRosterColumnPlotButton")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            plot_header.onclick.as_deref(),
            Some("housing:roster-sort:plot")
        );
        assert_eq!(
            view.model.navigation_request("housing:roster-sort:plot"),
            Ok(None)
        );
        assert_eq!(
            rows(&render(&view)),
            vec![housevisitor.clone(), houseproof.clone()]
        );
        view.model
            .navigation_request("housing:roster-sort:plot")
            .unwrap();
        assert_eq!(rows(&render(&view)), vec![houseproof, housevisitor]);
    }
}

#[test]
fn housing_editor_native_storage_and_owner_controls_follow_the_same_layout_in_both_skins() {
    use game_engine_ui_model::housing::HousingSurface;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for can_edit in [true, false] {
            let mut model = HousingDashboard::new(Arc::new(HousingCatalog {
                decor: vec![Decor {
                    id: 1693,
                    name: "Small Val'sharah Bookcase".into(),
                    item_id: 245259,
                    model_type: 1,
                    model_fdid: 1096761,
                    initial_rotation: [0.0; 3],
                    initial_scale: 1.0,
                    thumbnail_fdid: 7421877,
                    order: 328,
                    subcategories: vec![],
                    sources: vec![],
                    dyeable: false,
                }],
                ..Default::default()
            }));
            model.surface = HousingSurface::Editor;
            let authority = game_engine_ui_model::housing::HousingAuthority {
                state: Some(shared::protocol::HousingState {
                    build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                    stored: vec![shared::protocol::StoredDecor {
                        decor_id: 1693,
                        quantity: 1,
                    }],
                    layout: Some(shared::protocol::HouseLayout {
                        house_id: 41,
                        can_edit,
                        decor: vec![],
                    }),
                    ..Default::default()
                }),
                error: None,
            };
            let mut context = SharedContext::new();
            context.insert(HousingView {
                model,
                authority,
                viewport: [1280.0, 720.0],
                money: None,
                missing_thumbnails: BTreeSet::new(),
                vendor_names: Default::default(),
            });
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            Screen::new(housing_screen).sync(&context, &mut registry);
            let frame = registry
                .get(registry.get_by_name("HouseEditorStorageFrame").unwrap())
                .unwrap();
            assert_eq!(frame.width, Dimension::Fixed(600.0));
            assert_eq!(frame.height, Dimension::Fixed(651.0));
            let button = registry
                .get(registry.get_by_name("HouseEditorStored1693").unwrap())
                .unwrap();
            assert_eq!(button.onclick.as_deref(), Some("housing:place:1693"));
            let Some(WidgetData::Button(button)) = &button.widget_data else {
                panic!("real storage button required")
            };
            assert_eq!(
                button.state == ui_toolkit::widgets::button::ButtonState::Disabled,
                !can_edit
            );
        }
    }
}

#[test]
fn catalog_preview_reports_owned_count_sources_and_missing_image_in_both_skins() {
    use game_engine_core::housing::DecorSource;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for confirmed in [false, true] {
            set_thread_skin(skin);
            // Local Retail rows: Pawdo sells the Small Val'sharah Bookcase in Stormwind.
            let catalog = HousingCatalog {
                decor: vec![Decor {
                    id: 1693,
                    name: "Small Val'sharah Bookcase".into(),
                    item_id: 245259,
                    model_type: 1,
                    model_fdid: 1096761,
                    initial_rotation: [0.0; 3],
                    initial_scale: 1.0,
                    thumbnail_fdid: 7421877,
                    order: 328,
                    subcategories: vec![],
                    sources: vec![DecorSource::Vendor {
                        creature_id: 252312,
                        area: Some("Stormwind City".into()),
                    }],
                    dyeable: false,
                }],
                sources_imported: confirmed,
                ..HousingCatalog::default()
            };
            let mut model = HousingDashboard::new(Arc::new(catalog));
            model.tab = DashboardTab::Catalog;
            let mut authority = game_engine_ui_model::housing::HousingAuthority::default();
            if confirmed {
                let decor = |quantity| {
                    vec![shared::protocol::StoredDecor {
                        decor_id: 1693,
                        quantity,
                    }]
                };
                authority
                    .receive(shared::protocol::HousingReply {
                        request_id: 1,
                        state: shared::protocol::HousingState {
                            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                            stored: decor(1),
                            placed: decor(2),
                            ..Default::default()
                        },
                        error: None,
                    })
                    .unwrap();
            }
            let mut context = SharedContext::new();
            context.insert(HousingView {
                model,
                authority,
                viewport: [1920.0, 1080.0],
                money: None,
                missing_thumbnails: BTreeSet::from([7421877]),
                vendor_names: [(252312, "Second Chair Pawdo".to_string())].into(),
            });
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(housing_screen).sync(&context, &mut registry);
            let frame = |name| registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let text = |name| match &frame(name).widget_data {
                Some(WidgetData::FontString(text)) => text.text.clone(),
                _ => panic!("{name} is not a font string"),
            };
            assert_eq!(
                frame("HousingDashboardFrame").width,
                Dimension::Fixed(814.0)
            );
            assert_eq!(
                frame("HousingDashboardFrame").height,
                Dimension::Fixed(544.0)
            );
            assert!(frame("HousingPreviewThumbnail").hidden);
            assert_eq!(text("HousingPreviewMissing"), "Preview unavailable");
            assert_eq!(text("HousingPreviewName"), "Small Val'sharah Bookcase");
            if confirmed {
                // Owned = stored 1 + placed 2; the tile shows the stored quantity.
                assert_eq!(text("HousingPreviewNumOwned"), "3");
                assert!(!frame("HousingPreviewNumOwnedIcon").hidden);
                assert_eq!(text("HousingCatalogEntry1693Quantity"), "1");
                assert!(!frame("HousingCatalogEntry1693Quantity").hidden);
                assert_eq!(
                    text("HousingPreviewSourceInfo"),
                    "Vendor: Second Chair Pawdo in Stormwind City"
                );
            } else {
                assert_eq!(
                    text("HousingPreviewNumOwned"),
                    "Ownership unconfirmed by the server."
                );
                assert!(registry.get_by_name("HousingPreviewNumOwnedIcon").is_none());
                assert!(frame("HousingCatalogEntry1693Quantity").hidden);
                assert_eq!(
                    text("HousingPreviewSourceInfo"),
                    "Collection sources are not in the housing catalog."
                );
            }
            let search = frame("HousingCatalogSearchBox");
            assert!(matches!(search.widget_data, Some(WidgetData::EditBox(_))));
            let mut info = context.get::<HousingView>().unwrap().clone();
            info.model.tab = DashboardTab::HouseInfo;
            context.insert(info);
            let mut info_registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(housing_screen).sync(&context, &mut info_registry);
            let message = info_registry
                .get(
                    info_registry
                        .get_by_name("HouseInfoContentUnavailable")
                        .unwrap(),
                )
                .unwrap();
            let Some(WidgetData::FontString(message)) = &message.widget_data else {
                panic!("missing house info status")
            };
            if confirmed {
                assert!(message.text.contains("0 account-owned plot(s)"));
            } else {
                assert!(
                    message
                        .text
                        .contains("House ownership has not been confirmed by the server")
                );
            }
        }
    }
}

#[test]
fn catalog_subcategory_column_and_collection_filters_render_in_both_skins() {
    use game_engine_core::housing::{Category, Subcategory};
    use game_engine_ui_model::housing::HousingAuthority;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    // Local Retail DecorCategory 1 Furnishings with DecorSubcategory 1 Seating, 2 Beds.
    let subcategory = |id, name: &str, icon: &str, order| Subcategory {
        id,
        category_id: 1,
        name: name.into(),
        order,
        icon_element: 0,
        icon: Some(icon.into()),
    };
    let catalog = HousingCatalog {
        categories: vec![Category {
            id: 1,
            name: "Furnishings".into(),
            order: 2,
            icon_element: 29134,
            icon: Some("category-icons_furnishings".into()),
        }],
        subcategories: vec![
            subcategory(1, "Seating", "category-icons_seating", 0),
            subcategory(2, "Beds", "category-icons_beds", 3),
        ],
        ..HousingCatalog::default()
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let unknown = HousingAuthority::default();
        let mut model = HousingDashboard::new(Arc::new(catalog.clone()));
        model.tab = DashboardTab::Catalog;
        model.set_category(Some(1), &unknown);
        model.filters_open = true;
        let mut context = SharedContext::new();
        context.insert(HousingView {
            model,
            authority: unknown,
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        });
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        let frame = |name| registry.get(registry.get_by_name(name).unwrap()).unwrap();
        let disabled = |name| {
            matches!(&frame(name).widget_data, Some(WidgetData::Button(button))
                if button.state == ui_toolkit::widgets::button::ButtonState::Disabled)
        };
        assert!(registry.get_by_name("HousingCategory1").is_none());
        let Some(WidgetData::FontString(title)) = &frame("HousingCatalogCategoryText").widget_data
        else {
            panic!("missing category text")
        };
        assert_eq!(title.text, "Furnishings");
        assert_eq!(
            frame("HousingCategoryBackButton").onclick.as_deref(),
            Some("housing:category:back")
        );
        assert_eq!(
            frame("HousingSubcategoryAll").onclick.as_deref(),
            Some("housing:subcategory:all")
        );
        assert_eq!(
            frame("HousingSubcategory2").onclick.as_deref(),
            Some("housing:subcategory:2")
        );
        assert!(registry.get_by_name("HousingSubcategory2Icon").is_some());
        assert!(!disabled("HousingFilterDyeable"));
        assert!(!disabled("HousingFilterCollectionAll"));
        assert!(disabled("HousingFilterCollectionCollected"));
        assert!(disabled("HousingFilterCollectionUncollected"));
        assert!(
            registry
                .get_by_name("HousingFilterCollectionAllTick")
                .is_some()
        );
    }
}

#[test]
fn housing_expert_native_submodes_hide_storage_and_preserve_visitor_restrictions_both_skins() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use game_engine_ui_model::housing_editor::EditorMode;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for can_edit in [true, false] {
            let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
            model.surface = HousingSurface::Editor;
            model.editor.set_mode(EditorMode::Expert);
            let authority = HousingAuthority {
                state: Some(shared::protocol::HousingState {
                    build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                    layout: Some(shared::protocol::HouseLayout {
                        house_id: 1,
                        can_edit,
                        decor: vec![],
                    }),
                    ..Default::default()
                }),
                error: None,
            };
            let mut context = SharedContext::new();
            context.insert(HousingView {
                model,
                authority,
                viewport: [1280.0, 720.0],
                money: None,
                missing_thumbnails: BTreeSet::new(),
                vendor_names: Default::default(),
            });
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            Screen::new(housing_screen).sync(&context, &mut registry);
            assert!(registry.get_by_name("HouseEditorStorageFrame").is_none());
            for name in [
                "TranslateSubmodeButton",
                "RotateSubmodeButton",
                "ScaleSubmodeButton",
            ] {
                let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
                assert_eq!(frame.width, Dimension::Fixed(64.0));
                assert_eq!(frame.height, Dimension::Fixed(64.0));
                let Some(WidgetData::Button(button)) = &frame.widget_data else {
                    panic!("native precision button required")
                };
                assert_eq!(
                    button.state == ui_toolkit::widgets::button::ButtonState::Disabled,
                    !can_edit
                );
            }
        }
    }
}
