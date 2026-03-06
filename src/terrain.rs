use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};

use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::adt::{self, CHUNK_SIZE};
use crate::asset::adt_obj;
use crate::game_state::GameState;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::{self, TerrainMaterial};
use crate::terrain_objects;
use crate::water_material::{self, WaterMaterial, WaterSettings};

/// WoW tile size in yards: 16 chunks × 33.33 yards/chunk = 533.33.
const TILE_SIZE: f32 = CHUNK_SIZE * 16.0;

/// Marker component for the ADT terrain root entity.
#[derive(Component)]
pub struct AdtTerrain;

/// Marker component tagging all entities belonging to a specific ADT tile.
#[derive(Component, Clone)]
pub struct AdtTile {
    pub tile_x: u32,
    pub tile_y: u32,
}

/// LOD level for doodad/WMO placements on a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoodadLod {
    /// Full detail (_obj0.adt) — used for near tiles.
    Full,
    /// Reduced detail (_obj1.adt) — used for distant tiles.
    Lod1,
}

/// Distance threshold in tiles: tiles farther than this use LOD1 doodads.
const LOD1_TILE_DISTANCE: u32 = 2;

/// Parsed ADT data ready to be spawned on the main thread.
struct ParsedTile {
    tile_y: u32,
    tile_x: u32,
    adt_path: PathBuf,
    adt_data: adt::AdtData,
    tex_data: Option<adt::AdtTexData>,
    obj_data: Option<adt_obj::AdtObjData>,
    lod: DoodadLod,
}

/// Result from a background tile load task.
enum TileLoadResult {
    Success(ParsedTile),
    Failed {
        tile_y: u32,
        tile_x: u32,
        error: String,
    },
}

/// Manages multi-tile ADT streaming around the player.
#[derive(Resource)]
pub struct AdtManager {
    /// Map name extracted from the initial ADT (e.g., "azeroth").
    pub map_name: String,
    /// Currently loaded tiles: (row, col) → root entity.
    pub loaded: HashMap<(u32, u32), Entity>,
    /// Tiles that failed to load (missing files); don't retry.
    pub(crate) failed: HashSet<(u32, u32)>,
    /// Tiles currently being loaded in background threads.
    pub(crate) pending: HashSet<(u32, u32)>,
    /// Tiles explicitly requested by the server (beyond the local streaming radius).
    pub server_requested: HashSet<(u32, u32)>,
    /// Current doodad LOD level per loaded tile.
    pub(crate) tile_lod: HashMap<(u32, u32), DoodadLod>,
    /// Doodad/WMO entities per tile, for despawning on LOD swap.
    pub(crate) tile_doodad_entities: HashMap<(u32, u32), Vec<Entity>>,
    /// Receiver for completed background tile loads (Mutex for Sync).
    tile_rx: Mutex<mpsc::Receiver<TileLoadResult>>,
    /// Sender cloned into background threads.
    tile_tx: mpsc::Sender<TileLoadResult>,
    /// Radius of tiles to keep loaded around player (1 = 3×3 grid).
    pub load_radius: u32,
    /// Tile coordinates of the initially loaded tile.
    pub initial_tile: (u32, u32),
}

impl Default for AdtManager {
    fn default() -> Self {
        let (tile_tx, tile_rx) = mpsc::channel();
        Self {
            map_name: String::new(),
            loaded: HashMap::new(),
            failed: HashSet::new(),
            pending: HashSet::new(),
            server_requested: HashSet::new(),
            tile_lod: HashMap::new(),
            tile_doodad_entities: HashMap::new(),
            tile_rx: Mutex::new(tile_rx),
            tile_tx,
            load_radius: 1,
            initial_tile: (0, 0),
        }
    }
}

/// Result of spawning an ADT: camera and ground position for placing models.
pub struct AdtSpawnResult {
    pub camera: Transform,
    pub center: Vec3,
    /// Root entity of the spawned ADT tile.
    pub root_entity: Entity,
    /// Tile coordinates extracted from the filename.
    pub tile_y: u32,
    pub tile_x: u32,
    /// Map name extracted from the filename.
    pub map_name: String,
}

