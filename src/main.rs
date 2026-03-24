use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::Duration,
};

use bevy::{
    asset::RenderAssetUsages,
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    pbr::MaterialPlugin,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    window::WindowPlugin,
};
use game_engine::ipc::IpcPlugin;
use game_engine::status::{
    CharacterRosterStatusSnapshot, CharacterStatsSnapshot, CollectionStatusSnapshot,
    CombatLogStatusSnapshot, CurrenciesStatusSnapshot, EquipmentAppearanceStatusSnapshot,
    EquippedGearStatusSnapshot, GroupStatusSnapshot, GuildVaultStatusSnapshot, MapStatusSnapshot,
    NetworkStatusSnapshot, ProfessionStatusSnapshot, QuestLogStatusSnapshot,
    ReputationsStatusSnapshot,
    SoundStatusSnapshot, TerrainStatusSnapshot, WarbankStatusSnapshot,
};

mod action_bar;
mod animation;
mod asset;
mod camera;
mod campsite_popup_screen;
mod char_create;
mod char_create_scene;
mod char_select;
mod char_select_scene;
mod char_select_scene_tree;
mod character_customization;
mod character_models;
mod cli_args;
mod collision;
mod creature_display;
mod dump_systems;
mod equipment;
mod equipment_appearance;
mod game_state;
mod ground;
mod health_bar;
mod inworld_scene_tree;
mod login_screen;
mod login_screen_helpers;
mod m2_effect_material;
mod m2_scene;
pub mod m2_spawn;
mod minimap;
mod minimap_render;
mod nameplate;
mod networking;
mod networking_auth;
mod networking_messages;
mod particle;
mod scene_setup;
mod screen_auto_login;
mod sky;
mod sky_lightdata;
mod sky_material;
mod sound;
mod status_sync;
mod target;
mod terrain;
mod terrain_heightmap;
mod terrain_lod;
mod terrain_material;
mod terrain_objects;
mod terrain_tile;
mod trash_button_screen;
mod warband_scene;
mod water_material;
mod wow_cursor;

use animation::AnimationPlugin;
use camera::WowCameraPlugin;
use cli_args::*;
use collision::CollisionPlugin;
use scene_setup::{setup_default_world_scene, setup_explicit_asset_scene};
use terrain::AdtStreamingPlugin;

#[derive(Resource)]
struct DumpTreeFlag;
#[derive(Resource)]
struct DumpUiTreeFlag;
#[derive(Resource)]
struct DumpSceneFlag;
#[derive(Resource)]
struct ScreenshotRequest {
    output: PathBuf,
    frames_remaining: u32,
}

fn main() {
    ensure_asset_root();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if handle_simple_flags(&args) {
        return;
    }
    let cli = parse_cli_flags(&args);
    if cli.dump_ui_tree && !cli.dump_tree && cli.screenshot.is_none() {
        run_headless_ui_dump_app(cli.initial_state);
        return;
    }
    if let Some(path) = cli.load_scene {
        dump_loaded_scene_and_exit(&path, cli.dump_scene);
    }
    run_app(&args, cli.dump_tree, cli.dump_ui_tree, cli.dump_scene, cli.screenshot, cli.initial_state);
}

struct CliFlags {
    dump_tree: bool,
    dump_ui_tree: bool,
    dump_scene: bool,
    load_scene: Option<PathBuf>,
    screenshot: Option<ScreenshotRequest>,
    initial_state: Option<game_state::GameState>,
}

fn handle_simple_flags(args: &[String]) -> bool {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return true;
    }
    if args.iter().any(|a| a == "--version") {
        println!("game-engine {}", env!("CARGO_PKG_VERSION"));
        return true;
    }
    false
}

fn parse_cli_flags(args: &[String]) -> CliFlags {
    let load_scene = match parse_load_scene_arg(args) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    let initial_state = match parse_state_arg(args) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    CliFlags {
        dump_tree: args.iter().any(|a| a == "--dump-tree"),
        dump_ui_tree: args.iter().any(|a| a == "--dump-ui-tree"),
        dump_scene: args.iter().any(|a| a == "--dump-scene"),
        load_scene,
        screenshot: parse_screenshot_args(args),
        initial_state,
    }
}

