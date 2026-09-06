use super::*;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn no_directional_shadows_configures_startup_world_light_override() {
    let mut app = App::new();

    configure_directional_shadow_isolation(&mut app, &args(&["--no-directional-shadows"]));

    assert!(
        app.world()
            .contains_resource::<game_state::DirectionalShadowsDisabled>()
    );
}

#[test]
fn screenshot_args_allow_flags_before_command() {
    let parsed = parse_screenshot_args(&args(&[
        "--state",
        "login",
        "screenshot",
        "/tmp/codex/test.webp",
        "--server",
        "127.0.0.1:25565",
    ]))
    .expect("expected screenshot request");
    assert_eq!(parsed.output, PathBuf::from("/tmp/codex/test.webp"));
    assert_eq!(parsed.frames_remaining, 60);
}

#[test]
fn parse_inworld_scene_stage_supports_cumulative_stages() {
    let cases = [
        ("empty", InWorldSceneStage::Empty),
        ("character", InWorldSceneStage::Character),
        ("skybox", InWorldSceneStage::Skybox),
        ("terrain", InWorldSceneStage::Terrain),
        ("npcs", InWorldSceneStage::Npcs),
        ("lighting", InWorldSceneStage::Lighting),
        ("particles", InWorldSceneStage::Particles),
        ("ui", InWorldSceneStage::Ui),
    ];

    for (value, expected) in cases {
        let parsed = parse_inworld_scene_stage_arg(&args(&["--inworld-stage", value]))
            .expect("expected valid stage")
            .expect("expected configured stage");
        assert_eq!(parsed, expected);
    }

    assert!(InWorldSceneStage::Ui.includes(InWorldSceneStage::Character));
    assert!(InWorldSceneStage::Terrain.includes(InWorldSceneStage::Skybox));
    assert!(!InWorldSceneStage::Character.includes(InWorldSceneStage::Skybox));
}

#[test]
fn no_npcs_ui_scene_stage_preserves_full_scene_except_npcs_and_game_ui() {
    let stage = parse_inworld_scene_stage_arg(&args(&["--inworld-stage", "no-npcs-ui"]))
        .expect("no-npcs-ui selector must parse")
        .expect("no-npcs-ui selector must configure a stage");

    assert!(stage.includes(InWorldSceneStage::Character));
    assert!(stage.includes(InWorldSceneStage::Skybox));
    assert!(stage.includes(InWorldSceneStage::Terrain));
    assert!(stage.includes(InWorldSceneStage::Lighting));
    assert!(stage.includes(InWorldSceneStage::Particles));
    assert!(!stage.includes(InWorldSceneStage::Npcs));
    assert!(!stage.includes(InWorldSceneStage::Ui));

    let mut app = App::new();
    app.insert_resource(stage);
    apply_inworld_scene_stage_ui_gates(&mut app);

    use game_engine::ui::plugin::{UiProcessingEnabled, UiRenderEnabled, UiTextRenderEnabled};
    assert!(!app.world().resource::<UiProcessingEnabled>().0);
    assert!(!app.world().resource::<UiRenderEnabled>().0);
    assert!(!app.world().resource::<UiTextRenderEnabled>().0);
}

