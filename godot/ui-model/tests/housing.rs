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
#[ignore = "RED: dashboard catalog lacks the source filter template and Reset control"]
fn housing_catalog_filter_reset_visibility_and_source_header_in_both_skins() {
    use game_engine_ui_model::housing::{CollectionFilter, HousingAuthority};
    use ui_toolkit::layout_values::Val;
    use ui_toolkit::widgets::texture::TextureSource;

    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        model.tab = DashboardTab::Catalog;
        let mut view = HousingView {
            model,
            authority: HousingAuthority::default(),
            viewport: [1920.0, 1080.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(housing_screen);
        for (dyeable, collection, open, reset_visible) in [
            (false, CollectionFilter::All, false, false),
            (true, CollectionFilter::All, true, true),
            (false, CollectionFilter::Collected, false, true),
            (false, CollectionFilter::Uncollected, true, true),
            (false, CollectionFilter::All, true, false),
        ] {
            view.model.dyeable_only = dyeable;
            view.model.collection = collection;
            view.model.filters_open = open;
            context.insert(view.clone());
            screen.sync(&context, &mut registry);
            let frame = |name| registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let filter = frame("HousingCatalogFilters");
            assert_eq!(filter.width, Dimension::Fixed(100.0));
            assert_eq!(filter.height, Dimension::Fixed(20.0));
            assert_eq!(filter.position.left, Val::Px(704.0));
            assert_eq!(filter.position.top, Val::Px(30.0));
            let reset = registry
                .get_by_name("HousingCatalogFiltersResetButton")
                .and_then(|id| registry.get(id))
                .expect("Retail catalog filter Reset control is missing");
            assert_eq!(reset.visible, reset_visible);
            assert_eq!(reset.width, Dimension::Fixed(23.0));
            assert_eq!(reset.height, Dimension::Fixed(23.0));
            if reset_visible {
                assert_eq!(reset.onclick.as_deref(), Some("housing:filter:reset"));
            }
            assert!(
                matches!(&reset.widget_data, Some(WidgetData::Button(button))
                if button.normal_texture.as_ref().map(|texture| texture.source.clone())
                    == Some(TextureSource::Atlas("auctionhouse-ui-filter-redx".into())))
            );
        }
    }
}

#[test]
fn housing_catalog_input_postsetup_limits_unicode_in_both_skins() {
    use game_engine_ui_model::housing::HousingAuthority;
    use game_engine_ui_model::housing_component::apply_housing_postsetup;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        model.tab = DashboardTab::Catalog;
        let view = HousingView {
            model,
            authority: HousingAuthority::default(),
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        apply_housing_postsetup(&view, &mut registry);
        let frame = registry
            .get(registry.get_by_name("HousingCatalogSearchBox").unwrap())
            .unwrap();
        let Some(WidgetData::EditBox(data)) = &frame.widget_data else {
            panic!("Catalog must be editable");
        };
        let mut edited = data.clone();
        edited.insert_at_cursor(&"é".repeat(45));
        assert_eq!(edited.text, "é".repeat(40));
        assert_eq!(data.text_insets, [16.0, 20.0, 0.0, 0.0]);
    }
}

#[test]
fn housing_storage_input_postsetup_limits_unicode_and_tints_source_magnifier_both_skins() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use game_engine_ui_model::housing_component::apply_housing_postsetup;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        model.surface = HousingSurface::Editor;
        let mut view = HousingView {
            model,
            authority: HousingAuthority::default(),
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        for (focused, raw, channel) in [(false, "", 0.6), (true, "", 1.0), (false, "Tesla", 1.0)] {
            view.model.storage_search_focused = focused;
            view.model.set_storage_search(raw.into());
            let mut context = SharedContext::new();
            context.insert(view.clone());
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            Screen::new(housing_screen).sync(&context, &mut registry);
            apply_housing_postsetup(&view, &mut registry);
            let search = registry
                .get(registry.get_by_name("HouseEditorStorageSearchBox").unwrap())
                .unwrap();
            let Some(WidgetData::EditBox(data)) = &search.widget_data else {
                panic!("Search must be editable");
            };
            assert_eq!(data.text_insets, [16.0, 20.0, 0.0, 0.0]);
            if raw.is_empty() {
                let mut edited = data.clone();
                edited.insert_at_cursor(&"é".repeat(45));
                assert_eq!(edited.text, "é".repeat(40));
                assert_eq!(edited.text.len(), 80);
                edited.insert_at_cursor("x");
                assert_eq!(edited.text.chars().count(), 40);
            }
            let glyph = registry
                .get(
                    registry
                        .get_by_name("HouseEditorStorageSearchGlyph")
                        .unwrap(),
                )
                .unwrap();
            let Some(WidgetData::Texture(texture)) = &glyph.widget_data else {
                panic!("Source magnifier must be textured");
            };
            assert_eq!(texture.vertex_color, [channel, channel, channel, 1.0]);
            let hint = registry
                .get(
                    registry
                        .get_by_name("HouseEditorStorageSearchHint")
                        .unwrap(),
                )
                .unwrap();
            let Some(WidgetData::FontString(text)) = &hint.widget_data else {
                panic!("Source hint must be text");
            };
            assert_eq!(text.color, [0.35, 0.35, 0.35, 1.0]);
        }
    }
}

#[test]
fn housing_storage_search_clear_has_source_geometry_and_nonempty_visibility_in_both_skins() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use ui_toolkit::layout_values::Val;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        model.surface = HousingSurface::Editor;
        model.set_storage_search("Tesla".into());
        let mut view = HousingView {
            model,
            authority: HousingAuthority::default(),
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(housing_screen);
        screen.sync(&context, &mut registry);
        let clear = registry
            .get(
                registry
                    .get_by_name("HouseEditorStorageSearchClearButton")
                    .expect("Retail storage search clear control missing"),
            )
            .unwrap();
        assert!(clear.visible);
        assert_eq!(clear.width, Dimension::Fixed(17.0));
        assert_eq!(clear.height, Dimension::Fixed(17.0));
        assert_eq!(clear.position.left, Val::Px(420.0));
        assert_eq!(clear.position.top, Val::Px(26.5));
        assert_eq!(
            clear.onclick.as_deref(),
            Some("housing:storage:search:clear")
        );
        let art = registry
            .get(
                registry
                    .get_by_name("HouseEditorStorageSearchClearArt")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(art.alpha, 0.5);
        let icon = registry
            .get(
                registry
                    .get_by_name("HouseEditorStorageSearchClearIcon")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(icon.width, Dimension::Fixed(10.0));
        assert_eq!(icon.height, Dimension::Fixed(10.0));
        assert_eq!(
            (icon.position.left, icon.position.top),
            (Val::Px(3.0), Val::Px(3.0))
        );
        view.model.set_storage_search(String::new());
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(
            !registry
                .get(
                    registry
                        .get_by_name("HouseEditorStorageSearchClearButton")
                        .unwrap()
                )
                .unwrap()
                .visible
        );
        view.model.storage_search_focused = true;
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get(
                    registry
                        .get_by_name("HouseEditorStorageSearchClearButton")
                        .unwrap()
                )
                .unwrap()
                .visible
        );
    }
}

