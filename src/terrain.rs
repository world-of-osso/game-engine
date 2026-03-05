use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};

use bevy::image::Image;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::asset::adt::{self, ChunkHeightGrid, CHUNK_SIZE, UNIT_SIZE, vertex_index};
use crate::asset::{adt_obj, blp, wmo};
use crate::terrain_material::{self, TerrainMaterial};
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

/// Parsed ADT data ready to be spawned on the main thread.
struct ParsedTile {
    tile_y: u32,
    tile_x: u32,
    adt_path: PathBuf,
    adt_data: adt::AdtData,
    tex_data: Option<adt::AdtTexData>,
    obj_data: Option<adt_obj::AdtObjData>,
}

/// Result from a background tile load task.
enum TileLoadResult {
    Success(ParsedTile),
    Failed { tile_y: u32, tile_x: u32, error: String },
}

/// Manages multi-tile ADT streaming around the player.
#[derive(Resource)]
pub struct AdtManager {
    /// Map name extracted from the initial ADT (e.g., "azeroth").
    pub map_name: String,
    /// Currently loaded tiles: (row, col) → root entity.
    pub loaded: HashMap<(u32, u32), Entity>,
    /// Tiles that failed to load (missing files); don't retry.
    pub failed: HashSet<(u32, u32)>,
    /// Tiles currently being loaded in background threads.
    pending: HashSet<(u32, u32)>,
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
            tile_rx: Mutex::new(tile_rx),
            tile_tx,
            load_radius: 1,
            initial_tile: (0, 0),
        }
    }
}

/// Queryable heightmap for terrain collision across multiple tiles.
#[derive(Resource, Default)]
pub struct TerrainHeightmap {
    /// Per-tile grids: (tile_y, tile_x) → 256 chunk height grids.
    tiles: HashMap<(u32, u32), Vec<Option<ChunkHeightGrid>>>,
}

impl TerrainHeightmap {
    /// Add height grids from one ADT tile.
    pub fn insert_tile(&mut self, tile_y: u32, tile_x: u32, adt_data: &adt::AdtData) {
        let mut grids: Vec<Option<ChunkHeightGrid>> = vec![None; 256];
        for g in &adt_data.height_grids {
            let idx = (g.index_y * 16 + g.index_x) as usize;
            if idx < 256 {
                grids[idx] = Some(g.clone());
            }
        }
        self.tiles.insert((tile_y, tile_x), grids);
    }

    /// Get all loaded tile coordinate keys.
    pub fn tile_keys(&self) -> impl Iterator<Item = &(u32, u32)> {
        self.tiles.keys()
    }

    /// Get chunk grids for a specific tile.
    pub fn tile_chunks(&self, tile_y: u32, tile_x: u32) -> Option<&Vec<Option<ChunkHeightGrid>>> {
        self.tiles.get(&(tile_y, tile_x))
    }

    /// Remove height grids for a tile.
    pub fn remove_tile(&mut self, tile_y: u32, tile_x: u32) {
        self.tiles.remove(&(tile_y, tile_x));
    }

    /// Look up terrain height at a Bevy-space (x, z) position across all loaded tiles.
    pub fn height_at(&self, bx: f32, bz: f32) -> Option<f32> {
        self.tiles.values()
            .flat_map(|grids| grids.iter().flatten())
            .find_map(|g| chunk_height_at(g, bx, bz))
    }
}

/// Try to get height from a single chunk. Returns None if (bx, bz) is outside this chunk.
fn chunk_height_at(g: &ChunkHeightGrid, bx: f32, bz: f32) -> Option<f32> {
    // Terrain grows in -X from origin_x, +Z from origin_z
    let local_x = g.origin_x - bx;
    let local_z = bz - g.origin_z;
    if local_x < 0.0 || local_x >= CHUNK_SIZE || local_z < 0.0 || local_z >= CHUNK_SIZE {
        return None;
    }
    let col = (local_x / UNIT_SIZE).floor() as usize;
    let row = (local_z / UNIT_SIZE).floor() as usize;
    let col = col.min(7);
    let row = row.min(7);
    let frac_x = (local_x - col as f32 * UNIT_SIZE) / UNIT_SIZE;
    let frac_z = (local_z - row as f32 * UNIT_SIZE) / UNIT_SIZE;
    Some(interpolate_quad_height(g, row, col, frac_x, frac_z))
}