/// Load an ADT file, build meshes, and spawn them into the Bevy scene.
pub fn spawn_adt(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_path: &Path,
) -> Result<AdtSpawnResult, String> {
    let (map_name, tile_y, tile_x) = parse_tile_coords_from_path(adt_path)?;
    let tile = AdtTile { tile_x, tile_y };
    let adt_data = load_and_parse_adt(adt_path)?;

    let root = spawn_adt_entities(
        commands,
        meshes,
        materials,
        terrain_materials,
        water_materials,
        images,
        inverse_bp,
        adt_path,
        &adt_data,
        &tile,
    );

    heightmap.insert_tile(tile_y, tile_x, &adt_data);
    log_adt_spawn(&adt_data, adt_path);

    let spawn_result = compute_spawn_result(&adt_data);
    Ok(AdtSpawnResult {
        camera: spawn_result.0,
        center: spawn_result.1,
        root_entity: root,
        tile_y,
        tile_x,
        map_name,
    })
}

/// Load and parse an ADT file from disk.
fn load_and_parse_adt(adt_path: &Path) -> Result<adt::AdtData, String> {
    let data = std::fs::read(adt_path)
        .map_err(|e| format!("Failed to read {}: {e}", adt_path.display()))?;
    adt::load_adt(&data)
}

/// Spawn all entities for one ADT tile: terrain chunks, water, and objects.
fn spawn_adt_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    adt_path: &Path,
    adt_data: &adt::AdtData,
    tile: &AdtTile,
) -> Entity {
    let tex_data = load_tex0(adt_path);
    let obj_data = terrain_objects::load_obj0(adt_path);
    let root = spawn_from_parsed(
        commands,
        meshes,
        materials,
        terrain_materials,
        water_materials,
        images,
        inverse_bp,
        adt_path,
        adt_data,
        tex_data.as_ref(),
        tile,
    );
    if let Some(ref obj) = obj_data {
        terrain_objects::spawn_obj_entities(commands, meshes, materials, images, inverse_bp, obj);
    }
    root
}

/// Spawn entities from pre-parsed tile data (used by both sync and async paths).
fn spawn_from_parsed(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    adt_path: &Path,
    adt_data: &adt::AdtData,
    tex_data: Option<&adt::AdtTexData>,
    tile: &AdtTile,
) -> Entity {
    let ground_images =
        tex_data.map(|td| terrain_material::load_ground_images(images, td, adt_path));
    let chunk_materials = terrain_material::build_terrain_materials(
        terrain_materials,
        images,
        tex_data,
        ground_images.as_deref(),
    );

    let root = spawn_chunk_entities(commands, meshes, &chunk_materials, adt_data, tile);
    spawn_water(commands, meshes, water_materials, images, adt_data);
    root
}

/// Spawn entities from a fully-parsed tile (async receive path).
fn spawn_parsed_tile(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    parsed: &ParsedTile,
) -> (Entity, Vec<Entity>) {
    let tile = AdtTile {
        tile_x: parsed.tile_x,
        tile_y: parsed.tile_y,
    };
    let root = spawn_from_parsed(
        commands,
        meshes,
        materials,
        terrain_materials,
        water_materials,
        images,
        inverse_bp,
        &parsed.adt_path,
        &parsed.adt_data,
        parsed.tex_data.as_ref(),
        &tile,
    );
    // Spawn doodads/WMOs from pre-parsed obj data
    let doodad_entities = if let Some(ref obj_data) = parsed.obj_data {
        terrain_objects::spawn_obj_entities(
            commands, meshes, materials, images, inverse_bp, obj_data,
        )
    } else {
        Vec::new()
    };
    (root, doodad_entities)
}

