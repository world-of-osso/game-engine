#![allow(
    dead_code,
    clippy::collapsible_if,
    clippy::collapsible_str_replace,
    clippy::derivable_impls,
    clippy::needless_borrow,
    clippy::needless_lifetimes,
    clippy::needless_option_as_deref,
    clippy::question_mark,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

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
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::Duration,
};

mod action_bar;
mod animation;
mod asset {
    // Re-export the library asset module so process-global state such as the CASC
    // resolver OnceLock is shared instead of compiled into the binary twice.
    pub use game_engine::asset::*;
}
mod camera;
mod campsite_popup_screen;
mod char_create;
mod char_create_scene;
mod char_select;
mod char_select_input;
mod char_select_scene;
mod char_select_scene_tree;
mod character_customization;
mod character_models;
mod cli_args;
mod client_options;
mod collision;
mod creature_display;
mod dump_systems;
mod equipment;
mod equipment_appearance;
mod game_menu_options;
mod game_menu_screen;
mod game_state;
mod geoset_debug_scene;
mod ground;
mod health_bar;
mod inworld_scene_tree;
mod inworld_selection_debug_screen;
mod loading_screen;
mod login_screen;
mod login_screen_helpers;
mod m2_effect_material;
mod m2_scene;
pub mod m2_spawn;
mod m2_texture_composite;
mod minimap;
mod minimap_render;
mod nameplate;
mod networking;
mod networking_auth;
mod networking_messages;
mod networking_npc;
mod networking_player;
mod networking_reconnect;
mod orbit_camera;
mod particle;
mod particle_debug_scene;
mod process_limits;
mod scene_setup;
mod screen_auto_login;
mod selection_debug_screen;
mod sky;
mod sky_lightdata;
mod sky_material;
mod skybox_debug_scene;
mod skybox_m2_material;
mod sound;
mod sound_footsteps;
mod sound_music_catalog;
mod status_asset_stats;
mod status_sync;
mod target;
mod terrain;
mod terrain_heightmap;
mod terrain_load_limits;
mod terrain_load_progress;
mod terrain_lod;
mod terrain_material;
mod terrain_memory_debug;
mod terrain_objects;
mod terrain_tile;
mod trash_button_screen;
mod unit_frames;
mod warband_scene;
mod water_material;
mod wow_cursor;
mod zone_names;

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
    configure_thread_pools();
    ensure_asset_root();
    process_limits::apply_resource_limits();
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
    run_app(
        &args,
        cli.dump_tree,
        cli.dump_ui_tree,
        cli.dump_scene,
        cli.screenshot,
        cli.initial_state,
    );
}

fn configure_thread_pools() {
    if std::env::var_os("RAYON_NUM_THREADS").is_none() {
        // Lightyear transitively initializes Rayon during replication apply.
        // Keep that pool small so it doesn't oversubscribe alongside Bevy task pools.
        unsafe {
            std::env::set_var("RAYON_NUM_THREADS", "2");
        }
    }
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
    server_addr: Option<cli_args::ServerArg>,
    initial_state: Option<game_state::GameState>,
    auto_enter_world: bool,
    startup_login: Option<(String, String)>,
}

fn parse_run_args(args: &[String]) -> ParsedArgs {
    let mut parsed = parse_run_args_base(args);
    let startup_credentials =
        client_options::load_login_credentials().map(|creds| (creds.username, creds.password));
    let startup_credentials_path = client_options::login_credentials_path();
    let auth_server = parsed
        .server_addr
        .as_ref()
        .map(|server| server.hostname.as_str())
        .unwrap_or(cli_args::DEFAULT_SERVER_ADDR);
    let has_saved_auth_token = networking::load_auth_token(Some(auth_server)).is_some();
    if startup_credentials.is_some() && !has_saved_auth_token {
        info!(
            "Startup auth: using credentials file {} because no saved token was found",
            startup_credentials_path.display()
        );
    }
    finalize_run_args(&mut parsed, args, has_saved_auth_token, startup_credentials);
    parsed
}

fn parse_run_args_with_saved_token(
    args: &[String],
    has_saved_auth_token: bool,
    startup_credentials: Option<(String, String)>,
) -> ParsedArgs {
    let mut parsed = parse_run_args_base(args);
    finalize_run_args(&mut parsed, args, has_saved_auth_token, startup_credentials);
    parsed
}