/// Interpolate height within a quad using the 4-triangle fan from center vertex.
fn interpolate_quad_height(
    g: &ChunkHeightGrid,
    row: usize,
    col: usize,
    fx: f32,
    fz: f32,
) -> f32 {
    let h = |idx: usize| g.base_y + g.heights[idx];
    let tl = h(vertex_index(row * 2, col));
    let tr = h(vertex_index(row * 2, col + 1));
    let bl = h(vertex_index(row * 2 + 2, col));
    let br = h(vertex_index(row * 2 + 2, col + 1));
    let center = h(vertex_index(row * 2 + 1, col));

    // Determine which triangle: compare distance from center (0.5, 0.5)
    let dx = fx - 0.5;
    let dz = fz - 0.5;
    let (ha, hb, ax, az, bxx, bz) = if dz.abs() >= dx.abs() {
        if dz < 0.0 {
            // Top triangle: TL(0,0), TR(1,0), C(0.5,0.5)
            (tl, tr, 0.0, 0.0, 1.0, 0.0)
        } else {
            // Bottom triangle: BR(1,1), BL(0,1), C(0.5,0.5)
            (br, bl, 1.0, 1.0, 0.0, 1.0)
        }
    } else if dx > 0.0 {
        // Right triangle: TR(1,0), BR(1,1), C(0.5,0.5)
        (tr, br, 1.0, 0.0, 1.0, 1.0)
    } else {
        // Left triangle: BL(0,1), TL(0,0), C(0.5,0.5)
        (bl, tl, 0.0, 1.0, 0.0, 0.0)
    };
    barycentric_height(fx, fz, ax, az, ha, bxx, bz, hb, 0.5, 0.5, center)
}

