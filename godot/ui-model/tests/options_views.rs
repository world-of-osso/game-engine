use game_engine_ui_model::GameMenuModel;
use game_engine_ui_model::game_menu_component::{
    GameMenuView, GameMenuViewModel, game_menu_screen,
};
use game_engine_ui_model::input_bindings::{BindingSection, InputAction};
use game_engine_ui_model::nameplate_style::NameplateStyle;
use game_engine_ui_model::options_menu_component::{
    CameraOptionsView, GraphicsOptionsView, HudOptionsView, KeybindingRowView, KeybindingsView,
    OptionsCategory, OptionsViewModel, SoundOptionsView,
};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn model() -> GameMenuViewModel {
    GameMenuViewModel {
        logged_in: true,
        view: GameMenuView::Options,
        options: OptionsViewModel {
            category: OptionsCategory::Sound,
            position: [0.0, 0.0],
            graphics: GraphicsOptionsView {
                particle_density: 75.0,
                render_scale: 1.0,
                ui_scale: 1.0,
                vsync_enabled: false,
                frame_rate_limit_enabled: true,
                frame_rate_limit: 90.0,
                colorblind_mode: false,
                bloom_enabled: false,
                bloom_intensity: 0.08,
            },
            sound: SoundOptionsView {
                muted: false,
                music_enabled: true,
                master_volume: 0.8,
                music_volume: 0.4,
                ambient_volume: 0.3,
                effects_volume: 0.6,
            },
            camera: CameraOptionsView {
                mouse_sensitivity: 0.003,
                look_sensitivity: 0.01,
                invert_y: false,
                fov_degrees: 90.0,
                zoom_speed: 8.0,
                follow_speed: 10.0,
                min_distance: 2.0,
                max_distance: 40.0,
            },
            hud: HudOptionsView {
                show_minimap: true,
                show_action_bars: true,
                show_nameplates: true,
                nameplate_distance: 40.0,
                nameplate_style: NameplateStyle::default(),
                show_health_bars: true,
                show_target_marker: true,
                auto_loot: false,
                personal_resource_display: false,
                soft_target_interact: false,
                interact_key_icons:
                    game_engine_ui_model::soft_target_data::InteractKeyIcons::Default,
                show_fps_overlay: true,
                chat_font_size: 10.0,
                status_text_display: Default::default(),
            },
            bindings: KeybindingsView {
                section: BindingSection::Movement,
                capture_action: None,
                rows: vec![KeybindingRowView {
                    action: InputAction::MoveForward,
                    label: "Move Forward".into(),
                    binding_text: "W".into(),
                    capturing: false,
                    can_clear: true,
                }],
            },
        },
    }
}

fn label(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    match frame.widget_data.as_ref().unwrap() {
        WidgetData::FontString(data) => data.text.clone(),
        WidgetData::Button(data) => data.text.clone(),
        other => panic!("{name}: expected visible text, got {other:?}"),
    }
}

