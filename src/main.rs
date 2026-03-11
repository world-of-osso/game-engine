use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::WindowPlugin;
use game_engine::ipc::IpcPlugin;
use game_engine::status::{
    CharacterStatsSnapshot, CollectionStatusSnapshot, CombatLogStatusSnapshot,
    CurrenciesStatusSnapshot, EquippedGearStatusSnapshot, GroupStatusSnapshot,
    GuildVaultStatusSnapshot, MapStatusSnapshot, NetworkStatusSnapshot, ProfessionStatusSnapshot,
    QuestLogStatusSnapshot, ReputationsStatusSnapshot, SoundStatusSnapshot, TerrainStatusSnapshot,
    WarbankStatusSnapshot,
};

mod action_bar;
mod animation;
mod asset;
mod camera;
mod char_select;
mod creature_display;
mod equipment;
mod game_state;
mod ground;
mod health_bar;
mod login_screen;
mod m2_scene;
pub mod m2_spawn;
mod minimap;
mod nameplate;
mod networking;
mod networking_auth;
mod particle;
mod scene_setup;
mod sky;
mod sky_lightdata;
mod sky_material;
mod sound;
mod status_sync;
mod target;
mod terrain;
mod terrain_heightmap;
mod terrain_material;
mod terrain_objects;
mod water_material;
mod wow_cursor;

use animation::AnimationPlugin;
use camera::WowCameraPlugin;
use scene_setup::{setup_default_world_scene, setup_explicit_asset_scene};
use terrain::AdtStreamingPlugin;

#[derive(Resource)]
struct DumpTreeFlag;
#[derive(Resource)]
struct DumpUiTreeFlag;
#[derive(Resource)]
struct ScreenshotRequest {
    output: PathBuf,
    frames_remaining: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") {
        println!("game-engine {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
    let dump_tree = args.iter().any(|a| a == "--dump-tree");
    let dump_ui_tree = args.iter().any(|a| a == "--dump-ui-tree");
    let screenshot = parse_screenshot_args(&args);
    if dump_ui_tree && !dump_tree && screenshot.is_none() {
        run_headless_ui_dump_app(parse_state_arg(&args));
        return;
    }
    run_app(&args, dump_tree, dump_ui_tree, screenshot);
}

fn run_app(
    args: &[String],
    dump_tree: bool,
    dump_ui_tree: bool,
    screenshot: Option<ScreenshotRequest>,
) {
    let startup_actions = match load_startup_automation_actions(args) {
        Ok(a) => a,
        Err(err) => { eprintln!("{err}"); std::process::exit(1); }
    };
    let mut app = App::new();
    register_plugins(&mut app);
    configure_app_plugins(
        &mut app,
        args.iter().any(|a| a == "--sound"),
        parse_server_arg(args),
        parse_state_arg(args),
        dump_tree,
        dump_ui_tree,
        screenshot,
    );
    if !startup_actions.is_empty() {
        app.insert_resource(game_engine::ui::automation::UiAutomationQueue(
            VecDeque::from(startup_actions),
        ));
    }
    app.insert_resource(creature_display::CreatureDisplayMap::load_from_data_dir());
    app.run();
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
        .add_plugins(game_engine::culling::CullingPlugin)
        .add_plugins(AdtStreamingPlugin)
        .add_plugins(MaterialPlugin::<terrain_material::TerrainMaterial>::default())
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
            config: FpsOverlayConfig { refresh_interval: Duration::from_millis(500), ..default() },
        });
}

fn register_plugins(app: &mut App) {
    register_bevy_plugins(app);
    app.insert_resource(ui_toolkit::render_texture::BlpLoaderRes(Box::new(GameBlpLoader)));
    app.add_systems(Startup, (setup_explicit_asset_scene, wow_cursor::install_wow_cursor))
        .add_systems(Update, wow_cursor::update_wow_cursor_style.run_if(in_state(game_state::GameState::InWorld)));
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
) {
    if enable_sound { app.add_plugins(sound::SoundPlugin); }
    if let Some((addr, dev)) = server_addr {
        app.insert_resource(networking::ServerAddr(addr));
        if dev { app.insert_resource(login_screen::DevServer); }
    }
    if let Some(state) = initial_state {
        app.insert_resource(game_state::InitialGameState(state));
    }
}