/// Barycentric interpolation of height at (px, pz) within triangle (A, B, C).
fn barycentric_height(
    px: f32, pz: f32,
    ax: f32, az: f32, ha: f32,
    bx: f32, bz: f32, hb: f32,
    cx: f32, cz: f32, hc: f32,
) -> f32 {
    let det = (bz - cz) * (ax - cx) + (cx - bx) * (az - cz);
    if det.abs() < 1e-10 {
        return (ha + hb + hc) / 3.0;
    }
    let wa = ((bz - cz) * (px - cx) + (cx - bx) * (pz - cz)) / det;
    let wb = ((cz - az) * (px - cx) + (ax - cx) * (pz - cz)) / det;
    let wc = 1.0 - wa - wb;
    wa * ha + wb * hb + wc * hc
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
        commands, meshes, materials, terrain_materials, water_materials,
        images, inverse_bp, adt_path, &adt_data, &tile,
    );

    heightmap.insert_tile(tile_y, tile_x, &adt_data);
    log_adt_spawn(&adt_data, adt_path);

    let spawn_result = compute_spawn_result(&adt_data);
    Ok(AdtSpawnResult {
        camera: spawn_result.0,
        center: spawn_result.1,
        root_entity: root,
        tile_y, tile_x, map_name,
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
    let obj_data = load_obj0(adt_path);
    let root = spawn_from_parsed(
        commands, meshes, materials, terrain_materials, water_materials,
        images, inverse_bp, adt_path, adt_data, tex_data.as_ref(), tile,
    );
    if let Some(ref obj) = obj_data {
        spawn_obj0_doodads(commands, meshes, materials, images, inverse_bp, obj);
        spawn_obj0_wmos(commands, meshes, materials, images, obj);
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
    let ground_images = tex_data
        .map(|td| terrain_material::load_ground_images(images, td, adt_path));
    let chunk_materials = terrain_material::build_terrain_materials(
        terrain_materials, images, tex_data, ground_images.as_deref(),
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
) -> Entity {
    let tile = AdtTile { tile_x: parsed.tile_x, tile_y: parsed.tile_y };
    let root = spawn_from_parsed(
        commands, meshes, materials, terrain_materials, water_materials,
        images, inverse_bp, &parsed.adt_path, &parsed.adt_data,
        parsed.tex_data.as_ref(), &tile,
    );
    // Spawn doodads/WMOs from pre-parsed obj0 data
    if let Some(ref obj_data) = parsed.obj_data {
        spawn_obj0_doodads(commands, meshes, materials, images, inverse_bp, obj_data);
        spawn_obj0_wmos(commands, meshes, materials, images, obj_data);
    }
    root
}

/// Log a summary of a spawned ADT tile.
fn log_adt_spawn(adt_data: &adt::AdtData, adt_path: &Path) {
    let water_count = adt_data.water.as_ref()
        .map_or(0, |w| w.chunks.iter().filter(|c| !c.layers.is_empty()).count());
    eprintln!(
        "Spawned ADT terrain: {} chunks, {} water chunks from {}",
        adt_data.chunks.len(), water_count, adt_path.display(),
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
    Err(format!("ADT tile files not found: {} or {}", local.display(), fdid_path.display()))
}

/// Parse map name and tile coordinates from an ADT filename.
///
/// Supports both `mapname_Y_X.adt` and FDID-based `778027.adt` (via listfile reverse lookup).
fn parse_tile_coords_from_path(adt_path: &Path) -> Result<(String, u32, u32), String> {
    let stem = adt_path.file_stem()
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
    if parts.len() < 3 { return None; }
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
fn resolve_companion_path(adt_path: &Path, suffix: &str) -> Option<PathBuf> {
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
    if companion_path.exists() { Some(companion_path) } else { None }
}

/// Try to load the companion _tex0.adt file.
fn load_tex0(adt_path: &Path) -> Option<adt::AdtTexData> {
    let tex0_path = resolve_companion_path(adt_path, "_tex0")?;
    let data = std::fs::read(&tex0_path).ok()?;
    match adt::load_adt_tex0(&data) {
        Ok(td) => {
            eprintln!("Loaded _tex0: {} textures, {} chunks", td.texture_fdids.len(), td.chunk_layers.len());
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
        .spawn((AdtTerrain, tile.clone(), Transform::default(), Visibility::default()))
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
            if layer.width == 0 || layer.height == 0 { continue; }
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
    let Some(ref water_data) = adt_data.water else { return };
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

// ── _obj0.adt object spawning ────────────────────────────────────────────────

/// Try to load the companion _obj0.adt file.
fn load_obj0(adt_path: &Path) -> Option<adt_obj::AdtObjData> {
    let obj0_path = resolve_companion_path(adt_path, "_obj0")?;
    let data = std::fs::read(&obj0_path).ok()?;
    match adt_obj::load_adt_obj0(&data) {
        Ok(obj) => Some(obj),
        Err(e) => {
            eprintln!("Failed to parse _obj0: {e}");
            None
        }
    }
}


/// Spawn doodads (M2 models) from _obj0 placement data.
fn spawn_obj0_doodads(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    obj_data: &adt_obj::AdtObjData,
) {
    let mut spawned = 0u32;
    for doodad in &obj_data.doodads {
        if try_spawn_doodad(commands, meshes, materials, images, inverse_bp, doodad) {
            spawned += 1;
        }
    }
    eprintln!("Spawned {spawned}/{} doodads", obj_data.doodads.len());
}

/// Try to spawn a single doodad. Returns true if the M2 was found and spawned.
fn try_spawn_doodad(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bp: &mut Assets<SkinnedMeshInverseBindposes>,
    doodad: &adt_obj::DoodadPlacement,
) -> bool {
    let Some(m2_path) = resolve_doodad_m2(doodad) else { return false };
    if !m2_path.exists() {
        return false;
    }
    let transform = doodad_transform(doodad);
    let Some(entity) = super::spawn_static_m2(commands, meshes, materials, images, inverse_bp, &m2_path, transform) else {
        return false;
    };
    commands.entity(entity).insert(game_engine::culling::Doodad);
    true
}

/// Resolve a doodad placement to a local M2 file path.
fn resolve_doodad_m2(doodad: &adt_obj::DoodadPlacement) -> Option<std::path::PathBuf> {
    // If we have a direct FDID, look it up in the listfile for the path.
    if let Some(fdid) = doodad.fdid {
        return Some(std::path::PathBuf::from(format!("data/models/{fdid}.m2")));
    }
    // Otherwise resolve the WoW path to an FDID via listfile.
    let wow_path = doodad.path.as_ref()?;
    let fdid = game_engine::listfile::lookup_path(wow_path)?;
    Some(std::path::PathBuf::from(format!("data/models/{fdid}.m2")))
}

/// Convert WoW doodad placement (position + Euler degrees + scale) to a Bevy Transform.
fn doodad_transform(d: &adt_obj::DoodadPlacement) -> Transform {
    let pos = placement_to_bevy(d.position);
    let rotation = doodad_rotation(d.rotation);
    Transform::from_translation(Vec3::from(pos))
        .with_rotation(rotation)
        .with_scale(Vec3::splat(d.scale))
}

/// Convert WoW Euler rotation (degrees around Y, X, Z) to a Bevy quaternion.
fn doodad_rotation(rot: [f32; 3]) -> Quat {
    let rx = rot[0].to_radians();
    let ry = rot[1].to_radians();
    let rz = rot[2].to_radians();
    // WoW rotations: Y-up, applied as Y→X→Z. Remap to Bevy coordinate system.
    Quat::from_euler(EulerRot::YXZ, ry, rx, rz)
}

// ── _obj0.adt WMO spawning ──────────────────────────────────────────────────

/// Spawn WMOs from _obj0 placement data.
fn spawn_obj0_wmos(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    obj_data: &adt_obj::AdtObjData,
) {
    let mut spawned = 0u32;
    for placement in &obj_data.wmos {
        if try_spawn_wmo(commands, meshes, materials, images, placement) {
            spawned += 1;
        }
    }
    eprintln!("Spawned {spawned}/{} WMOs", obj_data.wmos.len());
}

/// Try to spawn a single WMO. Returns true if root file was found and spawned.
fn try_spawn_wmo(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    placement: &adt_obj::WmoPlacement,
) -> bool {
    let Some(root_fdid) = resolve_wmo_fdid(placement) else { return false };
    let root_path = format!("data/models/{root_fdid}.wmo");
    let Ok(root_data) = std::fs::read(&root_path) else { return false };
    let Ok(root) = wmo::load_wmo_root(&root_data) else {
        eprintln!("Failed to parse WMO root {root_fdid}");
        return false;
    };

    let group_fdids = resolve_wmo_group_fdids(root_fdid, root.n_groups);
    let transform = wmo_transform(placement);
    let root_entity = commands
        .spawn((
            Name::new(format!("wmo_{root_fdid}")),
            transform,
            Visibility::default(),
            game_engine::culling::Wmo,
        ))
        .id();

    let mut group_count = 0u32;
    for (i, group_fdid) in group_fdids.iter().enumerate() {
        let Some(fdid) = group_fdid else { continue };
        if spawn_wmo_group(commands, meshes, materials, images, &root, *fdid, root_entity) {
            group_count += 1;
        } else {
            eprintln!("  WMO {root_fdid} group {i}: missing or failed (FDID {fdid})");
        }
    }

    let pos = transform.translation;
    eprintln!(
        "WMO {root_fdid}: {group_count}/{} groups, {} materials, pos=[{:.0}, {:.0}, {:.0}]",
        root.n_groups, root.materials.len(), pos.x, pos.y, pos.z,
    );
    group_count > 0
}

/// Resolve a WMO placement to its root FileDataID.
fn resolve_wmo_fdid(wmo: &adt_obj::WmoPlacement) -> Option<u32> {
    if let Some(fdid) = wmo.fdid {
        return Some(fdid);
    }
    let wow_path = wmo.path.as_ref()?;
    game_engine::listfile::lookup_path(wow_path)
}

/// Resolve group file FDIDs by looking up root path in listfile and deriving group paths.
fn resolve_wmo_group_fdids(root_fdid: u32, n_groups: u32) -> Vec<Option<u32>> {
    let Some(root_path) = game_engine::listfile::lookup_fdid(root_fdid) else {
        eprintln!("  WMO {root_fdid}: not in listfile, cannot resolve group FDIDs");
        return vec![None; n_groups as usize];
    };

    let base = root_path.trim_end_matches(".wmo");
    (0..n_groups)
        .map(|i| {
            let group_path = format!("{base}_{i:03}.wmo");
            game_engine::listfile::lookup_path(&group_path)
        })
        .collect()
}

/// Parse and spawn one WMO group file as children of the root entity.
fn spawn_wmo_group(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: &wmo::WmoRootData,
    group_fdid: u32,
    root_entity: Entity,
) -> bool {
    let group_path = format!("data/models/{group_fdid}.wmo");
    let Ok(data) = std::fs::read(&group_path) else { return false };
    let Ok(group) = wmo::load_wmo_group(&data) else {
        eprintln!("Failed to parse WMO group {group_fdid}");
        return false;
    };

    for batch in group.batches {
        let mat = wmo_batch_material(materials, images, root, batch.material_index);
        let child = commands
            .spawn((
                Mesh3d(meshes.add(batch.mesh)),
                MeshMaterial3d(mat),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        commands.entity(root_entity).add_child(child);
    }
    true
}

/// Build a Bevy material for a WMO batch, loading the BLP texture if available.
fn wmo_batch_material(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: &wmo::WmoRootData,
    material_index: u16,
) -> Handle<StandardMaterial> {
    let mat_def = root.materials.get(material_index as usize);
    let texture_fdid = mat_def.map(|m| m.texture_fdid).unwrap_or(0);
    let blend_mode = mat_def.map(|m| m.blend_mode).unwrap_or(0);

    if texture_fdid > 0 {
        let blp_path = format!("data/textures/{texture_fdid}.blp");
        if let Ok((pixels, w, h)) = blp::load_blp_rgba(std::path::Path::new(&blp_path)) {
            let image = Image::new(
                bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                bevy::render::render_resource::TextureDimension::D2,
                pixels,
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::default(),
            );
            return materials.add(wmo_standard_material(Some(images.add(image)), blend_mode));
        }
    }

    // Fallback: gray material
    materials.add(wmo_standard_material(None, blend_mode))
}

fn wmo_standard_material(
    texture: Option<Handle<Image>>,
    blend_mode: u32,
) -> StandardMaterial {
    let alpha_mode = match blend_mode {
        1 => AlphaMode::Mask(0.5),
        2 | 3 => AlphaMode::Blend,
        _ => AlphaMode::Opaque,
    };
    StandardMaterial {
        base_color: if texture.is_none() { Color::srgb(0.6, 0.6, 0.6) } else { Color::WHITE },
        base_color_texture: texture,
        perceptual_roughness: 0.8,
        double_sided: true,
        cull_mode: None,
        alpha_mode,
        ..default()
    }
}

/// Convert MODF/MDDF placement position to Bevy-space.
///
/// MODF/MDDF store positions as `[X_adt, Height, Y_adt]` in a coordinate system where
/// world coords = 32*TILESIZE - adt coords (for X and Y_adt).
/// MCNK headers already store converted world coordinates, but MODF/MDDF do not.
fn placement_to_bevy(raw: [f32; 3]) -> [f32; 3] {
    use super::asset::m2::wow_to_bevy;
    const MAP_OFFSET: f32 = 32.0 * CHUNK_SIZE * 16.0; // 32 * 533.33 = 17066.67
    let wx = MAP_OFFSET - raw[0];
    let wy = MAP_OFFSET - raw[2];
    let wz = raw[1]; // height
    wow_to_bevy(wx, wy, wz)
}

/// Convert WMO placement to a Bevy Transform.
fn wmo_transform(w: &adt_obj::WmoPlacement) -> Transform {
    let pos = placement_to_bevy(w.position);
    let rotation = doodad_rotation(w.rotation);
    Transform::from_translation(Vec3::from(pos))
        .with_rotation(rotation)
        .with_scale(Vec3::splat(w.scale))
}

// ── ADT streaming ────────────────────────────────────────────────────────────

pub struct AdtStreamingPlugin;

impl Plugin for AdtStreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AdtManager>()
            .init_resource::<TerrainHeightmap>()
            .add_systems(Update, (adt_streaming_system, receive_loaded_tiles).chain());
    }
}

/// Dispatch background loads and unload distant tiles.
fn adt_streaming_system(
    mut commands: Commands,
    mut adt_manager: ResMut<AdtManager>,
    mut heightmap: ResMut<TerrainHeightmap>,
    player_q: Query<&Transform, With<crate::camera::Player>>,
) {
    if adt_manager.map_name.is_empty() { return; }
    let Ok(player_tf) = player_q.single() else { return; };

    let (center_y, center_x) = bevy_to_tile_coords(
        player_tf.translation.x, player_tf.translation.z,
    );
    let desired = compute_desired_tiles(center_y, center_x, adt_manager.load_radius);

    unload_distant_tiles(&mut commands, &mut adt_manager, &mut heightmap, &desired);
    dispatch_tile_loads(&mut adt_manager, &desired);
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
            &mut commands, &mut meshes, &mut materials, &mut terrain_mats,
            &mut water_mats, &mut images, &mut inverse_bp,
            &mut adt_manager, &mut heightmap, result,
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
            let root = spawn_parsed_tile(
                commands, meshes, materials, terrain_mats,
                water_mats, images, inverse_bp, &parsed,
            );
            heightmap.insert_tile(parsed.tile_y, parsed.tile_x, &parsed.adt_data);
            adt_manager.loaded.insert(key, root);
            log_adt_spawn(&parsed.adt_data, &parsed.adt_path);
        }
        TileLoadResult::Failed { tile_y, tile_x, error } => {
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
    let to_remove: Vec<(u32, u32)> = adt_manager.loaded.keys()
        .filter(|k| !desired.contains(k))
        .copied()
        .collect();

    for key in to_remove {
        if let Some(root) = adt_manager.loaded.remove(&key) {
            commands.entity(root).despawn();
        }
        heightmap.remove_tile(key.0, key.1);
        eprintln!("Unloaded ADT tile ({}, {})", key.0, key.1);
    }
}

/// Dispatch background thread loads for tiles not yet loaded or pending.
fn dispatch_tile_loads(adt_manager: &mut AdtManager, desired: &[(u32, u32)]) {
    for &(ty, tx) in desired {
        if adt_manager.loaded.contains_key(&(ty, tx)) { continue; }
        if adt_manager.failed.contains(&(ty, tx)) { continue; }
        if adt_manager.pending.contains(&(ty, tx)) { continue; }

        // Resolve path on main thread (needs listfile, which is global state)
        let path = match resolve_tile_path(&adt_manager.map_name, ty, tx) {
            Ok(p) => p,
            Err(e) => {
                adt_manager.failed.insert((ty, tx));
                eprintln!("Cannot load ADT tile ({ty}, {tx}): {e}");
                continue;
            }
        };

        adt_manager.pending.insert((ty, tx));
        let tx_chan = adt_manager.tile_tx.clone();
        std::thread::spawn(move || {
            tx_chan.send(parse_tile_background(ty, tx, path)).ok();
        });
    }
}

/// Parse an ADT tile and its companions on a background thread.
fn parse_tile_background(tile_y: u32, tile_x: u32, adt_path: PathBuf) -> TileLoadResult {
    let adt_data = match load_and_parse_adt(&adt_path) {
        Ok(d) => d,
        Err(e) => return TileLoadResult::Failed { tile_y, tile_x, error: e },
    };
    let tex_data = load_tex0(&adt_path);
    let obj_data = load_obj0(&adt_path);
    TileLoadResult::Success(ParsedTile { tile_y, tile_x, adt_path, adt_data, tex_data, obj_data })
}
