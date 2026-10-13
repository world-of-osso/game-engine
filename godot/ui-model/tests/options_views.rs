use game_engine_core::ui_layout_data::{LayoutFont, LayoutSettings, LayoutSkin, UnitFrameSettings};
use game_engine_ui_model::GameMenuModel;
use game_engine_ui_model::game_menu_component::{
    GameMenuView, GameMenuViewModel, game_menu_screen,
};
use game_engine_ui_model::input_bindings::{BindingSection, InputAction};
use game_engine_ui_model::nameplate_style::NameplateStyle;
use game_engine_ui_model::options_menu_component::{
    CameraOptionsView, GraphicsOptionsView, HudOptionsView, KeybindingRowView, KeybindingsView,
    LayoutOptionsView, LayoutSystem, OptionsCategory, OptionsViewModel, SoundOptionsView,
};
use game_engine_ui_model::options_menu_data::{
    LayoutAction, SliderField, apply_layout_action, apply_layout_slider, parse_layout_action,
    parse_slider_action,
};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn model() -> GameMenuViewModel {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    GameMenuViewModel {
        logged_in: true,
        view: GameMenuView::Options,
        options: OptionsViewModel {
            category: OptionsCategory::Sound,
            search: String::new(),
            addons_tab: false,
            pending_changes: false,
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
                ssao_enabled: false,
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
                cooldown_assist: Default::default(),
                extra_action_bars: Default::default(),
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
                encounter_timeline: true,
                encounter_bars: true,
                encounter_warnings: true,
                encounter_sounds: true,
                show_fps_overlay: true,
                chat_font_size: 10.0,
                status_text_display: Default::default(),
            },
            bindings: KeybindingsView {
                section: BindingSection::Movement,
                capture_action: None,
                output: None,
                rows: vec![KeybindingRowView {
                    action: InputAction::MoveForward,
                    label: "Move Forward".into(),
                    binding_text: Some("W".into()),
                    capturing: false,
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
        // The page's first row: Particle Density is below the scrolled area.
        (OptionsCategory::Graphics, "Vertical Sync"),
        (OptionsCategory::Sound, "Master Volume"),
        (OptionsCategory::Camera, "Mouse Sensitivity"),
        (OptionsCategory::Interface, "Chat Font Size"),
        (OptionsCategory::Hud, "Show Minimap"),
        (OptionsCategory::ActionBars, "Action Bar 2"),
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
        view.options.addons_tab = category == OptionsCategory::SocialAddons;
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        assert_eq!(label(&registry, "OptionsSectionTitle"), category.title());
        assert!(registry.frames_iter().any(|frame| {
            matches!(&frame.widget_data, Some(WidgetData::FontString(data)) if data.text == body_label)
        }), "Missing {category:?} content: {body_label}");
        for tab in OptionsCategory::ALL
            .into_iter()
            .filter(|tab| (*tab == OptionsCategory::SocialAddons) == view.options.addons_tab)
        {
            let id = registry
                .get_by_name(&format!("OptionsTab{}", tab.key()))
                .unwrap();
            assert_eq!(
                registry.click_frame(id),
                Some(format!("options_category:{}", tab.key()))
            );
        }
        assert!(registry.get_by_name("OptionsRoot").is_some());
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
fn ssao_graphics_row_discloses_restart_and_emits_toggle_action() {
    let mut view = model();
    view.options.category = OptionsCategory::Graphics;
    let mut native = GameMenuModel::from_view(1280.0, 720.0, view);
    native.sync();
    assert_eq!(
        label(&native.registry, "ToggleLabelssao_enabled"),
        "SSAO (Requires Restart)"
    );
    let on = native
        .registry
        .get_by_name("ToggleSwitchssao_enabled")
        .unwrap();
    assert_eq!(
        native.registry.click_frame(on).as_deref(),
        Some("options_toggle:ssao_enabled")
    );
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
    let category = native
        .registry
        .get(native.registry.get_by_name("OptionsTabsound").unwrap())
        .unwrap();
    assert_eq!(category.width, ui_toolkit::frame::Dimension::Fixed(175.0));
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
    shared.insert(popup_state("status_text_display"));
    Screen::new(game_menu_screen).sync(&shared, &mut registry);
    assert_eq!(
        label(&registry, "ChoiceLabelstatus_text_display"),
        "Status Text"
    );
    let labels: Vec<String> = (1..=4)
        .map(|value| {
            label(
                &registry,
                &format!("Choicestatus_text_display{value}HitText"),
            )
        })
        .collect();
    assert_eq!(labels, ["Numeric Value", "Percentage", "Both", "None"]);
    assert!(
        registry
            .get_by_name("Choicestatus_text_display3HitTick")
            .is_some()
    );
    let hit = registry
        .get_by_name("Choicestatus_text_display2Hit")
        .unwrap();
    assert_eq!(
        registry.click_frame(hit),
        Some("options_toggle:status_text_display:2".into())
    );
}

fn popup_state(key: &str) -> game_engine_ui_model::options_dropdown::OptionsDropdownState {
    let mut state = game_engine_ui_model::options_dropdown::OptionsDropdownState::default();
    state.toggle(key, [320.0, 180.0]);
    state
}

fn hud_registry(layout: &LayoutOptionsView, open_key: Option<&str>) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut view = model();
    view.options.category = OptionsCategory::Hud;
    view.options.layout = layout.clone();
    shared.insert(view);
    if let Some(key) = open_key {
        shared.insert(popup_state(key));
    }
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
    let mut registry = hud_registry(&layout, Some("ui_layout"));
    assert_eq!(label(&registry, "ChoiceLabelui_layout"), "Layout");
    let labels: Vec<String> = (0..4)
        .map(|value| label(&registry, &format!("Choiceui_layout{value}HitText")))
        .collect();
    assert_eq!(labels, layout.names);
    assert!(registry.get_by_name("Choiceui_layout2HitTick").is_some());
    for (index, name) in [(0, "Modern"), (1, "Forever"), (3, "Layout 2")] {
        let action = click(&mut registry, &format!("Choiceui_layout{index}Hit"));
        assert_eq!(
            parse_layout_action(&action),
            Some(LayoutAction::Select(index))
        );
        assert_eq!(layout.names[index], name);
    }
    assert_eq!(parse_layout_action("options_toggle:ui_layout:x"), None);
}

#[test]
fn hud_layout_offers_show_micro_menu_off_by_default_and_applies_its_click() {
    let mut layout = LayoutOptionsView::default();
    for skin in [LayoutSkin::Modern, LayoutSkin::Forever] {
        layout.skin = skin;
        for side in ["Right", "Left"] {
            let mut registry = hud_registry(&layout, None);
            assert_eq!(
                layout.settings.show_micro_menu.unwrap_or(false),
                side == "Left"
            );
            assert_eq!(
                label(&registry, "ToggleLabellayout_show_micro_menu"),
                "Show Micro Menu"
            );
            let action = click(&mut registry, "ToggleSwitchlayout_show_micro_menu");
            let parsed = parse_layout_action(&action).expect("layout toggle action");
            apply_layout_action(parsed, &mut layout);
        }
    }
}

/// "Layout Settings": the selector shows one system's controls; each control emits the
/// action that writes its setting, and the page then shows the stored value.
#[test]
fn hud_layout_settings_controls_edit_the_selected_system() {
    let mut layout = LayoutOptionsView::default();
    let mut registry = hud_registry(&layout, Some("layout_system"));
    assert_eq!(
        label(&registry, "ChoiceLabellayout_system"),
        "Layout Settings"
    );
    let systems: Vec<String> = (0..6)
        .map(|value| label(&registry, &format!("Choicelayout_system{value}HitText")))
        .collect();
    assert_eq!(
        systems,
        ["Player", "Target", "Focus", "Pet", "Chat", "Meter"]
    );
    // The player frame shows first: Frame Size and Text Size at 100 %, Friz Quadrata lit.
    assert_eq!(
        label(&registry, "NameplateLabellayout_frame_size"),
        "Frame Size"
    );
    assert_eq!(label(&registry, "SliderValuelayout_frame_size"), "100%");
    assert_eq!(label(&registry, "SliderValuelayout_text_size"), "100%");
    assert_eq!(label(&registry, "Choicelayout_fontText"), "Friz Quadrata");
    assert!(registry.get_by_name("Choicelayout_font0Hit").is_none());
    assert!(registry.get_by_name("Sliderlayout_chat_width").is_none());

    // Target frame: size 148 snaps to the 5 % step, text 126 to the 10 % step, Arial Narrow.
    let action = click(&mut registry, "Choicelayout_system1Hit");
    apply_layout_action(parse_layout_action(&action).unwrap(), &mut layout);
    assert_eq!(layout.system, LayoutSystem::TargetFrame);
    let mut registry = hud_registry(&layout, Some("layout_font"));
    for (slider, value) in [
        ("Sliderlayout_frame_size", 148.0),
        ("Sliderlayout_text_size", 126.0),
    ] {
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
    let registry = hud_registry(&layout, None);
    assert_eq!(label(&registry, "SliderValuelayout_frame_size"), "150%");
    assert_eq!(label(&registry, "SliderValuelayout_text_size"), "130%");
    assert_eq!(label(&registry, "Choicelayout_fontText"), "Arial Narrow");
    // The player frame still shows its own, unchanged values.
    layout.system = LayoutSystem::PlayerFrame;
    assert_eq!(
        label(&hud_registry(&layout, None), "SliderValuelayout_frame_size"),
        "100%"
    );

    // Chat and meter show their preset size until changed; Forever's differs from Modern's.
    for (system, keys, sizes) in [
        (
            LayoutSystem::ChatFrame,
            ["chat_width", "chat_height"],
            [640.0, 360.0],
        ),
        (
            LayoutSystem::DamageMeter,
            ["meter_width", "meter_height"],
            [550.0, 300.0],
        ),
    ] {
        layout.system = system;
        let mut registry = hud_registry(&layout, None);
        let preset = label(&registry, &format!("SliderValuelayout_{}", keys[0]));
        layout.skin = LayoutSkin::Forever;
        assert_ne!(
            label(
                &hud_registry(&layout, None),
                &format!("SliderValuelayout_{}", keys[0])
            ),
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
                label(
                    &hud_registry(&layout, None),
                    &format!("SliderValuelayout_{key}")
                ),
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
    let mut registry = hud_registry(&layout, None);
    assert_eq!(
        label(&registry, "RowLabelreset_layout_settings"),
        "Reset to Preset"
    );
    let action = click(&mut registry, "ActionButtonreset_layout_settings");
    assert_eq!(parse_layout_action(&action), Some(LayoutAction::Reset));
    apply_layout_action(LayoutAction::Reset, &mut layout);
    assert_eq!(layout.settings, LayoutSettings::default());
    assert_eq!(layout.system, LayoutSystem::DamageMeter);
}

/// Retail's settings panel is titled SETTINGS_TITLE "Options"
/// (Blizzard_Settings_Shared/Blizzard_SettingsPanel.lua:54); the main menu keeps "Game Menu".
#[test]
fn options_view_is_titled_options_and_main_menu_game_menu() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    let mut screen = Screen::new(game_menu_screen);
    shared.insert(model());
    screen.sync(&shared, &mut registry);
    assert_eq!(label(&registry, "OptionsTitleLabel"), "Options");

    let mut main = model();
    main.view = GameMenuView::MainMenu;
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    let mut screen = Screen::new(game_menu_screen);
    shared.insert(main);
    screen.sync(&shared, &mut registry);
    assert_eq!(label(&registry, "GameMenuTitleLabel"), "Game Menu");
}

/// Options > Nameplates has a "Show Health Value" row, off by default: only its On side is
/// clickable; with the value shown only its Off side is.
#[test]
fn nameplates_page_offers_show_health_value_off_by_default() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut screen = Screen::new(game_menu_screen);
    let mut view = model();
    view.options.category = OptionsCategory::Nameplates;
    shared.insert(view.clone());
    screen.sync(&shared, &mut registry);
    assert_eq!(
        label(&registry, "ToggleLabelnameplate_show_health_value"),
        "Show Health Value"
    );
    assert_eq!(
        click(&mut registry, "ToggleSwitchnameplate_show_health_value"),
        "options_toggle:nameplate_show_health_value"
    );
    let checked = registry
        .get_by_name("ToggleSwitchnameplate_show_health_valueChecked")
        .unwrap();
    assert!(registry.get(checked).unwrap().hidden);
    view.options.hud.nameplate_style.show_health_value = true;
    shared.insert(view);
    screen.sync(&shared, &mut registry);
    let checked = registry
        .get_by_name("ToggleSwitchnameplate_show_health_valueChecked")
        .unwrap();
    assert!(!registry.get(checked).unwrap().hidden);
    assert_eq!(
        click(&mut registry, "ToggleSwitchnameplate_show_health_value"),
        "options_toggle:nameplate_show_health_value"
    );
}

#[test]
fn settings_panel_and_category_rows_match_retail_xml() {
    let mut menu = GameMenuModel::from_view(1280.0, 720.0, model());
    menu.sync();
    let root = menu
        .registry
        .get(menu.registry.get_by_name("OptionsRoot").unwrap())
        .unwrap();
    assert_eq!(
        (root.width, root.height),
        (
            ui_toolkit::frame::Dimension::Fixed(920.0),
            ui_toolkit::frame::Dimension::Fixed(724.0)
        )
    );
    for name in ["OptionsGameTab", "OptionsAddOnsTab", "OptionsSearchBox"] {
        assert!(menu.registry.get_by_name(name).is_some(), "{name} missing");
    }
    for category in OptionsCategory::ALL
        .into_iter()
        .filter(|category| *category != OptionsCategory::SocialAddons)
    {
        let row = menu
            .registry
            .get(
                menu.registry
                    .get_by_name(&format!("OptionsTab{}", category.key()))
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            (row.width, row.height),
            (
                ui_toolkit::frame::Dimension::Fixed(175.0),
                ui_toolkit::frame::Dimension::Fixed(20.0)
            )
        );
        assert_eq!(
            row.onclick.as_deref(),
            Some(format!("options_category:{}", category.key()).as_str())
        );
    }
}

#[test]
fn settings_header_geometry_matches_settings_panel_xml_in_both_skins() {
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        frame::Dimension,
        layout_values::Val,
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut menu = GameMenuModel::from_view(1920.0, 1080.0, model());
        menu.sync();
        let frame = |name: &str| {
            menu.registry
                .get(menu.registry.get_by_name(name).unwrap())
                .unwrap()
        };
        let game = frame("OptionsGameTab");
        let addons = frame("OptionsAddOnsTab");
        assert_eq!(game.position.left, Val::Px(32.0));
        assert_eq!(game.position.top, Val::Px(27.0));
        assert_eq!(game.height, Dimension::Fixed(37.0));
        let Dimension::Fixed(game_width) = game.width else {
            panic!("fixed tab width")
        };
        assert_eq!(addons.position.left, Val::Px(32.0 + game_width + 5.0));
        assert_eq!(addons.position.top, Val::Px(27.0));
        assert_eq!(addons.height, Dimension::Fixed(37.0));
        let search = frame("OptionsSearchBox");
        assert_eq!(
            (search.width, search.height),
            (Dimension::Fixed(350.0), Dimension::Fixed(22.0))
        );
        assert_eq!(
            (search.position.left, search.position.top),
            (Val::Px(552.0), Val::Px(34.0))
        );
        // Container right920-22; SearchBox BOTTOMRIGHT relative TOPRIGHT +(4,20).
        assert_eq!(552.0 + 350.0, 920.0 - 22.0 + 4.0);
        assert_eq!(34.0 + 22.0, 76.0 - 20.0);
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn settings_search_filters_categories_and_setting_rows_case_insensitively() {
    let mut view = model();
    view.options.search = "mUsIc".into();
    let mut menu = GameMenuModel::from_view(1920.0, 1080.0, view.clone());
    menu.sync();
    assert!(menu.registry.get_by_name("OptionsTabsound").is_some());
    assert!(menu.registry.get_by_name("OptionsTabcamera").is_none());
    assert!(
        menu.registry
            .frames_iter()
            .any(|frame| matches!(&frame.widget_data,
        Some(WidgetData::FontString(data)) if data.text == "Music Volume"))
    );
    assert!(
        !menu
            .registry
            .frames_iter()
            .any(|frame| matches!(&frame.widget_data,
        Some(WidgetData::FontString(data)) if data.text == "Master Volume"))
    );
    view.options.search = "no settings match this".into();
    let mut empty = GameMenuModel::from_view(1920.0, 1080.0, view);
    empty.sync();
    assert!(empty.registry.get_by_name("OptionsNoResults").is_some());
    assert!(empty.registry.get_by_name("OptionsTabsound").is_none());
}

#[test]
fn settings_search_shared_controls_are_not_duplicated_across_categories() {
    let mut view = model();
    view.options.search = "UI Scale".into();
    let mut menu = GameMenuModel::from_view(1920.0, 1080.0, view);
    menu.sync();
    let count = menu
        .registry
        .frames_iter()
        .filter(|frame| {
            matches!(&frame.widget_data,
        Some(WidgetData::FontString(data)) if data.text == "UI Scale")
        })
        .count();
    assert_eq!(
        count, 1,
        "one shared setting appears once in search results"
    );
}

#[test]
fn settings_footer_has_retail_close_and_hides_apply_without_pending_settings() {
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        frame::Dimension,
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut menu = GameMenuModel::from_view(1920.0, 1080.0, model());
        menu.sync();
        let close = menu
            .registry
            .get_by_name("OptionsCloseButton")
            .expect("Retail CloseButton");
        let frame = menu.registry.get(close).unwrap();
        assert_eq!(
            (frame.width, frame.height),
            (Dimension::Fixed(96.0), Dimension::Fixed(22.0))
        );
        assert_eq!(label(&menu.registry, "OptionsCloseButton"), "Close");
        assert_eq!(
            menu.registry.click_frame(close).as_deref(),
            Some("options_close")
        );
        let apply = menu
            .registry
            .get_by_name("OptionsApplyButton")
            .expect("Retail ApplyButton");
        assert!(menu.registry.get(apply).unwrap().hidden);
        assert_eq!(menu.registry.click_frame(apply), None);
        let defaults = menu
            .registry
            .get_by_name("OptionsDefaultsButton")
            .expect("Retail list header DefaultsButton");
        let parent = menu.registry.get(defaults).unwrap().parent_id.unwrap();
        assert_eq!(
            menu.registry.get(parent).unwrap().name.as_deref(),
            Some("OptionsDefaultsMount")
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn settings_addons_tab_contains_only_addon_categories() {
    let mut view = model();
    view.options.addons_tab = true;
    view.options.category = OptionsCategory::SocialAddons;
    let mut menu = GameMenuModel::from_view(1920.0, 1080.0, view);
    menu.sync();
    assert!(
        menu.registry
            .get_by_name("OptionsTabsocialaddons")
            .is_some()
    );
    assert!(menu.registry.get_by_name("OptionsTabsound").is_none());
    let id = menu.registry.get_by_name("OptionsGameTabControl").unwrap();
    assert_eq!(
        menu.registry.click_frame(id).as_deref(),
        Some("options_tab:game")
    );
}

#[test]
fn options_choice_is_one_dropdown_and_unclipped_radio_popup() {
    use game_engine_ui_model::options_dropdown::OptionsDropdownState;
    let mut view = model();
    view.options.category = OptionsCategory::Interface;
    let mut menu = GameMenuModel::from_view(1600.0, 900.0, view);
    menu.sync();
    let button = menu
        .registry
        .get_by_name("Choicestatus_text_display")
        .unwrap();
    assert_eq!(
        menu.registry.click_frame(button).as_deref(),
        Some("options_dropdown:status_text_display")
    );
    assert!(
        menu.registry
            .get_by_name("OptionsChoicePopupstatus_text_display")
            .is_none()
    );
    let mut state = OptionsDropdownState::default();
    state.toggle("status_text_display", [320.0, 180.0]);
    menu.shared.insert(state.clone());
    menu.sync();
    let popup = menu
        .registry
        .get(
            menu.registry
                .get_by_name("OptionsChoicePopupstatus_text_display")
                .unwrap(),
        )
        .unwrap();
    assert_eq!(popup.parent_id, menu.registry.get_by_name("OptionsRoot"));
    let radio = menu
        .registry
        .get_by_name("Choicestatus_text_display2Hit")
        .unwrap();
    assert_eq!(
        menu.registry.click_frame(radio).as_deref(),
        Some("options_toggle:status_text_display:2")
    );
    state.toggle("encounter_view", [300.0, 200.0]);
    menu.shared.insert(state.clone());
    menu.sync();
    assert!(
        menu.registry
            .get_by_name("OptionsChoicePopupstatus_text_display")
            .is_none()
    );
    assert!(
        menu.registry
            .get_by_name("OptionsChoicePopupencounter_view")
            .is_some()
    );
    assert!(state.close());
    menu.shared.insert(state);
    menu.sync();
    assert!(
        menu.registry
            .get_by_name("OptionsChoicePopupencounter_view")
            .is_none()
    );
}
