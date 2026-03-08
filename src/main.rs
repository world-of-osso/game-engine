use std::f32::consts::PI;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::pbr::MaterialPlugin;
use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::{CursorIcon, CursorOptions, CustomCursor, CustomCursorImage, PrimaryWindow};
use game_engine::ipc::IpcPlugin;
use game_engine::ipc::plugin::{EquipmentControlCommand, EquipmentControlQueue};
use game_engine::status::{
    CharacterStatsSnapshot, CollectionStatusSnapshot, CombatLogStatusSnapshot,
    CurrenciesStatusSnapshot, EquippedGearEntry, EquippedGearStatusSnapshot, GroupStatusSnapshot,
    GuildVaultStatusSnapshot, MapStatusSnapshot, NetworkStatusSnapshot, ProfessionStatusSnapshot,
    QuestLogStatusSnapshot, ReputationsStatusSnapshot, SoundStatusSnapshot, TerrainStatusSnapshot,
    WarbankStatusSnapshot,
};
use lightyear::prelude::client::Connected;
use shared::components::{
    Health as NetHealth, Mana as NetMana, MovementSpeed as NetMovementSpeed, Player as NetPlayer,
};

mod action_bar;
mod animation;
mod asset;
mod camera;
mod char_select;
mod creature_display;
mod equipment;
mod game_state;
mod health_bar;
mod login_screen;
pub mod m2_spawn;
mod minimap;
mod nameplate;
mod networking;
mod networking_auth;
mod particle;
mod sky;
mod sky_material;
mod sound;
mod target;
mod terrain;
mod terrain_heightmap;
mod terrain_material;
mod terrain_objects;
mod water_material;

use crate::networking::RemoteEntity;
use animation::{AnimationPlugin, BonePivot, M2AnimData, M2AnimPlayer};
use camera::{CharacterFacing, MovementState, Player, WowCamera, WowCameraPlugin};
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

#[derive(Resource)]
struct WowCursorAssets {
    default_point: Handle<Image>,
    hover_point: Handle<Image>,
}

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum ActiveWowCursor {
    Default,
    Hover,
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

    if dump_ui_tree && !dump_tree && screenshot.is_none() {
        run_headless_ui_dump_app();
        return;
    }

    let mut app = App::new();
    register_plugins(&mut app);
    configure_app_plugins(&mut app, enable_sound, server_addr, dump_tree, dump_ui_tree, screenshot);
    app.insert_resource(creature_display::CreatureDisplayMap::load_from_data_dir());
    app.run();
}

