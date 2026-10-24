use super::*;
use std::{path::PathBuf, sync::Arc};

#[test]
fn housing_catalog_reset_action_restores_results_without_changing_search_category_or_ownership() {
    use game_engine_ui_model::housing::CollectionFilter;
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

const SIZE_TAG_GROUP: u32 = 2;
const SMALL_DATA_TAG: u32 = 66;
const TINY_DATA_TAG: u32 = 65;
const CULTURE_TAG_GROUP: u32 = 3;
const ELVEN_DATA_TAG: u32 = 83;
const BOOKCASE_DECOR: u32 = 1693;

fn load_published_tag_catalog() -> Arc<HousingCatalog> {
    let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    Arc::new(HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap())
}

fn matching_ids(model: &HousingDashboard, authority: &HousingAuthority) -> Vec<u32> {
    model
        .matching(authority)
        .iter()
        .map(|decor| decor.id)
        .collect()
}

#[test]
fn housing_catalog_tag_toggle_preserves_search_category_menu_and_authority() {
    let catalog = load_published_tag_catalog();
    assert!(catalog.tags.available);
    let small = catalog
        .tags
        .info
        .iter()
        .find(|tag| tag.record_id == 2)
        .unwrap();
    assert_eq!((small.value, small.name.as_str()), (66, "Small"));
    let fel = catalog
        .tags
        .info
        .iter()
        .find(|tag| tag.record_id == 66)
        .unwrap();
    assert_eq!((fel.value, fel.name.as_str()), (147, "Fel"));
    let authority = HousingAuthority::default();
    let original = authority.clone();
    //70448: Bookcase1693 link1531 has DataTagID66; Info.ID2 names DataTag66 Small.
    // GroupX record4 associates that literal DataTag66 with Size group2.
    let toggle = format!("housing:filter:tag:{SIZE_TAG_GROUP}:{SMALL_DATA_TAG}");
    for focused_category in [false, true] {
        let mut model = HousingDashboard::new(catalog.clone());
        if focused_category {
            model.set_category(Some(1), &authority);
            model.set_subcategory(Some(6), &authority);
        } else {
            model.set_search("Small Val'sharah Bookcase".into(), &authority);
        }
        model.filters_open = true;
        let search = model.search.clone();
        let category = (model.category, model.subcategory);
        let before = matching_ids(&model, &authority);
        assert!(before.contains(&BOOKCASE_DECOR));

        apply_catalog_action(&mut model, &authority, &toggle)
            .expect("the native tag action must accept source group2/DataTag66");
        assert!(!matching_ids(&model, &authority).contains(&BOOKCASE_DECOR));
        assert_eq!(model.search, search);
        assert_eq!((model.category, model.subcategory), category);
        assert!(model.filters_open);
        assert_eq!(authority, original);

        apply_catalog_action(&mut model, &authority, &toggle).unwrap();
        assert_eq!(matching_ids(&model, &authority), before);
        assert_eq!(authority, original);
    }
}

#[test]
fn housing_catalog_tag_group_all_none_and_reset_restore_the_same_query() {
    let authority = HousingAuthority::default();
    let original = authority.clone();
    let mut model = HousingDashboard::new(load_published_tag_catalog());
    model.set_search("Small Val'sharah Bookcase".into(), &authority);
    model.filters_open = true;
    let before = matching_ids(&model, &authority);
    assert_eq!(before, vec![BOOKCASE_DECOR]);
    let clear_group = format!("housing:filter:tags-none:{SIZE_TAG_GROUP}");
    let all_group = format!("housing:filter:tags-all:{SIZE_TAG_GROUP}");

    apply_catalog_action(&mut model, &authority, &clear_group)
        .expect("source group Uncheck All must dispatch without changing search");
    assert!(matching_ids(&model, &authority).is_empty());
    apply_catalog_action(&mut model, &authority, &all_group).unwrap();
    assert_eq!(matching_ids(&model, &authority), before);
    apply_catalog_action(&mut model, &authority, &clear_group).unwrap();
    let toggle_small = format!("housing:filter:tag:{SIZE_TAG_GROUP}:{SMALL_DATA_TAG}");
    let toggle_tiny = format!("housing:filter:tag:{SIZE_TAG_GROUP}:{TINY_DATA_TAG}");
    apply_catalog_action(&mut model, &authority, &toggle_small).unwrap();
    assert_eq!(matching_ids(&model, &authority), before);
    apply_catalog_action(&mut model, &authority, &toggle_tiny).unwrap();
    // Any within Size: selecting Tiny as well must not remove the Small Bookcase.
    assert_eq!(matching_ids(&model, &authority), before);
    let clear_culture = format!("housing:filter:tags-none:{CULTURE_TAG_GROUP}");
    apply_catalog_action(&mut model, &authority, &clear_culture).unwrap();
    assert!(matching_ids(&model, &authority).is_empty());
    let toggle_elven = format!("housing:filter:tag:{CULTURE_TAG_GROUP}:{ELVEN_DATA_TAG}");
    apply_catalog_action(&mut model, &authority, &toggle_elven).unwrap();
    assert_eq!(matching_ids(&model, &authority), before);
    // The Bookcase has Elven83 and Small66, not Tiny65: both group filters apply.
    apply_catalog_action(&mut model, &authority, &toggle_small).unwrap();
    assert!(matching_ids(&model, &authority).is_empty());
    apply_catalog_action(&mut model, &authority, "housing:filter:reset").unwrap();
    assert_eq!(matching_ids(&model, &authority), before);
    assert_eq!(model.search, "Small Val'sharah Bookcase");
    assert!(model.filters_open);
    assert_eq!(authority, original);
}

#[test]
fn housing_catalog_tag_actions_refuse_wrong_groups_and_unavailable_sources() {
    let authority = HousingAuthority::default();
    let original = authority.clone();
    let mut model = HousingDashboard::new(load_published_tag_catalog());
    model.set_search("Small Val'sharah Bookcase".into(), &authority);
    let before = matching_ids(&model, &authority);
    // DataTag83 Elven belongs to Culture3, not Size2; group0 is not authored.
    for action in [
        "housing:filter:tag:2:83",
        "housing:filter:tag:0:66",
        // Info record2 must not be accepted as an alias for literal DataTag66.
        "housing:filter:tag:2:2",
        "housing:filter:tags-all:0",
        "housing:filter:tags-none:0",
        "housing:filter:tag:2:66:83",
        "housing:filter:tag:2:not-a-tag",
    ] {
        assert!(apply_catalog_action(&mut model, &authority, action).is_err());
        assert_eq!(matching_ids(&model, &authority), before);
        assert_eq!(authority, original);
    }
    let mut unavailable = HousingDashboard::new(Arc::new(HousingCatalog::default()));
    assert!(apply_catalog_action(&mut unavailable, &authority, "housing:filter:tag:2:66").is_err());
    assert_eq!(authority, original);
}

#[test]
fn housing_catalog_tag_query_reports_untagged_entries_without_guessing() {
    use game_engine_ui_model::housing::CatalogTagGap;
    let authority = HousingAuthority::default();
    let original = authority.clone();
    let mut model = HousingDashboard::new(load_published_tag_catalog());
    model.set_category(Some(1), &authority);
    model.set_subcategory(Some(6), &authority);
    let before = matching_ids(&model, &authority);
    assert!(before.contains(&1727) && before.contains(&15255));

    apply_catalog_action(&mut model, &authority, "housing:filter:tags-none:2").unwrap();
    let query = model.catalog_query(&authority);
    assert!(query.matches.is_empty());
    let mut gaps = query.unqualified;
    assert!(
        gaps.iter()
            .all(|gap| gap.code() == "HOUSING_TAG_MEMBERSHIP_UNQUALIFIED")
    );
    gaps.sort_by_key(|gap| gap.decor_id);
    assert_eq!(
        gaps,
        vec![
            CatalogTagGap {
                decor_id: 1727,
                group_id: 2
            },
            CatalogTagGap {
                decor_id: 15255,
                group_id: 2
            },
        ]
    );
    apply_catalog_action(&mut model, &authority, "housing:filter:tags-all:2").unwrap();
    assert!(model.catalog_query(&authority).unqualified.is_empty());
    assert_eq!(matching_ids(&model, &authority), before);

    apply_catalog_action(&mut model, &authority, "housing:filter:tag:2:66").unwrap();
    model.set_search(
        "[DNT] [AUTOGEN] 11WE_WildElves_Storage_Handcart02.m2".into(),
        &authority,
    );
    let query = model.catalog_query(&authority);
    assert!(query.matches.is_empty());
    assert_eq!(
        query.unqualified,
        vec![CatalogTagGap {
            decor_id: 1727,
            group_id: 2
        }]
    );
    apply_catalog_action(&mut model, &authority, "housing:filter:reset").unwrap();
    assert!(model.catalog_query(&authority).unqualified.is_empty());
    assert_eq!(matching_ids(&model, &authority), vec![1727]);
    assert_eq!(authority, original);
}