#[test]
fn no_frame_time_graph_stops_buffer_updates_but_keeps_numeric_fps() {
    use bevy::dev_tools::frame_time_graph::FrametimeGraphMaterial;
    use bevy::render::storage::ShaderBuffer;
    use bevy::time::TimeUpdateStrategy;
    use bevy::ui_render::prelude::MaterialNode;

    const SENTINEL: [u8; 4] = [7, 11, 13, 17];
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::text::TextPlugin::default());
    app.init_asset::<bevy::shader::Shader>();
    app.init_asset::<ShaderBuffer>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    app.add_plugins(FpsOverlayPlugin::default());
    app.update();

    configure_frame_time_graph_isolation(&mut app, &args(&["--no-frame-time-graph"]));
    let disabled = app
        .world()
        .contains_resource::<client_options::FrameTimeGraphDisabled>();
    client_options::apply_fps_overlay_visibility(
        &mut app.world_mut().resource_mut::<FpsOverlayConfig>(),
        true,
        Some(InWorldSceneStage::NoNpcsUi),
        disabled,
    );
    for (_, buffer) in app
        .world_mut()
        .resource_mut::<Assets<ShaderBuffer>>()
        .iter_mut()
    {
        buffer.data = Some(SENTINEL.to_vec());
    }
    app.update();
    app.update();

    let buffers = app.world().resource::<Assets<ShaderBuffer>>();
    assert!(!buffers.is_empty());
    assert!(
        buffers
            .iter()
            .all(|(_, buffer)| buffer.data.as_deref() == Some(SENTINEL.as_slice()))
    );
    let mut graphs = app
        .world_mut()
        .query_filtered::<&Node, With<MaterialNode<FrametimeGraphMaterial>>>();
    assert!(
        graphs
            .iter(app.world())
            .all(|node| node.display == Display::None)
    );
    let mut spans = app.world_mut().query::<&TextSpan>();
    assert!(
        spans
            .iter(app.world())
            .any(|span| span.0.parse::<f64>().is_ok_and(|value| value > 0.0))
    );
    assert!(app.world().resource::<FpsOverlayConfig>().enabled);

    app.world_mut()
        .remove_resource::<client_options::FrameTimeGraphDisabled>();
    client_options::apply_fps_overlay_visibility(
        &mut app.world_mut().resource_mut::<FpsOverlayConfig>(),
        true,
        None,
        false,
    );
    app.update();
    assert!(
        app.world()
            .resource::<Assets<ShaderBuffer>>()
            .iter()
            .any(|(_, buffer)| buffer.data.as_deref() != Some(SENTINEL.as_slice()))
    );
}

#[test]
fn no_terrain_meshes_startup_flag_changes_only_ground_rendering() {
    let mut app = App::new();
    app.init_resource::<terrain::AdtManager>();
    configure_terrain_mesh_isolation(&mut app, &args(&[]));
    assert!(app.world().resource::<terrain::AdtManager>().render_terrain);

    configure_terrain_mesh_isolation(&mut app, &args(&["--no-terrain-meshes"]));
    let manager = app.world().resource::<terrain::AdtManager>();
    assert!(!manager.render_terrain);
    assert!(manager.render_textures);
    assert!(manager.load_objects);
    assert!(manager.load_water);
}

#[test]
fn no_terrain_textures_startup_flag_preserves_other_loading_options() {
    let mut app = App::new();
    app.init_resource::<terrain::AdtManager>();
    configure_terrain_texture_isolation(&mut app, &args(&[]));
    assert!(
        app.world()
            .resource::<terrain::AdtManager>()
            .render_textures
    );

    configure_terrain_texture_isolation(&mut app, &args(&["--no-terrain-textures"]));
    let manager = app.world().resource::<terrain::AdtManager>();
    assert!(!manager.render_textures);
    assert!(manager.load_objects);
    assert!(manager.load_water);
}

#[test]
fn water_skybox_isolation_startup_flags_are_opt_in() {
    let mut app = App::new();
    app.init_resource::<terrain::AdtManager>();
    configure_water_and_skybox_isolation(&mut app, &args(&[]));
    assert!(app.world().resource::<terrain::AdtManager>().load_water);
    assert!(
        !app.world()
            .contains_resource::<sky::SkyboxVisualsDisabled>()
    );

    configure_water_and_skybox_isolation(&mut app, &args(&["--no-terrain-water", "--no-skybox"]));
    assert!(!app.world().resource::<terrain::AdtManager>().load_water);
    assert!(
        app.world()
            .contains_resource::<sky::SkyboxVisualsDisabled>()
    );
    assert!(app.world().resource::<terrain::AdtManager>().load_objects);
}

