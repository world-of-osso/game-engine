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
use crate::terrain_lod::{doodad_lod_swap_system, load_obj_for_lod};
use crate::terrain_material::{self, TerrainMaterial};
use crate::terrain_objects;
use crate::terrain_tile::{
    load_tex0, parse_tile_coords_from_path, resolve_tile_path, tile_lod_for_distance,
};
// Re-export for callers that reference these via crate::terrain::.
pub use crate::terrain_tile::bevy_to_tile_coords;
pub(crate) use crate::terrain_tile::resolve_companion_path;
use crate::water_material::WaterMaterial;

#[path = "terrain_background_parse.rs"]
mod terrain_background_parse;
#[path = "terrain_spawn.rs"]
mod terrain_spawn;
#[path = "terrain_spawn_position.rs"]
mod terrain_spawn_position;
#[path = "terrain_streaming.rs"]
mod terrain_streaming;

use terrain_background_parse::parse_tile_background;
use terrain_spawn::{
    SpawnRefs, compute_spawn_result, load_and_parse_adt, log_adt_spawn, spawn_chunk_entities,
    spawn_parsed_tile, spawn_terrain_chunks, spawn_water,
};
use terrain_streaming::{
    compute_desired_tiles, dispatch_tile_loads, handle_tile_result,
    report_initial_world_load_complete, unload_distant_tiles,
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

/// Doodad M2 model pre-loaded on the background thread.
pub(super) struct PreloadedDoodad {
    pub(super) path: PathBuf,
    pub(super) model: crate::asset::m2::M2Model,
}

/// WMO root + groups pre-loaded on the background thread.
pub(super) struct PreloadedWmo {
    pub(super) root_fdid: u32,
    pub(super) root: crate::asset::wmo::WmoRootData,
    /// Group FDID + parsed group data for each successfully loaded group.
    pub(super) groups: Vec<(u32, crate::asset::wmo::WmoGroupData)>,
    pub(super) group_fdids: Vec<Option<u32>>,
}

/// Parsed ADT data ready to be spawned on the main thread.
struct ParsedTile {
    map_name: String,
    tile_y: u32,
    tile_x: u32,
    adt_path: PathBuf,
    adt_data: adt::AdtData,
    tex_data: Option<adt::AdtTexData>,
    obj_data: Option<adt_obj::AdtObjData>,
    lod: DoodadLod,
    /// Pre-decoded ground BLP textures (loaded in background thread).
    ground_images: Vec<Option<Image>>,
    /// Pre-decoded height BLP textures (loaded in background thread).
    height_images: Vec<Option<Image>>,
    /// Pre-packed alpha maps, one per chunk (packed in background thread).
    chunk_alpha_maps: Vec<Image>,
    /// Pre-packed shadow maps, one per chunk (packed in background thread).
    chunk_shadow_maps: Vec<Image>,
    /// Pre-loaded doodad M2 models, indexed by doodad placement index.
    preloaded_doodads: Vec<Option<PreloadedDoodad>>,
    /// Pre-loaded WMO root + group data, indexed by WMO placement index.
    preloaded_wmos: Vec<Option<PreloadedWmo>>,
}

/// Result from a background tile load task.
enum TileLoadResult {
    Success(Box<ParsedTile>),
    Failed {
        map_name: String,
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

impl AdtManager {
    fn clear_completed_tile_results(&mut self) {
        let rx = self.tile_rx.lock().unwrap();
        for _ in rx.try_iter() {}
    }
}

pub(crate) fn reset_streamed_terrain(
    commands: &mut Commands,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
) {
    for root in adt_manager.loaded.drain().map(|(_, root)| root) {
        commands.entity(root).despawn();
    }
    adt_manager.map_name.clear();
    adt_manager.failed.clear();
    adt_manager.pending.clear();
    adt_manager.server_requested.clear();
    adt_manager.tile_lod.clear();
    adt_manager.tile_doodad_entities.clear();
    adt_manager.initial_tile = (0, 0);
    adt_manager.initial_load_reported = false;
    adt_manager.clear_completed_tile_results();
    *heightmap = TerrainHeightmap::default();
}

pub(crate) fn replace_streamed_map(
    commands: &mut Commands,
    adt_manager: &mut AdtManager,
    heightmap: &mut TerrainHeightmap,
    map_name: String,
    initial_tile: (u32, u32),
) -> bool {
    let map_changed = !adt_manager.map_name.is_empty() && adt_manager.map_name != map_name;
    if map_changed {
        reset_streamed_terrain(commands, adt_manager, heightmap);
    }
    if adt_manager.map_name.is_empty() {
        adt_manager.map_name = map_name;
        adt_manager.initial_tile = initial_tile;
        adt_manager.initial_load_reported = false;
    }
    adt_manager.server_requested.insert(initial_tile);
    map_changed
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
    let adt_data = load_and_parse_adt(adt_path)?;
    let tex_data = load_tex0(adt_path, Some(&adt_data));
    Ok(SpawnAdtInputs {
        map_name,
        tile_y,
        tile_x,
        tile: AdtTile {
            _tile_x: tile_x,
            _tile_y: tile_y,
        },
        adt_data,
        tex_data,
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
        refs.water_materials,
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
    let adt_data = load_and_parse_adt(adt_path)?;
    let tex_data = load_tex0(adt_path, Some(&adt_data));
    Ok(TerrainOnlySpawnInputs {
        map_name,
        tile_y,
        tile_x,
        tile: AdtTile {
            _tile_x: tile_x,
            _tile_y: tile_y,
        },
        adt_data,
        tex_data,
    })
}

fn spawn_terrain_only_content(
    assets: &mut TerrainOnlySpawnAssets<'_, '_, '_>,
    heightmap: &mut TerrainHeightmap,
    adt_path: &Path,
    inputs: &TerrainOnlySpawnInputs,
) -> Entity {
    let chunk_materials = build_terrain_only_chunk_materials(assets, inputs, adt_path);
    let root = spawn_terrain_only_entities(assets, inputs, &chunk_materials);
    finalize_terrain_only_spawn(heightmap, adt_path, inputs);
    root
}

fn build_terrain_only_chunk_materials(
    assets: &mut TerrainOnlySpawnAssets<'_, '_, '_>,
    inputs: &TerrainOnlySpawnInputs,
    adt_path: &Path,
) -> Vec<Handle<terrain_material::TerrainMaterial>> {
    let ground_images = inputs
        .tex_data
        .as_ref()
        .map(|td| terrain_material::load_ground_images(assets.images, td, adt_path));
    let height_images = inputs
        .tex_data
        .as_ref()
        .map(|td| terrain_material::load_height_images(assets.images, td, adt_path));
    eprintln!("build_terrain_materials {}", adt_path.display());
    terrain_material::build_terrain_materials(
        assets.terrain_materials,
        assets.images,
        &inputs.adt_data,
        inputs.tex_data.as_ref(),
        ground_images.as_deref(),
        height_images.as_deref(),
        None,
        None,
    )
}

fn spawn_terrain_only_entities(
    assets: &mut TerrainOnlySpawnAssets<'_, '_, '_>,
    inputs: &TerrainOnlySpawnInputs,
    chunk_materials: &[Handle<terrain_material::TerrainMaterial>],
) -> Entity {
    let root = spawn_chunk_entities(
        assets.commands,
        assets.meshes,
        chunk_materials,
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
    root
}

fn finalize_terrain_only_spawn(
    heightmap: &mut TerrainHeightmap,
    adt_path: &Path,
    inputs: &TerrainOnlySpawnInputs,
) {
    heightmap.register_tile(
        inputs.tile_y,
        inputs.tile_x,
        &inputs.adt_data,
        inputs.tex_data.as_ref(),
    );
    log_adt_spawn(&inputs.adt_data, adt_path);
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
///
/// Budgets application to at most `max_tiles_per_frame()` tiles so that
/// multiple tiles finishing in the same interval don't cause a single
/// long frame.  Unprocessed results stay in the mpsc channel and are
/// picked up on subsequent frames.
fn receive_loaded_tiles(mut params: LoadedTileSpawnParams) {
    let budget = crate::terrain_load_limits::max_tiles_per_frame();
    let results: Vec<_> = {
        let rx = params.adt_manager.tile_rx.lock().unwrap();
        rx.try_iter().take(budget).collect()
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
