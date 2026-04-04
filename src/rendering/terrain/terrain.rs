use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};

use bevy::ecs::system::SystemParam;
use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::adt::{self};
use crate::asset::adt_format::adt_obj;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_load_progress;
use crate::terrain_lod::{despawn_tile_doodad_entities, doodad_lod_swap_system, load_obj_for_lod};
use crate::terrain_material::{self, TerrainMaterial};
use crate::terrain_objects;
use crate::terrain_tile::{
    load_tex0, parse_tile_coords_from_path, resolve_tile_path, tile_lod_for_distance,
};
// Re-export for callers that reference these via crate::terrain::.
pub use crate::terrain_tile::bevy_to_tile_coords;
pub(crate) use crate::terrain_tile::resolve_companion_path;
use crate::water_material::WaterMaterial;

#[path = "terrain_spawn.rs"]
mod terrain_spawn;
#[path = "terrain_spawn_position.rs"]
mod terrain_spawn_position;

use terrain_spawn::{
    SpawnRefs, compute_spawn_result, load_and_parse_adt, log_adt_spawn, spawn_chunk_entities,
    spawn_parsed_tile, spawn_terrain_chunks, spawn_water,
};

/// Marker component for the ADT terrain root entity.
#[derive(Component)]
pub struct AdtTerrain;

/// Marker component tagging all entities belonging to a specific ADT tile.
#[derive(Component, Clone)]
pub struct AdtTile {
    pub _tile_x: u32,
    pub _tile_y: u32,
}

/// LOD level for doodad/WMO placements on a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoodadLod {
    /// Full detail (_obj0.adt) — used for near tiles.
    Full,
    /// Reduced detail (_obj1.adt) — used for mid-distance tiles.
    Lod1,
    /// Lowest detail (_obj2.adt) — used for far tiles.
    Lod2,
}

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
    Success(Box<ParsedTile>),
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
    /// Whether we've already reported that the initial terrain load finished.
    pub initial_load_reported: bool,
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
            load_radius: 0,
            initial_tile: (0, 0),
            initial_load_reported: false,
        }
    }
}

/// Result of spawning an ADT: camera and ground position for placing models.
pub struct AdtSpawnResult {
    pub camera: Transform,
    pub center: Vec3,
    /// Root entity of the spawned ADT tile.
    pub root_entity: Entity,
    pub doodad_count: usize,
    pub wmo_entities: Vec<(Entity, String)>,
    pub spawned_object_entities: Vec<Entity>,
    /// Tile coordinates extracted from the filename.
    pub tile_y: u32,
    pub tile_x: u32,
    /// Map name extracted from the filename.
    pub map_name: String,
}

pub struct AdtSpawnAssets<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
    pub effect_materials: &'a mut Assets<M2EffectMaterial>,
    pub terrain_materials: &'a mut Assets<TerrainMaterial>,
    pub water_materials: &'a mut Assets<WaterMaterial>,
    pub images: &'a mut Assets<Image>,
    pub inverse_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
}

pub struct TerrainOnlySpawnAssets<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub terrain_materials: &'a mut Assets<TerrainMaterial>,
    pub water_materials: &'a mut Assets<WaterMaterial>,
    pub images: &'a mut Assets<Image>,
}

#[derive(SystemParam)]
struct LoadedTileSpawnParams<'w, 's> {
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    terrain_mats: ResMut<'w, Assets<TerrainMaterial>>,
    water_mats: ResMut<'w, Assets<WaterMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    inverse_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
    adt_manager: ResMut<'w, AdtManager>,
    heightmap: ResMut<'w, TerrainHeightmap>,
}

struct SpawnAdtInputs {
    map_name: String,
    tile_y: u32,
    tile_x: u32,
    tile: AdtTile,
    adt_data: adt::AdtData,
    tex_data: Option<adt::AdtTexData>,
    obj_data: Option<adt_obj::AdtObjData>,
}

struct SpawnedAdtContent {
    root: Entity,
    spawned_objects: terrain_objects::SpawnedTerrainObjects,
    spawned_fog_volumes: terrain_objects::SpawnedFogVolumes,
}

struct TerrainOnlySpawnInputs {
    map_name: String,
    tile_y: u32,
    tile_x: u32,
    tile: AdtTile,
    adt_data: adt::AdtData,
    tex_data: Option<adt::AdtTexData>,
}

