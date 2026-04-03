use super::*;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
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
fn binary_asset_module_reuses_library_casc_resolver() {
    let binary_fn: fn(u32) -> Option<PathBuf> = crate::asset::asset_cache::model;
    let lib_fn: fn(u32) -> Option<PathBuf> = game_engine::asset::asset_cache::model;
    assert!(std::ptr::fn_addr_eq(binary_fn, lib_fn));
}