#[test]
fn no_terrain_objects_startup_flag_keeps_default_loading_unchanged() {
    let mut app = App::new();
    app.init_resource::<terrain::AdtManager>();
    configure_terrain_object_loading(&mut app, &args(&[]));
    assert!(app.world().resource::<terrain::AdtManager>().load_objects);

    let flags = args(&["--no-terrain-objects"]);
    configure_terrain_object_loading(&mut app, &flags);
    assert!(!app.world().resource::<terrain::AdtManager>().load_objects);
    assert!(parse_asset_path_from_args(&flags).is_none());
}

#[test]
fn terrain_material_freeze_startup_config_is_opt_in() {
    let mut app = App::new();
    insert_terrain_material_freeze_resource(&mut app, &args(&[]));
    assert!(
        app.world()
            .get_resource::<terrain_material::TerrainMaterialFreezeAfter>()
            .is_none()
    );

    let flags = args(&["--freeze-terrain-materials-after", "30"]);
    insert_terrain_material_freeze_resource(&mut app, &flags);
    assert_eq!(
        app.world()
            .resource::<terrain_material::TerrainMaterialFreezeAfter>()
            .0,
        Duration::from_secs(30)
    );
    assert!(parse_asset_path_from_args(&flags).is_none());
    assert!(
        parse_u32_flag(
            &args(&["--freeze-terrain-materials-after", "-1"]),
            "--freeze-terrain-materials-after"
        )
        .is_err()
    );
}

#[test]
fn parse_inworld_scene_stage_rejects_unknown_values() {
    let error = parse_inworld_scene_stage_arg(&args(&["--inworld-stage", "everything"]))
        .expect_err("unknown stage must fail");
    assert!(error.contains("everything"));
}

#[test]
fn every_pre_ui_stage_disables_ui_processing_and_rendering() {
    use game_engine::ui::plugin::{UiProcessingEnabled, UiRenderEnabled, UiTextRenderEnabled};

    for stage in [
        InWorldSceneStage::Empty,
        InWorldSceneStage::Character,
        InWorldSceneStage::Skybox,
        InWorldSceneStage::Terrain,
        InWorldSceneStage::Npcs,
        InWorldSceneStage::Lighting,
        InWorldSceneStage::Particles,
    ] {
        let mut app = App::new();
        app.insert_resource(stage);
        apply_inworld_scene_stage_ui_gates(&mut app);

        assert!(!app.world().resource::<UiProcessingEnabled>().0);
        assert!(!app.world().resource::<UiRenderEnabled>().0);
        assert!(!app.world().resource::<UiTextRenderEnabled>().0);
    }
}

#[test]
fn ui_and_unconfigured_stages_preserve_enabled_ui_defaults() {
    use game_engine::ui::plugin::{UiProcessingEnabled, UiRenderEnabled, UiTextRenderEnabled};

    for stage in [Some(InWorldSceneStage::Ui), None] {
        let mut app = App::new();
        if let Some(stage) = stage {
            app.insert_resource(stage);
        }
        app.insert_resource(UiProcessingEnabled::default());
        app.insert_resource(UiRenderEnabled::default());
        app.insert_resource(UiTextRenderEnabled::default());

        apply_inworld_scene_stage_ui_gates(&mut app);

        assert!(app.world().resource::<UiProcessingEnabled>().0);
        assert!(app.world().resource::<UiRenderEnabled>().0);
        assert!(app.world().resource::<UiTextRenderEnabled>().0);
    }
}

#[test]
fn pre_ui_processing_gate_keeps_fps_overlay_active() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::text::TextPlugin::default());
    app.init_asset::<bevy::shader::Shader>();
    app.init_asset::<bevy::render::storage::ShaderBuffer>();
    app.add_plugins(FpsOverlayPlugin {
        config: FpsOverlayConfig {
            refresh_interval: Duration::from_secs(1),
            ..default()
        },
    });
    app.insert_resource(InWorldSceneStage::Empty);
    apply_inworld_scene_stage_ui_gates(&mut app);

    let overlay = app.world().resource::<FpsOverlayConfig>();
    assert!(overlay.enabled);
    assert!(!overlay.frame_time_graph_config.enabled);

    app.update();

    let mut text_query = app.world_mut().query::<&Text>();
    assert!(
        text_query.iter(app.world()).any(|text| text.0 == "FPS: "),
        "FPS overlay text was not spawned while game UI processing was disabled"
    );
}

