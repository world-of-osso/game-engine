use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, mpsc};

use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::adt::{self};
use crate::asset::adt_obj;
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_heightmap::sample_chunk_height;
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
    effect_materials: &'a mut Assets<M2EffectMaterial>,
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
    pub doodad_count: usize,
    pub wmo_entities: Vec<(Entity, String)>,
    pub spawned_object_entities: Vec<Entity>,
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
    effect_materials: &mut Assets<M2EffectMaterial>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    heightmap: &mut TerrainHeightmap,
    adt_path: &Path,
) -> Result<AdtSpawnResult, String> {
    let (map_name, tile_y, tile_x) = parse_tile_coords_from_path(adt_path)?;
    let tile = AdtTile {
        _tile_x: tile_x,
        _tile_y: tile_y,
    };
    let adt_data = load_and_parse_adt(adt_path)?;
    let tex_data = load_tex0(adt_path);
    let obj_data = terrain_objects::load_obj0(adt_path);

    let mut refs = SpawnRefs {
        commands,
        meshes,
        materials,
        effect_materials,
        terrain_materials,
        water_materials,
        images,
        inverse_bp,
    };
    let root = spawn_terrain_chunks(&mut refs, adt_path, &adt_data, tex_data.as_ref(), &tile);
    heightmap.insert_tile(tile_y, tile_x, &adt_data);
    let spawned_objects = if let Some(ref obj) = obj_data {
        terrain_objects::spawn_obj_entities(
            refs.commands,
            refs.meshes,
            refs.materials,
            refs.effect_materials,
            refs.images,
            refs.inverse_bp,
            Some(heightmap),
            obj,
        )
    } else {
        terrain_objects::SpawnedTerrainObjects::default()
    };
    log_adt_spawn(&adt_data, adt_path);

    let (camera, center) = compute_spawn_result(&adt_data, obj_data.as_ref());
    let doodad_count = spawned_objects.doodads.len();
    let wmo_entities = spawned_objects
        .wmos
        .iter()
        .map(|wmo| (wmo.entity, wmo.model.clone()))
        .collect();
    let spawned_object_entities = spawned_objects.all_entities();
    Ok(AdtSpawnResult {
        camera,
        center,
        root_entity: root,
        doodad_count,
        wmo_entities,
        spawned_object_entities,
        tile_y,
        tile_x,
        map_name,
    })
}

/// Load and parse an ADT file from disk.
fn load_and_parse_adt(adt_path: &Path) -> Result<adt::AdtData, String> {
    let (_, tile_y, tile_x) = parse_tile_coords_from_path(adt_path)?;
    let data = std::fs::read(adt_path)
        .map_err(|e| format!("Failed to read {}: {e}", adt_path.display()))?;
    let adt = adt::load_adt_for_tile(&data, tile_y, tile_x)?;
    if let Some(err) = &adt.water_error {
        warn_mh2o_once(adt_path, err);
    }
    Ok(adt)
}

fn warn_mh2o_once(adt_path: &Path, err: &str) {
    static WARNED_TILES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let warned = WARNED_TILES.get_or_init(|| Mutex::new(HashSet::new()));
    let key = format!("{}:{err}", adt_path.display());
    let mut warned = warned.lock().unwrap();
    if warned.insert(key) {
        warn!("Ignoring malformed MH2O in {}: {err}", adt_path.display());
    }
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
        refs.terrain_materials,
        refs.images,
        tex_data,
        ground_images.as_deref(),
    );
    let root = spawn_chunk_entities(refs.commands, refs.meshes, &chunk_materials, adt_data, tile);
    spawn_water(
        refs.commands,
        refs.meshes,
        refs.water_materials,
        refs.images,
        adt_data,
    );
    root
}

