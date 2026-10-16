//! HouseSettingsAccessOptionsTemplate XML4–32 and its Lua SetupAccessTypeDropdown.
use game_engine_core::housing::HousingCatalog;
use game_engine_ui_model::{
    housing::{HousingAuthority, HousingDashboard, HousingSurface},
    housing_component::{HousingView, housing_screen},
    housing_settings::HouseSettings,
};
use std::sync::Arc;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Frame, WidgetData},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widgets::button::ButtonState,
};

fn view() -> HousingView {
    let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
    model.surface = HousingSurface::HouseSettings;
    model.settings = Some(HouseSettings::new(41, Some(Default::default())));
    HousingView {
        model,
        authority: HousingAuthority::default(),
        viewport: [1920.0, 1080.0],
        money: None,
        missing_thumbnails: Default::default(),
        vendor_names: Default::default(),
    }
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

#[test]
fn house_access_source_dropdown_menu_and_staged_selection_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = view();
        let mut shared = SharedContext::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(housing_screen);
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        let dropdown = frame(&registry, "HousingSettingsHouseMode");
        assert_eq!(
            (dropdown.width.value(), dropdown.height.value()),
            (122.0, 25.0)
        );
        assert_eq!(dropdown.position.top, Val::Px(25.0));
        assert_eq!(dropdown.onclick.as_deref(), Some("housing:settings:menu"));
        assert!(
            registry
                .get_by_name("HousingSettingsHouseModeBackground")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("HousingSettingsHouseModeArrow")
                .is_some()
        );
        assert_eq!(
            frame(&registry, "HousingSettingsFlagfriendsRow")
                .position
                .top,
            Val::Px(162.0)
        );
        for (key, label) in [
            ("anyone", "Anyone"),
            ("none", "No One"),
            ("limited", "Limited"),
        ] {
            view.model
                .settings_request(&view.authority, "housing:settings:menu")
                .unwrap();
            shared.insert(view.clone());
            screen.sync(&shared, &mut registry);
            let menu = frame(&registry, "HousingSettingsAccessMenu");
            assert_eq!((menu.width.value(), menu.height.value()), (122.0, 83.0));
            assert_eq!(menu.position.top, Val::Px(50.0));
            let row = frame(&registry, &format!("HousingSettingsMode{key}"));
            assert_eq!((row.width.value(), row.height.value()), (106.0, 20.0));
            let action = row.onclick.as_ref().unwrap().clone();
            view.model
                .settings_request(&view.authority, &action)
                .unwrap();
            shared.insert(view.clone());
            screen.sync(&shared, &mut registry);
            assert!(registry.get_by_name("HousingSettingsAccessMenu").is_none());
            let Some(WidgetData::FontString(caption)) =
                &frame(&registry, "HousingSettingsHouseModeText").widget_data
            else {
                panic!("source caption missing");
            };
            assert_eq!(caption.text, label);
            assert!(
                view.authority.state.is_none(),
                "Mode changes remain draft-only"
            );
        }
        view.model.pending = true;
        shared.insert(view);
        screen.sync(&shared, &mut registry);
        let Some(WidgetData::Button(button)) =
            &frame(&registry, "HousingSettingsHouseMode").widget_data
        else {
            panic!("source dropdown must be a button");
        };
        assert_eq!(button.state, ButtonState::Disabled);
        assert!(
            frame(&registry, "HousingSettingsHouseMode")
                .onclick
                .as_deref()
                .unwrap_or("")
                .is_empty()
        );
    }
}

#[test]
fn house_settings_paints_source_header_foliage_and_divider_both_skins() {
    use ui_toolkit::atlas::{AtlasSource, resolve_region};
    use ui_toolkit::widgets::texture::TextureSource;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let foliage = resolve_region("housing-decorative-foliage-left", skin).unwrap();
        println!(
            "SOURCE_FOLIAGE {skin:?} {}x{} {:?}",
            foliage.width, foliage.height, foliage.source
        );
        let mut shared = SharedContext::new();
        shared.insert(view());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(housing_screen).sync(&shared, &mut registry);
        for (name, atlas, rect) in [
            (
                "HousingSettingsHeaderGradient",
                "housing-basic-panel-gradient-header-bg",
                (100.0, 0.0, 400.0, 80.0),
            ),
            (
                "HousingSettingsPlantDecoLeft",
                "housing-decorative-foliage-left",
                (6.0, 7.0, foliage.width, foliage.height),
            ),
            (
                "HousingSettingsSpacer",
                "Options_HorizontalDivider",
                (25.0, 174.0, 550.0, 1.0),
            ),
        ] {
            let source = resolve_region(atlas, skin).unwrap();
            let AtlasSource::FileDataId(fdid) = source.source else {
                panic!("source must resolve to installed DB2 file")
            };
            println!("SOURCE_REGION {skin:?} {atlas} fdid={fdid} rect={rect:?}");
            let region = frame(&registry, name);
            assert_eq!(
                (region.position.left, region.position.top),
                (Val::Px(rect.0), Val::Px(rect.1))
            );
            assert_eq!(
                (region.width.value(), region.height.value()),
                (rect.2, rect.3)
            );
            let Some(WidgetData::Texture(texture)) = &region.widget_data else {
                panic!("source decorative texture missing")
            };
            assert_eq!(texture.source, TextureSource::FileDataId(fdid));
            assert_eq!(
                texture.tex_coords,
                [source.left, source.right, source.top, source.bottom]
            );
        }
        for name in [
            "HousingSettingsClose",
            "HousingSettingsSave",
            "HousingSettingsHouseMode",
        ] {
            assert!(
                registry.get_by_name(name).is_some(),
                "{name} remains present"
            );
        }
    }
}