#[test]
fn housing_storage_transition_frames_slide_in_source_order_in_both_skins() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use ui_toolkit::layout_values::Val;
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        model.surface = HousingSurface::Editor;
        model.storage_panel.resize([725.0, 730.0]).unwrap();
        let mut view = HousingView {
            model,
            authority: HousingAuthority::default(),
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
        let left = |registry: &FrameRegistry, name: &str| {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Val::Px(left) = frame.position.left else {
                panic!("Source slide offset must be pixels")
            };
            left
        };
        view.model.start_storage_transition(true);
        view.model.advance_storage_transition(0.1).unwrap();
        let registry = render(&view);
        assert!((left(&registry, "HouseEditorStorageFrame") + 725.0 * 0.96875).abs() < 0.001);
        assert!(
            registry
                .get_by_name("HouseEditorStorageExpandButton")
                .is_none()
        );
        view.model.advance_storage_transition(0.1).unwrap();
        let registry = render(&view);
        assert!(registry.get_by_name("HouseEditorStorageFrame").is_none());
        assert!((left(&registry, "HouseEditorStorageExpandButton") + 64.0 * 0.67).abs() < 0.001);
        view.model.advance_storage_transition(0.15).unwrap();
        assert_eq!(left(&render(&view), "HouseEditorStorageExpandButton"), 0.0);
        view.model.start_storage_transition(false);
        view.model.advance_storage_transition(0.15).unwrap();
        let registry = render(&view);
        assert!(
            registry
                .get_by_name("HouseEditorStorageExpandButton")
                .is_none()
        );
        assert!((left(&registry, "HouseEditorStorageFrame") + 725.0).abs() < 0.001);
        view.model.advance_storage_transition(0.2).unwrap();
        assert_eq!(left(&render(&view), "HouseEditorStorageFrame"), 0.0);
    }
}

#[test]
fn housing_storage_drawer_collapse_expand_and_resize_reflow_keep_both_skins_equivalent() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use shared::protocol::{HousingState, StoredDecor};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let catalog = HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap();
        let stored = catalog
            .decor
            .iter()
            .take(7)
            .map(|decor| StoredDecor {
                decor_id: decor.id,
                quantity: 1,
            })
            .collect();
        let first = catalog.decor[0].id;
        let fifth = catalog.decor[4].id;
        let mut model = HousingDashboard::new(Arc::new(catalog));
        model.surface = HousingSurface::Editor;
        let mut view = HousingView {
            model,
            authority: HousingAuthority {
                state: Some(HousingState {
                    build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                    stored,
                    ..Default::default()
                }),
                error: None,
            },
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
        let registry = render(&view);
        let collapse = registry
            .get(
                registry
                    .get_by_name("HouseEditorStorageCollapseButton")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(&collapse.widget_data, Some(WidgetData::Button(button)) if button.highlight_texture.as_ref().map(|texture| texture.source.clone()) == Some(ui_toolkit::widgets::texture::TextureSource::Atlas("house-chest-close-drawer-button".into())) && button.highlight_texture.as_ref().unwrap().alpha == 0.5),
            "actual source collapse data: {:?}",
            collapse.widget_data
        );
        assert!(
            registry
                .get_by_name("HouseEditorStorageResizeButton")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("HouseEditorStorageExpandButton")
                .is_none()
        );
        let top = |registry: &FrameRegistry, id: u32| {
            registry
                .get(
                    registry
                        .get_by_name(&format!("HouseEditorStored{id}"))
                        .unwrap(),
                )
                .unwrap()
                .position
                .top
        };
        assert_ne!(top(&registry, fifth), top(&registry, first));
        view.model.storage_panel.resize([700.0, 730.0]).unwrap();
        view.model.storage_panel.finish_resize();
        let registry = render(&view);
        assert_eq!(top(&registry, fifth), top(&registry, first));
        let frame = registry
            .get(registry.get_by_name("HouseEditorStorageFrame").unwrap())
            .unwrap();
        assert_eq!(frame.width, Dimension::Fixed(725.0));
        assert_eq!(frame.height, Dimension::Fixed(730.0));
        view.model.storage_search = "Tesla".into();
        view.model.storage_dyeable_only = true;
        view.model.storage_panel.collapsed = true;
        let registry = render(&view);
        assert!(registry.get_by_name("HouseEditorStorageFrame").is_none());
        let expand = registry
            .get(
                registry
                    .get_by_name("HouseEditorStorageExpandButton")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(&expand.widget_data, Some(WidgetData::Button(button)) if button.highlight_texture.as_ref().map(|texture| texture.source.clone()) == Some(ui_toolkit::widgets::texture::TextureSource::Atlas("house-chest-open-drawer-button".into())) && button.highlight_texture.as_ref().unwrap().alpha == 0.5)
        );
        view.model
            .editor
            .set_mode(game_engine_ui_model::housing_editor::EditorMode::Cleanup);
        assert!(
            render(&view)
                .get_by_name("HouseEditorStorageExpandButton")
                .is_none()
        );
        view.model
            .editor
            .set_mode(game_engine_ui_model::housing_editor::EditorMode::Basic);
        view.model.storage_panel.collapsed = false;
        let registry = render(&view);
        assert!(registry.get_by_name("HouseEditorStorageFrame").is_some());
        assert_eq!(view.model.storage_search, "Tesla");
        assert!(view.model.storage_dyeable_only);
    }
}

#[test]
fn housing_storage_filters_render_independent_choices_and_reset_in_both_skins() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use shared::protocol::{HousingState, StoredDecor};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(
            HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap(),
        ));
        model.surface = HousingSurface::Editor;
        let mut view = HousingView {
            model,
            authority: HousingAuthority {
                state: Some(HousingState {
                    build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                    stored: vec![StoredDecor {
                        decor_id: 1693,
                        quantity: 1,
                    }],
                    ..Default::default()
                }),
                error: None,
            },
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(housing_screen);
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert_storage_filter_header(&registry, false, false);
        assert!(
            registry
                .get_by_name("HouseEditorStorageFilterMenu")
                .is_none()
        );
        view.model
            .storage_filter_action("housing:storage:filters")
            .unwrap();
        view.model
            .storage_filter_action("housing:storage:sort")
            .unwrap();
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert_storage_filter_header(&registry, true, false);
        assert_storage_filter_popup(&registry);
        let date = registry
            .get(
                registry
                    .get_by_name("HouseEditorStorageSortDateAdded")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(&date.widget_data, Some(WidgetData::Button(button)) if button.state == ui_toolkit::widgets::button::ButtonState::Disabled)
        );
        assert!(
            registry
                .get_by_name("HouseEditorStorageFilterCollection")
                .is_none()
        );
        assert!(
            registry
                .get_by_name("HouseEditorStorageFilterFirstAcquisition")
                .is_none()
        );
        view.model
            .storage_filter_action("housing:storage:sort:alphabetical")
            .unwrap();
        assert!(!view.model.storage_filters_open && !view.model.storage_sort_open);
        view.model
            .storage_filter_action("housing:storage:filters")
            .unwrap();
        view.model
            .storage_filter_action("housing:storage:filter:dyeable")
            .unwrap();
        // Checkbox Refresh keeps the root; reopen Sort to inspect retained radio state.
        assert!(view.model.storage_filters_open && !view.model.storage_sort_open);
        view.model
            .storage_filter_action("housing:storage:sort")
            .unwrap();
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name("HouseEditorStorageDyeableTick")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("HouseEditorStorageSortAlphabeticalTick")
                .is_some()
        );
        assert!(registry.get_by_name("HouseEditorStored1693").is_none());
        assert!(registry.get_by_name("HouseEditorStorageEmpty").is_some());
        assert_storage_filter_header(&registry, true, true);
        view.model
            .storage_filter_action("housing:storage:filter:reset")
            .unwrap();
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name("HouseEditorStorageDyeableTick")
                .is_none()
        );
        assert!(
            registry
                .get_by_name("HouseEditorStorageSortAlphabeticalTick")
                .is_some()
        );
        assert!(registry.get_by_name("HouseEditorStored1693").is_some());
        assert!(!view.model.filters_open);
        assert_storage_filter_header(&registry, true, false);
    }
}