/// Spawn entities from a fully-parsed tile (async receive path).
fn spawn_parsed_tile(
    refs: &mut SpawnRefs,
    heightmap: &TerrainHeightmap,
    parsed: &ParsedTile,
) -> (Entity, Vec<Entity>) {
    let tile = AdtTile {
        _tile_x: parsed.tile_x,
        _tile_y: parsed.tile_y,
    };
    let root = spawn_terrain_chunks(
        refs,
        &parsed.adt_path,
        &parsed.adt_data,
        parsed.tex_data.as_ref(),
        &tile,
    );
    let doodad_entities = if let Some(ref obj_data) = parsed.obj_data {
        terrain_objects::spawn_obj_entities(
            refs.commands,
            refs.meshes,
            refs.materials,
            refs.effect_materials,
            refs.images,
            refs.inverse_bp,
            Some(heightmap),
            obj_data,
        )
        .all_entities()
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

fn compute_spawn_result(
    adt_data: &adt::AdtData,
    obj_data: Option<&adt_obj::AdtObjData>,
) -> (Transform, Vec3) {
    let spawn = choose_safe_spawn_position(adt_data, obj_data)
        .unwrap_or_else(|| adt_data.center_surface.into());
    let focus = spawn + Vec3::Y * 1.8;
    let eye = focus + Vec3::new(0.0, 28.0, 18.0);
    let camera = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
    (camera, spawn)
}

fn choose_safe_spawn_position(
    adt_data: &adt::AdtData,
    obj_data: Option<&adt_obj::AdtObjData>,
) -> Option<Vec3> {
    let tile_center = Vec2::new(adt_data.center_surface[0], adt_data.center_surface[2]);
    adt_data
        .height_grids
        .iter()
        .enumerate()
        .filter(|(i, _)| !chunk_has_water(adt_data, *i))
        .filter_map(|(_, grid)| {
            let center = chunk_center_position(grid)?;
            let relief = spawn_patch_relief(grid, center)?;
            let dist = Vec2::new(center.x, center.z).distance(tile_center) / adt::CHUNK_SIZE;
            let occupancy_penalty = spawn_occupancy_penalty(center, obj_data);
            Some((spawn_score(relief, dist, occupancy_penalty), center))
        })
        .min_by(|(score_a, _), (score_b, _)| score_a.total_cmp(score_b))
        .map(|(_, center)| center)
}

fn chunk_has_water(adt_data: &adt::AdtData, index: usize) -> bool {
    adt_data
        .water
        .as_ref()
        .and_then(|water| water.chunks.get(index))
        .is_some_and(|chunk| {
            chunk
                .layers
                .iter()
                .any(|layer| layer.width > 0 && layer.height > 0)
        })
}

fn chunk_center_position(grid: &adt::ChunkHeightGrid) -> Option<Vec3> {
    let x = grid.origin_x - adt::CHUNK_SIZE / 2.0;
    let z = grid.origin_z + adt::CHUNK_SIZE / 2.0;
    let y = sample_chunk_height(grid, x, z)?;
    Some(Vec3::new(x, y, z))
}

fn spawn_patch_relief(grid: &adt::ChunkHeightGrid, center: Vec3) -> Option<f32> {
    let sample_radius = adt::UNIT_SIZE;
    let mut min_height = f32::INFINITY;
    let mut max_height = f32::NEG_INFINITY;
    let mut sampled = 0usize;
    for (dx, dz) in [
        (0.0, 0.0),
        (-sample_radius, 0.0),
        (sample_radius, 0.0),
        (0.0, -sample_radius),
        (0.0, sample_radius),
    ] {
        let height = sample_chunk_height(grid, center.x + dx, center.z + dz)?;
        min_height = min_height.min(height);
        max_height = max_height.max(height);
        sampled += 1;
    }
    (sampled > 0).then_some(max_height - min_height)
}

fn spawn_occupancy_penalty(center: Vec3, obj_data: Option<&adt_obj::AdtObjData>) -> f32 {
    let Some(obj_data) = obj_data else {
        return 0.0;
    };
    let candidate = Vec2::new(center.x, center.z);
    let mut penalty = 0.0;

    for wmo in &obj_data.wmos {
        let distance = candidate.distance(world_position_2d(wmo.position));
        if distance < adt::CHUNK_SIZE * 0.75 {
            penalty += 1_000.0;
        }
    }

    let doodads_nearby = obj_data
        .doodads
        .iter()
        .filter(|doodad| {
            candidate.distance(world_position_2d(doodad.position)) < adt::UNIT_SIZE * 3.0
        })
        .count() as f32;
    penalty + doodads_nearby.min(12.0)
}

fn world_position_2d(wow_position: [f32; 3]) -> Vec2 {
    let [x, _, z] =
        crate::asset::m2::wow_to_bevy(wow_position[0], wow_position[1], wow_position[2]);
    Vec2::new(x, z)
}

fn spawn_score(relief: f32, dist_from_center_chunks: f32, occupancy_penalty: f32) -> f32 {
    relief * 10.0 + dist_from_center_chunks + occupancy_penalty
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_grid(
        index_x: u32,
        index_y: u32,
        origin_x: f32,
        origin_z: f32,
        height: f32,
    ) -> adt::ChunkHeightGrid {
        adt::ChunkHeightGrid {
            index_x,
            index_y,
            origin_x,
            origin_z,
            base_y: height,
            heights: [0.0; 145],
        }
    }

    fn rough_grid(
        index_x: u32,
        index_y: u32,
        origin_x: f32,
        origin_z: f32,
        base_y: f32,
        peak_delta: f32,
    ) -> adt::ChunkHeightGrid {
        let mut heights = [0.0; 145];
        heights[adt::vertex_index(8, 4)] = peak_delta;
        heights[adt::vertex_index(8, 5)] = peak_delta;
        heights[adt::vertex_index(9, 4)] = peak_delta;
        heights[adt::vertex_index(10, 4)] = peak_delta;
        heights[adt::vertex_index(10, 5)] = peak_delta;
        adt::ChunkHeightGrid {
            index_x,
            index_y,
            origin_x,
            origin_z,
            base_y,
            heights,
        }
    }

    fn empty_adt(height_grids: Vec<adt::ChunkHeightGrid>) -> adt::AdtData {
        adt::AdtData {
            chunks: Vec::new(),
            height_grids,
            center_surface: [0.0, 0.0, 0.0],
            chunk_positions: Vec::new(),
            water: None,
            water_error: None,
        }
    }

    #[test]
    fn choose_safe_spawn_position_prefers_flat_chunk_over_rough_center() {
        let adt = empty_adt(vec![
            rough_grid(8, 8, 0.0, 0.0, 40.0, 24.0),
            flat_grid(8, 7, 0.0, -adt::CHUNK_SIZE, 12.0),
        ]);

        let spawn = choose_safe_spawn_position(&adt, None).expect("spawn position");

        assert!(
            spawn.z < 0.0,
            "expected flatter chunk north of center to win"
        );
        assert!((spawn.y - 12.0).abs() < 0.01, "expected flat chunk height");
    }

    #[test]
    fn choose_safe_spawn_position_skips_water_chunks() {
        let mut adt = empty_adt(vec![
            flat_grid(8, 8, 0.0, 0.0, 8.0),
            flat_grid(8, 7, 0.0, -adt::CHUNK_SIZE, 12.0),
        ]);
        adt.water = Some(crate::asset::adt_tex::AdtWaterData {
            chunks: (0..256)
                .map(|i| crate::asset::adt_tex::ChunkWater {
                    layers: if i == 0 {
                        vec![crate::asset::adt_tex::WaterLayer {
                            liquid_type: 0,
                            liquid_object: 0,
                            min_height: 0.0,
                            max_height: 0.0,
                            x_offset: 0,
                            y_offset: 0,
                            width: 8,
                            height: 8,
                            exists: [0; 8],
                            vertex_heights: Vec::new(),
                        }]
                    } else {
                        Vec::new()
                    },
                })
                .collect(),
        });

        let spawn = choose_safe_spawn_position(&adt, None).expect("spawn position");

        assert!(spawn.z < 0.0, "expected non-water chunk to win");
        assert!((spawn.y - 12.0).abs() < 0.01, "expected dry chunk height");
    }

    #[test]
    fn choose_safe_spawn_position_avoids_nearby_wmo_chunk() {
        let adt = empty_adt(vec![
            flat_grid(8, 8, 0.0, 0.0, 8.0),
            flat_grid(8, 7, 0.0, -adt::CHUNK_SIZE, 12.0),
        ]);
        let obj_data = adt_obj::AdtObjData {
            doodads: Vec::new(),
            wmos: vec![adt_obj::WmoPlacement {
                name_id: 0,
                unique_id: 0,
                position: [-adt::CHUNK_SIZE / 2.0, -adt::CHUNK_SIZE / 2.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                flags: 0,
                doodad_set: 0,
                name_set: 0,
                scale: 1.0,
                fdid: None,
                path: None,
            }],
        };

        let spawn = choose_safe_spawn_position(&adt, Some(&obj_data)).expect("spawn position");

        assert!(
            spawn.z < 0.0,
            "expected spawn to move away from occupied center chunk"
        );
        assert!(
            (spawn.y - 12.0).abs() < 0.01,
            "expected alternate flat chunk height"
        );
    }

    #[test]
    fn bootstrap_terrain_streaming_uses_local_player_tile_when_server_did_not_seed_it() {
        use bevy::ecs::system::RunSystemOnce;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<AdtManager>();
        app.world_mut().spawn((
            crate::camera::Player,
            Transform::from_xyz(-8912.9, 80.2, 207.8),
        ));

        let _ = app.world_mut().run_system_once(bootstrap_terrain_streaming);

        let adt_manager = app.world().resource::<AdtManager>();
        assert_eq!(adt_manager.map_name, "azeroth");
        assert_eq!(adt_manager.initial_tile, (32, 48));
        assert_eq!(adt_manager.server_requested.len(), 9);
        assert!(adt_manager.server_requested.contains(&(32, 48)));
    }
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
                    bootstrap_terrain_streaming,
                    adt_streaming_system,
                    receive_loaded_tiles,
                    doodad_lod_swap_system,
                )
                    .chain()
                    .run_if(in_state(GameState::InWorld)),
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
#[allow(clippy::too_many_arguments)]
fn receive_loaded_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
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
        effect_materials: &mut effect_materials,
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
    heightmap.insert_tile(parsed.tile_y, parsed.tile_x, &parsed.adt_data);
    let (root, doodad_entities) = spawn_parsed_tile(refs, heightmap, &parsed);
    adt_manager.loaded.insert(key, root);
    adt_manager.tile_lod.insert(key, parsed.lod);
    adt_manager
        .tile_doodad_entities
        .insert(key, doodad_entities);
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
            return TileLoadResult::Failed {
                tile_y,
                tile_x,
                error: e,
            };
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