#[test]
fn character_stage_preserves_fps_overlay_graph() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::text::TextPlugin::default());
    app.init_asset::<bevy::shader::Shader>();
    app.init_asset::<bevy::render::storage::ShaderBuffer>();
    app.add_plugins(FpsOverlayPlugin::default());
    app.insert_resource(InWorldSceneStage::Character);

    apply_inworld_scene_stage_ui_gates(&mut app);

    let overlay = app.world().resource::<FpsOverlayConfig>();
    assert!(overlay.enabled);
    assert!(overlay.frame_time_graph_config.enabled);
}

fn world_environment_test_app(stage: InWorldSceneStage) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.insert_resource(networking::ServerAddr(
        "127.0.0.1:5000".parse().expect("valid test server address"),
    ));
    app.insert_resource(game_state::InitialGameState(game_state::GameState::InWorld));
    app.insert_resource(stage);
    app.add_plugins(game_state::GameStatePlugin);
    app
}

fn world_camera_counts(app: &mut App) -> (usize, usize) {
    let wow_camera_count = app
        .world_mut()
        .query_filtered::<Entity, With<camera::WowCamera>>()
        .iter(app.world())
        .count();
    let camera_3d_count = app
        .world_mut()
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(app.world())
        .count();
    (wow_camera_count, camera_3d_count)
}

#[test]
fn empty_world_environment_has_no_world_camera() {
    let mut app = world_environment_test_app(InWorldSceneStage::Empty);

    app.update();

    assert_eq!(world_camera_counts(&mut app), (0, 0));
}

#[test]
fn character_world_environment_keeps_one_world_camera_across_reentry() {
    let mut app = world_environment_test_app(InWorldSceneStage::Character);

    app.update();
    assert_eq!(world_camera_counts(&mut app), (1, 1));

    app.world_mut()
        .resource_mut::<NextState<game_state::GameState>>()
        .set(game_state::GameState::Login);
    app.update();
    app.world_mut()
        .resource_mut::<NextState<game_state::GameState>>()
        .set(game_state::GameState::InWorld);
    app.update();

    assert_eq!(world_camera_counts(&mut app), (1, 1));
}

#[test]
fn particle_plugin_is_enabled_only_from_particles_stage() {
    for stage in [
        InWorldSceneStage::Empty,
        InWorldSceneStage::Character,
        InWorldSceneStage::Skybox,
        InWorldSceneStage::Terrain,
        InWorldSceneStage::Npcs,
        InWorldSceneStage::Lighting,
    ] {
        assert!(!app_setup::particle_plugin_is_enabled(Some(stage)));
    }
    assert!(app_setup::particle_plugin_is_enabled(Some(
        InWorldSceneStage::Particles
    )));
    assert!(app_setup::particle_plugin_is_enabled(Some(
        InWorldSceneStage::Ui
    )));
    assert!(app_setup::particle_plugin_is_enabled(None));
}