/// Log a summary of a spawned ADT tile.
fn log_adt_spawn(adt_data: &adt::AdtData, adt_path: &Path) {
    let water_count = adt_data.water.as_ref().map_or(0, |w| {
        w.chunks.iter().filter(|c| !c.layers.is_empty()).count()
    });
    eprintln!(
        "Spawned ADT terrain: {} chunks, {} water chunks from {}",
        adt_data.chunks.len(),
        water_count,
        adt_path.display(),
    );
}

/// Resolve the local file path for an ADT tile via listfile FDID lookup.
fn resolve_tile_path(map_name: &str, tile_y: u32, tile_x: u32) -> Result<PathBuf, String> {
    let wow_path = format!("world/maps/{map_name}/{map_name}_{tile_y}_{tile_x}.adt");
    let fdid = game_engine::listfile::lookup_path(&wow_path)
        .ok_or_else(|| format!("Tile ({tile_y},{tile_x}) not in listfile: {wow_path}"))?;
    let local = PathBuf::from(format!("data/terrain/{map_name}_{tile_y}_{tile_x}.adt"));
    if local.exists() {
        return Ok(local);
    }
    // Fall back to FDID-based naming.
    let fdid_path = PathBuf::from(format!("data/terrain/{fdid}.adt"));
    if fdid_path.exists() {
        return Ok(fdid_path);
    }
    Err(format!(
        "ADT tile files not found: {} or {}",
        local.display(),
        fdid_path.display()
    ))
}

/// Parse map name and tile coordinates from an ADT filename.
///
/// Supports both `mapname_Y_X.adt` and FDID-based `778027.adt` (via listfile reverse lookup).
fn parse_tile_coords_from_path(adt_path: &Path) -> Result<(String, u32, u32), String> {
    let stem = adt_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Invalid ADT path: {}", adt_path.display()))?;

    // Try name-based: "azeroth_32_48"
    if let Some(result) = try_parse_named_stem(stem) {
        return Ok(result);
    }
    // Try FDID-based: "778027" → reverse lookup via listfile
    if let Ok(fdid) = stem.parse::<u32>() {
        return parse_coords_from_fdid(fdid);
    }
    Err(format!("Cannot parse tile coords from: {stem}"))
}

/// Try to parse "mapname_Y_X" from an ADT stem.
fn try_parse_named_stem(stem: &str) -> Option<(String, u32, u32)> {
    let parts: Vec<&str> = stem.rsplitn(3, '_').collect();
    if parts.len() < 3 {
        return None;
    }
    let tile_x = parts[0].parse::<u32>().ok()?;
    let tile_y = parts[1].parse::<u32>().ok()?;
    let map_name = parts[2].to_string();
    Some((map_name, tile_y, tile_x))
}

/// Reverse-lookup an FDID to extract map name and tile coordinates.
fn parse_coords_from_fdid(fdid: u32) -> Result<(String, u32, u32), String> {
    let wow_path = game_engine::listfile::lookup_fdid(fdid)
        .ok_or_else(|| format!("FDID {fdid} not in listfile"))?;
    // Path like "world/maps/azeroth/azeroth_32_48.adt"
    let filename = wow_path.rsplit('/').next().unwrap_or(wow_path);
    let stem = filename.strip_suffix(".adt").unwrap_or(filename);
    try_parse_named_stem(stem)
        .ok_or_else(|| format!("Cannot parse tile coords from listfile path: {wow_path}"))
}