fn add_status_sync_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            status_sync::sync_network_status_snapshot,
            status_sync::sync_terrain_status_snapshot,
            status_sync::sync_sound_status_snapshot,
            status_sync::sync_character_stats_snapshot,
            status_sync::apply_equipment_ipc_commands,
            status_sync::sync_equipped_gear_status_snapshot,
            status_sync::sync_map_status_snapshot,
        ).run_if(in_state(game_state::GameState::InWorld)),
    );
}

fn configure_app_plugins(
    app: &mut App,
    enable_sound: bool,
    server_addr: Option<(std::net::SocketAddr, bool)>,
    initial_state: Option<game_state::GameState>,
    dump_tree: bool,
    dump_ui_tree: bool,
    screenshot: Option<ScreenshotRequest>,
) {
    configure_server_resources(app, enable_sound, server_addr, initial_state);
    app.add_plugins(game_state::GameStatePlugin);
    app.add_plugins(networking::NetworkPlugin);
    app.add_plugins(login_screen::LoginScreenPlugin);
    app.add_plugins(char_select::CharSelectPlugin);
    app.add_systems(OnEnter(game_state::GameState::InWorld), setup_default_world_scene);
    app.add_systems(Update, handle_automation_dump_tree_request);
    app.add_systems(Update, handle_automation_dump_ui_tree_request);
    add_status_sync_systems(app);
    if dump_tree { app.insert_resource(DumpTreeFlag); app.add_systems(PostStartup, dump_tree_and_exit); }
    if dump_ui_tree { app.insert_resource(DumpUiTreeFlag); app.add_systems(PostStartup, dump_ui_tree_and_exit); }
    if let Some(req) = screenshot { app.insert_resource(req); app.add_systems(Update, take_screenshot); }
}

fn run_headless_ui_dump_app(initial_state: Option<game_state::GameState>) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Insert UiState directly instead of UiPlugin to avoid rendering systems
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
    app.add_systems(PostStartup, headless_dump_ui_tree_immediate);
    app.run();
}

fn init_status_resources(app: &mut App) {
    app.insert_resource(NetworkStatusSnapshot::default())
        .insert_resource(TerrainStatusSnapshot::default())
        .insert_resource(SoundStatusSnapshot::default())
        .insert_resource(CharacterStatsSnapshot::default())
        .insert_resource(EquippedGearStatusSnapshot::default())
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

fn screenshot_arg_index(args: &[String]) -> Option<usize> {
    args.iter().position(|arg| arg == "screenshot")
}

fn parse_screenshot_args(args: &[String]) -> Option<ScreenshotRequest> {
    let screenshot_idx = screenshot_arg_index(args)?;
    let output = args.get(screenshot_idx + 1).map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("screenshot.webp"));
    let has_server = args.windows(2).any(|w| w[0] == "--server");
    Some(ScreenshotRequest { output, frames_remaining: if has_server { 60 } else { 3 } })
}

fn parse_server_arg(args: &[String]) -> Option<(std::net::SocketAddr, bool)> {
    let w = args.windows(2).find(|w| w[0] == "--server")?;
    if w[1] == "dev" { Some(("127.0.0.1:5000".parse().unwrap(), true)) }
    else { w[1].parse().ok().map(|addr| (addr, false)) }
}

fn parse_state_arg(args: &[String]) -> Option<game_state::GameState> {
    args.windows(2).find(|w| w[0] == "--state" || w[0] == "--screen")
        .and_then(|w| parse_game_state_value(&w[1]))
}

