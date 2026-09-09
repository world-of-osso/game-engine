use super::*;
use game_engine::input_bindings::{InputAction, InputBinding};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    Path::new("target/test-artifacts")
        .join("client_options")
        .join(format!("{name}-{nanos}"))
}

#[test]
fn no_ui_keeps_numeric_fps_visible_across_loaded_settings_and_hud_updates() {
    let mut app = App::new();
    app.insert_resource(UiDisabled);
    app.insert_resource(FpsOverlayConfig::default());
    app.insert_resource(LoadedClientOptions {
        file: ClientOptionsFile {
            hud: HudOptionsFile {
                show_fps_overlay: false,
                ..default()
            },
            ..default()
        },
        applied: false,
    });
    app.insert_resource(HudOptions::default());
    app.insert_resource(HudVisibilityToggles::from_hud_options(
        &HudOptions::default(),
    ));
    app.add_systems(
        Update,
        (apply_loaded_client_options, sync_hud_visibility_toggles).chain(),
    );
    for show in [true, false, true] {
        app.world_mut()
            .resource_mut::<HudOptions>()
            .show_fps_overlay = show;
        app.update();
        let fps = app.world().resource::<FpsOverlayConfig>();
        assert!(fps.enabled);
        assert!(!fps.frame_time_graph_config.enabled);
    }
}

#[test]
fn default_file_uses_expected_modal_position() {
    let file = ClientOptionsFile::default();
    assert!(!file.accepted_eula);
    assert_eq!(file.preferred_realm, default_realm_preset());
    assert_eq!(file.modal_offset, None);
    assert_eq!(file.modal_position, None);
    assert_eq!(file.bindings, InputBindings::default());
}

#[test]
fn hud_defaults_are_visible() {
    let defaults = HudOptions::default();
    assert!(defaults.show_minimap);
    assert!(defaults.show_action_bars);
    assert!(defaults.show_target_marker);
    assert!((defaults.nameplate_distance - DEFAULT_NAMEPLATE_DISTANCE).abs() < 0.0001);
    assert!((defaults.chat_font_size - 10.0).abs() < 0.0001);
}

#[test]
fn hud_visibility_toggles_follow_hud_options() {
    let toggles = HudVisibilityToggles::from_hud_options(&HudOptions {
        show_minimap: false,
        show_action_bars: true,
        show_nameplates: false,
        nameplate_distance: default_nameplate_distance(),
        show_health_bars: false,
        show_target_marker: true,
        show_fps_overlay: false,
        chat_font_size: default_chat_font_size(),
    });
    assert!(!toggles.show_minimap);
    assert!(toggles.show_action_bars);
    assert!(!toggles.show_player_frame);
    assert!(!toggles.show_target_frame);
    assert!(!toggles.show_nameplates);
    assert!(!toggles.show_health_bars);
    assert!(toggles.show_target_marker);
    assert!(!toggles.show_fps_overlay);
}

#[test]
fn fps_overlay_visibility_keeps_graph_disabled_at_empty() {
    let mut overlay = FpsOverlayConfig::default();

    apply_fps_overlay_visibility(&mut overlay, true, Some(InWorldSceneStage::Empty), false);
    assert!(overlay.enabled);
    assert!(!overlay.frame_time_graph_config.enabled);

    apply_fps_overlay_visibility(
        &mut overlay,
        true,
        Some(InWorldSceneStage::Character),
        false,
    );
    assert!(overlay.enabled);
    assert!(overlay.frame_time_graph_config.enabled);

    apply_fps_overlay_visibility(
        &mut overlay,
        false,
        Some(InWorldSceneStage::Character),
        false,
    );
    assert!(!overlay.enabled);
    assert!(!overlay.frame_time_graph_config.enabled);
}

#[test]
fn graphics_defaults_use_full_particle_density() {
    let defaults = GraphicsOptions::default();
    assert_eq!(defaults.particle_density, 100);
    assert!((defaults.render_scale - 1.0).abs() < 0.0001);
    assert!((defaults.ui_scale - 1.0).abs() < 0.0001);
    assert!(defaults.vsync_enabled);
    assert!(!defaults.frame_rate_limit_enabled);
    assert_eq!(defaults.frame_rate_limit, 144);
    assert!(!defaults.colorblind_mode);
    assert!(!defaults.bloom_enabled);
    assert!((defaults.bloom_intensity - 0.08).abs() < 0.0001);
    assert!((defaults.particle_density_multiplier() - 1.0).abs() < 0.0001);
}