/// Configure optional plugins/systems based on CLI flags.
fn configure_app_plugins(
    app: &mut App,
    enable_sound: bool,
    server_addr: Option<std::net::SocketAddr>,
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
    app.add_plugins(game_state::GameStatePlugin);
    app.add_plugins(networking::NetworkPlugin);
    app.add_plugins(login_screen::LoginScreenPlugin);
    app.add_plugins(char_select::CharSelectPlugin);
    app.add_systems(
        Update,
        (
            sync_network_status_snapshot,
            sync_terrain_status_snapshot,
            sync_sound_status_snapshot,
            sync_character_stats_snapshot,
            apply_equipment_ipc_commands,
            sync_equipped_gear_status_snapshot,
            sync_map_status_snapshot,
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

/// Register all core engine plugins and the startup system.
fn register_plugins(app: &mut App) {
    app.add_plugins(DefaultPlugins)
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
        .add_systems(Startup, (setup, install_wow_cursor))
        .add_systems(
            Update,
            update_wow_cursor_style.run_if(in_state(game_state::GameState::InWorld)),
        );
    init_status_resources(app);
}

fn init_status_resources(app: &mut App) {
    app.init_resource::<NetworkStatusSnapshot>()
        .init_resource::<TerrainStatusSnapshot>()
        .init_resource::<SoundStatusSnapshot>()
        .init_resource::<CurrenciesStatusSnapshot>()
        .init_resource::<ReputationsStatusSnapshot>()
        .init_resource::<CharacterStatsSnapshot>()
        .init_resource::<GuildVaultStatusSnapshot>()
        .init_resource::<WarbankStatusSnapshot>()
        .init_resource::<EquippedGearStatusSnapshot>()
        .init_resource::<QuestLogStatusSnapshot>()
        .init_resource::<GroupStatusSnapshot>()
        .init_resource::<CombatLogStatusSnapshot>()
        .init_resource::<CollectionStatusSnapshot>()
        .init_resource::<ProfessionStatusSnapshot>()
        .init_resource::<MapStatusSnapshot>();
}

/// Load WoW cursor BLP images from disk. Returns None if either fails.
fn load_cursor_images(images: &mut Assets<Image>) -> Option<(Handle<Image>, Handle<Image>)> {
    let default_path = Path::new("/syncthing/Sync/Projects/wow/Interface/CURSOR/Point.blp");
    let hover_path =
        Path::new("/syncthing/Sync/Projects/wow/Interface/CURSOR/Crosshair/Point.blp");
    let default_image = match asset::blp::load_blp_gpu_image(default_path) {
        Ok(image) => image,
        Err(error) => {
            warn!("failed to load WoW cursor {}: {error}", default_path.display());
            return None;
        }
    };
    let hover_image = match asset::blp::load_blp_gpu_image(hover_path) {
        Ok(image) => image,
        Err(error) => {
            warn!("failed to load WoW cursor {}: {error}", hover_path.display());
            return None;
        }
    };
    Some((images.add(default_image), images.add(hover_image)))
}

fn install_wow_cursor(
    mut commands: Commands,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(window_entity) = primary_window.single() else {
        return;
    };
    let Some((default_cursor, hover_cursor)) = load_cursor_images(&mut images) else {
        return;
    };
    commands.insert_resource(WowCursorAssets {
        default_point: default_cursor.clone(),
        hover_point: hover_cursor,
    });
    commands.insert_resource(ActiveWowCursor::Default);
    commands
        .entity(window_entity)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle: default_cursor,
            hotspot: (0, 0),
            ..default()
        })));
}

/// Raycast to determine whether the cursor hovers over a remote entity.
fn pick_desired_cursor(
    window: &Window,
    camera: (&Camera, &GlobalTransform),
    remote_q: &Query<Entity, (With<RemoteEntity>, Without<Player>)>,
    ray_cast: &mut MeshRayCast,
) -> Option<ActiveWowCursor> {
    let cursor = window.cursor_position()?;
    let (cam, cam_tf) = camera;
    let ray = cam.viewport_to_world(cam_tf, cursor).ok()?;
    let hover = ray_cast
        .cast_ray(ray, &default())
        .iter()
        .any(|(entity, _)| remote_q.get(*entity).is_ok());
    Some(if hover {
        ActiveWowCursor::Hover
    } else {
        ActiveWowCursor::Default
    })
}

fn update_wow_cursor_style(
    windows: Query<(&Window, &CursorOptions, Entity), With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<WowCamera>>,
    remote_q: Query<Entity, (With<RemoteEntity>, Without<Player>)>,
    assets: Option<Res<WowCursorAssets>>,
    active: Option<ResMut<ActiveWowCursor>>,
    mut ray_cast: MeshRayCast,
    mut commands: Commands,
) {
    let (window, cursor_opts, window_entity) = match windows.single() {
        Ok(value) => value,
        Err(_) => return,
    };
    if !cursor_opts.visible {
        return;
    }
    let Ok(camera) = cameras.single() else { return };
    let Some(assets) = assets else { return };
    let Some(mut active) = active else { return };

    let desired = pick_desired_cursor(window, camera, &remote_q, &mut ray_cast)
        .unwrap_or(ActiveWowCursor::Default);
    if *active == desired {
        return;
    }
    *active = desired;
    let handle = match desired {
        ActiveWowCursor::Default => assets.default_point.clone(),
        ActiveWowCursor::Hover => assets.hover_point.clone(),
    };
    commands
        .entity(window_entity)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle,
            hotspot: (0, 0),
            ..default()
        })));
}