fn dump_loaded_scene_and_exit(path: &Path, dump_scene: bool) -> ! {
    if !dump_scene {
        eprintln!("--load-scene currently requires --dump-scene");
        std::process::exit(1);
    }
    let snapshot = match game_engine::scene_tree::read_scene_snapshot_file(path) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    println!("{}", game_engine::dump::build_scene_snapshot(&snapshot));
    std::process::exit(0);
}

fn ensure_asset_root() {
    if std::env::var_os("BEVY_ASSET_ROOT").is_none() {
        unsafe {
            std::env::set_var("BEVY_ASSET_ROOT", env!("CARGO_MANIFEST_DIR"));
        }
    }
}

struct ParsedArgs {
    startup_actions: Vec<game_engine::ui::automation::UiAutomationAction>,
    server_addr: Option<(std::net::SocketAddr, bool)>,
    initial_state: Option<game_state::GameState>,
    auto_enter_world: bool,
    startup_login: Option<(String, String)>,
}

fn parse_run_args(args: &[String]) -> ParsedArgs {
    parse_run_args_with_saved_token(args, networking::load_auth_token().is_some())
}

fn parse_run_args_with_saved_token(args: &[String], has_saved_auth_token: bool) -> ParsedArgs {
    let mut parsed = parse_run_args_base(args);
    screen_auto_login::apply(
        &mut parsed.startup_actions,
        &mut parsed.server_addr,
        &mut parsed.initial_state,
        &mut parsed.auto_enter_world,
        &mut parsed.startup_login,
        has_saved_auth_token,
    );
    apply_login_dev_admin(args, &mut parsed);
    apply_auto_connecting(&parsed.startup_actions, parsed.server_addr, &mut parsed.initial_state, has_saved_auth_token);
    parsed
}

fn parse_run_args_base(args: &[String]) -> ParsedArgs {
    let startup_actions = match load_startup_automation_actions(args) {
        Ok(a) => a,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    let initial_state = match parse_state_arg(args) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    ParsedArgs {
        startup_actions,
        server_addr: parse_server_arg(args),
        initial_state,
        auto_enter_world: false,
        startup_login: None,
    }
}

fn apply_login_dev_admin(args: &[String], parsed: &mut ParsedArgs) {
    if !has_flag(args, "--login-dev-admin") {
        return;
    }
    parsed.server_addr = Some(("127.0.0.1:5000".parse().unwrap(), true));
    parsed.initial_state = Some(game_state::GameState::Connecting);
    parsed.startup_login = Some(("admin".to_string(), "admin".to_string()));
    parsed.startup_actions.clear();
    parsed.auto_enter_world = false;
}

fn apply_auto_connecting(
    actions: &[game_engine::ui::automation::UiAutomationAction],
    server_addr: Option<(std::net::SocketAddr, bool)>,
    initial_state: &mut Option<game_state::GameState>,
    has_saved_auth_token: bool,
) {
    if actions.is_empty() && server_addr.is_some() && initial_state.is_none() && has_saved_auth_token {
        *initial_state = Some(game_state::GameState::Connecting);
    }
}

fn run_app(
    args: &[String],
    dump_tree: bool,
    dump_ui_tree: bool,
    dump_scene: bool,
    screenshot: Option<ScreenshotRequest>,
    initial_state: Option<game_state::GameState>,
) {
    let mut parsed = parse_run_args(args);
    parsed.initial_state = parsed.initial_state.or(initial_state);
    let mut app = App::new();
    app.insert_resource(game_state::StartupPerfTimer(std::time::Instant::now()));
    register_plugins(&mut app);
    configure_app_plugins(&mut app, args, &parsed);
    dump_systems::configure_dump_systems(&mut app, dump_tree, dump_ui_tree, dump_scene, screenshot);
    insert_startup_resources(&mut app, args, parsed.startup_actions);
    if parsed.auto_enter_world {
        app.insert_resource(char_select::AutoEnterWorld);
    }
    app.run();
}

fn insert_startup_resources(
    app: &mut App,
    args: &[String],
    startup_actions: Vec<game_engine::ui::automation::UiAutomationAction>,
) {
    if !startup_actions.is_empty() {
        app.insert_resource(game_engine::ui::automation::UiAutomationQueue(
            VecDeque::from(startup_actions),
        ));
    }
    if let Some(name) = parse_char_arg(args) {
        app.insert_resource(char_select::PreselectedCharName(name));
    }
    insert_screen_resources(app, args);
    insert_data_resources(app);
}

fn insert_screen_resources(app: &mut App, args: &[String]) {
    match parse_screen_arg(args) {
        Ok(Some(game_engine::game_state_enum::ScreenArg::CharCreateCustomize)) => {
            app.insert_resource(char_create::StartupCharCreateMode(
                game_engine::ui::screens::char_create_component::CharCreateMode::Customize,
            ));
        }
        Ok(_) => {}
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn insert_data_resources(app: &mut App) {
    app.insert_resource(creature_display::CreatureDisplayMap::load_from_data_dir());
    app.insert_resource(game_engine::customization_data::CustomizationDb::load(
        Path::new("data"),
    ));
    app.insert_resource(game_engine::asset::char_texture::CharTextureData::load(
        Path::new("data"),
    ));
    app.insert_resource(game_engine::outfit_data::OutfitData::load(Path::new("data")));
    let warband = warband_scene::WarbandScenes::load();
    if let Some(first) = warband.scenes.first() {
        app.insert_resource(warband_scene::SelectedWarbandScene { scene_id: first.id });
    }
    app.insert_resource(warband);
}

fn default_plugins() -> bevy::app::PluginGroupBuilder {
    DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            name: Some("com.worldofosso.game-engine".to_string()),
            ..default()
        }),
        ..default()
    })
}