#[test]
fn options_categories_emit_original_actions_and_replace_visible_section() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    let mut screen = Screen::new(game_menu_screen);
    let mut view = model();
    let categories = [
        (OptionsCategory::Graphics, "Particle Density"),
        (OptionsCategory::Sound, "Master Volume"),
        (OptionsCategory::Camera, "Mouse Sensitivity"),
        (OptionsCategory::Interface, "Chat Font Size"),
        (OptionsCategory::Hud, "Show Minimap"),
        (OptionsCategory::Nameplates, "Border"),
        (OptionsCategory::Controls, "Mouse Turn Style"),
        (OptionsCategory::Accessibility, "UI Scale"),
        (OptionsCategory::Keybindings, "Move Forward"),
        (OptionsCategory::Macros, "General Macros"),
        (OptionsCategory::SocialAddons, "Addon Directory"),
        (OptionsCategory::Advanced, "Show FPS Overlay"),
        (OptionsCategory::Support, "About"),
    ];
    assert_eq!(categories.len(), OptionsCategory::ALL.len());
    for (category, body_label) in categories {
        view.options.category = category;
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        assert_eq!(label(&registry, "OptionsSectionTitle"), category.title());
        assert!(registry.frames_iter().any(|frame| {
            matches!(&frame.widget_data, Some(WidgetData::FontString(data)) if data.text == body_label)
        }), "Missing {category:?} content: {body_label}");
        for tab in OptionsCategory::ALL {
            let id = registry
                .get_by_name(&format!("OptionsTab{}", tab.key()))
                .unwrap();
            assert_eq!(
                registry.click_frame(id),
                Some(format!("options_category:{}", tab.key()))
            );
        }
        let root = registry
            .get(registry.get_by_name("OptionsRoot").unwrap())
            .unwrap();
        assert_eq!(root.width, Dimension::Fixed(980.0));
        assert!(registry.get_by_name("MenuBtnResume").is_none());
    }
    view.view = GameMenuView::MainMenu;
    shared.insert(view);
    screen.sync(&shared, &mut registry);
    assert!(registry.get_by_name("OptionsRoot").is_none());
    assert_eq!(label(&registry, "MenuBtnOptions"), "Options");
    let addons = registry.get_by_name("MenuBtnAddons").unwrap();
    assert_eq!(registry.click_frame(addons).as_deref(), Some("menu_addons"));
}

#[test]
fn native_game_menu_model_projects_authored_options_and_reactive_slider_values() {
    let mut view = model();
    let mut native = GameMenuModel::from_view(1280.0, 720.0, view.clone());
    native.sync();
    let slider = native.registry.get_by_name("Slidermaster_volume").unwrap();
    assert!(matches!(
        native.registry.get(slider).unwrap().widget_data,
        Some(WidgetData::Slider(_))
    ));
    assert_eq!(label(&native.registry, "SliderValuemaster_volume"), "0.80");
    assert!(
        native
            .registry
            .get(native.registry.get_by_name("OptionsTabPanel").unwrap())
            .unwrap()
            .nine_slice
            .is_some()
    );
    view.options.sound.master_volume = 0.25;
    native.shared.insert(view);
    native.sync();
    assert_eq!(label(&native.registry, "SliderValuemaster_volume"), "0.25");
}

#[test]
fn sound_values_update_without_replacing_the_screen() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    let mut screen = Screen::new(game_menu_screen);
    let mut view = model();
    shared.insert(view.clone());
    screen.sync(&shared, &mut registry);
    let slider = registry.get_by_name("Slidermaster_volume").unwrap();
    assert_eq!(
        registry.click_frame(slider).as_deref(),
        Some("options_slider:master_volume")
    );
    assert_eq!(label(&registry, "SliderValuemaster_volume"), "0.80");
    view.options.sound.master_volume = 0.25;
    shared.insert(view);
    screen.sync(&shared, &mut registry);
    assert_eq!(label(&registry, "SliderValuemaster_volume"), "0.25");
}

/// Interface → Display "Status Text": Numeric Value, Percentage, Both, None; the current
/// choice is lit and the others click `options_toggle:status_text_display:<value>`.
#[test]
fn interface_status_text_row_offers_the_four_retail_choices() {
    use game_engine_ui_model::status_text_data::StatusTextDisplay;
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    let mut view = model();
    view.options.category = OptionsCategory::Interface;
    view.options.hud.status_text_display = StatusTextDisplay::Both;
    shared.insert(view);
    Screen::new(game_menu_screen).sync(&shared, &mut registry);
    assert_eq!(
        label(&registry, "ChoiceLabelstatus_text_display"),
        "Status Text"
    );
    let labels: Vec<String> = (1..=4)
        .map(|value| label(&registry, &format!("Choicestatus_text_display{value}Label")))
        .collect();
    assert_eq!(labels, ["Numeric Value", "Percentage", "Both", "None"]);
    assert!(
        registry
            .get_by_name("Choicestatus_text_display3Hit")
            .is_none()
    );
    let hit = registry
        .get_by_name("Choicestatus_text_display2Hit")
        .unwrap();
    assert_eq!(
        registry.click_frame(hit),
        Some("options_toggle:status_text_display:2".into())
    );
}