/// Convert a Bevy world position to WoW ADT tile coordinates.
///
/// Returns (row, col) matching the ADT filename convention: `map_{row}_{col}.adt`.
/// WoW MCNK stores position as [Y, X, Z]; Bevy maps: bx=wow_x, bz=-wow_y.
/// ADT filename row = f(wow_x) = f(bx), col = f(wow_y) = f(-bz).
pub fn bevy_to_tile_coords(bx: f32, bz: f32) -> (u32, u32) {
    let center = 32.0 * TILE_SIZE;
    let row = ((center - bx) / TILE_SIZE).floor() as i32;
    let col = ((center + bz) / TILE_SIZE).floor() as i32;
    (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
}

/// Resolve companion file path (e.g. "_tex0", "_obj0") for an ADT.
///
/// For name-based files (e.g. `azeroth_32_48.adt`), appends suffix directly.
/// For FDID-based files (e.g. `778027.adt`), looks up the companion FDID via listfile.
pub(crate) fn resolve_companion_path(adt_path: &Path, suffix: &str) -> Option<PathBuf> {
    let stem = adt_path.file_stem()?.to_str()?;
    // Name-based: "azeroth_32_48" → "azeroth_32_48_tex0.adt"
    let direct = adt_path.with_file_name(format!("{stem}{suffix}.adt"));
    if direct.exists() {
        return Some(direct);
    }
    // FDID-based: reverse lookup to get WoW path, then find companion FDID
    let fdid: u32 = stem.parse().ok()?;
    let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
    let wow_stem = wow_path.strip_suffix(".adt")?;
    let companion_wow = format!("{wow_stem}{suffix}.adt");
    let companion_fdid = game_engine::listfile::lookup_path(&companion_wow)?;
    let companion_path = adt_path.with_file_name(format!("{companion_fdid}.adt"));
    if companion_path.exists() {
        Some(companion_path)
    } else {
        None
    }
}

/// Try to load the companion _tex0.adt file.
fn load_tex0(adt_path: &Path) -> Option<adt::AdtTexData> {
    let tex0_path = resolve_companion_path(adt_path, "_tex0")?;
    let data = std::fs::read(&tex0_path).ok()?;
    match adt::load_adt_tex0(&data) {
        Ok(td) => {
            eprintln!(
                "Loaded _tex0: {} textures, {} chunks",
                td.texture_fdids.len(),
                td.chunk_layers.len()
            );
            Some(td)
        }
        Err(e) => {
            eprintln!("Failed to parse _tex0: {e}");
            None
        }
    }
}

fn spawn_chunk_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    chunk_materials: &[Handle<TerrainMaterial>],
    adt_data: &adt::AdtData,
    tile: &AdtTile,
) -> Entity {
    let root = commands
        .spawn((
            AdtTerrain,
            tile.clone(),
            Transform::default(),
            Visibility::default(),
        ))
        .id();

    for (i, chunk) in adt_data.chunks.iter().enumerate() {
        let mesh_handle = meshes.add(chunk.mesh.clone());
        let mat = chunk_materials.get(i).unwrap_or(&chunk_materials[0]);
        let mut spawn = commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(mat.clone()),
            tile.clone(),
            Transform::default(),
            Visibility::default(),
        ));
        if let Some(grid) = adt_data.height_grids.get(i) {
            spawn.insert(game_engine::culling::TerrainChunk {
                world_center: Vec3::new(
                    grid.origin_x + CHUNK_SIZE / 2.0,
                    grid.base_y,
                    grid.origin_z - CHUNK_SIZE / 2.0,
                ),
            });
        }
        let child = spawn.id();
        commands.entity(root).add_child(child);
    }
    root
}

fn compute_spawn_result(adt_data: &adt::AdtData) -> (Transform, Vec3) {
    let center: Vec3 = adt_data.center_surface.into();

    // Position near Crystal Lake water (first water chunk in Elwynn)
    let target = first_water_position(adt_data).unwrap_or(center);
    let eye = target + Vec3::new(30.0, 20.0, 30.0);
    let camera = Transform::from_translation(eye).looking_at(target, Vec3::Y);

    (camera, center)
}

/// Find the Bevy-space center of the first water chunk (for camera positioning).
fn first_water_position(adt_data: &adt::AdtData) -> Option<Vec3> {
    let water = adt_data.water.as_ref()?;
    for (i, cw) in water.chunks.iter().enumerate() {
        if let Some(layer) = cw.layers.first() {
            if layer.width == 0 || layer.height == 0 {
                continue;
            }
            let pos = adt_data.chunk_positions[i];
            use crate::asset::m2::wow_to_bevy;
            let center_col = layer.x_offset as f32 + layer.width as f32 / 2.0;
            let center_row = layer.y_offset as f32 + layer.height as f32 / 2.0;
            let wx = pos[1] - center_col * CHUNK_SIZE / 8.0;
            let wy = pos[0] - center_row * CHUNK_SIZE / 8.0;
            let [bx, by, bz] = wow_to_bevy(wx, wy, layer.min_height);
            return Some(Vec3::new(bx, by, bz));
        }
    }
    None
}