fn parse_game_state_value(value: &str) -> Option<game_state::GameState> {
    match value {
        "login" => Some(game_state::GameState::Login),
        "connecting" => Some(game_state::GameState::Connecting),
        "charselect" => Some(game_state::GameState::CharSelect),
        "loading" => Some(game_state::GameState::Loading),
        "inworld" => Some(game_state::GameState::InWorld),
        _ => None,
    }
}

fn load_startup_automation_actions(
    args: &[String],
) -> Result<Vec<game_engine::ui::automation::UiAutomationAction>, String> {
    let mut actions = Vec::new();
    if let Some(script) = game_engine::ui::automation_script::parse_automation_script_arg(args) {
        actions.extend(game_engine::ui::automation_script::load_automation_script(&script.path)?);
    }
    if let Some(script) = game_engine::ui::js_automation::parse_js_automation_arg(args) {
        actions.extend(game_engine::ui::js_automation::load_js_automation_script(&script.path)?);
    }
    Ok(actions)
}

fn parse_asset_path_from_args(args: &[String]) -> Option<PathBuf> {
    let screenshot_idx = screenshot_arg_index(args);
    let mut i = 0;
    while i < args.len() {
        if screenshot_idx == Some(i) {
            i += 2;
            continue;
        }
        match args[i].as_str() {
            "--server" | "--state" | "--screen" => { i += 2; }
            arg if arg.starts_with("--") => { i += 1; }
            path => return Some(PathBuf::from(path)),
        }
    }
    None
}

pub(crate) fn parse_asset_path() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    parse_asset_path_from_args(&args)
}

fn take_screenshot(mut commands: Commands, req: Option<ResMut<ScreenshotRequest>>) {
    let Some(mut req) = req else { return };
    if req.frames_remaining > 0 { req.frames_remaining -= 1; return; }
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
        Err(err) => { eprintln!("{err}"); return; }
    };
    std::fs::write(output, &webp_data)
        .unwrap_or_else(|e| eprintln!("Failed to write {}: {e}", output.display()));
    println!("Saved {} ({} bytes)", output.display(), webp_data.len());
}