/// Load an ADT file, build meshes, and spawn them into the Bevy scene.
pub fn spawn_adt(
    assets: &mut AdtSpawnAssets<'_, '_, '_>,
    heightmap: &mut TerrainHeightmap,
    adt_path: &Path,
) -> Result<AdtSpawnResult, String> {
    let inputs = load_spawn_adt_inputs(adt_path)?;
    let mut refs = SpawnRefs {
        commands: assets.commands,
        meshes: assets.meshes,
        materials: assets.materials,
        effect_materials: assets.effect_materials,
        terrain_materials: assets.terrain_materials,
        water_materials: assets.water_materials,
        images: assets.images,
        inverse_bp: assets.inverse_bp,
    };
    finish_spawn_adt(&mut refs, heightmap, adt_path, inputs)
}

fn finish_spawn_adt(
    refs: &mut SpawnRefs,
    heightmap: &mut TerrainHeightmap,
    adt_path: &Path,
    inputs: SpawnAdtInputs,
) -> Result<AdtSpawnResult, String> {
    let spawned = spawn_adt_content(refs, heightmap, adt_path, &inputs);
    log_adt_spawn(&inputs.adt_data, adt_path);

    let (camera, center) = compute_spawn_result(&inputs.adt_data, inputs.obj_data.as_ref());
    let doodad_count = spawned.spawned_objects.doodads.len();
    let wmo_entities = spawned
        .spawned_objects
        .wmos
        .iter()
        .map(|wmo| (wmo.entity, wmo.model.clone()))
        .collect();
    let mut spawned_object_entities = spawned.spawned_objects.all_entities();
    spawned_object_entities.extend(spawned.spawned_fog_volumes.entities);
    Ok(build_spawn_adt_result(
        inputs,
        spawned.root,
        camera,
        center,
        doodad_count,
        wmo_entities,
        spawned_object_entities,
    ))
}

fn load_spawn_adt_inputs(adt_path: &Path) -> Result<SpawnAdtInputs, String> {
    let (map_name, tile_y, tile_x) = parse_tile_coords_from_path(adt_path)?;
    Ok(SpawnAdtInputs {
        map_name,
        tile_y,
        tile_x,
        tile: AdtTile {
            _tile_x: tile_x,
            _tile_y: tile_y,
        },
        adt_data: load_and_parse_adt(adt_path)?,
        tex_data: load_tex0(adt_path),
        obj_data: terrain_objects::load_obj0(adt_path),
    })
}

fn spawn_adt_content(
    refs: &mut SpawnRefs,
    heightmap: &mut TerrainHeightmap,
    adt_path: &Path,
    inputs: &SpawnAdtInputs,
) -> SpawnedAdtContent {
    let root = spawn_terrain_chunks(
        refs,
        adt_path,
        &inputs.adt_data,
        inputs.tex_data.as_ref(),
        &inputs.tile,
    );
    heightmap.register_tile(
        inputs.tile_y,
        inputs.tile_x,
        &inputs.adt_data,
        inputs.tex_data.as_ref(),
    );
    SpawnedAdtContent {
        root,
        spawned_objects: spawn_adt_objects(refs, heightmap, inputs),
        spawned_fog_volumes: terrain_objects::spawn_map_fog_volumes(
            refs.commands,
            refs.meshes,
            refs.materials,
            refs.effect_materials,
            refs.images,
            refs.inverse_bp,
            &inputs.map_name,
            Some(root),
        ),
    }
}

fn spawn_adt_objects(
    refs: &mut SpawnRefs,
    heightmap: &mut TerrainHeightmap,
    inputs: &SpawnAdtInputs,
) -> terrain_objects::SpawnedTerrainObjects {
    let Some(obj) = inputs.obj_data.as_ref() else {
        return terrain_objects::SpawnedTerrainObjects::default();
    };
    terrain_objects::spawn_obj_entities(
        refs.commands,
        refs.meshes,
        refs.materials,
        refs.effect_materials,
        refs.images,
        refs.inverse_bp,
        Some(heightmap),
        inputs.tile_y,
        inputs.tile_x,
        obj,
    )
}

fn build_spawn_adt_result(
    inputs: SpawnAdtInputs,
    root_entity: Entity,
    camera: Transform,
    center: Vec3,
    doodad_count: usize,
    wmo_entities: Vec<(Entity, String)>,
    spawned_object_entities: Vec<Entity>,
) -> AdtSpawnResult {
    AdtSpawnResult {
        camera,
        center,
        root_entity,
        doodad_count,
        wmo_entities,
        spawned_object_entities,
        tile_y: inputs.tile_y,
        tile_x: inputs.tile_x,
        map_name: inputs.map_name,
    }
}

