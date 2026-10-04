use game_engine_ui_model::GameMenuModel;
use game_engine_ui_model::game_menu_component::{
    GameMenuView, GameMenuViewModel, game_menu_screen,
};
use game_engine_ui_model::input_bindings::{BindingSection, InputAction};
use game_engine_ui_model::nameplate_style::NameplateStyle;
use game_engine_core::ui_layout_data::{
    LayoutFont, LayoutSettings, LayoutSkin, UnitFrameSettings,
};
use game_engine_ui_model::options_menu_component::{
    CameraOptionsView, GraphicsOptionsView, HudOptionsView, KeybindingRowView, KeybindingsView,
    LayoutOptionsView, LayoutSystem, OptionsCategory, OptionsViewModel, SoundOptionsView,
};
use game_engine_ui_model::options_menu_data::{
    LayoutAction, SliderField, apply_layout_action, apply_layout_slider, parse_layout_action,
    parse_slider_action,
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
            layout: Default::default(),
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

fn hud_registry(layout: &LayoutOptionsView) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut view = model();
    view.options.category = OptionsCategory::Hud;
    view.options.layout = layout.clone();
    shared.insert(view);
    Screen::new(game_menu_screen).sync(&shared, &mut registry);
    registry
}

/// The action a click on `name` emits.
fn click(registry: &mut FrameRegistry, name: &str) -> String {
    let id = registry.get_by_name(name).expect(name);
    registry.click_frame(id).expect(name)
}

/// The HUD "Layout" dropdown lists the system presets and then the player layouts; the
/// active one is lit and each other carries the action that selects it.
#[test]
fn hud_layout_row_offers_the_presets_and_the_player_layouts() {
    let layout = LayoutOptionsView {
        active: "Layout 1".into(),
        names: ["Modern", "Forever", "Layout 1", "Layout 2"]
            .map(String::from)
            .to_vec(),
        ..Default::default()
    };
    let mut registry = hud_registry(&layout);
    assert_eq!(label(&registry, "ChoiceLabelui_layout"), "Layout");
    let labels: Vec<String> = (0..4)
        .map(|value| label(&registry, &format!("Choiceui_layout{value}Label")))
        .collect();
    assert_eq!(labels, layout.names);
    assert!(registry.get_by_name("Choiceui_layout2Hit").is_none());
    for (index, name) in [(0, "Modern"), (1, "Forever"), (3, "Layout 2")] {
        let action = click(&mut registry, &format!("Choiceui_layout{index}Hit"));
        assert_eq!(parse_layout_action(&action), Some(LayoutAction::Select(index)));
        assert_eq!(layout.names[index], name);
    }
    assert_eq!(parse_layout_action("options_toggle:ui_layout:x"), None);
}