fn finalize_run_args(
    parsed: &mut ParsedArgs,
    args: &[String],
    has_saved_auth_token: bool,
    startup_credentials: Option<(String, String)>,
) {
    if parsed.initial_state.is_none() && parsed.startup_login.is_none() {
        parsed.startup_login = startup_credentials.clone();
    }
    screen_auto_login::apply(
        &mut parsed.startup_actions,
        &mut parsed.server_addr,
        &mut parsed.initial_state,
        &mut parsed.auto_enter_world,
        &mut parsed.startup_login,
        has_saved_auth_token,
        startup_credentials,
    );
    apply_login_dev_admin(args, parsed);
    apply_auto_connecting(
        &parsed.startup_actions,
        &mut parsed.initial_state,
        has_saved_auth_token,
        parsed.startup_login.is_some(),
    );
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
    parsed.server_addr = Some(cli_args::ServerArg::dev());
    parsed.initial_state = Some(game_state::GameState::Connecting);
    parsed.startup_login = Some(("admin".to_string(), "admin".to_string()));
    parsed.startup_actions.clear();
    parsed.auto_enter_world = false;
}

fn apply_auto_connecting(
    actions: &[game_engine::ui::automation::UiAutomationAction],
    initial_state: &mut Option<game_state::GameState>,
    has_saved_auth_token: bool,
    has_startup_login: bool,
) {
    if actions.is_empty() && initial_state.is_none() && (has_saved_auth_token || has_startup_login)
    {
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
    configure_app_plugins(&mut app, args, &mut parsed);
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
        Ok(Some(
            screen @ (game_engine::game_state_enum::ScreenArg::CharCreate
            | game_engine::game_state_enum::ScreenArg::CharCreateCustomize),
        )) => {
            app.insert_resource(game_state::StartupScreenTarget(screen.into()));
            if matches!(
                screen,
                game_engine::game_state_enum::ScreenArg::CharCreateCustomize
            ) {
                app.insert_resource(char_create::StartupCharCreateMode(
                    game_engine::ui::screens::char_create_component::CharCreateMode::Customize,
                ));
            }
        }
        Ok(Some(game_engine::game_state_enum::ScreenArg::OptionsMenu)) => {
            app.insert_resource(game_menu_screen::StartupGameMenuView(
                game_engine::ui::screens::game_menu_component::GameMenuView::Options,
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
    app.insert_resource(game_engine::outfit_data::OutfitData::load(Path::new(
        "data",
    )));
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
        .add_plugins(client_options::ClientOptionsPlugin)
        .add_plugins(WowCameraPlugin)
        .add_plugins(AnimationPlugin)
        .add_plugins(CollisionPlugin)
        .add_plugins(game_engine::culling::CullingPlugin)
        .add_plugins(AdtStreamingPlugin)
        .add_plugins(MaterialPlugin::<terrain_material::TerrainMaterial>::default())
        .add_plugins(m2_effect_material::M2EffectMaterialPlugin)
        .add_plugins(skybox_m2_material::SkyboxM2MaterialPlugin)
        .add_plugins(water_material::WaterMaterialPlugin)
        .add_plugins(minimap::MinimapPlugin)
        .add_plugins(action_bar::ActionBarPlugin)
        .add_plugins(unit_frames::InWorldUnitFramesPlugin)
        .add_plugins(sky::SkyPlugin)
        .add_plugins(health_bar::HealthBarPlugin)
        .add_plugins(nameplate::NameplatePlugin)
        .add_plugins(target::TargetPlugin)
        .add_plugins(particle::ParticlePlugin)
        .add_plugins(equipment::EquipmentPlugin)
        .add_plugins(character_customization::CharacterCustomizationPlugin)
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
            log_window_backend,
            setup_explicit_asset_scene,
            wow_cursor::install_wow_cursor,
            game_engine::ui::panel_styles::register_panel_styles,
        ),
    )
    .add_systems(
        Update,
        wow_cursor::update_wow_cursor_style.run_if(in_state(game_state::GameState::InWorld)),
    );
    status_sync::init_status_resources(app);
}

fn log_window_backend(display: Option<Res<bevy::winit::DisplayHandleWrapper>>) {
    let Some(display) = display else {
        info!("Window backend: unavailable (no display handle resource)");
        return;
    };
    let Ok(handle) = display.0.display_handle() else {
        warn!("Window backend: unavailable (failed to acquire display handle)");
        return;
    };
    let backend = match handle.as_raw() {
        RawDisplayHandle::Wayland(_) => "Wayland",
        RawDisplayHandle::Xlib(_) => "X11 (Xlib)",
        RawDisplayHandle::Xcb(_) => "X11 (XCB)",
        RawDisplayHandle::Windows(_) => "Windows",
        RawDisplayHandle::AppKit(_) => "AppKit",
        RawDisplayHandle::UiKit(_) => "UIKit",
        RawDisplayHandle::Android(_) => "Android",
        RawDisplayHandle::Web(_) => "Web",
        RawDisplayHandle::Orbital(_) => "Orbital",
        _ => "Unknown",
    };
    info!(
        "Window backend: {backend} (DISPLAY={:?}, WAYLAND_DISPLAY={:?})",
        std::env::var_os("DISPLAY"),
        std::env::var_os("WAYLAND_DISPLAY")
    );
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
    server_arg: Option<cli_args::ServerArg>,
    initial_state: Option<game_state::GameState>,
    startup_login: Option<(String, String)>,
) {
    if enable_sound {
        app.add_plugins(sound::SoundPlugin);
    }
    let server_arg = server_arg.or_else(|| {
        if initial_state == Some(game_state::GameState::Connecting) {
            match cli_args::default_server_arg() {
                Ok(server) => Some(server),
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            }
        } else {
            None
        }
    });
    if let Some(server) = server_arg {
        app.insert_resource(networking::ServerAddr(server.addr));
        app.insert_resource(networking::ServerHostname(server.hostname));
        if server.dev {
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

fn configure_app_plugins(app: &mut App, args: &[String], parsed: &mut ParsedArgs) {
    #[cfg(debug_assertions)]
    game_engine::ui::screen::init_global_hot_reload(vec![std::path::PathBuf::from(
        "src/ui/screens",
    )]);

    configure_server_resources(
        app,
        args.iter().any(|a| a == "--sound"),
        parsed.server_addr.take(),
        parsed.initial_state,
        parsed.startup_login.clone(),
    );
    add_screen_plugins(app, parsed.initial_state);
    app.add_systems(
        OnEnter(game_state::GameState::InWorld),
        setup_default_world_scene,
    );
    status_sync::register_status_sync_systems(app);
}

fn add_screen_plugins(app: &mut App, initial_state: Option<game_state::GameState>) {
    app.add_plugins((
        game_state::GameStatePlugin,
        networking::NetworkPlugin,
        login_screen::LoginScreenPlugin,
        loading_screen::LoadingScreenPlugin,
        char_select::CharSelectPlugin,
        char_select_scene::CharSelectScenePlugin,
        selection_debug_screen::SelectionDebugScreenPlugin,
        inworld_selection_debug_screen::InWorldSelectionDebugScreenPlugin,
        char_create::CharCreatePlugin,
        char_create_scene::CharCreateScenePlugin,
        campsite_popup_screen::CampsitePopupScreenPlugin,
        game_menu_screen::GameMenuScreenPlugin,
        trash_button_screen::TrashButtonScreenPlugin,
        orbit_camera::OrbitCameraPlugin,
    ));
    match initial_state {
        Some(game_state::GameState::DebugCharacter) => {
            app.add_plugins(geoset_debug_scene::DebugCharacterScenePlugin);
        }
        Some(game_state::GameState::SkyboxDebug) => {
            app.add_plugins(skybox_debug_scene::SkyboxDebugScenePlugin);
        }
        Some(game_state::GameState::ParticleDebug) => {
            app.add_plugins(particle_debug_scene::ParticleDebugScenePlugin);
        }
        _ => {}
    }
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
        } else if matches!(state, game_state::GameState::SelectionDebug) {
            app.add_plugins(selection_debug_screen::SelectionDebugScreenPlugin);
        } else if matches!(state, game_state::GameState::InWorldSelectionDebug) {
            app.add_plugins(inworld_selection_debug_screen::InWorldSelectionDebugScreenPlugin);
        }
    }
    app.add_systems(PostStartup, dump_systems::headless_dump_ui_tree_immediate);
    app.run();
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
#[path = "main_tests.rs"]
mod tests;