fn assert_storage_filter_popup(registry: &FrameRegistry) {
    let frame = |name| registry.get(registry.get_by_name(name).unwrap()).unwrap();
    let root = frame("HouseEditorStorageFilterMenu");
    assert_eq!(root.width, Dimension::Fixed(125.0));
    assert_eq!(root.height, Dimension::Fixed(109.0));
    assert_eq!(
        frame("HouseEditorStorageSortMenu").height,
        Dimension::Fixed(63.0)
    );
    for (name, expected) in [
        ("HouseEditorStorageSortText", "Sort By"),
        ("HouseEditorStorageLocationUnavailableText", "Placeable"),
        ("HouseEditorStorageSortDateAddedText", "Date Added"),
        ("HouseEditorStorageSortAlphabeticalText", "Alphabetical"),
    ] {
        assert!(
            matches!(&frame(name).widget_data, Some(WidgetData::FontString(text)) if text.text == expected)
        );
    }
    assert!(
        registry
            .get_by_name("HouseEditorStorageSortSubmenuArrow")
            .is_some()
    );
    assert!(
        registry
            .get_by_name("HouseEditorStorageFilterDivider")
            .is_some()
    );
    assert!(
        matches!(&frame("HouseEditorStorageLocationUnavailable").widget_data,
        Some(WidgetData::Button(button)) if button.state == ui_toolkit::widgets::button::ButtonState::Disabled)
    );
}

fn assert_storage_filter_header(registry: &FrameRegistry, open: bool, reset_visible: bool) {
    use ui_toolkit::widgets::texture::TextureSource;
    let frame = |name| registry.get(registry.get_by_name(name).unwrap()).unwrap();
    let filter = frame("HouseEditorStorageFilters");
    assert_eq!(filter.width, Dimension::Fixed(125.0));
    assert_eq!(filter.height, Dimension::Fixed(20.0));
    let expected = if open {
        "common-dropdown-b-button-open"
    } else {
        "common-dropdown-b-button"
    };
    let background = frame("HouseEditorStorageFiltersBackground");
    assert!(
        matches!(&background.widget_data, Some(WidgetData::Button(button))
        if button.normal_texture.as_ref().map(|texture| texture.source.clone()) == Some(TextureSource::Atlas(expected.into())))
    );
    let reset = frame("HouseEditorStorageFiltersResetButton");
    assert_eq!(reset.visible, reset_visible);
    assert_eq!(reset.width, Dimension::Fixed(23.0));
    assert_eq!(reset.height, Dimension::Fixed(23.0));
    assert!(
        matches!(&reset.widget_data, Some(WidgetData::Button(button))
        if button.normal_texture.as_ref().map(|texture| texture.source.clone()) == Some(TextureSource::Atlas("auctionhouse-ui-filter-redx".into())))
    );
}

#[test]
fn housing_house_settings_both_skins_stage_without_changing_confirmed_authority() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use shared::protocol::{HouseAccess, HousingHouse, HousingState};
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let access = HouseAccess {
            friends: true,
            ..Default::default()
        };
        let authority = HousingAuthority {
            state: Some(HousingState {
                build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                owned_houses: vec![HousingHouse {
                    house_id: 41,
                    neighborhood_id: 1,
                    plot_id: 417,
                    owner_character_id: 32,
                    can_edit: true,
                    house_access: Some(access),
                    exterior_grant_available: false,
                    interior_grant_available: false,
                }],
                ..Default::default()
            }),
            error: None,
        };
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        assert!(
            model
                .settings_request(&authority, "housing:settings:open:42")
                .is_err()
        );
        model
            .settings_request(&authority, "housing:settings:open:41")
            .unwrap();
        assert_eq!(model.surface, HousingSurface::HouseSettings);
        let mut view = HousingView {
            model,
            authority,
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(housing_screen);
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        let panel = registry
            .get(registry.get_by_name("HousingHouseSettingsFrame").unwrap())
            .unwrap();
        assert_eq!(panel.width, Dimension::Fixed(600.0));
        assert_eq!(panel.height, Dimension::Fixed(550.0));
        let disabled = |registry: &FrameRegistry, name: &str| matches!(&registry.get(registry.get_by_name(name).unwrap()).unwrap().widget_data,Some(WidgetData::Button(data)) if data.state == ui_toolkit::widgets::button::ButtonState::Disabled);
        assert!(
            registry
                .get_by_name("HousingSettingsFlagfriendsTick")
                .is_some()
        );
        assert!(!disabled(&registry, "HousingSettingsFlagfriends"));
        for name in [
            "HousingSettingsPlotMode",
            "HousingSettingsBlueprintMode",
            "HousingSettingsOwner",
            "HousingSettingsAbandon",
        ] {
            assert!(disabled(&registry, name));
        }
        view.model
            .settings_request(&view.authority, "housing:settings:anyone")
            .unwrap();
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        for key in ["neighbors", "guild", "friends", "party"] {
            assert!(disabled(&registry, &format!("HousingSettingsFlag{key}")));
            assert!(
                registry
                    .get_by_name(&format!("HousingSettingsFlag{key}Tick"))
                    .is_some()
            );
        }
        assert_eq!(
            view.authority.state.as_ref().unwrap().owned_houses[0].house_access,
            Some(access)
        );
        assert_eq!(
            view.model
                .settings_request(&view.authority, "housing:settings:save")
                .unwrap(),
            Some(shared::protocol::HousingCommand::SaveHouseAccess {
                house_id: 41,
                access: HouseAccess {
                    anyone: true,
                    neighbors: true,
                    guild: true,
                    friends: true,
                    party: true
                }
            })
        );
        view.model
            .settings_request(&view.authority, "housing:settings:none")
            .unwrap();
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name("HousingSettingsFlagfriendsTick")
                .is_none()
        );
        assert!(disabled(&registry, "HousingSettingsFlagfriends"));
        view.model
            .settings_request(&view.authority, "housing:settings:limited")
            .unwrap();
        view.model
            .settings_request(&view.authority, "housing:settings:toggle:friends")
            .unwrap();
        assert_eq!(
            view.model
                .settings_request(&view.authority, "housing:settings:save")
                .unwrap(),
            Some(shared::protocol::HousingCommand::SaveHouseAccess {
                house_id: 41,
                access
            })
        );
    }
}

#[test]
fn housing_catalog_model_scene_replaces_the_thumbnail_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
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
            order: 0,
            subcategories: vec![],
            sources: vec![],
            dyeable: false,
        }],
        ..Default::default()
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(catalog.clone()));
        model.tab = DashboardTab::Catalog;
        model.selection = Some(1693);
        let mut view = HousingView {
            model,
            authority: Default::default(),
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(housing_screen);
        screen.sync(&context, &mut registry);
        let scene = registry
            .get(
                registry
                    .get_by_name("HousingCatalogModelScene")
                    .expect("3D preview host"),
            )
            .unwrap();
        assert_eq!(scene.width, Dimension::Fixed(301.0));
        assert_eq!(scene.height, Dimension::Fixed(334.0));
        assert!(registry.get_by_name("HousingPreviewThumbnail").is_none());
        assert!(
            registry
                .get_by_name("HousingCatalogEntry1693Icon")
                .is_some()
        );
        view.model.tab = DashboardTab::HouseInfo;
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert!(registry.get_by_name("HousingCatalogModelScene").is_none());
    }
}

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
            owner_type: shared::protocol::HousingPlotOwnerType::None,
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
            let mut view = HousingView {
                model,
                authority,
                viewport: [1280.0, 720.0],
                money: None,
                missing_thumbnails: BTreeSet::new(),
                vendor_names: Default::default(),
            };
            context.insert(view.clone());
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
            let icon = registry
                .get(registry.get_by_name("HouseEditorIcon1693").unwrap())
                .unwrap();
            assert!(
                matches!(&icon.widget_data, Some(WidgetData::Texture(texture)) if texture.source == ui_toolkit::widgets::texture::TextureSource::FileDataId(7421877)),
                "exported storage image must resolve the real thumbnail FDID"
            );
            let search = registry
                .get(
                    registry
                        .get_by_name("HouseEditorStorageSearchBox")
                        .expect("Retail storage search"),
                )
                .unwrap();
            assert!(matches!(&search.widget_data, Some(WidgetData::EditBox(_))));
            view.model.storage_search = "Tesla".into();
            context.insert(view);
            let mut filtered = FrameRegistry::new(1280.0, 720.0);
            Screen::new(housing_screen).sync(&context, &mut filtered);
            assert!(filtered.get_by_name("HouseEditorStored1693").is_none());
            assert!(filtered.get_by_name("HouseEditorStorageEmpty").is_some());
        }
    }
}