#[test]
fn house_settings_source_close_button_both_skins() {
    use ui_toolkit::widgets::texture::TextureSource;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for pending in [false, true] {
            let mut view = view();
            view.model.pending = pending;
            let mut shared = SharedContext::new();
            shared.insert(view);
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(housing_screen).sync(&shared, &mut registry);
            let close = frame(&registry, "HousingSettingsClose");
            assert_eq!((close.width.value(), close.height.value()), (24.0, 24.0));
            assert_eq!(
                (close.position.left, close.position.top),
                (Val::Px(577.0), Val::Px(0.0))
            );
            assert_eq!(close.frame_level, 510);
            assert_eq!(close.onclick.as_deref(), Some("housing:close"));
            let Some(WidgetData::Button(button)) = &close.widget_data else {
                panic!("HouseSettings close must be a button")
            };
            assert!(button.text.is_empty());
            assert!(!button.use_default_skin);
            assert!(button.enabled);
            assert_eq!(button.state, ButtonState::Normal);
            for (actual, atlas) in [
                (&button.normal_texture, "RedButton-Exit"),
                (&button.pushed_texture, "RedButton-exit-pressed"),
                (&button.disabled_texture, "RedButton-Exit-Disabled"),
                (&button.highlight_texture, "RedButton-Highlight"),
            ] {
                assert_eq!(*actual, Some(TextureSource::Atlas(atlas.into())));
            }
            assert_eq!(button.highlight_alpha, 1.0);
        }
    }
}

#[test]
fn house_settings_action_button_art_both_skins() {
    use ui_toolkit::widgets::texture::TextureSource;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for pending in [false, true] {
            let mut view = view();
            view.model.pending = pending;
            let mut shared = SharedContext::new();
            shared.insert(view);
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(housing_screen).sync(&shared, &mut registry);
            for (name, caption, left, top, disabled, action) in [
                (
                    "HousingSettingsSave",
                    "Save",
                    402.0,
                    488.0,
                    pending,
                    Some("housing:settings:save"),
                ),
                (
                    "HousingSettingsIgnoreList",
                    "Ignore List",
                    46.0,
                    488.0,
                    true,
                    None,
                ),
                (
                    "HousingSettingsAbandon",
                    "Abandon House",
                    401.0,
                    132.0,
                    true,
                    None,
                ),
            ] {
                let control = frame(&registry, name);
                assert_eq!(
                    (control.width.value(), control.height.value()),
                    (152.0, 28.0)
                );
                assert_eq!(
                    (control.position.left, control.position.top),
                    (Val::Px(left), Val::Px(top))
                );
                assert_eq!(control.onclick.as_deref(), action);
                let Some(WidgetData::Button(button)) = &control.widget_data else {
                    panic!("{name} must remain a button")
                };
                assert_eq!(button.text, caption);
                assert_eq!(
                    button.state,
                    if disabled {
                        ButtonState::Disabled
                    } else {
                        ButtonState::Normal
                    }
                );
                for (actual, fdid) in [
                    (&button.normal_texture, 130828),
                    (&button.pushed_texture, 130825),
                    (&button.disabled_texture, 130824),
                    (&button.highlight_texture, 130826),
                ] {
                    assert_eq!(*actual, Some(TextureSource::FileDataId(fdid)), "{name}");
                }
            }
        }
    }
}

#[test]
fn house_settings_checkbox_hit_area_excludes_labels_both_skins() {
    use game_engine_ui_model::housing_settings::AccessMode;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for mode in [AccessMode::Anyone, AccessMode::NoOne, AccessMode::Limited] {
            for pending in [false, true] {
                let mut view = view();
                view.model.settings.as_mut().unwrap().mode = mode;
                view.model.pending = pending;
                let mut shared = SharedContext::new();
                shared.insert(view);
                let mut registry = FrameRegistry::new(1920.0, 1080.0);
                Screen::new(housing_screen).sync(&shared, &mut registry);
                for (index, key) in ["neighbors", "guild", "friends", "party"]
                    .into_iter()
                    .enumerate()
                {
                    let name = format!("HousingSettingsFlag{key}");
                    let checkbox = frame(&registry, &name);
                    assert_eq!(
                        (checkbox.width.value(), checkbox.height.value()),
                        (30.0, 29.0)
                    );
                    assert_eq!(
                        (checkbox.position.left, checkbox.position.top),
                        (Val::Px(0.0), Val::Px(0.5))
                    );
                    assert_eq!(
                        checkbox.onclick.as_deref(),
                        Some(format!("housing:settings:toggle:{key}").as_str())
                    );
                    let row = frame(&registry, &format!("{name}Row"));
                    assert_eq!((row.width.value(), row.height.value()), (100.0, 30.0));
                    assert_eq!(
                        (row.position.left, row.position.top),
                        (Val::Px(20.0), Val::Px(74.0 + index as f32 * 44.0))
                    );
                    assert!(!row.mouse_enabled && row.onclick.is_none());
                    let label = frame(&registry, &format!("{name}Label"));
                    assert!(!label.mouse_enabled && label.onclick.is_none());
                    let Some(WidgetData::Button(button)) = &checkbox.widget_data else {
                        panic!("checkbox action target")
                    };
                    assert_eq!(
                        button.state,
                        if pending || mode != AccessMode::Limited {
                            ButtonState::Disabled
                        } else {
                            ButtonState::Normal
                        }
                    );
                }
            }
        }
    }
}
