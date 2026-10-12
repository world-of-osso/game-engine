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
        }
    }
}