fn sync_network_status_snapshot(
    mut snapshot: ResMut<NetworkStatusSnapshot>,
    server_addr: Option<Res<networking::ServerAddr>>,
    game_state: Option<Res<State<game_state::GameState>>>,
    local_client_id: Option<Res<networking::LocalClientId>>,
    current_zone: Res<networking::CurrentZone>,
    chat_log: Res<networking::ChatLog>,
    connected_query: Query<(), With<Connected>>,
    remote_query: Query<(), With<networking::RemoteEntity>>,
    local_player_query: Query<(), With<networking::LocalPlayer>>,
) {
    snapshot.server_addr = server_addr.map(|addr| addr.0.to_string());
    snapshot.game_state = game_state
        .map(|state| format!("{:?}", state.get()))
        .unwrap_or_else(|| "Unavailable".into());
    snapshot.connected = !connected_query.is_empty();
    snapshot.connected_links = connected_query.iter().count();
    snapshot.local_client_id = local_client_id.map(|id| id.0);
    snapshot.zone_id = current_zone.zone_id;
    snapshot.remote_entities = remote_query.iter().count();
    snapshot.local_players = local_player_query.iter().count();
    snapshot.chat_messages = chat_log.messages.len();
}

fn sync_terrain_status_snapshot(
    mut snapshot: ResMut<TerrainStatusSnapshot>,
    adt_manager: Res<AdtManager>,
    heightmap: Res<TerrainHeightmap>,
) {
    snapshot.map_name = adt_manager.map_name.clone();
    snapshot.initial_tile = adt_manager.initial_tile;
    snapshot.load_radius = adt_manager.load_radius;
    snapshot.loaded_tiles = adt_manager.loaded.len();
    snapshot.pending_tiles = adt_manager.pending.len();
    snapshot.failed_tiles = adt_manager.failed.len();
    snapshot.server_requested_tiles = adt_manager.server_requested.len();
    snapshot.heightmap_tiles = heightmap.tile_keys().count();
}

fn sync_sound_status_snapshot(
    mut snapshot: ResMut<SoundStatusSnapshot>,
    sound_settings: Option<Res<sound::SoundSettings>>,
    ambient_query: Query<(), With<sound::AmbientSound>>,
    sinks: Query<&AudioSink>,
) {
    if let Some(settings) = sound_settings {
        snapshot.enabled = true;
        snapshot.muted = settings.muted;
        snapshot.master_volume = settings.master_volume;
        snapshot.footstep_volume = settings.footstep_volume;
        snapshot.ambient_volume = settings.ambient_volume;
    } else {
        *snapshot = SoundStatusSnapshot::default();
    }
    snapshot.ambient_entities = ambient_query.iter().count();
    snapshot.active_sinks = sinks.iter().count();
}

/// Fill health/mana/speed from the local player entity into the snapshot.
fn fill_local_player_stats(
    snapshot: &mut CharacterStatsSnapshot,
    local_player_query: &Query<
        (
            Option<&NetPlayer>,
            Option<&NetHealth>,
            Option<&NetMana>,
            Option<&NetMovementSpeed>,
        ),
        With<networking::LocalPlayer>,
    >,
) {
    if let Some((_, health, mana, speed)) = local_player_query.iter().next() {
        snapshot.health_current = health.map(|v| v.current);
        snapshot.health_max = health.map(|v| v.max);
        snapshot.mana_current = mana.map(|v| v.current);
        snapshot.mana_max = mana.map(|v| v.max);
        snapshot.movement_speed = speed.map(|v| v.0);
    } else {
        snapshot.health_current = None;
        snapshot.health_max = None;
        snapshot.mana_current = None;
        snapshot.mana_max = None;
        snapshot.movement_speed = None;
    }
}

fn sync_character_stats_snapshot(
    mut snapshot: ResMut<CharacterStatsSnapshot>,
    character_list: Res<networking::CharacterList>,
    selected_character_id: Res<networking::SelectedCharacterId>,
    current_zone: Res<networking::CurrentZone>,
    local_player_query: Query<
        (
            Option<&NetPlayer>,
            Option<&NetHealth>,
            Option<&NetMana>,
            Option<&NetMovementSpeed>,
        ),
        With<networking::LocalPlayer>,
    >,
) {
    let selected_character = selected_character_id.0.and_then(|character_id| {
        character_list
            .0
            .iter()
            .find(|entry| entry.character_id == character_id)
    });
    snapshot.name = selected_character
        .map(|entry| entry.name.clone())
        .or_else(|| {
            local_player_query
                .iter()
                .find_map(|(player, _, _, _)| player.map(|player| player.name.clone()))
        });
    snapshot.level = selected_character.map(|entry| entry.level);
    snapshot.race = selected_character.map(|entry| entry.race);
    snapshot.class = selected_character.map(|entry| entry.class);
    snapshot.zone_id = current_zone.zone_id;
    fill_local_player_stats(&mut snapshot, &local_player_query);
}