/// Load an ADT file and spawn only the terrain mesh + water.
///
/// This avoids the expensive doodad/WMO path and is used by the fast
/// char-select background so the screen becomes visible quickly.
pub fn spawn_adt_terrain_only(
    assets: &mut TerrainOnlySpawnAssets<'_, '_, '_>,
    heightmap: &mut TerrainHeightmap,
    adt_path: &Path,
) -> Result<AdtSpawnResult, String> {
    let inputs = load_terrain_only_spawn_inputs(adt_path)?;
    let root = spawn_terrain_only_content(assets, heightmap, adt_path, &inputs);
    Ok(build_terrain_only_spawn_result(inputs, root))
}

fn load_terrain_only_spawn_inputs(adt_path: &Path) -> Result<TerrainOnlySpawnInputs, String> {
    let (map_name, tile_y, tile_x) = parse_tile_coords_from_path(adt_path)?;
    Ok(TerrainOnlySpawnInputs {
        map_name,
        tile_y,
        tile_x,
        tile: AdtTile {
            _tile_x: tile_x,
            _tile_y: tile_y,
        },
        adt_data: load_and_parse_adt(adt_path)?,
        tex_data: load_tex0(adt_path),
    })
}

fn spawn_terrain_only_content(
    assets: &mut TerrainOnlySpawnAssets<'_, '_, '_>,
    heightmap: &mut TerrainHeightmap,
    adt_path: &Path,
    inputs: &TerrainOnlySpawnInputs,
) -> Entity {
    let ground_images = inputs
        .tex_data
        .as_ref()
        .map(|td| terrain_material::load_ground_images(assets.images, td, adt_path));
    eprintln!("build_terrain_materials {}", adt_path.display());
    let chunk_materials = terrain_material::build_terrain_materials(
        assets.terrain_materials,
        assets.images,
        inputs.tex_data.as_ref(),
        ground_images.as_deref(),
    );
    let root = spawn_chunk_entities(
        assets.commands,
        assets.meshes,
        &chunk_materials,
        &inputs.adt_data,
        &inputs.tile,
    );
    spawn_water(
        assets.commands,
        assets.meshes,
        assets.water_materials,
        assets.images,
        &inputs.adt_data,
    );
    heightmap.register_tile(
        inputs.tile_y,
        inputs.tile_x,
        &inputs.adt_data,
        inputs.tex_data.as_ref(),
    );
    log_adt_spawn(&inputs.adt_data, adt_path);
    root
}

fn build_terrain_only_spawn_result(
    inputs: TerrainOnlySpawnInputs,
    root_entity: Entity,
) -> AdtSpawnResult {
    let (camera, center) = compute_spawn_result(&inputs.adt_data, None);
    AdtSpawnResult {
        camera,
        center,
        root_entity,
        doodad_count: 0,
        wmo_entities: Vec::new(),
        spawned_object_entities: Vec::new(),
        tile_y: inputs.tile_y,
        tile_x: inputs.tile_x,
        map_name: inputs.map_name,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/terrain_tests.rs"]
mod tests;

// ── ADT streaming ────────────────────────────────────────────────────────────

pub struct AdtStreamingPlugin;

impl Plugin for AdtStreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AdtManager>()
            .init_resource::<TerrainHeightmap>()
            .add_systems(
                Update,
                (
                    bootstrap_terrain_streaming,
                    adt_streaming_system,
                    receive_loaded_tiles,
                    report_initial_world_load_complete,
                    doodad_lod_swap_system,
                )
                    .chain()
                    .run_if(in_state(GameState::Loading).or(in_state(GameState::InWorld))),
            );
    }
}

/// Fall back to bootstrapping terrain streaming from the local player's position.
///
/// The server normally seeds this via `LoadTerrain`, but if that message is missed we still
/// want the in-world scene to become navigable instead of rendering actors over empty space.
fn bootstrap_terrain_streaming(
    mut adt_manager: ResMut<AdtManager>,
    player_q: Query<&Transform, With<crate::camera::Player>>,
) {
    if !adt_manager.map_name.is_empty() {
        return;
    }
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let (tile_y, tile_x) = bevy_to_tile_coords(player_tf.translation.x, player_tf.translation.z);
    adt_manager.map_name = "azeroth".into();
    adt_manager.initial_tile = (tile_y, tile_x);
    for key in compute_desired_tiles(tile_y, tile_x, adt_manager.load_radius) {
        adt_manager.server_requested.insert(key);
    }
    info!("Bootstrapped terrain streaming from local player position at tile ({tile_y}, {tile_x})");
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

    // Use player position if available, otherwise fall back to the initial tile center
    // so terrain starts loading before the player entity is tagged.
    let (center_y, center_x) = if let Ok(player_tf) = player_q.single() {
        bevy_to_tile_coords(player_tf.translation.x, player_tf.translation.z)
    } else {
        adt_manager.initial_tile
    };
    let desired = compute_desired_tiles(center_y, center_x, adt_manager.load_radius);

    unload_distant_tiles(&mut commands, &mut adt_manager, &mut heightmap, &desired);
    dispatch_tile_loads(&mut adt_manager, &desired, center_y, center_x);
}