#[test]
fn options_file_serializes_particle_density_with_cvar_name() {
    let file = ClientOptionsFile {
        accepted_eula: true,
        preferred_realm: RealmPreset::Prod,
        graphics: GraphicsOptionsFile {
            particle_density: 80,
            render_scale: 0.67,
            ui_scale: 1.2,
            vsync_enabled: false,
            frame_rate_limit_enabled: true,
            frame_rate_limit: 120,
            colorblind_mode: false,
            bloom_enabled: false,
            bloom_intensity: 0.12,
            ..GraphicsOptionsFile::default()
        },
        ..ClientOptionsFile::default()
    };

    let serialized = ron::ser::to_string(&file).unwrap();

    assert!(serialized.contains("accepted_eula:true"));
    assert!(serialized.contains("particleDensity:80"));
    assert!(serialized.contains("preferredRealm:Prod"));
    assert!(serialized.contains("renderScale:0.67"));
    assert!(serialized.contains("uiScale:1.2"));
    assert!(serialized.contains("vsyncEnabled:false"));
    assert!(serialized.contains("frameRateLimitEnabled:true"));
    assert!(serialized.contains("frameRateLimit:120"));
    assert!(serialized.contains("colorblindMode:false"));
    assert!(serialized.contains("bloomEnabled:false"));
    assert!(serialized.contains("bloomIntensity:0.12"));
}

fn load_graphics_config_fixture(name: &str, contents: &str) -> (PathBuf, ClientOptionsFile) {
    let directory = unique_test_dir(name);
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(OPTIONS_FILE_NAME);
    fs::write(&path, contents).unwrap();
    let file = storage::load_options_file_from_path(&path);
    (path, file)
}

fn assert_graphics_effect_controls(
    graphics: &GraphicsOptions,
    (particles, blur, glow, aa, ssao): (bool, bool, bool, AntiAliasMode, bool),
) {
    assert_eq!(graphics.particle_effects_enabled, particles);
    assert_eq!(graphics.depth_of_field, blur);
    assert_eq!(graphics.bloom_enabled, glow);
    assert_eq!(graphics.anti_alias, aa);
    assert_eq!(graphics.ssao_enabled, ssao);
}

#[test]
fn graphics_config_effects_round_trip_preserves_unrelated_options() {
    let (path, loaded) = load_graphics_config_fixture(
        "graphics-controls-roundtrip",
        r#"(
            accepted_eula: true,
            preferredRealm: Prod,
            graphics: (
                particleEffectsEnabled: false, particleDensity: 62,
                depthOfField: true, bloomEnabled: true, bloomIntensity: 0.25,
                antiAlias: Taa, ssaoEnabled: true,
                renderScale: 0.75, uiScale: 1.25, vsyncEnabled: false,
                frameRateLimitEnabled: true, frameRateLimit: 120,
                colorblindMode: true,
            ),
            modal_offset: Some((13.0, -9.0)),
        )"#,
    );
    let graphics = GraphicsOptions::from_file(&loaded.graphics);
    assert_graphics_effect_controls(&graphics, (false, true, true, AntiAliasMode::Taa, true));
    let mut bindings = loaded.bindings.clone();
    bindings.assign(
        InputAction::TargetNearest,
        InputBinding::Keyboard(KeyCode::F5),
    );
    let saved = storage::build_options_file_from_existing(
        &loaded,
        Some(&loaded.sound.to_runtime()),
        &CameraOptions::from_file(&loaded.camera),
        &graphics,
        &HudOptions::from_file(&loaded.hud),
        &bindings,
        [13.0, -9.0],
    );
    storage::save_options_file_to_path(&path, &saved).unwrap();
    let restored = storage::load_options_file_from_path(&path);
    assert_eq!(GraphicsOptions::from_file(&restored.graphics), graphics);
    assert_eq!(restored.graphics.particle_density, 62);
    assert_eq!(restored.graphics.render_scale, 0.75);
    assert_eq!(restored.graphics.ui_scale, 1.25);
    assert!(!restored.graphics.vsync_enabled);
    assert!(restored.graphics.frame_rate_limit_enabled);
    assert_eq!(restored.graphics.frame_rate_limit, 120);
    assert!(restored.graphics.colorblind_mode);
    assert_eq!(restored.graphics.bloom_intensity, 0.25);
    assert!(restored.accepted_eula);
    assert_eq!(restored.preferred_realm, RealmPreset::Prod);
    assert_eq!(restored.modal_offset, Some([13.0, -9.0]));
    assert_eq!(restored.bindings, bindings);
}