fn sync_equipped_gear_status_snapshot(
    mut snapshot: ResMut<EquippedGearStatusSnapshot>,
    local_player_query: Query<&equipment::Equipment, With<Player>>,
) {
    snapshot.entries.clear();
    if let Some(equipment) = local_player_query.iter().next() {
        let mut entries = Vec::with_capacity(equipment.slots.len());
        for (slot, path) in &equipment.slots {
            entries.push(EquippedGearEntry {
                slot: format!("{slot:?}"),
                path: path.display().to_string(),
            });
        }
        entries.sort_by(|a, b| a.slot.cmp(&b.slot));
        snapshot.entries = entries;
    }
}

fn apply_equipment_ipc_commands(
    mut queue: ResMut<EquipmentControlQueue>,
    mut commands: Commands,
    mut local_player_query: Query<(Entity, Option<&mut equipment::Equipment>), With<Player>>,
) {
    if queue.pending.is_empty() {
        return;
    }
    let Some((entity, maybe_equipment)) = local_player_query.iter_mut().next() else {
        queue.pending.clear();
        return;
    };
    let mut pending = std::mem::take(&mut queue.pending);
    if let Some(mut equipment) = maybe_equipment {
        for command in pending.drain(..) {
            apply_equipment_command(&mut equipment, command);
        }
        return;
    }
    let mut equipment = equipment::Equipment::default();
    for command in pending.drain(..) {
        apply_equipment_command(&mut equipment, command);
    }
    commands.entity(entity).insert(equipment);
}

fn apply_equipment_command(equipment: &mut equipment::Equipment, command: EquipmentControlCommand) {
    match command {
        EquipmentControlCommand::Set { slot, model_path } => {
            let Some(slot) = parse_equipment_slot(&slot) else {
                warn!("Ignoring equipment set with invalid slot '{slot}'");
                return;
            };
            let path = PathBuf::from(model_path);
            if !path.exists() {
                warn!(
                    "Ignoring equipment set for missing model path {}",
                    path.display()
                );
                return;
            }
            equipment.slots.insert(slot, path);
        }
        EquipmentControlCommand::Clear { slot } => {
            let Some(slot) = parse_equipment_slot(&slot) else {
                warn!("Ignoring equipment clear with invalid slot '{slot}'");
                return;
            };
            equipment.slots.remove(&slot);
        }
    }
}

fn parse_equipment_slot(value: &str) -> Option<equipment::EquipmentSlot> {
    match value.to_ascii_lowercase().as_str() {
        "mainhand" | "main-hand" | "main" | "mh" => Some(equipment::EquipmentSlot::MainHand),
        "offhand" | "off-hand" | "off" | "oh" => Some(equipment::EquipmentSlot::OffHand),
        _ => None,
    }
}

fn sync_map_status_snapshot(
    mut snapshot: ResMut<MapStatusSnapshot>,
    current_zone: Res<networking::CurrentZone>,
    player_query: Query<&Transform, With<Player>>,
) {
    snapshot.zone_id = current_zone.zone_id;
    if let Some(transform) = player_query.iter().next() {
        snapshot.player_x = transform.translation.x;
        snapshot.player_z = transform.translation.z;
    }
}