/// Receive parsed tiles from background threads and spawn entities.
fn receive_loaded_tiles(mut params: LoadedTileSpawnParams) {
    let results: Vec<_> = {
        let rx = params.adt_manager.tile_rx.lock().unwrap();
        rx.try_iter().collect()
    };
    let mut refs = SpawnRefs {
        commands: &mut params.commands,
        meshes: &mut params.meshes,
        materials: &mut params.materials,
        effect_materials: &mut params.effect_materials,
        terrain_materials: &mut params.terrain_mats,
        water_materials: &mut params.water_mats,
        images: &mut params.images,
        inverse_bp: &mut params.inverse_bp,
    };
    for result in results {
        handle_tile_result(
            &mut refs,
            &mut params.adt_manager,
            &mut params.heightmap,
            result,
        );
    }
}

fn report_initial_world_load_complete(mut adt_manager: ResMut<AdtManager>) {
    if !terrain_load_progress::should_report_initial_world_load(&adt_manager) {
        return;
    }
    let desired_tiles = terrain_load_progress::initial_desired_tiles(&adt_manager);
    let (loaded, failed, pending) =
        terrain_load_progress::count_initial_tile_progress(&adt_manager, &desired_tiles);
    if pending != 0 || loaded + failed != desired_tiles.len() {
        return;
    }
    info!(
        "Initial world load complete: map={} initial_tile=({}, {}) desired_tiles={} loaded={} failed={}",
        adt_manager.map_name,
        adt_manager.initial_tile.0,
        adt_manager.initial_tile.1,
        desired_tiles.len(),
        loaded,
        failed,
    );
    adt_manager.initial_load_reported = true;
}