// ── water spawning ──────────────────────────────────────────────────────────

fn spawn_water(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    adt_data: &adt::AdtData,
) {
    let Some(ref water_data) = adt_data.water else {
        return;
    };
    let normal_map = images.add(water_material::generate_water_normal_map());
    let mat = water_materials.add(WaterMaterial {
        settings: WaterSettings::default(),
        normal_map,
    });

    for (i, chunk_water) in water_data.chunks.iter().enumerate() {
        for layer in &chunk_water.layers {
            if layer.width == 0 || layer.height == 0 {
                continue;
            }
            let chunk_pos = adt_data.chunk_positions[i];
            let mesh = adt::build_water_mesh(chunk_pos, layer);
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(mat.clone()),
                Transform::default(),
                Visibility::default(),
            ));
        }
    }
}

// ── ADT streaming ────────────────────────────────────────────────────────────

pub struct AdtStreamingPlugin;

impl Plugin for AdtStreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AdtManager>()
            .init_resource::<TerrainHeightmap>()
            .add_systems(
                Update,
                (
                    adt_streaming_system,
                    receive_loaded_tiles,
                    doodad_lod_swap_system,
                )
                    .chain()
                    .run_if(in_state(GameState::InWorld)),
            );
    }
}

/// Dispatch background loads and unload distant tiles.
fn adt_streaming_system(
    mut commands: Commands,
    mut adt_manager: ResMut<AdtManager>,
    mut heightmap: ResMut<TerrainHeightmap>,
    player_q: Query<&Transform, With<crate::camera::Player>>,
) {
    if adt_manager.map_name.is_empty() {
        return;
    }
    let Ok(player_tf) = player_q.single() else {
        return;
    };

    let (center_y, center_x) =
        bevy_to_tile_coords(player_tf.translation.x, player_tf.translation.z);
    let desired = compute_desired_tiles(center_y, center_x, adt_manager.load_radius);

    unload_distant_tiles(&mut commands, &mut adt_manager, &mut heightmap, &desired);
    dispatch_tile_loads(&mut adt_manager, &desired, center_y, center_x);
}

/// Receive parsed tiles from background threads and spawn entities.
#[allow(clippy::too_many_arguments)]
fn receive_loaded_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain_mats: ResMut<Assets<TerrainMaterial>>,
    mut water_mats: ResMut<Assets<WaterMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut adt_manager: ResMut<AdtManager>,
    mut heightmap: ResMut<TerrainHeightmap>,
) {
    // Drain all ready results (non-blocking), then process them
    let results: Vec<_> = {
        let rx = adt_manager.tile_rx.lock().unwrap();
        rx.try_iter().collect()
    };
    for result in results {
        handle_tile_result(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut terrain_mats,
            &mut water_mats,
            &mut images,
            &mut inverse_bp,
            &mut adt_manager,
            &mut heightmap,
            result,
        );
    }
}

fn handle_tile_result(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    terrain_mats: &mut Assets<TerrainMaterial>,
    water_mats: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
    result: TileLoadResult,
) {
    match result {
        TileLoadResult::Success(parsed) => {
            let key = (parsed.tile_y, parsed.tile_x);
            adt_manager.pending.remove(&key);
            let (root, doodad_entities) = spawn_parsed_tile(
                commands,
                meshes,
                materials,
                terrain_mats,
                water_mats,
                images,
                inverse_bp,
                &parsed,
            );
            heightmap.insert_tile(parsed.tile_y, parsed.tile_x, &parsed.adt_data);
            adt_manager.loaded.insert(key, root);
            let lod = parsed.lod;
            adt_manager.tile_lod.insert(key, lod);
            adt_manager
                .tile_doodad_entities
                .insert(key, doodad_entities);
            log_adt_spawn(&parsed.adt_data, &parsed.adt_path);
        }
        TileLoadResult::Failed {
            tile_y,
            tile_x,
            error,
        } => {
            adt_manager.pending.remove(&(tile_y, tile_x));
            adt_manager.failed.insert((tile_y, tile_x));
            eprintln!("Cannot load ADT tile ({tile_y}, {tile_x}): {error}");
        }
    }
}