fn register_bevy_plugins(app: &mut App) {
    app.add_plugins(default_plugins())
        .add_plugins(game_engine::auction_house::AuctionHousePlugin)
        .add_plugins(game_engine::mail::MailPlugin)
        .add_plugins(game_engine::ui::plugin::UiPlugin)
        .add_plugins(game_engine::ui::automation::UiAutomationPlugin)
        .add_plugins(IpcPlugin)
        .add_plugins(WowCameraPlugin)
        .add_plugins(AnimationPlugin)
        .add_plugins(CollisionPlugin)
        .add_plugins(game_engine::culling::CullingPlugin)
        .add_plugins(AdtStreamingPlugin)
        .add_plugins(MaterialPlugin::<terrain_material::TerrainMaterial>::default())
        .add_plugins(m2_effect_material::M2EffectMaterialPlugin)
        .add_plugins(water_material::WaterMaterialPlugin)
        .add_plugins(minimap::MinimapPlugin)
        .add_plugins(action_bar::ActionBarPlugin)
        .add_plugins(sky::SkyPlugin)
        .add_plugins(health_bar::HealthBarPlugin)
        .add_plugins(nameplate::NameplatePlugin)
        .add_plugins(target::TargetPlugin)
        .add_plugins(particle::ParticlePlugin)
        .add_plugins(equipment::EquipmentPlugin)
        .add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
                refresh_interval: Duration::from_millis(500),
                ..default()
            },
        });
}

fn register_plugins(app: &mut App) {
    register_bevy_plugins(app);
    app.insert_resource(ui_toolkit::render_texture::BlpLoaderRes(Box::new(
        GameBlpLoader,
    )));
    app.add_systems(
        Startup,
        (
            setup_explicit_asset_scene,
            wow_cursor::install_wow_cursor,
            game_engine::ui::panel_styles::register_panel_styles,
        ),
    )
    .add_systems(
        Update,
        wow_cursor::update_wow_cursor_style.run_if(in_state(game_state::GameState::InWorld)),
    );
    init_status_resources(app);
}

struct GameBlpLoader;

impl ui_toolkit::render_texture::BlpLoader for GameBlpLoader {
    fn load_blp_to_image(&self, path: &std::path::Path) -> Result<bevy::image::Image, String> {
        game_engine::asset::blp::load_blp_to_image(path)
    }
    fn load_blp_gpu_image(&self, path: &std::path::Path) -> Result<bevy::image::Image, String> {
        game_engine::asset::blp::load_blp_gpu_image(path)
    }
    fn ensure_texture(&self, fdid: u32) -> Option<PathBuf> {
        let path = PathBuf::from(format!("data/textures/{fdid}.blp"));
        if path.exists() { Some(path) } else { None }
    }
}

