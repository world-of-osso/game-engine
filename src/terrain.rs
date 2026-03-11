use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};

use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::adt::{self};
use crate::asset::adt_obj;
use crate::game_state::GameState;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_lod::{despawn_tile_doodad_entities, doodad_lod_swap_system, load_obj_for_lod};
use crate::terrain_material::{self, TerrainMaterial};
use crate::terrain_objects;
use crate::terrain_tile::{
    load_tex0, parse_tile_coords_from_path, resolve_tile_path, tile_lod_for_distance,
};
// Re-export for callers that reference these via crate::terrain::.
pub use crate::terrain_tile::bevy_to_tile_coords;
pub(crate) use crate::terrain_tile::resolve_companion_path;
use crate::water_material::{self, WaterMaterial, WaterSettings};

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

/// Grouped asset refs for spawn helpers (reduces per-function argument count).
struct SpawnRefs<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    terrain_materials: &'a mut Assets<TerrainMaterial>,
    water_materials: &'a mut Assets<WaterMaterial>,
    images: &'a mut Assets<Image>,
    inverse_bp: &'a mut Assets<SkinnedMeshInverseBindposes>,
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
#[allow(clippy::too_many_arguments)]
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
    let tile = AdtTile { _tile_x: tile_x, _tile_y: tile_y };
    let adt_data = load_and_parse_adt(adt_path)?;
    let tex_data = load_tex0(adt_path);
    let obj_data = terrain_objects::load_obj0(adt_path);

    let mut refs = SpawnRefs { commands, meshes, materials, terrain_materials, water_materials, images, inverse_bp };
    let root = spawn_terrain_chunks(&mut refs, adt_path, &adt_data, tex_data.as_ref(), &tile);
    if let Some(ref obj) = obj_data {
        terrain_objects::spawn_obj_entities(
            refs.commands, refs.meshes, refs.materials, refs.images, refs.inverse_bp, obj,
        );
    }

    heightmap.insert_tile(tile_y, tile_x, &adt_data);
    log_adt_spawn(&adt_data, adt_path);

    let (camera, center) = compute_spawn_result(&adt_data);
    Ok(AdtSpawnResult { camera, center, root_entity: root, tile_y, tile_x, map_name })
}

/// Load and parse an ADT file from disk.
fn load_and_parse_adt(adt_path: &Path) -> Result<adt::AdtData, String> {
    let data = std::fs::read(adt_path)
        .map_err(|e| format!("Failed to read {}: {e}", adt_path.display()))?;
    adt::load_adt(&data)
}

/// Spawn terrain mesh + water for one tile (no doodads).
fn spawn_terrain_chunks(
    refs: &mut SpawnRefs,
    adt_path: &Path,
    adt_data: &adt::AdtData,
    tex_data: Option<&adt::AdtTexData>,
    tile: &AdtTile,
) -> Entity {
    let ground_images =
        tex_data.map(|td| terrain_material::load_ground_images(refs.images, td, adt_path));
    let chunk_materials = terrain_material::build_terrain_materials(
        refs.terrain_materials, refs.images, tex_data, ground_images.as_deref(),
    );
    let root = spawn_chunk_entities(refs.commands, refs.meshes, &chunk_materials, adt_data, tile);
    spawn_water(refs.commands, refs.meshes, refs.water_materials, refs.images, adt_data);
    root
}

/// Spawn entities from a fully-parsed tile (async receive path).
fn spawn_parsed_tile(refs: &mut SpawnRefs, parsed: &ParsedTile) -> (Entity, Vec<Entity>) {
    let tile = AdtTile { _tile_x: parsed.tile_x, _tile_y: parsed.tile_y };
    let root = spawn_terrain_chunks(
        refs, &parsed.adt_path, &parsed.adt_data, parsed.tex_data.as_ref(), &tile,
    );
    let doodad_entities = if let Some(ref obj_data) = parsed.obj_data {
        terrain_objects::spawn_obj_entities(
            refs.commands, refs.meshes, refs.materials, refs.images, refs.inverse_bp, obj_data,
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
            use crate::asset::adt::CHUNK_SIZE;
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
    let target = first_water_position(adt_data).unwrap_or(center);
    let eye = target + Vec3::new(30.0, 20.0, 30.0);
    let camera = Transform::from_translation(eye).looking_at(target, Vec3::Y);
    (camera, center)
}

/// Find the Bevy-space center of the first water chunk (for camera positioning).
fn first_water_position(adt_data: &adt::AdtData) -> Option<Vec3> {
    use crate::asset::adt::CHUNK_SIZE;
    use crate::asset::m2::wow_to_bevy;
    let water = adt_data.water.as_ref()?;
    for (i, cw) in water.chunks.iter().enumerate() {
        if let Some(layer) = cw.layers.first() {
            if layer.width == 0 || layer.height == 0 {
                continue;
            }
            let pos = adt_data.chunk_positions[i];
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
    let results: Vec<_> = {
        let rx = adt_manager.tile_rx.lock().unwrap();
        rx.try_iter().collect()
    };
    let mut refs = SpawnRefs {
        commands: &mut commands,
        meshes: &mut meshes,
        materials: &mut materials,
        terrain_materials: &mut terrain_mats,
        water_materials: &mut water_mats,
        images: &mut images,
        inverse_bp: &mut inverse_bp,
    };
    for result in results {
        handle_tile_result(&mut refs, &mut adt_manager, &mut heightmap, result);
    }
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
        TileLoadResult::Failed { tile_y, tile_x, error } => {
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
    let (root, doodad_entities) = spawn_parsed_tile(refs, &parsed);
    heightmap.insert_tile(parsed.tile_y, parsed.tile_x, &parsed.adt_data);
    adt_manager.loaded.insert(key, root);
    adt_manager.tile_lod.insert(key, parsed.lod);
    adt_manager.tile_doodad_entities.insert(key, doodad_entities);
    log_adt_spawn(&parsed.adt_data, &parsed.adt_path);
}

/// Compute the set of (tile_y, tile_x) that should be loaded around a center tile.
fn compute_desired_tiles(center_y: u32, center_x: u32, radius: u32) -> Vec<(u32, u32)> {
    let r = radius as i32;
    let mut tiles = Vec::with_capacity(((2 * r + 1) * (2 * r + 1)) as usize);
    for dy in -r..=r {
        for dx in -r..=r {
            let ty = center_y as i32 + dy;
            let tx = center_x as i32 + dx;
            if (0..64).contains(&ty) && (0..64).contains(&tx) {
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
            return TileLoadResult::Failed { tile_y, tile_x, error: e };
        }
    };
    let tex_data = load_tex0(&adt_path);
    let obj_data = load_obj_for_lod(&adt_path, lod);
    TileLoadResult::Success(Box::new(ParsedTile {
        tile_y,
        tile_x,
        adt_path,
        adt_data,
        tex_data,
        obj_data,
        lod,
    }))
}
