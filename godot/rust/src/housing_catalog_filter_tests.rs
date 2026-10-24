use super::*;
use std::{path::PathBuf, sync::Arc};

#[test]
fn housing_catalog_reset_action_restores_results_without_changing_search_category_or_ownership() {
    use game_engine_ui_housing::housing::CollectionFilter;
    use shared::protocol::{HousingState, StoredDecor};

    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = Arc::new(HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap());
    let authority = HousingAuthority {
        state: Some(HousingState {
            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
            stored: vec![StoredDecor {
                decor_id: 1693,
                quantity: 1,
            }],
            ..Default::default()
        }),
        error: None,
    };
    let original = authority.clone();
    for focused_category in [false, true] {
        for (dyeable, collection) in [
            (true, CollectionFilter::All),
            (false, CollectionFilter::Uncollected),
            (true, CollectionFilter::Collected),
        ] {
            let mut model = HousingDashboard::new(catalog.clone());
            if focused_category {
                model.set_category(Some(1), &authority);
                model.set_subcategory(Some(6), &authority);
            } else {
                model.set_search("Small Val'sharah Bookcase".into(), &authority);
            }
            model.dyeable_only = dyeable;
            model.set_collection(collection, &authority);
            model.filters_open = true;
            let search = model.search.clone();
            let category = (model.category, model.subcategory);

            apply_catalog_action(&mut model, &authority, "housing:filter:reset")
                .expect("Retail catalog reset must dispatch through the native action path");

            assert!(!model.dyeable_only);
            assert_eq!(model.collection, CollectionFilter::All);
            assert_eq!(model.search, search);
            assert_eq!((model.category, model.subcategory), category);
            assert!(model.filters_open);
            assert!(
                model
                    .matching(&authority)
                    .iter()
                    .any(|decor| decor.id == 1693)
            );
            assert_eq!(authority, original);
        }
    }
}