#[test]
fn parse_screen_alias_matches_state_parser() {
    let parsed = parse_state_arg(&args(&["--screen", "charselect"]))
        .expect("expected valid parse")
        .expect("expected screen alias");
    assert_eq!(parsed, game_state::GameState::CharSelect);

    let parsed = parse_state_arg(&args(&["--screen", "selectiondebug"]))
        .expect("expected valid parse")
        .expect("expected selectiondebug");
    assert_eq!(parsed, game_state::GameState::SelectionDebug);

    let parsed = parse_state_arg(&args(&["--screen", "inworldselectiondebug"]))
        .expect("expected valid parse")
        .expect("expected inworldselectiondebug");
    assert_eq!(parsed, game_state::GameState::InWorldSelectionDebug);

    let parsed = parse_state_arg(&args(&["--screen", "login"]))
        .expect("expected valid parse")
        .expect("expected login");
    assert_eq!(parsed, game_state::GameState::Login);

    let parsed = parse_state_arg(&args(&["--screen", "eula"]))
        .expect("expected valid parse")
        .expect("expected eula");
    assert_eq!(parsed, game_state::GameState::Eula);

    let parsed = parse_state_arg(&args(&["--screen", "charcreate-customize"]))
        .expect("expected valid parse")
        .expect("expected charcreate customize");
    assert_eq!(parsed, game_state::GameState::CharCreate);

    let parsed = parse_state_arg(&args(&["--screen", "campsitepopup"]))
        .expect("expected valid parse")
        .expect("expected campsitepopup");
    assert_eq!(parsed, game_state::GameState::CampsitePopup);

    let parsed = parse_state_arg(&args(&["--screen", "loading"]))
        .expect("expected valid parse")
        .expect("expected loading");
    assert_eq!(parsed, game_state::GameState::Loading);

    let parsed = parse_state_arg(&args(&["--screen", "skyboxdebug"]))
        .expect("expected valid parse")
        .expect("expected skyboxdebug");
    assert_eq!(parsed, game_state::GameState::SkyboxDebug);

    let parsed = parse_state_arg(&args(&["--screen", "m2debug"]))
        .expect("expected valid parse")
        .expect("expected m2debug");
    assert_eq!(parsed, game_state::GameState::M2Debug);

    let parsed = parse_state_arg(&args(&["--screen", "trashbutton"]))
        .expect("expected valid parse")
        .expect("expected trashbutton");
    assert_eq!(parsed, game_state::GameState::TrashButton);
}

#[test]
fn parse_screen_rejects_non_screen_states() {
    let err = parse_state_arg(&args(&["--screen", "connecting"]))
        .expect_err("connecting should be rejected for --screen");
    assert!(
        err.contains("invalid --screen value 'connecting'"),
        "unexpected error: {err}"
    );
}

#[test]
fn parse_screen_arg_preserves_charcreate_customize_variant() {
    let parsed = parse_screen_arg(&args(&["--screen", "charcreate-customize"]))
        .expect("expected valid parse")
        .expect("expected screen alias");
    assert_eq!(
        parsed,
        game_engine::game_state_enum::ScreenArg::CharCreateCustomize
    );
}

#[test]
fn parse_screen_arg_preserves_selectiondebug_variant() {
    let parsed = parse_screen_arg(&args(&["--screen", "selectiondebug"]))
        .expect("expected valid parse")
        .expect("expected screen alias");
    assert_eq!(
        parsed,
        game_engine::game_state_enum::ScreenArg::SelectionDebug
    );
}

#[test]
fn parse_screen_arg_preserves_inworldselectiondebug_variant() {
    let parsed = parse_screen_arg(&args(&["--screen", "inworldselectiondebug"]))
        .expect("expected valid parse")
        .expect("expected screen alias");
    assert_eq!(
        parsed,
        game_engine::game_state_enum::ScreenArg::InWorldSelectionDebug
    );
}

#[test]
fn parse_load_scene_flag() {
    let parsed = parse_load_scene_arg(&args(&["--load-scene", "data/debug/scene.json"]))
        .expect("expected valid parse");
    assert_eq!(parsed, Some(PathBuf::from("data/debug/scene.json")));
}

#[test]
fn parse_skybox_debug_override_accepts_skybox_fdid() {
    let parsed = parse_skybox_debug_override(&args(&["--skybox-fdid", "5412968"]))
        .expect("expected valid parse");
    assert_eq!(
        parsed,
        Some(crate::scenes::skybox_debug::SkyboxDebugOverride::SkyboxFileDataId(5_412_968))
    );
}