/// Parse `screenshot <output> [model]` from args. Returns None if not a screenshot command.
fn parse_screenshot_args(args: &[String]) -> Option<ScreenshotRequest> {
    if args.first().map(|s| s.as_str()) != Some("screenshot") {
        return None;
    }
    let output = args
        .get(1)
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

/// Find the asset path from CLI args. Returns None when no explicit path given.
/// Normal: `game-engine [asset]`
/// Screenshot: `game-engine screenshot [output.webp] [asset]`
fn parse_asset_path() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("screenshot") {
        // screenshot <output> [asset] -- skip flags like --server
        let mut skip_next = false;
        args.iter()
            .skip(2)
            .find(|a| {
                if skip_next {
                    skip_next = false;
                    return false;
                }
                if *a == "--server" {
                    skip_next = true;
                    return false;
                }
                !a.starts_with("--")
            })
            .map(PathBuf::from)
    } else {
        // Skip --server and its value
        let mut skip_next = false;
        args.iter()
            .find(|a| {
                if skip_next {
                    skip_next = false;
                    return false;
                }
                if *a == "--server" {
                    skip_next = true;
                    return false;
                }
                !a.starts_with("--")
            })
            .map(PathBuf::from)
    }
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
    let Some(data) = img.data.as_ref() else {
        eprintln!("Screenshot has no pixel data");
        return;
    };
    let size = img.size();
    let encoder = webp::Encoder::from_rgba(data, size.x, size.y);
    let webp_data = encoder.encode(15.0);
    std::fs::write(output, &*webp_data)
        .unwrap_or_else(|e| eprintln!("Failed to write {}: {e}", output.display()));
    println!("Saved {} ({} bytes)", output.display(), webp_data.len());
}

/// Spawn minimal server-mode scene: dark background + camera only, no world content.
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
                commands, meshes, materials, terrain_mats, water_mats, images, inverse_bp,
                heightmap, adt_manager, camera, &p,
            );
            let m2_path = Path::new(DEFAULT_M2);
            if m2_path.exists() {
                spawn_m2_model(commands, meshes, materials, images, inverse_bp, m2_path, creature_display_map);
            }
            if let Some(pos) = center {
                set_player_position(commands, pos);
            }
        }
        Some(p) => spawn_m2_scene(commands, meshes, materials, images, inverse_bp, &p, creature_display_map),
        None => spawn_default_scene(
            commands, meshes, materials, terrain_mats, water_mats, images, inverse_bp,
            heightmap, adt_manager, creature_display_map,
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
        &mut commands, &mut meshes, &mut materials, &mut terrain_mats, &mut water_mats,
        &mut sky_mats, &mut images, &mut inverse_bp, &mut heightmap, &mut adt_manager,
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
        commands, meshes, materials, terrain_mats, water_mats, images, inverse_bp,
        heightmap, adt_path,
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
        commands, meshes, materials, terrain_mats, water_mats, images, inverse_bp,
        heightmap, adt_path,
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
        commands, meshes, materials, terrain_mats, water_mats, images, inverse_bp,
        heightmap, adt_manager,
    );
    let m2_path = Path::new(DEFAULT_M2);
    if m2_path.exists() {
        spawn_m2_model(commands, meshes, materials, images, inverse_bp, m2_path, creature_display_map);
    }
    let chest_offset = Vec3::new(5.0, 0.0, 0.0);
    let chest_path = Path::new("data/models/chest01.m2");
    if chest_path.exists() {
        spawn_static_m2(
            commands, meshes, materials, images, inverse_bp, chest_path,
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
    spawn_m2_model(commands, meshes, materials, images, inverse_bindposes, m2_path, creature_display_map);
    spawn_ground_clutter(commands, meshes, materials, images, inverse_bindposes, creature_display_map);
    let chest_path = Path::new("data/models/chest01.m2");
    if chest_path.exists() {
        spawn_static_m2(
            commands, meshes, materials, images, inverse_bindposes, chest_path,
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
        spawn_ground_plane(commands, meshes, materials, images);
    }
    camera
}

/// Load the grass BLP texture with repeat tiling and spawn the ground plane.
fn spawn_ground_plane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let grass_path = asset::casc_resolver::ensure_texture(187126)
        .unwrap_or_else(|| PathBuf::from("data/textures/187126.blp"));
    let mut grass_image = load_blp_as_image(&grass_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        generate_grass_texture()
    });
    grass_image.sampler =
        bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::Repeat,
            address_mode_v: bevy::image::ImageAddressMode::Repeat,
            ..bevy::image::ImageSamplerDescriptor::linear()
        });
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(grass_image)),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mut mesh = Plane3d::default().mesh().size(100.0, 100.0).build();
    scale_mesh_uvs(&mut mesh, 20.0);
    commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material)));
}