fn configure_server_resources(
    app: &mut App,
    enable_sound: bool,
    server_addr: Option<(std::net::SocketAddr, bool)>,
    initial_state: Option<game_state::GameState>,
    startup_login: Option<(String, String)>,
) {
    if enable_sound {
        app.add_plugins(sound::SoundPlugin);
    }
    if let Some((addr, dev)) = server_addr {
        app.insert_resource(networking::ServerAddr(addr));
        if dev {
            app.insert_resource(login_screen::DevServer);
        }
    }
    if let Some(state) = initial_state {
        app.insert_resource(game_state::InitialGameState(state));
    }
    if let Some((username, password)) = startup_login {
        app.insert_resource(networking::LoginUsername(username));
        app.insert_resource(networking::LoginPassword(password));
        app.insert_resource(networking::LoginMode::Login);
    }
}

fn add_status_sync_systems(app: &mut App) {
    app.add_systems(Update, status_sync::sync_character_roster_status_snapshot);
    app.add_systems(
        Update,
        (
            status_sync::sync_network_status_snapshot,
            status_sync::sync_terrain_status_snapshot,
            status_sync::sync_sound_status_snapshot,
            status_sync::sync_character_stats_snapshot,
            status_sync::apply_equipment_ipc_commands,
            status_sync::sync_equipped_gear_status_snapshot,
            status_sync::sync_equipment_appearance_status_snapshot,
            status_sync::sync_map_status_snapshot,
        )
            .run_if(in_state(game_state::GameState::InWorld)),
    );
}

fn configure_app_plugins(app: &mut App, args: &[String], parsed: &ParsedArgs) {
    configure_server_resources(
        app,
        args.iter().any(|a| a == "--sound"),
        parsed.server_addr,
        parsed.initial_state,
        parsed.startup_login.clone(),
    );
    app.add_plugins(game_state::GameStatePlugin);
    app.add_plugins(networking::NetworkPlugin);
    app.add_plugins(login_screen::LoginScreenPlugin);
    app.add_plugins(char_select::CharSelectPlugin);
    app.add_plugins(char_select_scene::CharSelectScenePlugin);
    app.add_plugins(char_create::CharCreatePlugin);
    app.add_plugins(char_create_scene::CharCreateScenePlugin);
    app.add_plugins(campsite_popup_screen::CampsitePopupScreenPlugin);
    app.add_plugins(trash_button_screen::TrashButtonScreenPlugin);
    app.add_systems(
        OnEnter(game_state::GameState::InWorld),
        setup_default_world_scene,
    );
    add_status_sync_systems(app);
}

fn run_headless_ui_dump_app(initial_state: Option<game_state::GameState>) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(game_engine::ui::plugin::UiState {
        registry: game_engine::ui::registry::FrameRegistry::new(1920.0, 1080.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(DumpUiTreeFlag);
    if let Some(state) = initial_state {
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_resource(game_state::InitialGameState(state));
        app.add_plugins(game_state::GameStatePlugin);
        if matches!(state, game_state::GameState::Login) {
            app.init_resource::<networking::AuthUiFeedback>();
            app.add_plugins(login_screen::LoginScreenPlugin);
        }
    }
    app.add_systems(PostStartup, dump_systems::headless_dump_ui_tree_immediate);
    app.run();
}

fn init_status_resources(app: &mut App) {
    app.insert_resource(NetworkStatusSnapshot::default())
        .insert_resource(TerrainStatusSnapshot::default())
        .insert_resource(SoundStatusSnapshot::default())
        .insert_resource(CharacterRosterStatusSnapshot::default())
        .insert_resource(CharacterStatsSnapshot::default())
        .insert_resource(EquippedGearStatusSnapshot::default())
        .insert_resource(EquipmentAppearanceStatusSnapshot::default())
        .insert_resource(MapStatusSnapshot::default())
        .insert_resource(CollectionStatusSnapshot::default())
        .insert_resource(CombatLogStatusSnapshot::default())
        .insert_resource(CurrenciesStatusSnapshot::default())
        .insert_resource(GroupStatusSnapshot::default())
        .insert_resource(GuildVaultStatusSnapshot::default())
        .insert_resource(ProfessionStatusSnapshot::default())
        .insert_resource(QuestLogStatusSnapshot::default())
        .insert_resource(ReputationsStatusSnapshot::default())
        .insert_resource(WarbankStatusSnapshot::default());
}

fn take_screenshot(
    mut commands: Commands,
    req: Option<ResMut<ScreenshotRequest>>,
    automation_queue: Option<Res<game_engine::ui::automation::UiAutomationQueue>>,
    state: Res<State<crate::game_state::GameState>>,
) {
    let Some(mut req) = req else { return };
    if automation_queue.is_some_and(|q| !q.0.is_empty()) {
        return;
    }
    if matches!(
        *state.get(),
        crate::game_state::GameState::Login | crate::game_state::GameState::Connecting
    ) {
        return;
    }
    if req.frames_remaining > 0 {
        req.frames_remaining -= 1;
        return;
    }
    commands.remove_resource::<ScreenshotRequest>();
    let output = req.output.clone();
    commands.spawn(Screenshot::primary_window()).observe(
        move |trigger: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            save_screenshot(&trigger.image, &output);
            exit.write(AppExit::Success);
        },
    );
}

fn save_screenshot(img: &bevy::image::Image, output: &PathBuf) {
    let webp_data = match game_engine::screenshot::encode_webp(img, 15.0) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };
    std::fs::write(output, &webp_data)
        .unwrap_or_else(|e| eprintln!("Failed to write {}: {e}", output.display()));
    println!("Saved {} ({} bytes)", output.display(), webp_data.len());
}