/// Compute the set of (tile_y, tile_x) that should be loaded around a center tile.
fn compute_desired_tiles(center_y: u32, center_x: u32, radius: u32) -> Vec<(u32, u32)> {
    let r = radius as i32;
    let mut tiles = Vec::with_capacity(((2 * r + 1) * (2 * r + 1)) as usize);
    for dy in -r..=r {
        for dx in -r..=r {
            let ty = center_y as i32 + dy;
            let tx = center_x as i32 + dx;
            if ty >= 0 && ty < 64 && tx >= 0 && tx < 64 {
                tiles.push((ty as u32, tx as u32));
            }
        }
    }
    tiles
}

/// Unload tiles that are no longer in the desired set.
fn unload_distant_tiles(
    commands: &mut Commands,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
    desired: &[(u32, u32)],
) {
    let to_remove: Vec<(u32, u32)> = adt_manager
        .loaded
        .keys()
        .filter(|k| !desired.contains(k))
        .copied()
        .collect();

    for key in to_remove {
        if let Some(root) = adt_manager.loaded.remove(&key) {
            commands.entity(root).despawn();
        }
        adt_manager.tile_lod.remove(&key);
        despawn_tile_doodad_entities(commands, adt_manager, key);
        heightmap.remove_tile(key.0, key.1);
        eprintln!("Unloaded ADT tile ({}, {})", key.0, key.1);
    }
}

/// Compute the LOD level for a tile based on Chebyshev distance from player.
fn tile_lod_for_distance(ty: u32, tx: u32, center_y: u32, center_x: u32) -> DoodadLod {
    let dy = ty.abs_diff(center_y);
    let dx = tx.abs_diff(center_x);
    let dist = dy.max(dx);
    if dist > LOD1_TILE_DISTANCE {
        DoodadLod::Lod1
    } else {
        DoodadLod::Full
    }
}

/// Dispatch background thread loads for tiles not yet loaded or pending.
fn dispatch_tile_loads(
    adt_manager: &mut AdtManager,
    desired: &[(u32, u32)],
    center_y: u32,
    center_x: u32,
) {
    for &(ty, tx) in desired {
        dispatch_single_tile(adt_manager, ty, tx, center_y, center_x);
    }
    let requested: Vec<_> = adt_manager.server_requested.drain().collect();
    for (ty, tx) in requested {
        dispatch_single_tile(adt_manager, ty, tx, center_y, center_x);
    }
}

/// Dispatch a single tile load if not already loaded/pending/failed.
fn dispatch_single_tile(
    adt_manager: &mut AdtManager,
    ty: u32,
    tx: u32,
    center_y: u32,
    center_x: u32,
) {
    if adt_manager.loaded.contains_key(&(ty, tx)) {
        return;
    }
    if adt_manager.failed.contains(&(ty, tx)) {
        return;
    }
    if adt_manager.pending.contains(&(ty, tx)) {
        return;
    }

    let path = match resolve_tile_path(&adt_manager.map_name, ty, tx) {
        Ok(p) => p,
        Err(e) => {
            adt_manager.failed.insert((ty, tx));
            eprintln!("Cannot load ADT tile ({ty}, {tx}): {e}");
            return;
        }
    };

    let lod = tile_lod_for_distance(ty, tx, center_y, center_x);
    adt_manager.pending.insert((ty, tx));
    let tx_chan = adt_manager.tile_tx.clone();
    std::thread::spawn(move || {
        tx_chan.send(parse_tile_background(ty, tx, path, lod)).ok();
    });
}