#[test]
fn parse_skybox_debug_override_accepts_light_skybox_id() {
    let parsed = parse_skybox_debug_override(&args(&["--light-skybox-id", "653"]))
        .expect("expected valid parse");
    assert_eq!(
        parsed,
        Some(crate::scenes::skybox_debug::SkyboxDebugOverride::LightSkyboxId(653))
    );
}

#[test]
fn parse_skybox_debug_override_rejects_conflicting_flags() {
    let err = parse_skybox_debug_override(&args(&[
        "--light-skybox-id",
        "653",
        "--skybox-fdid",
        "5412968",
    ]))
    .expect_err("conflicting override flags should fail");
    assert_eq!(
        err,
        "use only one of --skybox-fdid or --light-skybox-id when forcing skyboxdebug"
    );
}

#[test]
fn parse_skybox_debug_view_mode_accepts_verification_flag() {
    let parsed = parse_skybox_debug_view_mode(&args(&["--skybox-verify"]));
    assert_eq!(
        parsed,
        crate::scenes::skybox_debug::SkyboxDebugViewMode::AuthoredOnlyVerification
    );
}

#[test]
fn parse_screen_requires_value() {
    let err =
        parse_state_arg(&args(&["--screen"])).expect_err("missing --screen value should fail");
    assert_eq!(err, "missing value for --screen");
}

#[test]
fn asset_path_skips_flags_and_screenshot_output() {
    for flag in ["--state", "--screen"] {
        let parsed = parse_asset_path_from_args(&args(&[
            flag,
            "login",
            "screenshot",
            "/tmp/codex/test.webp",
            "--server",
            "127.0.0.1:25565",
        ]));
        assert_eq!(parsed, None, "flag {flag} should not produce asset path");
    }
}

#[test]
fn asset_path_after_screenshot_is_preserved() {
    let parsed = parse_asset_path_from_args(&args(&[
        "--state",
        "inworld",
        "screenshot",
        "/tmp/codex/test.webp",
        "data/models/humanmale_hd.m2",
    ]));
    assert_eq!(parsed, Some(PathBuf::from("data/models/humanmale_hd.m2")));
}

#[test]
fn asset_path_skips_skybox_debug_override_flags() {
    let parsed = parse_asset_path_from_args(&args(&[
        "--screen",
        "skyboxdebug",
        "--light-skybox-id",
        "628",
        "screenshot",
        "data/skybox.webp",
    ]));
    assert_eq!(parsed, None);

    let parsed = parse_asset_path_from_args(&args(&[
        "--screen",
        "skyboxdebug",
        "--skybox-fdid",
        "5412968",
        "screenshot",
        "data/skybox.webp",
    ]));
    assert_eq!(parsed, None);
}

#[test]
fn startup_flag_loads_ui_script_path() {
    let actions = load_startup_automation_actions(&args(&[
        "--run-ui-script",
        "/tmp/codex/test-ui-script.json",
    ]));
    assert!(actions.is_err());
    let parsed = game_engine::ui::automation_script::parse_automation_script_arg(&args(&[
        "--run-ui-script",
        "debug/login.json",
    ]))
    .expect("expected UI script path");
    assert_eq!(parsed.path, PathBuf::from("debug/login.json"));
}

#[test]
fn asset_path_skips_login_dev_admin_flag() {
    let parsed =
        parse_asset_path_from_args(&args(&["--login-dev-admin", "data/models/humanmale_hd.m2"]));
    assert_eq!(parsed, Some(PathBuf::from("data/models/humanmale_hd.m2")));
}

#[test]
fn parse_js_automation_flag() {
    let parsed = game_engine::ui::js_automation::parse_js_automation_arg(&args(&[
        "--state",
        "login",
        "--run-js-ui-script",
        "debug/login.js",
    ]))
    .expect("expected JS automation path");
    assert_eq!(parsed.path, PathBuf::from("debug/login.js"));
}