#[test]
fn graphics_config_missing_controls_preserve_effective_defaults() {
    let (_, file) = load_graphics_config_fixture(
        "graphics-controls-missing",
        "(graphics: (particleDensity: 42,))",
    );
    let graphics = GraphicsOptions::from_file(&file.graphics);
    assert_graphics_effect_controls(
        &graphics,
        (true, false, false, AntiAliasMode::Msaa4x, false),
    );
    assert_eq!(graphics.particle_density, 42);
    assert_graphics_effect_controls(
        &GraphicsOptions::default(),
        (true, false, false, AntiAliasMode::Msaa4x, false),
    );
}

#[test]
fn graphics_config_controls_load_independently() {
    let cases = [
        (false, false, false, AntiAliasMode::None, false),
        (true, false, false, AntiAliasMode::None, false),
        (false, true, false, AntiAliasMode::None, false),
        (false, false, true, AntiAliasMode::None, false),
        (false, false, false, AntiAliasMode::Taa, false),
        (false, false, false, AntiAliasMode::None, true),
        (false, false, false, AntiAliasMode::Msaa4x, false),
    ];
    for (particles, blur, glow, aa, ssao) in cases {
        let contents = format!(
            "(graphics:(particleEffectsEnabled:{particles},depthOfField:{blur},bloomEnabled:{glow},antiAlias:{aa:?},ssaoEnabled:{ssao},))"
        );
        let (_, file) = load_graphics_config_fixture("graphics-independent", &contents);
        assert_graphics_effect_controls(
            &GraphicsOptions::from_file(&file.graphics),
            (particles, blur, glow, aa, ssao),
        );
    }
}

fn graphics_config_load_error(contents: &str) -> String {
    let failure =
        std::panic::catch_unwind(|| load_graphics_config_fixture("graphics-invalid", contents))
            .expect_err("invalid existing graphics config must fail instead of loading defaults");
    if let Some(message) = failure.downcast_ref::<String>() {
        return message.clone();
    }
    failure
        .downcast_ref::<&str>()
        .expect("panic message")
        .to_string()
}

#[test]
fn graphics_config_invalid_aa_enum_fails_visibly() {
    let message = graphics_config_load_error("(graphics:(antiAlias:Smaa,))");
    assert!(message.contains("Smaa"), "{message}");
    assert!(message.contains("Msaa4x"), "{message}");
    assert!(message.contains("Taa"), "{message}");
}

#[test]
fn graphics_config_unsupported_ssao_msaa_fails_on_load() {
    let message = graphics_config_load_error("(graphics:(antiAlias:Msaa4x,ssaoEnabled:true,))");
    for required in ["SSAO", "Msaa4x", "None", "Taa"] {
        assert!(message.contains(required), "{message}");
    }
}

