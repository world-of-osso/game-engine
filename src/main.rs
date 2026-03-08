use std::f32::consts::PI;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
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
mod sky;
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
use camera::{Player, WowCamera, WowCameraPlugin};
use terrain::{AdtManager, AdtStreamingPlugin};
use terrain_heightmap::TerrainHeightmap;

const DEFAULT_M2: &str = "data/models/humanmale_hd.m2";
const DEFAULT_ADT: &str = "data/terrain/azeroth_32_48.adt";

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
    let screenshot = parse_screenshot_args(&args);
    let dump_tree = args.iter().any(|a| a == "--dump-tree");
    let dump_ui_tree = args.iter().any(|a| a == "--dump-ui-tree");
    let enable_sound = args.iter().any(|a| a == "--sound");
    let server_addr = parse_server_arg(&args);
    let initial_state = parse_state_arg(&args);

    if dump_ui_tree && !dump_tree && screenshot.is_none() {
        run_headless_ui_dump_app();
        return;
    }

    let mut app = App::new();
    register_plugins(&mut app);
    configure_app_plugins(
        &mut app,
        enable_sound,
        server_addr,
        initial_state,
        dump_tree,
        dump_ui_tree,
        screenshot,
    );
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

fn register_plugins(app: &mut App) {
    app.add_plugins(default_plugins())
        .add_plugins(game_engine::auction_house::AuctionHousePlugin)
        .add_plugins(game_engine::mail::MailPlugin)
        .add_plugins(game_engine::ui::plugin::UiPlugin)
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
            config: FpsOverlayConfig {
                refresh_interval: Duration::from_millis(500),
                ..default()
            },
        })
        .add_systems(Startup, (setup, wow_cursor::install_wow_cursor))
        .add_systems(
            Update,
            wow_cursor::update_wow_cursor_style.run_if(in_state(game_state::GameState::InWorld)),
        );
    init_status_resources(app);
}

fn configure_app_plugins(
    app: &mut App,
    enable_sound: bool,
    server_addr: Option<std::net::SocketAddr>,
    initial_state: Option<game_state::GameState>,
    dump_tree: bool,
    dump_ui_tree: bool,
    screenshot: Option<ScreenshotRequest>,
) {
    if enable_sound {
        app.add_plugins(sound::SoundPlugin);
    }
    if let Some(addr) = server_addr {
        app.insert_resource(networking::ServerAddr(addr));
    }
    if let Some(state) = initial_state {
        app.insert_resource(game_state::InitialGameState(state));
    }
    app.add_plugins(game_state::GameStatePlugin);
    app.add_plugins(networking::NetworkPlugin);
    app.add_plugins(login_screen::LoginScreenPlugin);
    app.add_plugins(char_select::CharSelectPlugin);
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
        )
            .run_if(in_state(game_state::GameState::InWorld)),
    );
    if dump_tree {
        app.insert_resource(DumpTreeFlag);
        app.add_systems(PostStartup, dump_tree_and_exit);
    }
    if dump_ui_tree {
        app.insert_resource(DumpUiTreeFlag);
        app.add_systems(PostStartup, dump_ui_tree_and_exit);
    }
    if let Some(req) = screenshot {
        app.insert_resource(req);
        app.add_systems(Update, take_screenshot);
    }
}

fn run_headless_ui_dump_app() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(game_engine::ui::plugin::UiPlugin)
        .insert_resource(DumpUiTreeFlag)
        .add_systems(PostStartup, dump_ui_tree_and_exit);
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

/// Parse `screenshot <output> [model]` from args. Supports flags before `screenshot`.
fn parse_screenshot_args(args: &[String]) -> Option<ScreenshotRequest> {
    let screenshot_idx = screenshot_arg_index(args)?;
    let output = args
        .get(screenshot_idx + 1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("screenshot.webp"));
    let has_server = args.windows(2).any(|w| w[0] == "--server");
    let frames_remaining = if has_server { 60 } else { 3 };
    Some(ScreenshotRequest {
        output,
        frames_remaining,
    })
}

/// Parse `--server <addr>` from args. Returns None if not present.
fn parse_server_arg(args: &[String]) -> Option<std::net::SocketAddr> {
    args.windows(2)
        .find(|w| w[0] == "--server")
        .and_then(|w| w[1].parse().ok())
}

fn parse_state_arg(args: &[String]) -> Option<game_state::GameState> {
    args.windows(2)
        .find(|w| w[0] == "--state")
        .and_then(|w| match w[1].as_str() {
            "login" => Some(game_state::GameState::Login),
            "connecting" => Some(game_state::GameState::Connecting),
            "charselect" => Some(game_state::GameState::CharSelect),
            "loading" => Some(game_state::GameState::Loading),
            "inworld" => Some(game_state::GameState::InWorld),
            _ => None,
        })
}