#[test]
fn parse_server_arg_supports_prod_alias() {
    let parsed = parse_server_arg(&args(&["--server", "prod"])).expect("expected prod server");
    assert_eq!(parsed.hostname, "game.worldofosso.com:5000");
    assert!(!parsed.dev);
}

#[test]
fn default_connecting_server_arg_uses_preferred_realm() {
    let parsed = super::default_connecting_server_arg(
        Some(game_state::GameState::Connecting),
        crate::cli_args::RealmPreset::Dev,
    )
    .expect("expected connecting server");
    assert_eq!(parsed.hostname, "127.0.0.1:5000");
    assert!(parsed.dev);
}

#[test]
fn parse_run_args_starts_connecting_when_saved_token_exists() {
    let parsed =
        parse_run_args_with_saved_token(&args(&["--server", "127.0.0.1:25565"]), true, None);
    assert_eq!(
        parsed.initial_state,
        Some(game_state::GameState::Connecting)
    );
    assert!(parsed.startup_actions.is_empty());
}

#[test]
fn parse_run_args_starts_connecting_when_startup_credentials_exist() {
    let parsed = parse_run_args_with_saved_token(
        &args(&["--server", "prod"]),
        false,
        Some(("prod-user".to_string(), "prod-pass".to_string())),
    );
    assert_eq!(
        parsed.initial_state,
        Some(game_state::GameState::Connecting)
    );
    assert_eq!(
        parsed.startup_login,
        Some(("prod-user".to_string(), "prod-pass".to_string()))
    );
    assert!(parsed.startup_actions.is_empty());
}

#[test]
fn parse_run_args_without_server_starts_connecting_when_saved_token_exists() {
    let parsed = parse_run_args_with_saved_token(&args(&[]), true, None);
    assert_eq!(
        parsed.initial_state,
        Some(game_state::GameState::Connecting)
    );
    assert!(parsed.server_addr.is_none());
    assert!(parsed.startup_actions.is_empty());
}

#[test]
fn parse_run_args_charselect_without_server_keeps_server_unset() {
    let parsed = parse_run_args_with_saved_token(&args(&["--screen", "charselect"]), true, None);
    assert_eq!(
        parsed.initial_state,
        Some(game_state::GameState::Connecting)
    );
    assert!(parsed.server_addr.is_none());
}

#[test]
fn parse_run_args_keeps_explicit_login_screen_with_saved_token() {
    let parsed = parse_run_args_with_saved_token(
        &args(&["--server", "127.0.0.1:25565", "--screen", "login"]),
        true,
        None,
    );
    assert_eq!(parsed.initial_state, Some(game_state::GameState::Login));
    assert!(parsed.startup_actions.is_empty());
}

#[test]
fn parse_run_args_login_dev_admin_forces_login_flow() {
    let parsed = parse_run_args_with_saved_token(&args(&["--login-dev-admin"]), true, None);
    assert_eq!(
        parsed.initial_state,
        Some(game_state::GameState::Connecting)
    );
    assert!(
        parsed
            .server_addr
            .as_ref()
            .is_some_and(|s| s.dev && s.addr.to_string() == "127.0.0.1:5000")
    );
    assert_eq!(
        parsed.startup_login,
        Some(("admin".to_string(), "admin".to_string()))
    );
    assert!(parsed.startup_actions.is_empty());
    assert!(!parsed.auto_enter_world);
}

#[test]
fn resolved_initial_state_keeps_parsed_connecting_when_cli_state_is_absent() {
    let parsed =
        parse_run_args_with_saved_token(&args(&["--server", "127.0.0.1:25565"]), true, None);
    let resolved = parsed.initial_state.or(None);
    assert_eq!(resolved, Some(game_state::GameState::Connecting));
}

#[test]
fn resolved_initial_state_keeps_parsed_rewritten_state() {
    let parsed =
        parse_run_args_with_saved_token(&args(&["--server", "127.0.0.1:25565"]), true, None);
    let resolved = parsed.initial_state.or(Some(game_state::GameState::Login));
    assert_eq!(resolved, Some(game_state::GameState::Connecting));
}