/// Multiply all UV coordinates in a mesh by the given factor for texture tiling.
fn scale_mesh_uvs(mesh: &mut Mesh, factor: f32) {
    use bevy::mesh::VertexAttributeValues;
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs.iter_mut() {
            uv[0] *= factor;
            uv[1] *= factor;
        }
    }
}

/// Generate a 64x64 procedural grass texture with color variation.
fn generate_grass_texture() -> Image {
    const SIZE: u32 = 64;
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    // Simple hash for deterministic pseudo-random variation
    for y in 0..SIZE {
        for x in 0..SIZE {
            let hash = ((x.wrapping_mul(7919) ^ y.wrapping_mul(6271)).wrapping_mul(2903)) % 256;
            let noise = hash as f32 / 255.0;
            let r = (0.25 + noise * 0.1) * 255.0;
            let g = (0.45 + noise * 0.15) * 255.0;
            let b = (0.15 + noise * 0.08) * 255.0;
            pixels.extend_from_slice(&[r as u8, g as u8, b as u8, 255]);
        }
    }
    Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

const HERB_MODELS: &[&str] = &[
    "data/models/bush_peacebloom01.m2",
    "data/models/bush_silverleaf01.m2",
];

/// Compute a deterministic scatter position from index. Returns None if too close to origin.
fn scatter_position(i: u32) -> Option<(f32, f32, u32, u32)> {
    let hash1 = (i.wrapping_mul(7919).wrapping_add(1301)) % 10000;
    let hash2 = (i.wrapping_mul(6271).wrapping_add(3571)) % 10000;
    let x = (hash1 as f32 / 10000.0 - 0.5) * 60.0;
    let z = (hash2 as f32 / 10000.0 - 0.5) * 60.0;
    if x * x + z * z < 9.0 {
        return None;
    }
    Some((x, z, hash1, hash2))
}

/// Scatter rocks and herb models across the ground.
fn spawn_ground_clutter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    spawn_rock_clutter(commands, meshes, materials);
    spawn_herb_clutter(commands, meshes, materials, images, inverse_bindposes, creature_display_map);
}

fn spawn_rock_clutter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let rock_mesh = meshes.add(Sphere::new(0.15).mesh().ico(2).unwrap());
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.42, 0.38),
        perceptual_roughness: 0.95,
        ..default()
    });
    let dark_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.33, 0.30),
        perceptual_roughness: 0.95,
        ..default()
    });
    for i in 0u32..30 {
        let Some((x, z, hash1, hash2)) = scatter_position(i) else {
            continue;
        };
        if i % 3 == 0 {
            continue;
        } // leave gaps for herbs
        let (mat, scale) = if i % 2 == 0 {
            (&dark_mat, 0.6 + (hash2 % 80) as f32 / 100.0)
        } else {
            (&rock_mat, 0.5 + (hash1 % 100) as f32 / 100.0)
        };
        commands.spawn((
            Mesh3d(rock_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(x, 0.0, z).with_scale(Vec3::new(1.0, scale, 1.0)),
        ));
    }
}

fn spawn_herb_clutter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    for i in 0u32..15 {
        let Some((x, z, hash1, _)) = scatter_position(i.wrapping_mul(3).wrapping_add(7)) else {
            continue;
        };
        let herb_path = Path::new(HERB_MODELS[(hash1 as usize) % HERB_MODELS.len()]);
        let yaw = (hash1 % 628) as f32 / 100.0;
        let transform = Transform::from_xyz(x, 0.0, z)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(0.3));
        spawn_static_m2(
            commands, meshes, materials, images, inverse_bindposes, herb_path, transform,
            creature_display_map,
        );
    }
}