#[test]
fn graphics_config_unsupported_ssao_msaa_save_preserves_existing_file() {
    let (path, _) = load_graphics_config_fixture("graphics-invalid-save", "()");
    let before = fs::read(&path).unwrap();
    let invalid: ClientOptionsFile =
        ron::de::from_str("(graphics:(antiAlias:Msaa4x,ssaoEnabled:true,))").unwrap();
    let error = storage::save_options_file_to_path(&path, &invalid)
        .expect_err("unsupported SSAO/MSAA combination must not be saved");
    for required in ["SSAO", "Msaa4x", "None", "Taa"] {
        assert!(error.contains(required), "{error}");
    }
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn camera_defaults_include_mouse_sensitivity() {
    let defaults = CameraOptions::default();
    assert!((defaults.mouse_sensitivity - default_mouse_sensitivity()).abs() < 0.0001);
    assert!((defaults.fov_degrees - DEFAULT_CAMERA_FOV_DEGREES).abs() < 0.0001);
}

#[test]
fn options_file_round_trips_target_nearest_binding() {
    let mut bindings = InputBindings::default();
    bindings.assign(
        InputAction::TargetNearest,
        InputBinding::Keyboard(KeyCode::F2),
    );

    let file = ClientOptionsFile {
        bindings: bindings.clone(),
        ..ClientOptionsFile::default()
    };

    let serialized = ron::ser::to_string(&file).unwrap();
    let parsed: ClientOptionsFile = ron::de::from_str(&serialized).unwrap();

    assert_eq!(
        parsed.bindings.binding(InputAction::TargetNearest),
        Some(InputBinding::Keyboard(KeyCode::F2))
    );
    assert!(serialized.contains("TargetNearest"));
    assert!(serialized.contains("key:F2"));
}

#[test]
fn prefers_config_path_when_both_exist() {
    let test_dir = unique_test_dir("prefer-config");
    fs::create_dir_all(&test_dir).unwrap();
    let config_path = test_dir.join("config").join(OPTIONS_FILE_NAME);
    let legacy_path = test_dir.join("legacy").join(OPTIONS_FILE_NAME);
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    fs::write(&config_path, "config").unwrap();
    fs::write(&legacy_path, "legacy").unwrap();

    let selected = storage::select_load_options_path(&config_path, &legacy_path);

    assert_eq!(selected, config_path);
}

#[test]
fn falls_back_to_legacy_path_when_config_missing() {
    let test_dir = unique_test_dir("fallback-legacy");
    fs::create_dir_all(&test_dir).unwrap();
    let config_path = test_dir.join("config").join(OPTIONS_FILE_NAME);
    let legacy_path = test_dir.join("legacy").join(OPTIONS_FILE_NAME);
    fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    fs::write(&legacy_path, "legacy").unwrap();

    let selected = storage::select_load_options_path(&config_path, &legacy_path);

    assert_eq!(selected, legacy_path);
}

#[test]
fn returns_config_path_when_no_file_exists() {
    let test_dir = unique_test_dir("default-config");
    fs::create_dir_all(&test_dir).unwrap();
    let config_path = test_dir.join("config").join(OPTIONS_FILE_NAME);
    let legacy_path = test_dir.join("legacy").join(OPTIONS_FILE_NAME);

    let selected = storage::select_load_options_path(&config_path, &legacy_path);

    assert_eq!(selected, config_path);
}

#[test]
fn save_options_file_to_path_persists_and_loads_back() {
    let test_dir = unique_test_dir("persist-options");
    let path = test_dir.join("config").join(OPTIONS_FILE_NAME);
    let mut bindings = InputBindings::default();
    bindings.assign(
        InputAction::TargetNearest,
        InputBinding::Keyboard(KeyCode::F3),
    );
    let file = ClientOptionsFile {
        accepted_eula: true,
        preferred_realm: RealmPreset::Prod,
        sound: storage::SoundOptionsFile {
            master_volume: 0.25,
            ambient_volume: 0.5,
            effects_volume: 0.75,
            music_volume: 0.9,
            music_enabled: false,
            muted: true,
        },
        camera: CameraOptionsFile {
            mouse_sensitivity: 0.006,
            look_sensitivity: 0.02,
            invert_y: true,
            fov_degrees: 110.0,
            follow_speed: 6.0,
            zoom_speed: 3.0,
            min_distance: 4.0,
            max_distance: 28.0,
        },
        graphics: GraphicsOptionsFile {
            particle_density: 60,
            render_scale: 0.8,
            ui_scale: 1.3,
            vsync_enabled: false,
            frame_rate_limit_enabled: true,
            frame_rate_limit: 165,
            colorblind_mode: true,
            bloom_enabled: true,
            bloom_intensity: 0.2,
            ..GraphicsOptionsFile::default()
        },
        hud: HudOptionsFile {
            show_minimap: false,
            show_action_bars: true,
            show_nameplates: false,
            nameplate_distance: 60.0,
            show_health_bars: true,
            show_target_marker: false,
            show_fps_overlay: false,
            chat_font_size: 13.0,
        },
        bindings: bindings.clone(),
        modal_offset: Some([123.0, -45.0]),
        modal_position: None,
    };

    storage::save_options_file_to_path(&path, &file).unwrap();
    let loaded = storage::load_options_file_from_path(&path);

    assert!(loaded.accepted_eula);
    assert_eq!(loaded.sound.master_volume, 0.25);
    assert_eq!(loaded.preferred_realm, RealmPreset::Prod);
    assert!(!loaded.sound.music_enabled);
    assert!((loaded.camera.mouse_sensitivity - 0.006).abs() < 0.0001);
    assert!(loaded.camera.invert_y);
    assert!((loaded.camera.fov_degrees - 110.0).abs() < 0.0001);
    assert_eq!(loaded.graphics.particle_density, 60);
    assert!((loaded.graphics.ui_scale - 1.3).abs() < 0.0001);
    assert!(!loaded.graphics.vsync_enabled);
    assert!(loaded.graphics.frame_rate_limit_enabled);
    assert_eq!(loaded.graphics.frame_rate_limit, 165);
    assert!(loaded.graphics.colorblind_mode);
    assert!(!loaded.hud.show_minimap);
    assert!((loaded.hud.nameplate_distance - 60.0).abs() < 0.0001);
    assert!((loaded.hud.chat_font_size - 13.0).abs() < 0.0001);
    assert_eq!(loaded.modal_offset, Some([123.0, -45.0]));
    assert_eq!(
        loaded.bindings.binding(InputAction::TargetNearest),
        Some(InputBinding::Keyboard(KeyCode::F3))
    );
}

#[test]
fn build_options_file_preserves_existing_eula_acceptance() {
    let existing = ClientOptionsFile {
        accepted_eula: true,
        preferred_realm: RealmPreset::Prod,
        ..ClientOptionsFile::default()
    };

    let file = storage::build_options_file_from_existing(
        &existing,
        Some(&SoundSettings::default()),
        &CameraOptions::default(),
        &GraphicsOptions::default(),
        &HudOptions::default(),
        &InputBindings::default(),
        [0.0, 0.0],
    );

    assert!(file.accepted_eula);
    assert_eq!(file.preferred_realm, RealmPreset::Prod);
}

#[test]
fn graphics_options_file_clamps_ui_scale_range() {
    let low = GraphicsOptionsFile {
        ui_scale: 0.1,
        ..GraphicsOptionsFile::default()
    };
    let high = GraphicsOptionsFile {
        ui_scale: 5.0,
        ..GraphicsOptionsFile::default()
    };

    assert!((GraphicsOptions::from_file(&low).ui_scale - MIN_UI_SCALE).abs() < 0.0001);
    assert!((GraphicsOptions::from_file(&high).ui_scale - MAX_UI_SCALE).abs() < 0.0001);
}

#[test]
fn graphics_options_file_clamps_frame_rate_limit_range() {
    let low = GraphicsOptionsFile {
        frame_rate_limit: 1,
        ..GraphicsOptionsFile::default()
    };
    let high = GraphicsOptionsFile {
        frame_rate_limit: 999,
        ..GraphicsOptionsFile::default()
    };

    assert_eq!(
        GraphicsOptions::from_file(&low).frame_rate_limit,
        MIN_FRAME_RATE_LIMIT
    );
    assert_eq!(
        GraphicsOptions::from_file(&high).frame_rate_limit,
        MAX_FRAME_RATE_LIMIT
    );
}

#[test]
fn camera_options_file_clamps_mouse_sensitivity_range() {
    let low = CameraOptionsFile {
        mouse_sensitivity: 0.0,
        ..CameraOptionsFile::default()
    };
    let high = CameraOptionsFile {
        mouse_sensitivity: 99.0,
        ..CameraOptionsFile::default()
    };

    assert!(
        (CameraOptions::from_file(&low).mouse_sensitivity - MIN_MOUSE_SENSITIVITY).abs() < 0.0001
    );
    assert!(
        (CameraOptions::from_file(&high).mouse_sensitivity - MAX_MOUSE_SENSITIVITY).abs() < 0.0001
    );
}

#[test]
fn camera_options_file_clamps_fov_range() {
    let low = CameraOptionsFile {
        fov_degrees: 10.0,
        ..CameraOptionsFile::default()
    };
    let high = CameraOptionsFile {
        fov_degrees: 200.0,
        ..CameraOptionsFile::default()
    };

    assert!((CameraOptions::from_file(&low).fov_degrees - MIN_CAMERA_FOV_DEGREES).abs() < 0.0001);
    assert!((CameraOptions::from_file(&high).fov_degrees - MAX_CAMERA_FOV_DEGREES).abs() < 0.0001);
}

#[test]
fn hud_options_file_clamps_chat_font_size_range() {
    let low = HudOptionsFile {
        chat_font_size: 1.0,
        ..HudOptionsFile::default()
    };
    let high = HudOptionsFile {
        chat_font_size: 99.0,
        ..HudOptionsFile::default()
    };

    assert!((HudOptions::from_file(&low).chat_font_size - MIN_CHAT_FONT_SIZE).abs() < 0.0001);
    assert!((HudOptions::from_file(&high).chat_font_size - MAX_CHAT_FONT_SIZE).abs() < 0.0001);
}

#[test]
fn hud_options_file_clamps_nameplate_distance_range() {
    let low = HudOptionsFile {
        nameplate_distance: 1.0,
        ..HudOptionsFile::default()
    };
    let high = HudOptionsFile {
        nameplate_distance: 999.0,
        ..HudOptionsFile::default()
    };

    assert!(
        (HudOptions::from_file(&low).nameplate_distance - MIN_NAMEPLATE_DISTANCE).abs() < 0.0001
    );
    assert!(
        (HudOptions::from_file(&high).nameplate_distance - MAX_NAMEPLATE_DISTANCE).abs() < 0.0001
    );
}

#[test]
fn sync_ui_scale_updates_ui_camera_projection() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GraphicsOptions {
        ui_scale: 1.25,
        ..GraphicsOptions::default()
    });
    app.add_systems(Update, sync_ui_scale);
    app.world_mut().spawn((
        UiCamera,
        Projection::Orthographic(OrthographicProjection::default_2d()),
    ));

    app.update();

    let mut query = app
        .world_mut()
        .query_filtered::<&Projection, With<UiCamera>>();
    let projection = query.single(app.world()).expect("ui camera projection");
    let Projection::Orthographic(orthographic) = projection else {
        panic!("expected orthographic ui camera");
    };
    assert!((orthographic.scale - 0.8).abs() < 0.0001);
}