#[test]
fn startup_scene_loading_only_runs_for_explicit_assets() {
    use crate::scenes::setup::should_load_explicit_scene_at_startup;
    use std::path::Path;
    assert!(!should_load_explicit_scene_at_startup(false, None));
    assert!(should_load_explicit_scene_at_startup(
        false,
        Some(Path::new("data/models/humanmale_hd.m2"))
    ));
    assert!(!should_load_explicit_scene_at_startup(
        true,
        Some(Path::new("data/models/humanmale_hd.m2"))
    ));
}

#[test]
fn parse_screen_menu_alias() {
    let parsed = parse_state_arg(&args(&["--screen", "menu"]))
        .expect("valid parse")
        .expect("expected menu screen");
    assert_eq!(parsed, game_state::GameState::GameMenu);

    let parsed = parse_state_arg(&args(&["--screen", "gamemenu"]))
        .expect("valid parse")
        .expect("expected gamemenu screen");
    assert_eq!(parsed, game_state::GameState::GameMenu);

    let parsed = parse_state_arg(&args(&["--screen", "optionsmenu"]))
        .expect("valid parse")
        .expect("expected optionsmenu screen");
    assert_eq!(parsed, game_state::GameState::GameMenu);
}

#[test]
fn parse_screen_arg_preserves_optionsmenu_variant() {
    let parsed = parse_screen_arg(&args(&["--screen", "optionsmenu"]))
        .expect("valid parse")
        .expect("expected optionsmenu variant");
    assert_eq!(parsed, game_engine::game_state_enum::ScreenArg::OptionsMenu);
}

#[test]
fn parse_screen_arg_preserves_loading_variant() {
    let parsed = parse_screen_arg(&args(&["--screen", "loading"]))
        .expect("valid parse")
        .expect("expected loading variant");
    assert_eq!(parsed, game_engine::game_state_enum::ScreenArg::Loading);
}

#[test]
fn parse_screen_arg_preserves_skyboxdebug_variant() {
    let parsed = parse_screen_arg(&args(&["--screen", "skyboxdebug"]))
        .expect("valid parse")
        .expect("expected skyboxdebug variant");
    assert_eq!(parsed, game_engine::game_state_enum::ScreenArg::SkyboxDebug);
}

#[test]
fn parse_screen_arg_preserves_m2debug_variant() {
    let parsed = parse_screen_arg(&args(&["--screen", "m2debug"]))
        .expect("valid parse")
        .expect("expected m2debug variant");
    assert_eq!(parsed, game_engine::game_state_enum::ScreenArg::M2Debug);
}

#[test]
fn world_builder_flag_is_opt_in() {
    let normal = parse_cli_flags(&args(&["--state", "inworld"]));
    let enabled = parse_cli_flags(&args(&["--state", "inworld", "--world-builder"]));

    assert!(!normal.world_builder);
    assert!(enabled.world_builder);
}

#[test]
fn world_builder_plugin_is_only_added_when_requested() {
    let mut normal = App::new();
    add_optional_world_builder_plugin(&mut normal, false);
    assert!(
        normal
            .world()
            .get_resource::<world_builder::WorldBuilderEnabled>()
            .is_none()
    );

    let mut enabled = App::new();
    add_optional_world_builder_plugin(&mut enabled, true);
    assert!(
        enabled
            .world()
            .get_resource::<world_builder::WorldBuilderEnabled>()
            .is_some()
    );
}

#[test]
fn binary_asset_module_reuses_library_casc_resolver() {
    let binary_fn: fn(u32) -> Option<PathBuf> = crate::asset::asset_cache::model;
    let lib_fn: fn(u32) -> Option<PathBuf> = game_engine::asset::asset_cache::model;
    assert!(std::ptr::fn_addr_eq(binary_fn, lib_fn));
}