#[test]
fn ui_template_behavior_services_catalog_preview_keeps_live_decor_owner_and_sources() {
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
            let hit = registry.get_by_name("HousingCatalogEntry1693").unwrap();
            let (owner, binding) = ui_toolkit::tooltip::tooltip_owner(&registry, hit)
                .expect("catalog live decor owner");
            assert_eq!(owner.id, hit);
            assert_eq!(binding.provider.as_deref(), Some("housing"));
            assert_eq!(binding.anchor, ui_toolkit::tooltip::TooltipAnchor::Right);
            assert!(frame("HousingCatalogEntry1693Icon").hidden);
            assert_eq!(
                text("HousingCatalogEntry1693Unavailable"),
                "Thumbnail unavailable"
            );
            assert!(registry.get_by_name("HousingPreviewThumbnail").is_none());
            assert!(registry.get_by_name("HousingCatalogModelScene").is_some());
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
fn housing_expert_mouse_controls_show_real_rotation_and_scale_values_in_both_skins() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use game_engine_ui_model::housing_editor::{EditorMode, PrecisionSubmode};
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let authority = HousingAuthority {
            state: Some(shared::protocol::HousingState {
                layout: Some(shared::protocol::HouseLayout {
                    house_id: 41,
                    can_edit: true,
                    decor: vec![shared::protocol::PlacedDecor {
                        dyes: Vec::new(),
                        instance_id: 7,
                        decor_id: 1693,
                        transform: shared::protocol::DecorTransform {
                            position: [1.0, 2.0, 3.0],
                            yaw: 0.0,
                            pitch: 0.0,
                            roll: 0.0,
                            scale: 1.2,
                        },
                    }],
                }),
                ..Default::default()
            }),
            error: None,
        };
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        model.surface = HousingSurface::Editor;
        model.editor.set_mode(EditorMode::Expert);
        model.editor.select(&authority, 7).unwrap();
        model.editor.scale_allowed = true;
        model.editor.mouse_control_anchor = Some([640.0, 300.0]);
        let mut view = HousingView {
            model,
            authority,
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: Default::default(),
            vendor_names: Default::default(),
        };
        for (submode, frame_name, value_name, expected, width, height) in [
            (
                PrecisionSubmode::Rotate,
                "HouseEditorRotateControl",
                "HouseEditorRotateAmount",
                "0°",
                80.0,
                20.0,
            ),
            (
                PrecisionSubmode::Scale,
                "HouseEditorScaleControl",
                "HouseEditorScaleAmount",
                "120%",
                200.0,
                19.0,
            ),
        ] {
            view.model.editor.set_submode(submode);
            let mut context = SharedContext::new();
            context.insert(view.clone());
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            Screen::new(housing_screen).sync(&context, &mut registry);
            let frame = registry
                .get(
                    registry
                        .get_by_name(frame_name)
                        .expect("sourced mouse control"),
                )
                .unwrap();
            assert_eq!(frame.width, Dimension::Fixed(width));
            assert_eq!(frame.height, Dimension::Fixed(height));
            let amount = registry
                .get(registry.get_by_name(value_name).unwrap())
                .unwrap();
            let Some(WidgetData::FontString(text)) = &amount.widget_data else {
                panic!("missing actual transform amount")
            };
            assert_eq!(text.text, expected);
            for side in ["Left", "Right"] {
                let arrow = registry
                    .get(
                        registry
                            .get_by_name(&format!("{frame_name}{side}"))
                            .unwrap(),
                    )
                    .unwrap();
                assert_eq!(arrow.width, Dimension::Fixed(13.0));
                assert_eq!(arrow.height, Dimension::Fixed(13.0));
            }
        }
        view.model.pending = true;
        let mut context = SharedContext::new();
        context.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        assert!(registry.get_by_name("HouseEditorScaleControl").is_none());
    }
}

#[test]
fn housing_storage_categories_show_owned_roots_then_back_all_and_source_filtered_children_both_skins()
 {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use shared::protocol::{HouseLayout, HousingState, StoredDecor};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let mut catalog = HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap();
    catalog
        .resolve_icons(&data.join("db2/active/UiTextureAtlasElement.csv"))
        .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(catalog.clone()));
        model.surface = HousingSurface::Editor;
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
                layout: Some(HouseLayout {
                    house_id: 1,
                    can_edit: true,
                    decor: vec![],
                }),
                ..Default::default()
            }),
            error: None,
        };
        let mut view = HousingView {
            model,
            authority,
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: Default::default(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(housing_screen);
        screen.sync(&context, &mut registry);
        for id in [18, 1, 2] {
            let frame = registry
                .get(
                    registry
                        .get_by_name(&format!("HouseEditorStorageCategory{id}"))
                        .expect("owned source category"),
                )
                .unwrap();
            assert_eq!(frame.width, Dimension::Fixed(64.0));
            assert_eq!(frame.height, Dimension::Fixed(64.0));
            assert_eq!(
                frame.onclick.as_deref(),
                Some(format!("housing:storage:category:{id}").as_str())
            );
        }
        for unowned in [4, 5, 6, 9, 17] {
            assert!(
                registry
                    .get_by_name(&format!("HouseEditorStorageCategory{unowned}"))
                    .is_none()
            );
        }
        view.model
            .set_storage_category(Some(1), &view.authority)
            .unwrap();
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(registry.get_by_name("HouseEditorStored1693").is_some());
        assert!(registry.get_by_name("HouseEditorStored2330").is_none());
        assert!(
            registry
                .get_by_name("HouseEditorStorageCategoryBack")
                .is_some()
        );
        let all = registry
            .get(
                registry
                    .get_by_name("HouseEditorStorageSubcategoryAll")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(all.width, Dimension::Fixed(54.0));
        assert!(
            registry
                .get_by_name("HouseEditorStorageSubcategory6")
                .is_none()
        ); // Source hides specific subcategory buttons when only one is showable.
        let title = registry
            .get(
                registry
                    .get_by_name("HouseEditorStorageCategoryPath")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(&title.widget_data, Some(WidgetData::FontString(text)) if text.text == "Furnishings")
        );
        view.model
            .set_storage_category(None, &view.authority)
            .unwrap();
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name("HouseEditorStorageCategoryBack")
                .is_none()
        );
        assert!(registry.get_by_name("HouseEditorStored1693").is_some());
        assert!(registry.get_by_name("HouseEditorStored2330").is_some());
    }
}