#[test]
fn sync_window_present_mode_updates_primary_window() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GraphicsOptions {
        vsync_enabled: false,
        ..GraphicsOptions::default()
    });
    app.add_systems(Update, sync_window_present_mode);
    let window_entity = app
        .world_mut()
        .spawn((PrimaryWindow, Window::default()))
        .id();

    app.update();

    let window = app.world().entity(window_entity).get::<Window>().unwrap();
    assert_eq!(window.present_mode, PresentMode::AutoNoVsync);
}

#[test]
fn sync_window_present_mode_maps_vsync_to_mailbox() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GraphicsOptions {
        vsync_enabled: true,
        ..GraphicsOptions::default()
    });
    app.add_systems(Update, sync_window_present_mode);
    let window_entity = app
        .world_mut()
        .spawn((
            PrimaryWindow,
            Window {
                present_mode: PresentMode::Fifo,
                ..Window::default()
            },
        ))
        .id();

    app.update();

    let window = app.world().entity(window_entity).get::<Window>().unwrap();
    assert_eq!(window.present_mode, PresentMode::Mailbox);
}

#[test]
fn frame_limit_interval_is_none_when_disabled() {
    assert_eq!(frame_limit_interval(false, 144), None);
}

#[test]
fn frame_limit_interval_uses_clamped_hz_when_enabled() {
    assert_eq!(
        frame_limit_interval(true, 1),
        Some(Duration::from_secs_f64(
            1.0 / f64::from(MIN_FRAME_RATE_LIMIT)
        ))
    );
    assert_eq!(
        frame_limit_interval(true, 999),
        Some(Duration::from_secs_f64(
            1.0 / f64::from(MAX_FRAME_RATE_LIMIT)
        ))
    );
}