fn parse_asset_path_from_args(args: &[String]) -> Option<PathBuf> {
    let screenshot_idx = screenshot_arg_index(args);
    let mut i = 0;
    while i < args.len() {
        if screenshot_idx == Some(i) {
            i += 1;
            if i < args.len() {
                i += 1;
            }
            continue;
        }
        match args[i].as_str() {
            "--server" | "--state" => {
                i += 2;
            }
            arg if arg.starts_with("--") => {
                i += 1;
            }
            path => return Some(PathBuf::from(path)),
        }
    }
    None
}

/// Find the asset path from CLI args. Returns None when no explicit path given.
fn parse_asset_path() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    parse_asset_path_from_args(&args)
}

fn take_screenshot(mut commands: Commands, req: Option<ResMut<ScreenshotRequest>>) {
    let Some(mut req) = req else { return };
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
    fn asset_path_skips_state_and_screenshot_output() {
        let parsed = parse_asset_path_from_args(&args(&[
            "--state",
            "login",
            "screenshot",
            "/tmp/codex/test.webp",
            "--server",
            "127.0.0.1:25565",
        ]));
        assert_eq!(parsed, None);
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
}

/// Spawn minimal server-mode scene: dark background + camera only.
fn setup_server_camera(commands: &mut Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.12)));
    commands.spawn((
        Camera3d::default(),
        Transform::default(),
        WowCamera::default(),
        AmbientLight {
            color: Color::WHITE,
            brightness: 0.0,
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_scene_for_asset(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_mats: &mut Assets<terrain_material::TerrainMaterial>,
    water_mats: &mut Assets<water_material::WaterMaterial>,
    sky_mats: &mut Assets<sky::SkyMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_manager: &mut AdtManager,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    let asset_path = parse_asset_path();
    let is_terrain = asset_path
        .as_ref()
        .is_some_and(|p| p.extension().is_some_and(|e| e == "adt"))
        || asset_path.is_none();
    let camera = spawn_scene_environment(commands, meshes, materials, sky_mats, images, is_terrain);
    match asset_path {
        Some(p) if p.extension().is_some_and(|e| e == "adt") => {
            let center = spawn_terrain(
                commands,
                meshes,
                materials,
                terrain_mats,
                water_mats,
                images,
                inverse_bp,
                heightmap,
                adt_manager,
                camera,
                &p,
            );
            let m2_path = Path::new(DEFAULT_M2);
            if m2_path.exists() {
                m2_scene::spawn_m2_model(
                    commands,
                    meshes,
                    materials,
                    images,
                    inverse_bp,
                    m2_path,
                    creature_display_map,
                );
            }
            if let Some(pos) = center {
                set_player_position(commands, pos);
            }
        }
        Some(p) => spawn_m2_scene(
            commands,
            meshes,
            materials,
            images,
            inverse_bp,
            &p,
            creature_display_map,
        ),
        None => spawn_default_scene(
            commands,
            meshes,
            materials,
            terrain_mats,
            water_mats,
            images,
            inverse_bp,
            heightmap,
            adt_manager,
            creature_display_map,
        ),
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain_mats: ResMut<Assets<terrain_material::TerrainMaterial>>,
    mut water_mats: ResMut<Assets<water_material::WaterMaterial>>,
    mut sky_mats: ResMut<Assets<sky::SkyMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut heightmap: ResMut<TerrainHeightmap>,
    mut adt_manager: ResMut<AdtManager>,
    server_addr: Option<Res<networking::ServerAddr>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
) {
    if server_addr.is_some() {
        setup_server_camera(&mut commands);
        return;
    }
    spawn_scene_for_asset(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut terrain_mats,
        &mut water_mats,
        &mut sky_mats,
        &mut images,
        &mut inverse_bp,
        &mut heightmap,
        &mut adt_manager,
        &creature_display_map,
    );
}

fn spawn_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_mats: &mut Assets<terrain_material::TerrainMaterial>,
    water_mats: &mut Assets<water_material::WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_manager: &mut AdtManager,
    camera: Entity,
    adt_path: &Path,
) -> Option<Vec3> {
    match terrain::spawn_adt(
        commands,
        meshes,
        materials,
        terrain_mats,
        water_mats,
        images,
        inverse_bp,
        heightmap,
        adt_path,
    ) {
        Ok(result) => {
            commands.entity(camera).insert(result.camera);
            adt_manager.map_name = result.map_name;
            adt_manager.initial_tile = (result.tile_y, result.tile_x);
            adt_manager
                .loaded
                .insert((result.tile_y, result.tile_x), result.root_entity);
            Some(result.center)
        }
        Err(e) => {
            eprintln!("ADT load error: {e}");
            None
        }
    }
}

/// Load default terrain from DEFAULT_ADT. Returns center position if successful.
fn load_default_terrain(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_mats: &mut Assets<terrain_material::TerrainMaterial>,
    water_mats: &mut Assets<water_material::WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_manager: &mut AdtManager,
) -> Option<Vec3> {
    let adt_path = Path::new(DEFAULT_ADT);
    if !adt_path.exists() {
        return None;
    }
    match terrain::spawn_adt(
        commands,
        meshes,
        materials,
        terrain_mats,
        water_mats,
        images,
        inverse_bp,
        heightmap,
        adt_path,
    ) {
        Ok(result) => {
            adt_manager.map_name = result.map_name;
            adt_manager.initial_tile = (result.tile_y, result.tile_x);
            adt_manager
                .loaded
                .insert((result.tile_y, result.tile_x), result.root_entity);
            Some(result.center)
        }
        Err(e) => {
            eprintln!("ADT load error: {e}");
            None
        }
    }
}

/// Default scene: terrain + HD human + chest, all together.
#[allow(clippy::too_many_arguments)]
fn spawn_default_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_mats: &mut Assets<terrain_material::TerrainMaterial>,
    water_mats: &mut Assets<water_material::WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_manager: &mut AdtManager,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    let center = load_default_terrain(
        commands,
        meshes,
        materials,
        terrain_mats,
        water_mats,
        images,
        inverse_bp,
        heightmap,
        adt_manager,
    );
    let m2_path = Path::new(DEFAULT_M2);
    if m2_path.exists() {
        m2_scene::spawn_m2_model(
            commands,
            meshes,
            materials,
            images,
            inverse_bp,
            m2_path,
            creature_display_map,
        );
    }
    let chest_offset = Vec3::new(5.0, 0.0, 0.0);
    let chest_path = Path::new("data/models/chest01.m2");
    if chest_path.exists() {
        m2_scene::spawn_static_m2(
            commands,
            meshes,
            materials,
            images,
            inverse_bp,
            chest_path,
            Transform::from_translation(center.unwrap_or_default() + chest_offset)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            creature_display_map,
        );
    }
    if let Some(pos) = center {
        set_player_position(commands, pos);
    }
}

/// Find the Player entity and move it to the given position.
fn set_player_position(commands: &mut Commands, pos: Vec3) {
    commands.queue(move |world: &mut World| {
        let mut q = world.query_filtered::<&mut Transform, With<Player>>();
        for mut xf in q.iter_mut(world) {
            xf.translation = pos;
        }
    });
}

fn spawn_m2_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    m2_scene::spawn_m2_model(
        commands,
        meshes,
        materials,
        images,
        inverse_bindposes,
        m2_path,
        creature_display_map,
    );
    ground::spawn_ground_clutter(
        commands,
        meshes,
        materials,
        images,
        inverse_bindposes,
        creature_display_map,
    );
    let chest_path = Path::new("data/models/chest01.m2");
    if chest_path.exists() {
        m2_scene::spawn_static_m2(
            commands,
            meshes,
            materials,
            images,
            inverse_bindposes,
            chest_path,
            Transform::from_xyz(5.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            creature_display_map,
        );
    }
}

fn spawn_scene_environment(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sky_materials: &mut Assets<sky::SkyMaterial>,
    images: &mut Assets<Image>,
    is_terrain: bool,
) -> Entity {
    let camera = commands
        .spawn((
            Camera3d::default(),
            Transform::default(),
            WowCamera::default(),
            AmbientLight {
                color: Color::WHITE,
                brightness: 150.0,
                ..default()
            },
        ))
        .id();
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-PI / 4.0)),
    ));
    sky::spawn_sky_dome(commands, meshes, sky_materials, images, camera);
    if !is_terrain {
        ground::spawn_ground_plane(commands, meshes, materials, images);
    }
    camera
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

#[allow(clippy::type_complexity)]
fn dump_tree_and_exit(
    tree_query: Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: Query<&ChildOf>,
    mut exit: MessageWriter<AppExit>,
) {
    let tree = game_engine::dump::build_tree(&tree_query, &parent_query, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}

fn dump_ui_tree_and_exit(
    mut ui_state: ResMut<game_engine::ui::plugin::UiState>,
    mut dioxus_runtime: NonSendMut<game_engine::ui::dioxus_runtime::DioxusUiRuntime>,
    mut exit: MessageWriter<AppExit>,
) {
    dioxus_runtime.sync(&mut ui_state.registry);
    action_bar::ensure_action_bars(&mut ui_state.registry);
    let tree = game_engine::dump::build_ui_tree(&ui_state.registry, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}