#[test]
fn housing_expert_placed_list_uses_source_bounds_unknown_budget_and_real_scrollable_rows_both_skins()
 {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use game_engine_ui_model::housing_editor::EditorMode;
    use game_engine_ui_model::housing_placed_list::LIST;
    use shared::protocol::{DecorTransform, HouseLayout, HousingState, PlacedDecor, StoredDecor};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let catalog = Arc::new(HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap());
    let authority = HousingAuthority {
        state: Some(HousingState {
            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
            placed: vec![StoredDecor {
                decor_id: 1693,
                quantity: 20,
            }],
            layout: Some(HouseLayout {
                house_id: 1,
                can_edit: true,
                decor: (1..=20)
                    .map(|instance_id| PlacedDecor {
                        instance_id,
                        decor_id: 1693,
                        transform: DecorTransform {
                            position: [-1000.0, -2.0, 996.0],
                            yaw: 0.0,
                            pitch: 0.0,
                            roll: 0.0,
                            scale: 1.0,
                        },
                        dyes: vec![],
                    })
                    .collect(),
            }),
            ..Default::default()
        }),
        error: None,
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(catalog.clone());
        model.surface = HousingSurface::Editor;
        model.editor.set_mode(EditorMode::Expert);
        let mut view = HousingView {
            model,
            authority: authority.clone(),
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: Default::default(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(housing_screen);
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name("HouseEditorPlacedDecorListButton")
                .is_some()
        );
        let budget = registry
            .get(registry.get_by_name("HouseEditorDecorBudgetText").unwrap())
            .unwrap();
        assert!(
            matches!(&budget.widget_data,Some(WidgetData::FontString(text)) if text.text=="— / —")
        );
        assert!(registry.get_by_name("HouseEditorPlacedDecorList").is_none());
        view.model.toggle_placed_list().unwrap();
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        let panel = registry
            .get(registry.get_by_name("HouseEditorPlacedDecorList").unwrap())
            .unwrap();
        assert_eq!(
            (panel.width, panel.height),
            (Dimension::Fixed(400.0), Dimension::Fixed(400.0))
        );
        assert_eq!(
            (panel.position.left, panel.position.top),
            (
                ui_toolkit::layout_values::Val::Px(830.0),
                ui_toolkit::layout_values::Val::Px(150.0)
            )
        );
        assert!(
            registry
                .get_by_name("HouseEditorPlacedDecorEntry1")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("HouseEditorPlacedDecorEntry20")
                .is_none()
        );
        assert!(
            registry
                .scroll_lists
                .get(LIST)
                .unwrap()
                .geometry
                .max_first_row()
                > 0
        );
        assert!(registry.scroll_lists.scroll_by(LIST, 10000));
        screen.sync(&context, &mut registry);
        let row = registry
            .get(
                registry
                    .get_by_name("HouseEditorPlacedDecorEntry20")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(row.height, Dimension::Fixed(20.0));
        assert_eq!(
            row.onclick.as_deref(),
            Some("housing:placed-list:select:20")
        );
        view.model
            .open_placed_list_menu(&view.authority, 20, [1000.0, 200.0])
            .unwrap();
        view.authority
            .state
            .as_mut()
            .unwrap()
            .layout
            .as_mut()
            .unwrap()
            .can_edit = false;
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        let remove = registry
            .get(
                registry
                    .get_by_name("HouseEditorPlacedDecorRemove")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            ui_toolkit::attrs::read_attribute(&registry, remove.id, "disabled"),
            Some("true".into())
        );
        view.model.close_placed_list();
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(registry.get_by_name("HouseEditorPlacedDecorList").is_none());
        view.model.editor.set_mode(EditorMode::Basic);
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name("HouseEditorPlacedDecorListButton")
                .is_none()
        );
        assert!(registry.get_by_name("HouseEditorDecorBudgetText").is_none());
    }
}

#[test]
fn housing_basic_selection_hides_both_drawer_parts_and_cancel_restores_preferences_both_skins() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use shared::protocol::{DecorTransform, HouseLayout, HousingState, PlacedDecor, StoredDecor};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let catalog = Arc::new(HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap());
    let authority = HousingAuthority {
        state: Some(HousingState {
            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
            stored: vec![StoredDecor {
                decor_id: 2330,
                quantity: 1,
            }],
            placed: vec![StoredDecor {
                decor_id: 1693,
                quantity: 1,
            }],
            layout: Some(HouseLayout {
                house_id: 1,
                can_edit: true,
                decor: vec![PlacedDecor {
                    instance_id: 26,
                    decor_id: 1693,
                    transform: DecorTransform {
                        position: [-1000.0, -2.0, 996.0],
                        yaw: 0.0,
                        pitch: 0.0,
                        roll: 0.0,
                        scale: 1.0,
                    },
                    dyes: vec![],
                }],
            }),
            ..Default::default()
        }),
        error: None,
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for (viewport_height, panel_height, panel_top) in [
            (720.0, 651.0, 0.0),
            (1080.0, 651.0, 64.5),
            (720.0, 730.0, 0.0),
            (1080.0, 730.0, 25.0),
            (720.0, 800.0, 0.0),
            (1080.0, 800.0, 0.0),
        ] {
            for collapsed in [false, true] {
                let mut model = HousingDashboard::new(catalog.clone());
                model.surface = HousingSurface::Editor;
                model.storage_panel.resize([725.0, panel_height]).unwrap();
                model.storage_panel.collapsed = collapsed;
                model.set_storage_search("tEs".into());
                model.storage_alphabetical = true;
                let saved_panel = model.storage_panel;
                let mut view = HousingView {
                    model,
                    authority: authority.clone(),
                    viewport: [1280.0, viewport_height],
                    money: None,
                    missing_thumbnails: Default::default(),
                    vendor_names: Default::default(),
                };
                let mut context = SharedContext::new();
                let mut screen = Screen::new(housing_screen);
                let mut registry = FrameRegistry::new(1280.0, viewport_height);
                context.insert(view.clone());
                screen.sync(&context, &mut registry);
                let shown = if collapsed {
                    "HouseEditorStorageExpandButton"
                } else {
                    "HouseEditorStorageFrame"
                };
                let expected_top = if collapsed {
                    (viewport_height - 72.0) / 2.0 - 150.0
                } else {
                    panel_top
                };
                let frame = registry.get(registry.get_by_name(shown).unwrap()).unwrap();
                assert_eq!(
                    frame.position.top,
                    ui_toolkit::layout_values::Val::Px(expected_top)
                );
                view.model.editor.select(&view.authority, 26).unwrap();
                context.insert(view.clone());
                screen.sync(&context, &mut registry);
                assert!(registry.get_by_name("HouseEditorStorageFrame").is_none());
                assert!(
                    registry
                        .get_by_name("HouseEditorStorageExpandButton")
                        .is_none()
                );
                assert!(
                    registry
                        .get_by_name("HouseEditorStorageSearchBox")
                        .is_none()
                );
                view.model.editor.cancel();
                context.insert(view.clone());
                screen.sync(&context, &mut registry);
                assert!(registry.get_by_name(shown).is_some());
                assert_eq!(view.model.storage_panel, saved_panel);
                assert_eq!(view.model.storage_search, "tEs");
                assert!(view.model.storage_alphabetical);
                assert_eq!(view.authority, authority);
                let source = view
                    .model
                    .catalog
                    .decor
                    .iter()
                    .find(|decor| decor.id == 2330)
                    .unwrap();
                view.model
                    .editor
                    .begin_place(&view.authority, source)
                    .unwrap();
                context.insert(view.clone());
                screen.sync(&context, &mut registry);
                assert!(registry.get_by_name("HouseEditorStorageFrame").is_none());
                assert!(
                    registry
                        .get_by_name("HouseEditorStorageExpandButton")
                        .is_none()
                );
                view.model.editor.cancel();
                context.insert(view.clone());
                screen.sync(&context, &mut registry);
                assert!(registry.get_by_name(shown).is_some());
                assert_eq!(view.model.storage_panel, saved_panel);
                assert_eq!(view.model.storage_search, "tEs");
                assert_eq!(view.authority, authority);
                let restored = registry.get(registry.get_by_name(shown).unwrap()).unwrap();
                assert_eq!(
                    restored.position.top,
                    ui_toolkit::layout_values::Val::Px(expected_top)
                );
            }
        }
    }
}