/// Attach equipment (attachment points + default main-hand torch) to a model entity.
fn attach_equipment_to_model(
    commands: &mut Commands,
    model_entity: Entity,
    attachments: &[asset::m2_attach::M2Attachment],
) {
    if attachments.is_empty() {
        return;
    }
    let attach_pts = equipment::build_attachment_points(attachments);
    let mut equip = equipment::Equipment::default();
    let torch = Path::new("data/models/club_1h_torch_a_01.m2");
    if torch.exists() {
        equip
            .slots
            .insert(equipment::EquipmentSlot::MainHand, torch.to_path_buf());
    }
    commands.entity(model_entity).insert((attach_pts, equip));
}

fn spawn_m2_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    let skin_fdids = creature_display_map
        .resolve_skin_fdids_for_model_path(m2_path)
        .unwrap_or([0, 0, 0]);
    let model = match asset::m2::load_m2(m2_path, &skin_fdids) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load M2 {}: {e}", m2_path.display());
            return;
        }
    };
    let asset::m2::M2Model {
        batches, bones, sequences, bone_tracks, global_sequences, particle_emitters, attachments, ..
    } = model;
    let model_entity = spawn_player_root(commands, m2_path);
    let skinning = m2_spawn::attach_m2_batches(
        commands, meshes, materials, images, inv_bp, batches, &bones, model_entity,
    );
    let joint_entities =
        attach_bone_pivots_and_player(commands, &bones, &sequences, &skinning, model_entity);
    if !particle_emitters.is_empty() {
        let bone_slice = skinning.as_ref().map(|(_, joints)| joints.as_slice());
        particle::spawn_emitters(commands, meshes, materials, images, &particle_emitters, bone_slice, model_entity);
    }
    if let Some(joints) = joint_entities {
        commands.insert_resource(M2AnimData {
            sequences,
            bone_tracks,
            global_sequences,
            joint_entities: joints,
        });
    }
    attach_equipment_to_model(commands, model_entity, &attachments);
}

fn spawn_player_root(commands: &mut Commands, m2_path: &Path) -> Entity {
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("m2_model");
    commands
        .spawn((
            Name::new(name.to_owned()),
            Player,
            MovementState::default(),
            CharacterFacing::default(),
            Transform::from_xyz(0.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
        ))
        .id()
}

/// Spawn a static (non-player) M2 model as a scene prop.
fn spawn_static_m2(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    transform: Transform,
    creature_display_map: &creature_display::CreatureDisplayMap,
) -> Option<Entity> {
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prop");
    let root = commands
        .spawn((Name::new(name.to_owned()), transform, Visibility::default()))
        .id();
    let skin_fdids = creature_display_map
        .resolve_skin_fdids_for_model_path(m2_path)
        .unwrap_or([0, 0, 0]);
    if m2_spawn::spawn_m2_on_entity(
        commands, meshes, materials, images, skinned_mesh_inverse_bindposes,
        m2_path, root, &skin_fdids,
    ) {
        Some(root)
    } else {
        commands.entity(root).despawn();
        None
    }
}

/// Attach BonePivot components to joint entities and insert M2AnimPlayer on the model.
/// Returns the joint entity list if animation is active, otherwise None.
fn attach_bone_pivots_and_player(
    commands: &mut Commands,
    bones: &[asset::m2_anim::M2Bone],
    sequences: &[asset::m2_anim::M2AnimSequence],
    skinning: &Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
    model_entity: Entity,
) -> Option<Vec<Entity>> {
    let (_, joints) = skinning.as_ref()?;
    for (i, bone) in bones.iter().enumerate() {
        let p = bone.pivot;
        commands
            .entity(joints[i])
            .insert(BonePivot(Vec3::new(p[0], p[2], -p[1])));
    }
    if sequences.is_empty() {
        return None;
    }
    let stand_idx = sequences.iter().position(|s| s.id == 0).unwrap_or(0);
    commands.entity(model_entity).insert(M2AnimPlayer {
        current_seq_idx: stand_idx,
        time_ms: 0.0,
        looping: true,
        transition: None,
    });
    Some(joints.clone())
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

/// Load a BLP file as a GPU-compressed Image (BC1/BC2/BC3) when possible.
fn load_blp_as_image(path: &Path) -> Result<Image, String> {
    asset::blp::load_blp_gpu_image(path)
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