pub fn rgba_image(pixels: Vec<u8>, w: u32, h: u32) -> Image {
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[cfg(test)]
mod tests {
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

        let parsed = parse_state_arg(&args(&["--screen", "trashbutton"]))
            .expect("expected valid parse")
            .expect("expected trashbutton");
        assert_eq!(parsed, game_state::GameState::TrashButton);
    }

    #[test]
    fn parse_screen_rejects_non_screen_states() {
        let err = parse_state_arg(&args(&["--screen", "connecting"]))
            .expect_err("connecting should be rejected for --screen");
        assert_eq!(
            err,
            "invalid --screen value 'connecting': expected one of: login, charselect, charcreate, charcreate-customize, campsitepopup, inworld, trashbutton"
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
    fn parse_load_scene_flag() {
        let parsed = parse_load_scene_arg(&args(&["--load-scene", "data/debug/scene.json"]))
            .expect("expected valid parse");
        assert_eq!(parsed, Some(PathBuf::from("data/debug/scene.json")));
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
        let parsed = parse_asset_path_from_args(&args(&[
            "--login-dev-admin",
            "data/models/humanmale_hd.m2",
        ]));
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
    fn parse_run_args_starts_connecting_when_saved_token_exists() {
        let parsed = parse_run_args_with_saved_token(&args(&["--server", "127.0.0.1:25565"]), true);

        assert_eq!(
            parsed.initial_state,
            Some(game_state::GameState::Connecting)
        );
        assert!(parsed.startup_actions.is_empty());
    }

    #[test]
    fn parse_run_args_keeps_explicit_login_screen_with_saved_token() {
        let parsed = parse_run_args_with_saved_token(
            &args(&["--server", "127.0.0.1:25565", "--screen", "login"]),
            true,
        );

        assert_eq!(parsed.initial_state, Some(game_state::GameState::Login));
        assert!(parsed.startup_actions.is_empty());
    }

    #[test]
    fn parse_run_args_login_dev_admin_forces_login_flow() {
        let parsed = parse_run_args_with_saved_token(&args(&["--login-dev-admin"]), true);

        assert_eq!(
            parsed.initial_state,
            Some(game_state::GameState::Connecting)
        );
        assert_eq!(
            parsed.server_addr,
            Some(("127.0.0.1:5000".parse().unwrap(), true))
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
        let parsed = parse_run_args_with_saved_token(&args(&["--server", "127.0.0.1:25565"]), true);
        let resolved = parsed.initial_state.or(None);
        assert_eq!(resolved, Some(game_state::GameState::Connecting));
    }

    #[test]
    fn resolved_initial_state_keeps_parsed_rewritten_state() {
        let parsed = parse_run_args_with_saved_token(&args(&["--server", "127.0.0.1:25565"]), true);
        let resolved = parsed.initial_state.or(Some(game_state::GameState::Login));
        assert_eq!(resolved, Some(game_state::GameState::Connecting));
    }

    #[test]
    fn startup_scene_loading_only_runs_for_explicit_assets() {
        use crate::scene_setup::should_load_explicit_scene_at_startup;
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
}