fn handle_tile_result(
    refs: &mut SpawnRefs,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
    result: TileLoadResult,
) {
    match result {
        TileLoadResult::Success(parsed) => {
            handle_tile_success(refs, adt_manager, heightmap, parsed);
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

fn handle_tile_success(
    refs: &mut SpawnRefs,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
    parsed: Box<ParsedTile>,
) {
    let key = (parsed.tile_y, parsed.tile_x);
    adt_manager.pending.remove(&key);
    eprintln!(
        "handle_tile_success before register_tile ({}, {}) {}",
        parsed.tile_y,
        parsed.tile_x,
        parsed.adt_path.display()
    );
    heightmap.register_tile(
        parsed.tile_y,
        parsed.tile_x,
        &parsed.adt_data,
        parsed.tex_data.as_ref(),
    );
    eprintln!(
        "handle_tile_success after register_tile ({}, {}) {}",
        parsed.tile_y,
        parsed.tile_x,
        parsed.adt_path.display()
    );
    let (root, doodad_entities) = spawn_parsed_tile(refs, heightmap, &parsed);
    adt_manager.loaded.insert(key, root);
    adt_manager.tile_lod.insert(key, parsed.lod);
    adt_manager
        .tile_doodad_entities
        .insert(key, doodad_entities);
    log_adt_spawn(&parsed.adt_data, &parsed.adt_path);
    log_tile_memory_stats(refs, &parsed);
}

fn log_tile_memory_stats(refs: &SpawnRefs, parsed: &ParsedTile) {
    crate::terrain_memory_debug::log_tile_spawn_stats(
        parsed.tile_y,
        parsed.tile_x,
        &parsed.adt_path,
        refs.images,
        refs.meshes,
        refs.materials,
        refs.terrain_materials,
        refs.water_materials,
        refs.effect_materials,
    );
}

/// Compute the set of (tile_y, tile_x) that should be loaded around a center tile.
fn compute_desired_tiles(center_y: u32, center_x: u32, radius: u32) -> Vec<(u32, u32)> {
    let r = radius as i32;
    (-r..=r)
        .flat_map(|dy| {
            (-r..=r).filter_map(move |dx| {
                let ty = center_y as i32 + dy;
                let tx = center_x as i32 + dx;
                ((0..64).contains(&ty) && (0..64).contains(&tx)).then_some((ty as u32, tx as u32))
            })
        })
        .collect()
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
    if adt_manager.pending.len() >= crate::terrain_load_limits::max_pending_tile_loads() {
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
    let thread_name = format!("adt-load-{ty}-{tx}");
    let spawn_result = std::thread::Builder::new()
        .name(thread_name)
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            tx_chan.send(parse_tile_background(ty, tx, path, lod)).ok();
        });
    if let Err(err) = spawn_result {
        adt_manager.pending.remove(&(ty, tx));
        eprintln!("Cannot spawn ADT loader thread ({ty}, {tx}): {err}");
    }
}

/// Parse an ADT tile and its companions on a background thread.
fn parse_tile_background(
    tile_y: u32,
    tile_x: u32,
    adt_path: PathBuf,
    lod: DoodadLod,
) -> TileLoadResult {
    let start_mem = crate::terrain_memory_debug::current_process_memory_kb();
    log_tile_background_parse_start(tile_y, tile_x, &adt_path, lod, &start_mem);
    let parsed = match build_parsed_tile(tile_y, tile_x, adt_path, lod) {
        Ok(parsed) => parsed,
        Err(error) => {
            return TileLoadResult::Failed {
                tile_y,
                tile_x,
                error,
            };
        }
    };
    log_tile_background_parse_success(&parsed, &start_mem);
    TileLoadResult::Success(Box::new(parsed))
}

fn build_parsed_tile(
    tile_y: u32,
    tile_x: u32,
    adt_path: PathBuf,
    lod: DoodadLod,
) -> Result<ParsedTile, String> {
    let adt_data = load_parsed_adt_data(tile_y, tile_x, &adt_path)?;
    let tex_data = load_parsed_tile_textures(tile_y, tile_x, &adt_path);
    let obj_data = load_parsed_tile_objects(tile_y, tile_x, &adt_path, lod);
    Ok(ParsedTile {
        tile_y,
        tile_x,
        adt_path,
        adt_data,
        tex_data,
        obj_data,
        lod,
    })
}

fn load_parsed_adt_data(tile_y: u32, tile_x: u32, adt_path: &Path) -> Result<adt::AdtData, String> {
    let adt_data = load_and_parse_adt(adt_path)?;
    eprintln!(
        "parse_tile_background adt ok ({}, {}) {} chunks={} height_grids={}",
        tile_y,
        tile_x,
        adt_path.display(),
        adt_data.chunks.len(),
        adt_data.height_grids.len(),
    );
    Ok(adt_data)
}

fn load_parsed_tile_textures(tile_y: u32, tile_x: u32, adt_path: &Path) -> Option<adt::AdtTexData> {
    let tex_data = load_tex0(adt_path);
    eprintln!(
        "parse_tile_background tex ok ({}, {}) {} tex={}",
        tile_y,
        tile_x,
        adt_path.display(),
        tex_data.as_ref().map_or(0, |td| td.texture_fdids.len()),
    );
    tex_data
}

fn load_parsed_tile_objects(
    tile_y: u32,
    tile_x: u32,
    adt_path: &Path,
    lod: DoodadLod,
) -> Option<adt_obj::AdtObjData> {
    let obj_data = load_obj_for_lod(adt_path, lod);
    eprintln!(
        "parse_tile_background obj ok ({}, {}) {} doodads={} wmos={}",
        tile_y,
        tile_x,
        adt_path.display(),
        obj_data.as_ref().map_or(0, |obj| obj.doodads.len()),
        obj_data.as_ref().map_or(0, |obj| obj.wmos.len()),
    );
    obj_data
}

fn log_tile_background_parse_start(
    tile_y: u32,
    tile_x: u32,
    adt_path: &Path,
    lod: DoodadLod,
    start_mem: &crate::terrain_memory_debug::ProcessMemoryKb,
) {
    eprintln!(
        "parse_tile_background start ({}, {}) {} lod={:?} rss={}MiB anon={}MiB",
        tile_y,
        tile_x,
        adt_path.display(),
        lod,
        start_mem.rss_kb / 1024,
        start_mem.anon_kb / 1024,
    );
}

fn log_tile_background_parse_success(
    parsed: &ParsedTile,
    start_mem: &crate::terrain_memory_debug::ProcessMemoryKb,
) {
    let end_mem = crate::terrain_memory_debug::current_process_memory_kb();
    eprintln!(
        "parse_tile_background success ({}, {}) {} rss={}MiB anon={}MiB delta_rss={}MiB",
        parsed.tile_y,
        parsed.tile_x,
        parsed.adt_path.display(),
        end_mem.rss_kb / 1024,
        end_mem.anon_kb / 1024,
        (end_mem.rss_kb as i64 - start_mem.rss_kb as i64) / 1024,
    );
}