#[test]
fn housing_cleanup_hides_storage_and_uses_source_cursor_tooltip_in_both_skins() {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use game_engine_ui_model::housing_editor::EditorMode;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let decor = Decor {
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
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for can_edit in [true, false] {
            let mut model = HousingDashboard::new(Arc::new(HousingCatalog {
                decor: vec![decor.clone()],
                ..Default::default()
            }));
            model.surface = HousingSurface::Editor;
            let authority = HousingAuthority {
                state: Some(shared::protocol::HousingState {
                    build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                    layout: Some(shared::protocol::HouseLayout {
                        house_id: 1,
                        can_edit,
                        decor: vec![shared::protocol::PlacedDecor {
                            instance_id: 7,
                            decor_id: 1693,
                            transform: shared::protocol::DecorTransform {
                                position: [0.0; 3],
                                yaw: 0.0,
                                pitch: 0.0,
                                roll: 0.0,
                                scale: 1.0,
                            },
                            dyes: vec![],
                        }],
                    }),
                    ..Default::default()
                }),
                error: None,
            };
            let mut context = SharedContext::new();
            let mut view = HousingView {
                model,
                authority,
                viewport: [1280.0, 720.0],
                money: None,
                missing_thumbnails: Default::default(),
                vendor_names: Default::default(),
            };
            context.insert(view.clone());
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            let mut screen = Screen::new(housing_screen);
            screen.sync(&context, &mut registry);
            let cleanup = registry
                .get(
                    registry
                        .get_by_name("HouseEditorCleanupModeButton")
                        .expect("mode entry"),
                )
                .unwrap();
            assert_eq!(cleanup.onclick.as_deref(), Some("housing:cleanup"));
            view.model.editor.set_mode(EditorMode::Cleanup);
            view.model
                .editor
                .hover(&view.authority, Some(7), [600.0, 300.0])
                .unwrap();
            context.insert(view.clone());
            screen.sync(&context, &mut registry);
            assert!(registry.get_by_name("HouseEditorStorageFrame").is_none());
            assert!(registry.get_by_name("HouseEditorStored1693").is_none());
            assert!(registry.get_by_name("HouseEditorRemove").is_none());
            let mode_bar = registry
                .get(registry.get_by_name("HouseEditorModeBar").unwrap())
                .unwrap();
            assert_eq!(mode_bar.width, Dimension::Fixed(266.0));
            assert_eq!(mode_bar.height, Dimension::Fixed(64.0));
            assert_eq!(
                mode_bar.position.left,
                ui_toolkit::layout_values::Val::Px(507.0)
            );
            assert_eq!(
                mode_bar.position.top,
                ui_toolkit::layout_values::Val::Px(656.0)
            );
            assert!(
                registry
                    .get_by_name("HouseEditorCustomizeModeButton")
                    .is_some()
            );
            assert!(registry.get_by_name("HouseEditorBasicSubmodeBar").is_none());
            assert!(
                registry
                    .get_by_name("HouseEditorExpertSubmodeBar")
                    .is_none()
            );
            let instruction = registry
                .get(
                    registry
                        .get_by_name("HouseEditorSelectInstruction")
                        .expect("Source Cleanup SelectInstruction"),
                )
                .unwrap();
            assert_eq!(instruction.height, Dimension::Fixed(45.0));
            let instruction_text = registry
                .get(
                    registry
                        .get_by_name("HouseEditorSelectInstructionText")
                        .unwrap(),
                )
                .unwrap();
            assert!(
                matches!(&instruction_text.widget_data, Some(WidgetData::FontString(font)) if font.text == "Remove Decor" && font.font_size == 16.0)
            );
            assert!(
                registry
                    .get_by_name("HouseEditorSelectInstructionControlIcon")
                    .is_some()
            );
            let instructions = registry
                .get(registry.get_by_name("HouseEditorInstructions").unwrap())
                .unwrap();
            assert_eq!(
                instructions.position.left,
                ui_toolkit::layout_values::Val::Px(1250.0 - instructions.width.value())
            );
            use game_engine_ui_model::game_tooltip::{TooltipAnchor, TooltipScreen, place};
            use game_engine_ui_model::tooltip_presentation::TOOLTIP_WHITE;
            let before = view.authority.clone();
            let tooltip = view.model.cleanup_hover_tooltip(&view.authority).unwrap();
            assert_eq!(tooltip.anchor, TooltipAnchor::Cursor);
            assert_eq!(tooltip.content.title, "Small Val'sharah Bookcase");
            assert_eq!(tooltip.content.title_color, TOOLTIP_WHITE);
            assert!(
                tooltip.content.lines.is_empty(),
                "No fabricated pet/lock/removability metadata"
            );
            let shown = place(
                tooltip,
                TooltipScreen {
                    size: view.viewport,
                    cursor: [600.0, 300.0],
                },
            );
            assert_eq!([shown.x, shown.y], [600.0, 280.0]);
            assert!(shown.visible);
            assert_eq!(view.authority, before);
            view.model.pending = true;
            assert!(view.model.cleanup_hover_tooltip(&view.authority).is_none());
            view.model.pending = false;
            let mut removed = view.authority.clone();
            removed
                .state
                .as_mut()
                .unwrap()
                .layout
                .as_mut()
                .unwrap()
                .decor
                .clear();
            assert!(view.model.cleanup_hover_tooltip(&removed).is_none());
            view.model
                .editor
                .hover(&view.authority, None, [0.0; 2])
                .unwrap();
            context.insert(view.clone());
            screen.sync(&context, &mut registry);
            assert!(view.model.cleanup_hover_tooltip(&view.authority).is_none());
            view.model.editor.set_mode(EditorMode::Basic);
            assert!(view.model.cleanup_hover_tooltip(&view.authority).is_none());
            context.insert(view);
            screen.sync(&context, &mut registry);
            assert!(registry.get_by_name("HouseEditorStorageFrame").is_some());
        }
    }
}

#[test]
fn housing_expert_native_submodes_hide_storage_and_preserve_visitor_restrictions_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
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

#[test]
fn housing_finder_draws_source_sale_and_real_friend_pins_not_strangers_or_self_both_skins() {
    use game_engine_core::housing::NeighborhoodMap;
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use shared::protocol::{HousingPlot, HousingPlotOwnerType, HousingState};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let maps = Arc::new(
        game_engine_ui_model::ui_map_data::UiMapCatalog::load(&data.join("db2/active")).unwrap(),
    );
    let roles = [
        HousingPlotOwnerType::None,
        HousingPlotOwnerType::Friend,
        HousingPlotOwnerType::Stranger,
        HousingPlotOwnerType::SelfOwned,
    ];
    let coordinates = [
        [2942.7439, -214.26563, 113.11285],
        [2674.3176, 651.8455, 49.225697],
        [2523.3455, 171.17882, 60.40972],
        [3166.5469, -525.7153, 190.65105],
    ];
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog {
            neighborhoods: vec![NeighborhoodMap {
                id: 1,
                map_id: 2735,
                texture_kit: 5721,
                texture_suffix: Some("elwynn".into()),
            }],
            ..Default::default()
        }));
        model.surface = HousingSurface::Finder;
        model.maps = Some(maps.clone());
        let plots = roles
            .into_iter()
            .enumerate()
            .map(|(index, owner_type)| HousingPlot {
                plot_id: 417 + index as u32,
                neighborhood_id: 1,
                plot_index: index as u32,
                name: String::new(),
                cornerstone: coordinates[index],
                teleport: coordinates[index],
                cost: 10_000_000,
                house_id: if index == 0 { None } else { Some(index as u64) },
                owner_character_id: if index == 0 {
                    None
                } else {
                    Some(31 + index as u64)
                },
                owner_type,
            })
            .collect();
        let authority = HousingAuthority {
            state: Some(HousingState {
                build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                plots,
                ..Default::default()
            }),
            error: None,
        };
        assert!(model.select_finder_plot(&authority, 419).is_err());
        let mut view = HousingView {
            model,
            authority,
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: BTreeSet::new(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        assert!(registry.get_by_name("HouseFinderMapTile0").is_some());
        assert!(
            registry
                .get_by_name("HouseFinderPlotForSalePin417")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("HouseFinderFriendsPlotPin418")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("HouseFinderFriendsPlotPin419")
                .is_none()
        );
        assert!(
            registry
                .get_by_name("HouseFinderFriendsPlotPin420")
                .is_none()
        );
        assert!(registry.get_by_name("HouseFinderVisitHouse1").is_none());
        view.model.select_finder_plot(&view.authority, 417).unwrap();
        context.insert(view.clone());
        Screen::new(housing_screen).sync(&context, &mut registry);
        let button = registry
            .get(registry.get_by_name("HouseFinderPlotVisitButton").unwrap())
            .unwrap();
        assert_eq!(button.onclick.as_deref(), Some("housing:visit-plot"));
        view.model.select_finder_plot(&view.authority, 418).unwrap();
        context.insert(view);
        Screen::new(housing_screen).sync(&context, &mut registry);
        let button = registry
            .get(registry.get_by_name("HouseFinderPlotVisitButton").unwrap())
            .unwrap();
        assert_eq!(button.onclick.as_deref(), Some("housing:home:1"));
    }
}

