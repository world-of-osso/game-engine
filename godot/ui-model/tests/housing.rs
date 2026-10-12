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
            missing_thumbnails: BTreeSet::new(),
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
        view.model.pending = true;
        context.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        let button = registry
            .get(registry.get_by_name("HousingCornerstoneBuyButton").unwrap())
            .unwrap();
        assert!(matches!(&button.widget_data,Some(WidgetData::Button(data)) if !data.enabled));
    }
}

#[test]
fn catalog_preview_reports_authentic_identity_and_missing_image_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for stored in [None, Some(1)] {
            set_thread_skin(skin);
            let catalog = HousingCatalog {
                decor: vec![Decor {
                    id: 80,
                    name: "Ornate Stonework Fireplace".into(),
                    item_id: 235994,
                    model_type: 1,
                    model_fdid: 6033663,
                    thumbnail_fdid: 7415905,
                    order: 1,
                    subcategories: vec![],
                }],
                ..HousingCatalog::default()
            };
            let mut model = HousingDashboard::new(Arc::new(catalog));
            model.tab = DashboardTab::Catalog;
            let mut authority = game_engine_ui_model::housing::HousingAuthority::default();
            if let Some(quantity) = stored {
                authority
                    .receive(shared::protocol::HousingReply {
                        request_id: 1,
                        state: shared::protocol::HousingState {
                            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                            stored: vec![shared::protocol::StoredDecor {
                                decor_id: 80,
                                quantity,
                            }],
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
                missing_thumbnails: BTreeSet::from([7415905]),
            });
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(housing_screen).sync(&context, &mut registry);
            let frame = |name| registry.get(registry.get_by_name(name).unwrap()).unwrap();
            assert_eq!(
                frame("HousingDashboardFrame").width,
                Dimension::Fixed(814.0)
            );
            assert_eq!(
                frame("HousingDashboardFrame").height,
                Dimension::Fixed(544.0)
            );
            assert!(frame("HousingPreviewThumbnail").hidden);
            assert!(!frame("HousingPreviewMissing").hidden);
            let Some(WidgetData::FontString(name)) = &frame("HousingPreviewName").widget_data
            else {
                panic!("missing preview title")
            };
            assert_eq!(name.text, "Ornate Stonework Fireplace");
            let Some(WidgetData::FontString(details)) = &frame("HousingPreviewDetails").widget_data
            else {
                panic!("missing preview details")
            };
            assert!(details.text.contains("Decor 80 · Item 235994"));
            match stored {
                None => assert!(details.text.contains("Ownership and acquisition unknown")),
                Some(quantity) => {
                    assert!(details.text.contains(&format!("Stored decor: {quantity}")))
                }
            }
            assert!(details.text.contains("Model 6033663 (type 1)"));
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
            match stored {
                None => assert!(
                    message
                        .text
                        .contains("House ownership has not been confirmed by the server")
                ),
                Some(_) => assert!(message.text.contains("0 account-owned plot(s)")),
            }
        }
    }
}
