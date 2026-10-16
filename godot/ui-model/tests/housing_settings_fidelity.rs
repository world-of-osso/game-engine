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
            frame(&registry, "HousingSettingsFlagfriends").position.top,
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