/// "Layout Settings": the selector shows one system's controls; each control emits the
/// action that writes its setting, and the page then shows the stored value.
#[test]
fn hud_layout_settings_controls_edit_the_selected_system() {
    let mut layout = LayoutOptionsView::default();
    let mut registry = hud_registry(&layout);
    assert_eq!(label(&registry, "ChoiceLabellayout_system"), "Layout Settings");
    let systems: Vec<String> = (0..6)
        .map(|value| label(&registry, &format!("Choicelayout_system{value}Label")))
        .collect();
    assert_eq!(systems, ["Player", "Target", "Focus", "Pet", "Chat", "Meter"]);
    // The player frame shows first: Frame Size and Text Size at 100 %, Friz Quadrata lit.
    assert_eq!(label(&registry, "NameplateLabellayout_frame_size"), "Frame Size");
    assert_eq!(label(&registry, "SliderValuelayout_frame_size"), "100%");
    assert_eq!(label(&registry, "SliderValuelayout_text_size"), "100%");
    assert_eq!(label(&registry, "Choicelayout_font0Label"), "Friz Quadrata");
    assert!(registry.get_by_name("Choicelayout_font0Hit").is_none());
    assert!(registry.get_by_name("Sliderlayout_chat_width").is_none());

    // Target frame: size 148 snaps to the 5 % step, text 126 to the 10 % step, Arial Narrow.
    let action = click(&mut registry, "Choicelayout_system1Hit");
    apply_layout_action(parse_layout_action(&action).unwrap(), &mut layout);
    assert_eq!(layout.system, LayoutSystem::TargetFrame);
    let mut registry = hud_registry(&layout);
    for (slider, value) in [("Sliderlayout_frame_size", 148.0), ("Sliderlayout_text_size", 126.0)]
    {
        let action = click(&mut registry, slider);
        let Some(SliderField::Layout(slider)) = parse_slider_action(&action) else {
            panic!("{action} is not a layout slider");
        };
        apply_layout_slider(slider, value, &mut layout);
    }
    let action = click(&mut registry, "Choicelayout_font1Hit");
    apply_layout_action(parse_layout_action(&action).unwrap(), &mut layout);
    let target = UnitFrameSettings {
        frame_size: Some(150),
        font: Some(LayoutFont::ArialNarrow),
        text_size: Some(130),
    };
    let expected = LayoutSettings {
        target_frame: target,
        ..Default::default()
    };
    assert_eq!(layout.settings, expected);
    let registry = hud_registry(&layout);
    assert_eq!(label(&registry, "SliderValuelayout_frame_size"), "150%");
    assert_eq!(label(&registry, "SliderValuelayout_text_size"), "130%");
    assert!(registry.get_by_name("Choicelayout_font1Hit").is_none());
    // The player frame still shows its own, unchanged values.
    layout.system = LayoutSystem::PlayerFrame;
    assert_eq!(
        label(&hud_registry(&layout), "SliderValuelayout_frame_size"),
        "100%"
    );

    // Chat and meter show their preset size until changed; Forever's differs from Modern's.
    for (system, keys, sizes) in [
        (LayoutSystem::ChatFrame, ["chat_width", "chat_height"], [640.0, 360.0]),
        (LayoutSystem::DamageMeter, ["meter_width", "meter_height"], [550.0, 300.0]),
    ] {
        layout.system = system;
        let mut registry = hud_registry(&layout);
        let preset = label(&registry, &format!("SliderValuelayout_{}", keys[0]));
        layout.skin = LayoutSkin::Forever;
        assert_ne!(
            label(&hud_registry(&layout), &format!("SliderValuelayout_{}", keys[0])),
            preset
        );
        layout.skin = LayoutSkin::Modern;
        assert!(registry.get_by_name("Sliderlayout_frame_size").is_none());
        assert!(registry.get_by_name("Choicelayout_font").is_none());
        for (key, size) in keys.into_iter().zip(sizes) {
            let action = click(&mut registry, &format!("Sliderlayout_{key}"));
            let Some(SliderField::Layout(slider)) = parse_slider_action(&action) else {
                panic!("{action} is not a layout slider");
            };
            apply_layout_slider(slider, size, &mut layout);
            assert_eq!(
                label(&hud_registry(&layout), &format!("SliderValuelayout_{key}")),
                format!("{size:.0}")
            );
        }
    }
    assert_eq!(layout.settings.chat.width, Some(640));
    assert_eq!(layout.settings.chat.height, Some(360));
    assert_eq!(layout.settings.damage_meter.width, Some(550));
    assert_eq!(layout.settings.damage_meter.height, Some(300));
    assert_eq!(layout.settings.target_frame, target);

    // "Reset to Preset" clears every setting and keeps the shown system.
    let mut registry = hud_registry(&layout);
    assert_eq!(label(&registry, "RowLabelreset_layout_settings"), "Reset to Preset");
    let action = click(&mut registry, "ActionButtonreset_layout_settings");
    assert_eq!(parse_layout_action(&action), Some(LayoutAction::Reset));
    apply_layout_action(LayoutAction::Reset, &mut layout);
    assert_eq!(layout.settings, LayoutSettings::default());
    assert_eq!(layout.system, LayoutSystem::DamageMeter);
}