#[test]
fn ui_template_blend_housing_storage_tiles_match_source_geometry_counts_and_background_states_both_skins()
 {
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use game_engine_ui_model::housing_component::apply_housing_postsetup;
    use shared::protocol::{HouseLayout, HousingState, StoredDecor};
    use ui_toolkit::widgets::button::ButtonState;
    use ui_toolkit::widgets::texture::{BlendMode, TextureSource};
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let catalog = Arc::new(HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap());
    let stored: Vec<_> = catalog
        .decor
        .iter()
        .filter(|decor| decor.thumbnail_fdid > 0)
        .take(6)
        .enumerate()
        .map(|(index, decor)| StoredDecor {
            decor_id: decor.id,
            quantity: index as u32 + 1,
        })
        .collect();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(catalog.clone());
        model.surface = HousingSurface::Editor;
        let authority = HousingAuthority {
            state: Some(HousingState {
                build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                stored: stored.clone(),
                layout: Some(HouseLayout {
                    house_id: 41,
                    can_edit: true,
                    decor: vec![],
                }),
                ..Default::default()
            }),
            error: None,
        };
        let view = HousingView {
            model,
            authority,
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: Default::default(),
            vendor_names: Default::default(),
        };
        let entries = view.model.stored_entries(&view.authority);
        assert_eq!(entries.len(), 6);
        let mut context = SharedContext::new();
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        let blend = |registry: &FrameRegistry, name: &str| match &registry
            .get(registry.get_by_name(name).unwrap())
            .unwrap()
            .widget_data
        {
            Some(WidgetData::Texture(texture)) => texture.blend_mode,
            _ => panic!("{name} must be a Texture"),
        };
        // HousingCatalogEntry.xml47/51: authored at construction, unchanged by state sync.
        for (_, decor) in &entries {
            let prefix = format!("HouseEditorStored{}", decor.id);
            assert_eq!(
                blend(&registry, &format!("{prefix}Background")),
                BlendMode::Blend
            );
            assert_eq!(
                blend(&registry, &format!("{prefix}HoverBackground")),
                BlendMode::Additive
            );
        }
        apply_housing_postsetup(&view, &mut registry);
        for (index, (quantity, decor)) in entries.iter().enumerate() {
            let prefix = format!("HouseEditorStored{}", decor.id);
            let tile = registry
                .get(registry.get_by_name(&prefix).unwrap())
                .unwrap();
            assert_eq!(
                (tile.width, tile.height),
                (Dimension::Fixed(97.0), Dimension::Fixed(97.0))
            );
            assert_eq!(
                tile.position.left,
                ui_toolkit::layout_values::Val::Px(85.0 + (index % 4) as f32 * 102.0)
            );
            assert_eq!(
                tile.position.top,
                ui_toolkit::layout_values::Val::Px(101.0 + (index / 4) as f32 * 107.0)
            );
            assert_eq!(
                tile.onclick.as_deref(),
                Some(format!("housing:place:{}", decor.id).as_str())
            );
            assert!(
                registry
                    .get_by_name(&format!("HouseEditorName{}", decor.id))
                    .is_none(),
                "source has no under-tile name/count label"
            );
            let icon = registry
                .get(
                    registry
                        .get_by_name(&format!("HouseEditorIcon{}", decor.id))
                        .unwrap(),
                )
                .unwrap();
            assert_eq!(
                (icon.width, icon.height),
                (Dimension::Fixed(87.0), Dimension::Fixed(87.0))
            );
            assert_eq!(
                (icon.position.left, icon.position.top),
                (
                    ui_toolkit::layout_values::Val::Px(5.0),
                    ui_toolkit::layout_values::Val::Px(5.0)
                )
            );
            let count = registry
                .get(registry.get_by_name(&format!("{prefix}Count")).unwrap())
                .unwrap();
            assert!(
                matches!(&count.widget_data,Some(WidgetData::FontString(text)) if text.text==quantity.quantity.to_string() && text.font_size==16.0 && text.shadow_color==Some([0.0,0.0,0.0,1.0]) && text.shadow_offset==[1.0,-1.0])
            );
            assert_eq!(count.height, Dimension::Fixed(20.0));
            assert_eq!(count.position.top, ui_toolkit::layout_values::Val::Px(70.0));
            assert_eq!(
                count.position.left,
                ui_toolkit::layout_values::Val::Px(88.0 - count.width.value())
            );
            let original = view.authority.clone();
            for (pressed, hovered, atlas, alpha) in [
                (false, false, "house-chest-list-item-default", 0.0),
                (false, true, "house-chest-list-item-default", 0.75),
                (true, true, "house-chest-list-item-pressed", 0.75),
                (false, false, "house-chest-list-item-default", 0.0),
            ] {
                let frame = registry
                    .get_mut(registry.get_by_name(&prefix).unwrap())
                    .unwrap();
                let Some(WidgetData::Button(button)) = frame.widget_data.as_mut() else {
                    panic!("storage tile must be native button")
                };
                button.state = if pressed {
                    ButtonState::Pushed
                } else {
                    ButtonState::Normal
                };
                button.hovered = hovered;
                apply_housing_postsetup(&view, &mut registry);
                for suffix in ["Background", "HoverBackground"] {
                    let frame = registry
                        .get(registry.get_by_name(&format!("{prefix}{suffix}")).unwrap())
                        .unwrap();
                    assert!(
                        matches!(&frame.widget_data,Some(WidgetData::Texture(texture)) if texture.source==TextureSource::Atlas(atlas.into()))
                    );
                    assert!(
                        matches!(&frame.widget_data,Some(WidgetData::Texture(texture)) if texture.tex_coords==[0.0,1.0,0.0,1.0]),
                        "member atlas state must not retain full-sheet UVs and crop twice"
                    );
                    let expected = if suffix == "HoverBackground" {
                        assert_eq!(frame.alpha, alpha);
                        BlendMode::Additive
                    } else {
                        BlendMode::Blend
                    };
                    assert!(
                        matches!(&frame.widget_data,Some(WidgetData::Texture(texture)) if texture.blend_mode==expected)
                    );
                }
                assert_eq!(
                    view.authority, original,
                    "visual tile input cannot spend/grant decor"
                );
            }
        }
    }
}