pub fn rgba_image(pixels: Vec<u8>, w: u32, h: u32) -> Image {
    Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[allow(clippy::type_complexity)]
fn dump_tree_and_exit(
    tree_query: Query<(Entity, Option<&Name>, Option<&Children>, Option<&Visibility>, &Transform)>,
    parent_query: Query<&ChildOf>,
    mut exit: MessageWriter<AppExit>,
) {
    let tree = game_engine::dump::build_tree(&tree_query, &parent_query, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}

fn dump_ui_tree_and_exit(
    mut ui_state: ResMut<game_engine::ui::plugin::UiState>,
    mut spellbook_runtime: Option<NonSendMut<game_engine::ui::spellbook_runtime::SpellbookUiRuntime>>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(ref mut rt) = spellbook_runtime { rt.sync(&mut ui_state.registry); }
    action_bar::ensure_action_bars(&mut ui_state.registry);
    let tree = game_engine::dump::build_ui_tree(&ui_state.registry, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}

fn headless_dump_ui_tree_immediate(ui_state: ResMut<game_engine::ui::plugin::UiState>) {
    let tree = game_engine::dump::build_ui_tree(&ui_state.registry, None);
    println!("{tree}");
    std::process::exit(0);
}

#[allow(clippy::type_complexity)]
fn handle_automation_dump_tree_request(
    request: Option<Res<game_engine::ui::automation::UiAutomationDumpTreeRequest>>,
    tree_query: Query<(Entity, Option<&Name>, Option<&Children>, Option<&Visibility>, &Transform)>,
    parent_query: Query<&ChildOf>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    if request.is_none() { return; }
    commands.remove_resource::<game_engine::ui::automation::UiAutomationDumpTreeRequest>();
    let tree = game_engine::dump::build_tree(&tree_query, &parent_query, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}

fn handle_automation_dump_ui_tree_request(
    request: Option<Res<game_engine::ui::automation::UiAutomationDumpUiTreeRequest>>,
    mut ui_state: ResMut<game_engine::ui::plugin::UiState>,
    mut spellbook_runtime: Option<NonSendMut<game_engine::ui::spellbook_runtime::SpellbookUiRuntime>>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    if request.is_none() { return; }
    commands.remove_resource::<game_engine::ui::automation::UiAutomationDumpUiTreeRequest>();
    if let Some(ref mut rt) = spellbook_runtime { rt.sync(&mut ui_state.registry); }
    action_bar::ensure_action_bars(&mut ui_state.registry);
    let tree = game_engine::dump::build_ui_tree(&ui_state.registry, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene_setup::should_load_explicit_scene_at_startup;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn screenshot_args_allow_flags_before_command() {
        let parsed = parse_screenshot_args(&args(&[
            "--state", "login", "screenshot", "/tmp/codex/test.webp", "--server", "127.0.0.1:25565",
        ])).expect("expected screenshot request");
        assert_eq!(parsed.output, PathBuf::from("/tmp/codex/test.webp"));
        assert_eq!(parsed.frames_remaining, 60);
    }

    #[test]
    fn parse_screen_alias_matches_state_parser() {
        let parsed = parse_state_arg(&args(&["--screen", "charselect"])).expect("expected screen alias");
        assert_eq!(parsed, game_state::GameState::CharSelect);
        let parsed = parse_state_arg(&args(&["--screen", "login"])).expect("expected login");
        assert_eq!(parsed, game_state::GameState::Login);
    }

    #[test]
    fn asset_path_skips_state_and_screenshot_output() {
        let parsed = parse_asset_path_from_args(&args(&[
            "--state", "login", "screenshot", "/tmp/codex/test.webp", "--server", "127.0.0.1:25565",
        ]));
        assert_eq!(parsed, None);
    }

    #[test]
    fn asset_path_skips_screen_alias_and_screenshot_output() {
        let parsed = parse_asset_path_from_args(&args(&[
            "--screen", "charselect", "screenshot", "/tmp/codex/test.webp", "--server", "127.0.0.1:25565",
        ]));
        assert_eq!(parsed, None);
    }

    #[test]
    fn asset_path_after_screenshot_is_preserved() {
        let parsed = parse_asset_path_from_args(&args(&[
            "--state", "inworld", "screenshot", "/tmp/codex/test.webp", "data/models/humanmale_hd.m2",
        ]));
        assert_eq!(parsed, Some(PathBuf::from("data/models/humanmale_hd.m2")));
    }

    #[test]
    fn startup_flag_loads_ui_script_path() {
        let actions = load_startup_automation_actions(&args(&["--run-ui-script", "/tmp/codex/test-ui-script.json"]));
        assert!(actions.is_err());
        let parsed = game_engine::ui::automation_script::parse_automation_script_arg(&args(&[
            "--run-ui-script", "debug/login.json",
        ])).expect("expected UI script path");
        assert_eq!(parsed.path, PathBuf::from("debug/login.json"));
    }

    #[test]
    fn parse_js_automation_flag() {
        let parsed = game_engine::ui::js_automation::parse_js_automation_arg(&args(&[
            "--state", "login", "--run-js-ui-script", "debug/login.js",
        ])).expect("expected JS automation path");
        assert_eq!(parsed.path, PathBuf::from("debug/login.js"));
    }

    #[test]
    fn startup_scene_loading_only_runs_for_explicit_assets() {
        use std::path::Path;
        use crate::scene_setup::should_load_explicit_scene_at_startup;
        assert!(!should_load_explicit_scene_at_startup(false, None));
        assert!(should_load_explicit_scene_at_startup(false, Some(Path::new("data/models/humanmale_hd.m2"))));
        assert!(!should_load_explicit_scene_at_startup(true, Some(Path::new("data/models/humanmale_hd.m2"))));
    }
}