/// Parse an ADT tile and its companions on a background thread.
fn parse_tile_background(
    tile_y: u32,
    tile_x: u32,
    adt_path: PathBuf,
    lod: DoodadLod,
) -> TileLoadResult {
    let adt_data = match load_and_parse_adt(&adt_path) {
        Ok(d) => d,
        Err(e) => {
            return TileLoadResult::Failed {
                tile_y,
                tile_x,
                error: e,
            };
        }
    };
    let tex_data = load_tex0(&adt_path);
    let obj_data = load_obj_for_lod(&adt_path, lod);
    TileLoadResult::Success(ParsedTile {
        tile_y,
        tile_x,
        adt_path,
        adt_data,
        tex_data,
        obj_data,
        lod,
    })
}

/// Load the appropriate obj file based on LOD level.
fn load_obj_for_lod(adt_path: &Path, lod: DoodadLod) -> Option<adt_obj::AdtObjData> {
    match lod {
        DoodadLod::Full => terrain_objects::load_obj0(adt_path),
        DoodadLod::Lod1 => terrain_objects::load_obj1(adt_path),
    }
}

/// Swap doodad LOD levels when player crosses distance thresholds.
#[allow(clippy::too_many_arguments)]
fn doodad_lod_swap_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut adt_manager: ResMut<AdtManager>,
    player_q: Query<&Transform, With<crate::camera::Player>>,
) {
    if adt_manager.map_name.is_empty() {
        return;
    }
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let (cy, cx) = bevy_to_tile_coords(player_tf.translation.x, player_tf.translation.z);
    let swaps = find_lod_swaps(&adt_manager, cy, cx);
    for (key, new_lod) in swaps {
        swap_tile_lod(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            &mut inverse_bp,
            &mut adt_manager,
            key,
            new_lod,
        );
    }
}

/// Find tiles whose LOD level needs changing.
fn find_lod_swaps(adt_manager: &AdtManager, cy: u32, cx: u32) -> Vec<((u32, u32), DoodadLod)> {
    adt_manager
        .tile_lod
        .iter()
        .filter_map(|(&key, &current_lod)| {
            let desired = tile_lod_for_distance(key.0, key.1, cy, cx);
            if desired != current_lod {
                Some((key, desired))
            } else {
                None
            }
        })
        .collect()
}

/// Swap a tile's doodads to a new LOD level.
fn swap_tile_lod(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    adt_manager: &mut AdtManager,
    key: (u32, u32),
    new_lod: DoodadLod,
) {
    let Ok(adt_path) = resolve_tile_path(&adt_manager.map_name, key.0, key.1) else {
        return;
    };
    despawn_tile_doodad_entities(commands, adt_manager, key);
    let new_entities = spawn_lod_doodads(
        commands, meshes, materials, images, inverse_bp, &adt_path, new_lod,
    );
    adt_manager.tile_lod.insert(key, new_lod);
    adt_manager.tile_doodad_entities.insert(key, new_entities);
    eprintln!("LOD swap tile ({}, {}): {:?}", key.0, key.1, new_lod);
}

/// Despawn doodad/WMO entities for a tile (without removing LOD tracking).
fn despawn_tile_doodad_entities(
    commands: &mut Commands,
    adt_manager: &mut AdtManager,
    key: (u32, u32),
) {
    if let Some(entities) = adt_manager.tile_doodad_entities.remove(&key) {
        for e in entities {
            commands.entity(e).despawn();
        }
    }
}

/// Load and spawn doodads/WMOs for a given LOD level.
fn spawn_lod_doodads(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    adt_path: &Path,
    lod: DoodadLod,
) -> Vec<Entity> {
    match load_obj_for_lod(adt_path, lod) {
        Some(ref obj) => terrain_objects::spawn_obj_entities(
            commands, meshes, materials, images, inverse_bp, obj,
        ),
        None => Vec::new(),
    }
}