#[test]
fn housing_expert_source_instruction_states_and_assigned_controls_both_skins() {
    use game_engine_core::input_bindings_data::{BindingKey, InputAction, InputBinding};
    use game_engine_ui_model::housing::{HousingAuthority, HousingSurface};
    use game_engine_ui_model::housing_editor::{EditorMode, PrecisionSubmode};
    use shared::protocol::{DecorTransform, HouseLayout, HousingState, PlacedDecor};
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for submode in [
            PrecisionSubmode::Translate,
            PrecisionSubmode::Rotate,
            PrecisionSubmode::Scale,
        ] {
            for state in 0..3 {
                let authority = HousingAuthority {
                    state: Some(HousingState {
                        build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
                        layout: Some(HouseLayout {
                            house_id: 1,
                            can_edit: true,
                            decor: vec![PlacedDecor {
                                instance_id: 23,
                                decor_id: 1693,
                                transform: DecorTransform {
                                    position: [1.0, 2.0, 3.0],
                                    yaw: 0.0,
                                    pitch: 0.0,
                                    roll: 0.0,
                                    scale: 1.0,
                                },
                                dyes: vec![],
                            }],
                        }),
                        ..Default::default()
                    }),
                    error: None,
                };
                let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
                model.surface = HousingSurface::Editor;
                model.editor.set_mode(EditorMode::Expert);
                if state != 0 {
                    model.editor.select(&authority, 23).unwrap();
                }
                model.editor.scale_allowed = true;
                model.editor.set_submode(submode);
                if state == 2 {
                    model.editor.begin_mouse_drag(&authority).unwrap();
                }
                model.editor_bindings.assign(
                    InputAction::HousingExpertRotateRight,
                    InputBinding::Keyboard(BindingKey::F6),
                );
                let view = HousingView {
                    model,
                    authority,
                    viewport: [1280.0, 720.0],
                    money: None,
                    missing_thumbnails: Default::default(),
                    vendor_names: Default::default(),
                };
                let mut context = SharedContext::new();
                context.insert(view.clone());
                let mut registry = FrameRegistry::new(1280.0, 720.0);
                Screen::new(housing_screen).sync(&context, &mut registry);
                assert_eq!(
                    registry
                        .get_by_name("HouseEditorSelectInstruction")
                        .is_some(),
                    state == 0
                );
                for (name, mode) in [
                    ("MoveForward", PrecisionSubmode::Translate),
                    ("RotateRight", PrecisionSubmode::Rotate),
                    ("ScaleUp", PrecisionSubmode::Scale),
                ] {
                    assert_eq!(
                        registry
                            .get_by_name(&format!("HouseEditor{name}Instruction"))
                            .is_some(),
                        state == 1 && submode == mode
                    );
                }
                assert_eq!(
                    registry
                        .get_by_name("HouseEditorCancelInstruction")
                        .is_some(),
                    state != 0
                );
                assert_eq!(
                    registry
                        .get_by_name("HouseEditorRotateSnapInstruction")
                        .is_some(),
                    state != 0 && submode == PrecisionSubmode::Rotate
                );
                assert_eq!(
                    registry
                        .get_by_name("HouseEditorScaleSnapInstruction")
                        .is_some(),
                    state != 0 && submode == PrecisionSubmode::Scale
                );
                let container = registry
                    .get(registry.get_by_name("HouseEditorInstructions").unwrap())
                    .unwrap();
                assert_eq!(
                    container.position.left,
                    ui_toolkit::layout_values::Val::Px(1250.0 - container.width.value())
                );
                if state == 1 && submode == PrecisionSubmode::Rotate {
                    let row = registry
                        .get(
                            registry
                                .get_by_name("HouseEditorRotateRightInstruction")
                                .unwrap(),
                        )
                        .unwrap();
                    assert_eq!(row.height, Dimension::Fixed(45.0));
                    let label = registry
                        .get(
                            registry
                                .get_by_name("HouseEditorRotateRightInstructionText")
                                .unwrap(),
                        )
                        .unwrap();
                    assert!(
                        matches!(&label.widget_data,Some(WidgetData::FontString(font)) if font.text=="Rotate Right" && font.font_size==16.0)
                    );
                    let control = registry
                        .get(
                            registry
                                .get_by_name("HouseEditorRotateRightInstructionControlText")
                                .unwrap(),
                        )
                        .unwrap();
                    assert!(
                        matches!(&control.widget_data,Some(WidgetData::FontString(font)) if font.text=="F6")
                    );
                }
                let modes = registry
                    .get(registry.get_by_name("HouseEditorModeBar").unwrap())
                    .unwrap();
                assert_eq!(modes.width, Dimension::Fixed(266.0));
                assert!(
                    registry
                        .get_by_name("HouseEditorCustomizeModeButton")
                        .is_some()
                );
                let bar = registry
                    .get(registry.get_by_name("HouseEditorExpertSubmodeBar").unwrap())
                    .unwrap();
                assert_eq!(bar.width, Dimension::Fixed(168.0));
                assert_eq!(bar.position.top, ui_toolkit::layout_values::Val::Px(586.0));
            }
        }
    }
}

#[test]
fn ui_template_blend_housing_basic_source_controls_and_unbound_instruction_rows_both_skins() {
    use game_engine_ui_model::housing::HousingSurface;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
        model.surface = HousingSurface::Editor;
        model.editor.selected = Some(23);
        let view = HousingView {
            model,
            authority: Default::default(),
            viewport: [1280.0, 720.0],
            money: None,
            missing_thumbnails: Default::default(),
            vendor_names: Default::default(),
        };
        let mut context = SharedContext::new();
        context.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&context, &mut registry);
        for (name, text) in [
            ("Place", "Place Decor"),
            ("RotateLeft", "Rotate Left"),
            ("RotateRight", "Rotate Right"),
            ("Remove", "Remove Decor"),
            ("ToggleGrid", "Toggle Grid"),
            ("Cancel", "Cancel"),
        ] {
            let row = registry
                .get(
                    registry
                        .get_by_name(&format!("HouseEditor{name}Instruction"))
                        .expect("Retail instruction row required"),
                )
                .unwrap();
            assert_eq!(row.height, Dimension::Fixed(45.0));
            let frame = registry
                .get(
                    registry
                        .get_by_name(&format!("HouseEditor{name}InstructionText"))
                        .unwrap(),
                )
                .unwrap();
            let Some(WidgetData::FontString(font)) = &frame.widget_data else {
                panic!("Instruction label required")
            };
            assert_eq!(font.text, text);
            assert_eq!(font.font_size, 16.0);
            assert_eq!(font.shadow_color, Some([0.0, 0.0, 0.0, 1.0]));
            assert_eq!(font.shadow_offset, [1.0, -1.0]);
            if name != "Place" {
                let frame = registry
                    .get(
                        registry
                            .get_by_name(&format!("HouseEditor{name}InstructionControlText"))
                            .unwrap(),
                    )
                    .unwrap();
                let Some(WidgetData::FontString(font)) = &frame.widget_data else {
                    panic!("Binding label required")
                };
                assert_eq!(
                    font.font,
                    ui_toolkit::widgets::font_string::GameFont::ArialNarrow
                );
                assert_eq!(
                    font.text,
                    if name == "Cancel" {
                        "Escape"
                    } else {
                        "[UNBOUND]"
                    }
                );
            }
        }
        for name in [
            "BasicDecorMode",
            "ExpertDecorMode",
            "CleanupMode",
            "CustomizeMode",
            "Snap",
            "FreePlace",
        ] {
            let button = registry
                .get(
                    registry
                        .get_by_name(&format!("HouseEditor{name}Button"))
                        .unwrap(),
                )
                .unwrap();
            assert_eq!(button.width, Dimension::Fixed(64.0));
            assert_eq!(button.height, Dimension::Fixed(64.0));
            // HouseEditorModeButtons.xml33: only HoverIcon is ADD; Icon keeps BLEND.
            for (suffix, blend) in [
                ("Icon", ui_toolkit::widgets::texture::BlendMode::Blend),
                ("Hover", ui_toolkit::widgets::texture::BlendMode::Additive),
            ] {
                let art = registry
                    .get(
                        registry
                            .get_by_name(&format!("HouseEditor{name}Button{suffix}"))
                            .unwrap(),
                    )
                    .unwrap();
                assert!(
                    matches!(&art.widget_data,Some(WidgetData::Texture(texture)) if texture.blend_mode==blend),
                    "{skin:?} {name}{suffix}"
                );
            }
        }
        assert!(registry.get_by_name("HouseEditorRotateLeft").is_none());
        assert!(registry.get_by_name("HouseEditorDone").is_none());
        let gradient = registry
            .get(
                registry
                    .get_by_name("HouseEditorModeBarGradientAlpha")
                    .expect("Retail mode bar gradient required"),
            )
            .unwrap();
        assert_eq!(gradient.width, Dimension::Fixed(800.0));
        assert_eq!(gradient.height, Dimension::Fixed(260.0));
        assert_eq!(gradient.alpha, 0.5);
        let gradient = registry
            .get(
                registry
                    .get_by_name("HouseEditorModeBarGradientBackground")
                    .unwrap(),
            )
            .unwrap();
        assert!(
            matches!(&gradient.widget_data,Some(WidgetData::Texture(texture)) if texture.source==ui_toolkit::widgets::texture::TextureSource::FileDataId(5215694))
        );
    }
}
