//! Async local-CASC parsing for one native map and its requested terrain tiles.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use super::assets::{NativeMapWdt, NativeTerrainAssets, NativeTerrainTile};

const MAP_TILE_BOUND: u32 = 64;

pub(crate) trait TerrainReader: Send + 'static {
    fn read_map_wdt(&self, map: &str) -> Result<NativeMapWdt, String>;
    fn read_tile(&self, map: &str, tile: (u32, u32)) -> Result<NativeTerrainTile, String>;
}

impl TerrainReader for NativeTerrainAssets {
    fn read_map_wdt(&self, map: &str) -> Result<NativeMapWdt, String> {
        Self::read_map_wdt(self, map)
    }

    fn read_tile(&self, map: &str, tile: (u32, u32)) -> Result<NativeTerrainTile, String> {
        Self::read_tile(self, map, tile.0, tile.1)
    }
}

enum WorkerRequest {
    Map {
        generation: u64,
        map: String,
    },
    Tile {
        generation: u64,
        map: String,
        tile: (u32, u32),
    },
}

enum WorkerResult {
    Map {
        generation: u64,
        result: Result<NativeMapWdt, String>,
    },
    Tile {
        generation: u64,
        tile: (u32, u32),
        result: Result<NativeTerrainTile, String>,
    },
}

pub(crate) struct ParsedTileState {
    pub tile: (u32, u32),
    pub root_path: PathBuf,
    pub tex_path: Option<PathBuf>,
    pub obj_path: Option<PathBuf>,
    pub root_chunks: usize,
    pub root_height_grids: usize,
    pub tex_chunk_layers: usize,
    pub obj_doodads: usize,
    pub obj_wmos: usize,
}

pub(crate) struct TileFailure {
    pub tile: (u32, u32),
    pub error: String,
}

pub(crate) struct TerrainStreamState {
    pub map: Option<String>,
    pub wdt_path: Option<PathBuf>,
    pub wdt_flags: Option<u32>,
    pub global_wmo_fdid: Option<u32>,
    pub global_wmo_present: bool,
    pub pending_map: bool,
    pub pending_tiles: Vec<(u32, u32)>,
    pub parsed_tiles: Vec<ParsedTileState>,
    pub failures: Vec<TileFailure>,
    pub map_error: Option<String>,
}

pub(crate) struct StreamedTerrain {
    requests: Option<Sender<WorkerRequest>>,
    results: Receiver<WorkerResult>,
    worker: Option<JoinHandle<()>>,
    terminal_error: Option<String>,
    generation: u64,
    map: Option<String>,
    initial_tiles: BTreeSet<(u32, u32)>,
    pending_map: bool,
    pub(crate) map_wdt: Option<NativeMapWdt>,
    pub(crate) parsed_tiles: BTreeMap<(u32, u32), NativeTerrainTile>,
    pending_tiles: BTreeSet<(u32, u32)>,
    requested_tiles: BTreeSet<(u32, u32)>,
    failures: BTreeMap<(u32, u32), String>,
    map_error: Option<String>,
}

impl StreamedTerrain {
    /// The tiles the current map request started from: the player's tile first needs them.
    pub fn initial_tiles(&self) -> &BTreeSet<(u32, u32)> {
        &self.initial_tiles
    }

    pub fn new(data_root: PathBuf) -> Self {
        Self::with_reader(NativeTerrainAssets::new(data_root))
    }

    fn with_reader(reader: impl TerrainReader) -> Self {
        let (requests, incoming) = mpsc::channel();
        let (outgoing, results) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("native-terrain-assets".into())
            .spawn(move || run_worker(reader, incoming, outgoing));
        let worker = worker.expect("Cannot spawn native terrain asset worker");
        Self {
            requests: Some(requests),
            results,
            worker: Some(worker),
            terminal_error: None,
            generation: 0,
            map: None,
            initial_tiles: BTreeSet::new(),
            pending_map: false,
            map_wdt: None,
            parsed_tiles: BTreeMap::new(),
            pending_tiles: BTreeSet::new(),
            requested_tiles: BTreeSet::new(),
            failures: BTreeMap::new(),
            map_error: None,
        }
    }

    pub fn map_name(&self) -> Option<&str> {
        self.map.as_deref()
    }

    pub fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        self.parsed_tiles
            .values()
            .flat_map(|tile| &tile.root.height_grids)
            .find_map(|grid| game_engine_core::terrain_height_data::sample_chunk_height(grid, x, z))
    }

    pub fn area_id_at(&self, x: f32, z: f32) -> Option<u32> {
        use game_engine_core::asset::adt_format::adt::CHUNK_SIZE;
        use game_engine_core::terrain_height_data::bevy_to_tile_coords;

        let root = &self.parsed_tiles.get(&bevy_to_tile_coords(x, z))?.root;
        let grid = root.height_grids.iter().find(|grid| {
            (0.0..CHUNK_SIZE).contains(&(grid.origin_x - x))
                && (0.0..CHUNK_SIZE).contains(&(z - grid.origin_z))
        })?;
        root.chunks
            .iter()
            .find(|chunk| chunk.index_x == grid.index_x && chunk.index_y == grid.index_y)
            .map(|chunk| chunk.area_id)
            .filter(|&area_id| area_id != 0)
    }

    pub fn surface_at(
        &self,
        x: f32,
        z: f32,
    ) -> Option<game_engine_core::footstep_data::FootstepSurface> {
        use game_engine_core::asset::adt_format::adt::CHUNK_SIZE;
        use game_engine_core::terrain_height_data::bevy_to_tile_coords;

        let tile = self.parsed_tiles.get(&bevy_to_tile_coords(x, z))?;
        let grid = tile.root.height_grids.iter().find(|grid| {
            (0.0..CHUNK_SIZE).contains(&(grid.origin_x - x))
                && (0.0..CHUNK_SIZE).contains(&(z - grid.origin_z))
        })?;
        tile.chunk_surfaces
            .get(&(grid.index_x, grid.index_y))
            .copied()
    }

    /// Root-wide WMO override at actual world XYZ; Dirt is the legacy missing-terrain policy.
    pub fn surface_at_position(
        &self,
        position: [f32; 3],
    ) -> game_engine_core::footstep_data::FootstepSurface {
        use super::assets::wmo_surface_bounds;
        use game_engine_core::wmo_surface_data::select_footstep_surface;

        let global = self
            .map_wdt
            .iter()
            .filter_map(|map| map.global_wmo.as_ref())
            .filter_map(|wmo| {
                wmo.surface
                    .map(|surface| (wmo_surface_bounds(&wmo.placement, true), surface))
            });
        let streamed = self
            .parsed_tiles
            .values()
            .flat_map(|tile| tile.wmo_surfaces.iter().copied());
        select_footstep_surface(
            position,
            self.surface_at(position[0], position[2]),
            global.chain(streamed),
        )
    }

    pub fn water_surface_at(&self, x: f32, z: f32) -> Option<f32> {
        use game_engine_core::terrain_height_data::{
            WaterLayerSurface, bevy_to_tile_coords, layer_has_water, sample_water_layer_height,
        };

        let tile = self.parsed_tiles.get(&bevy_to_tile_coords(x, z))?;
        let water = tile.root.water.as_ref()?;
        water
            .chunks
            .iter()
            .enumerate()
            .filter_map(|(index, chunk)| {
                tile.root
                    .chunk_positions
                    .get(index)
                    .map(|position| (chunk, *position))
            })
            .flat_map(|(chunk, position)| {
                chunk.layers.iter().filter_map(move |layer| {
                    if !layer_has_water(layer) {
                        return None;
                    }
                    let surface = WaterLayerSurface::from_layer(layer, position);
                    sample_water_layer_height(&surface, x, z)
                })
            })
            .max_by(f32::total_cmp)
    }

    pub fn request_map(&mut self, map: String, tile: (u32, u32)) -> Result<(), String> {
        validate_tile(tile)?;
        if self.map.as_ref() != Some(&map) {
            return self.begin_map(map, square_tiles(tile).collect());
        }
        self.request_tile(tile)
    }

    pub fn request_map_tiles(
        &mut self,
        map: String,
        primary: (u32, u32),
        tiles: &[(u32, u32)],
    ) -> Result<(), String> {
        validate_tile(primary)?;
        for &tile in tiles {
            validate_tile(tile)?;
        }
        let initial_tiles = tiles.iter().copied().chain([primary]).collect();
        if self.map.as_ref() != Some(&map) {
            return self.begin_map(map, initial_tiles);
        }
        for tile in initial_tiles {
            self.request_tile(tile)?;
        }
        Ok(())
    }

    fn begin_map(
        &mut self,
        map: String,
        initial_tiles: BTreeSet<(u32, u32)>,
    ) -> Result<(), String> {
        self.reset()?;
        self.map = Some(map.clone());
        self.initial_tiles = initial_tiles;
        self.pending_map = true;
        self.send(WorkerRequest::Map {
            generation: self.generation,
            map,
        })
    }

    fn request_tile(&mut self, tile: (u32, u32)) -> Result<(), String> {
        if self.initial_tiles.contains(&tile) || !self.requested_tiles.insert(tile) {
            return Ok(());
        }
        // A global-WMO map has no ADT tiles.
        if self
            .map_wdt
            .as_ref()
            .is_some_and(|wdt| wdt.global_wmo.is_none())
        {
            self.queue_tile(tile)?;
        }
        Ok(())
    }

    pub fn poll(&mut self) -> Result<(), String> {
        if let Some(error) = &self.terminal_error {
            return Err(error.clone());
        }
        loop {
            match self.results.try_recv() {
                Ok(result) => self.accept_result(result)?,
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    let worker = self.worker.take().expect("worker exists while polling");
                    let error = match worker.join() {
                        Ok(()) => "Native terrain worker exited unexpectedly".into(),
                        Err(panic) => {
                            format!("Native terrain worker panicked: {}", panic_message(panic))
                        }
                    };
                    self.terminal_error = Some(error.clone());
                    return Err(error);
                }
            }
        }
    }

    pub fn reset(&mut self) -> Result<(), String> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or("Terrain generation overflow")?;
        self.map = None;
        self.initial_tiles.clear();
        self.pending_map = false;
        self.map_wdt = None;
        self.parsed_tiles.clear();
        self.pending_tiles.clear();
        self.requested_tiles.clear();
        self.failures.clear();
        self.map_error = None;
        Ok(())
    }

    /// Tiles whose files failed to read or parse, keyed `(tile_y, tile_x)`.
    pub fn failures(&self) -> impl Iterator<Item = (&(u32, u32), &String)> {
        self.failures.iter()
    }

    pub fn map_error(&self) -> Option<&str> {
        self.map_error.as_deref()
    }

    pub fn state(&self) -> TerrainStreamState {
        TerrainStreamState {
            map: self.map.clone(),
            wdt_path: self.map_wdt.as_ref().map(|wdt| wdt.path.clone()),
            wdt_flags: self.map_wdt.as_ref().map(|wdt| wdt.flags.raw),
            global_wmo_fdid: self
                .map_wdt
                .as_ref()
                .and_then(|wdt| wdt.global_wmo.as_ref())
                .and_then(|wmo| wmo.placement.fdid),
            global_wmo_present: self
                .map_wdt
                .as_ref()
                .is_some_and(|wdt| wdt.global_wmo.is_some()),
            pending_map: self.pending_map,
            pending_tiles: self.pending_tiles.iter().copied().collect(),
            parsed_tiles: self
                .parsed_tiles
                .iter()
                .map(|(&tile, parsed)| parsed_tile_state(tile, parsed))
                .collect(),
            failures: self
                .failures
                .iter()
                .map(|(&tile, error)| TileFailure {
                    tile,
                    error: error.clone(),
                })
                .collect(),
            map_error: self.map_error.clone(),
        }
    }

    fn send(&self, request: WorkerRequest) -> Result<(), String> {
        self.requests
            .as_ref()
            .expect("worker request channel exists")
            .send(request)
            .map_err(|_| "Native terrain worker is unavailable".into())
    }

    fn queue_tile(&mut self, tile: (u32, u32)) -> Result<(), String> {
        if self.pending_tiles.contains(&tile)
            || self.parsed_tiles.contains_key(&tile)
            || self.failures.contains_key(&tile)
        {
            return Ok(());
        }
        let map = self
            .map
            .as_ref()
            .expect("map established before tile request")
            .clone();
        self.send(WorkerRequest::Tile {
            generation: self.generation,
            map,
            tile,
        })?;
        self.pending_tiles.insert(tile);
        Ok(())
    }

    fn accept_result(&mut self, result: WorkerResult) -> Result<(), String> {
        match result {
            WorkerResult::Map { generation, result } if generation == self.generation => {
                self.pending_map = false;
                match result {
                    Ok(wdt) => {
                        let global_wmo = wdt.global_wmo.is_some();
                        self.map_wdt = Some(wdt);
                        if !global_wmo {
                            let tiles: Vec<_> = self
                                .initial_tiles
                                .union(&self.requested_tiles)
                                .copied()
                                .collect();
                            for tile in tiles {
                                self.queue_tile(tile)?;
                            }
                        }
                    }
                    Err(error) => self.map_error = Some(error),
                }
            }
            WorkerResult::Tile {
                generation,
                tile,
                result,
            } if generation == self.generation => {
                self.pending_tiles.remove(&tile);
                match result {
                    Ok(parsed) => {
                        self.parsed_tiles.insert(tile, parsed);
                    }
                    Err(error) => {
                        self.failures.insert(tile, error);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl Drop for StreamedTerrain {
    fn drop(&mut self) {
        self.requests.take();
        if let Some(worker) = self.worker.take() {
            if let Err(panic) = worker.join() {
                eprintln!("Native terrain worker panicked: {}", panic_message(panic));
            }
        }
    }
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&str>() {
        return (*message).into();
    }
    "unknown panic".into()
}

fn run_worker(
    reader: impl TerrainReader,
    requests: Receiver<WorkerRequest>,
    results: Sender<WorkerResult>,
) {
    for request in requests {
        let result = match request {
            WorkerRequest::Map { generation, map } => WorkerResult::Map {
                generation,
                result: reader.read_map_wdt(&map),
            },
            WorkerRequest::Tile {
                generation,
                map,
                tile,
            } => WorkerResult::Tile {
                generation,
                tile,
                result: reader.read_tile(&map, tile),
            },
        };
        if results.send(result).is_err() {
            return;
        }
    }
}

fn validate_tile(tile: (u32, u32)) -> Result<(), String> {
    if tile.0 >= MAP_TILE_BOUND || tile.1 >= MAP_TILE_BOUND {
        return Err(format!(
            "Terrain tile ({}, {}) outside 0..64",
            tile.0, tile.1
        ));
    }
    Ok(())
}

fn square_tiles(center: (u32, u32)) -> impl Iterator<Item = (u32, u32)> {
    let start_y = center.0.saturating_sub(1);
    let start_x = center.1.saturating_sub(1);
    let end_y = (center.0 + 1).min(MAP_TILE_BOUND - 1);
    let end_x = (center.1 + 1).min(MAP_TILE_BOUND - 1);
    (start_y..=end_y).flat_map(move |y| (start_x..=end_x).map(move |x| (y, x)))
}

fn parsed_tile_state(tile: (u32, u32), parsed: &NativeTerrainTile) -> ParsedTileState {
    ParsedTileState {
        tile,
        root_path: parsed.root_path.clone(),
        tex_path: parsed.tex_path.clone(),
        obj_path: parsed.obj_path.clone(),
        root_chunks: parsed.root.chunks.len(),
        root_height_grids: parsed.root.height_grids.len(),
        tex_chunk_layers: parsed.tex.as_ref().map_or(0, |tex| tex.chunk_layers.len()),
        obj_doodads: parsed.obj.as_ref().map_or(0, |obj| obj.doodads.len()),
        obj_wmos: parsed.obj.as_ref().map_or(0, |obj| obj.wmos.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, mpsc};
    use std::time::{Duration, Instant};

    fn cached_assets() -> NativeTerrainAssets {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        NativeTerrainAssets::new(data_root)
    }

    struct ControlledReader {
        assets: NativeTerrainAssets,
        started: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl TerrainReader for ControlledReader {
        fn read_map_wdt(&self, map: &str) -> Result<NativeMapWdt, String> {
            if map == "azeroth" {
                self.started.send(()).expect("start notification");
                self.release.lock().unwrap().recv().expect("release read");
            }
            self.assets.read_map_wdt(map)
        }

        fn read_tile(&self, map: &str, tile: (u32, u32)) -> Result<NativeTerrainTile, String> {
            if tile == (32, 48) {
                self.assets.read_tile(map, tile.0, tile.1)
            } else {
                Err(format!("missing tile ({}, {})", tile.0, tile.1))
            }
        }
    }

    fn wait_for(stream: &mut StreamedTerrain, completed: impl Fn(&TerrainStreamState) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            stream.poll().expect("worker alive");
            if completed(&stream.state()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("stream timed out");
    }

    fn water_tile(
        layers: Vec<game_engine_core::asset::adt_format::adt_tex::WaterLayer>,
    ) -> NativeTerrainTile {
        use game_engine_core::adt::Root;
        use game_engine_core::asset::adt_format::adt_tex::{AdtWaterData, ChunkWater};

        NativeTerrainTile {
            root_path: PathBuf::new(),
            tex_path: None,
            obj_path: None,
            root: Root {
                chunks: Vec::new(),
                height_grids: Vec::new(),
                center_surface: [0.0; 3],
                chunk_positions: vec![[0.0; 3]],
                blend_mesh: None,
                flight_bounds: None,
                water: Some(AdtWaterData {
                    chunks: vec![ChunkWater {
                        layers,
                        attributes: None,
                    }],
                }),
                water_error: None,
            },
            tex: None,
            obj: None,
            textures: BTreeMap::new(),
            chunk_surfaces: BTreeMap::new(),
            wmo_floors: Vec::new(),
            wmo_surfaces: Vec::new(),
            liquid_materials: BTreeMap::new(),
        }
    }

    fn flat_water(
        height: f32,
        exists: u8,
    ) -> game_engine_core::asset::adt_format::adt_tex::WaterLayer {
        game_engine_core::asset::adt_format::adt_tex::WaterLayer {
            liquid_type: 0,
            liquid_object: 0,
            min_height: height,
            max_height: height,
            x_offset: 0,
            y_offset: 0,
            width: 1,
            height: 1,
            exists: [exists, 0, 0, 0, 0, 0, 0, 0],
            vertex_heights: Vec::new(),
            vertex_uvs: Vec::new(),
            vertex_depths: Vec::new(),
            object_vertex_bytes: Vec::new(),
        }
    }

    fn area_fixture() -> NativeTerrainTile {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/terrain/azeroth_32_48.adt");
        let bytes = std::fs::read(&path).expect("local ADT fixture");
        let root = game_engine_core::adt::parse_root_for_tile(&bytes, 32, 48, None)
            .expect("parsed ADT fixture");
        let mut tile = water_tile(Vec::new());
        tile.root = root;
        tile
    }

    #[test]
    fn surface_query_matches_adjacent_authored_chunks_and_reset() {
        use game_engine_core::asset::adt_format::adt::CHUNK_SIZE;
        use game_engine_core::footstep_data::FootstepSurface;

        let mut stream = StreamedTerrain::with_reader(cached_assets());
        let mut tile = area_fixture();
        let first = tile
            .root
            .height_grids
            .iter()
            .find(|grid| (grid.index_x, grid.index_y) == (0, 0))
            .unwrap();
        let (x, z) = (first.origin_x, first.origin_z);
        tile.chunk_surfaces.insert((0, 0), FootstepSurface::Grass);
        tile.chunk_surfaces.insert((0, 1), FootstepSurface::Stone);
        assert_eq!(stream.surface_at(x - 1.0, z + 1.0), None);
        stream.parsed_tiles.insert((32, 48), tile);
        assert_eq!(stream.surface_at(x, z + 1.0), Some(FootstepSurface::Grass));
        let adjacent = tile_chunk_origin_x(&stream.parsed_tiles[&(32, 48)], 0, 1) - 1.0;
        assert_eq!(
            stream.surface_at(adjacent, z + 1.0),
            Some(FootstepSurface::Stone)
        );
        assert_eq!(stream.surface_at(x - 2.0 * CHUNK_SIZE - 1.0, z + 1.0), None);
        assert_eq!(stream.surface_at(x + 1.0, z + 1.0), None);
        assert_eq!(stream.surface_at(f32::NAN, z), None);
        stream.reset().unwrap();
        assert_eq!(stream.surface_at(x - 1.0, z + 1.0), None);
        let mut reloaded = area_fixture();
        reloaded
            .chunk_surfaces
            .insert((0, 0), FootstepSurface::Wood);
        stream.parsed_tiles.insert((32, 48), reloaded);
        assert_eq!(
            stream.surface_at(x - 1.0, z + 1.0),
            Some(FootstepSurface::Wood)
        );
    }

    fn tile_chunk_origin_x(tile: &NativeTerrainTile, index_x: u32, index_y: u32) -> f32 {
        tile.root
            .height_grids
            .iter()
            .find(|grid| (grid.index_x, grid.index_y) == (index_x, index_y))
            .unwrap()
            .origin_x
    }

    #[test]
    fn surface_query_missing_tex_has_no_classification() {
        let mut stream = StreamedTerrain::with_reader(cached_assets());
        let tile = area_fixture();
        let grid = &tile.root.height_grids[0];
        let (x, z) = (grid.origin_x - 1.0, grid.origin_z + 1.0);
        stream.parsed_tiles.insert((32, 48), tile);
        assert_eq!(stream.surface_at(x, z), None);
    }

    #[test]
    fn positioned_surface_uses_wmo_before_terrain_even_without_physics_nodes() {
        use game_engine_core::{
            footstep_data::FootstepSurface, wmo_surface_data::WmoSurfaceBounds,
        };
        let mut stream = StreamedTerrain::with_reader(cached_assets());
        let mut tile = area_fixture();
        let grid = &tile.root.height_grids[0];
        let position = [grid.origin_x - 1.0, 5.0, grid.origin_z + 1.0];
        tile.chunk_surfaces
            .insert((grid.index_x, grid.index_y), FootstepSurface::Grass);
        tile.wmo_surfaces.push((
            WmoSurfaceBounds {
                world_min: [position[0] - 5.0, 0.0, position[2] - 5.0],
                world_max: [position[0] + 5.0, 10.0, position[2] + 5.0],
            },
            FootstepSurface::Wood,
        ));
        stream.parsed_tiles.insert((32, 48), tile);
        assert_eq!(stream.surface_at_position(position), FootstepSurface::Wood);
        assert_eq!(
            stream.surface_at_position([position[0], 11.0, position[2]]),
            FootstepSurface::Grass
        );
        stream.reset().unwrap();
        assert_eq!(stream.surface_at_position(position), FootstepSurface::Dirt);
    }

    #[test]
    fn area_query_uses_loaded_tile_and_chunk_bounds() {
        use game_engine_core::asset::adt_format::adt::CHUNK_SIZE;

        let mut stream = StreamedTerrain::with_reader(cached_assets());
        let mut tile = area_fixture();
        for chunk in &mut tile.root.chunks {
            chunk.area_id = match (chunk.index_x, chunk.index_y) {
                (0, 0) => 1519,
                (0, 1) => 1537,
                (0, 2) => 0,
                _ => 0,
            };
        }
        let first = tile
            .root
            .height_grids
            .iter()
            .find(|g| (g.index_x, g.index_y) == (0, 0))
            .unwrap();
        let (x, z) = (first.origin_x, first.origin_z);
        let next = tile
            .root
            .height_grids
            .iter()
            .find(|g| (g.index_x, g.index_y) == (0, 1))
            .unwrap();
        let adjacent_x = next.origin_x - 1.0;
        stream.parsed_tiles.insert((32, 48), tile);

        assert_eq!(stream.area_id_at(x, z + 1.0), Some(1519));
        assert_eq!(stream.area_id_at(x - 1.0, z + 1.0), Some(1519));
        assert_eq!(stream.area_id_at(adjacent_x, z + 1.0), Some(1537));
        assert_eq!(stream.area_id_at(x - 2.0 * CHUNK_SIZE - 1.0, z + 1.0), None);
        assert_eq!(stream.area_id_at(x + 1.0, z + 1.0), None);
        assert_eq!(stream.area_id_at(x - 1.0, z - 1.0), None);
        assert_eq!(stream.area_id_at(f32::NAN, z), None);
    }

    #[test]
    fn area_query_forgets_cleared_generation_and_reads_reloaded_root() {
        let mut stream = StreamedTerrain::with_reader(cached_assets());
        let mut tile = area_fixture();
        let first = tile
            .root
            .height_grids
            .iter()
            .find(|g| (g.index_x, g.index_y) == (0, 0))
            .unwrap();
        let (x, z) = (first.origin_x - 1.0, first.origin_z + 1.0);
        tile.root
            .chunks
            .iter_mut()
            .find(|c| (c.index_x, c.index_y) == (0, 0))
            .unwrap()
            .area_id = 1519;
        stream.parsed_tiles.insert((32, 48), tile);
        assert_eq!(stream.area_id_at(x, z), Some(1519));
        stream.reset().unwrap();
        assert_eq!(stream.area_id_at(x, z), None);
        let mut reloaded = area_fixture();
        reloaded
            .root
            .chunks
            .iter_mut()
            .find(|c| (c.index_x, c.index_y) == (0, 0))
            .unwrap()
            .area_id = 1537;
        stream.parsed_tiles.insert((32, 48), reloaded);
        assert_eq!(stream.area_id_at(x, z), Some(1537));
    }

    #[test]
    fn water_query_selects_only_coordinate_tile_and_highest_existing_layer() {
        let mut stream = StreamedTerrain::with_reader(cached_assets());
        stream.parsed_tiles.insert(
            (32, 32),
            water_tile(vec![
                flat_water(5.0, 1),
                flat_water(40.0, 0),
                flat_water(12.0, 1),
            ]),
        );
        stream
            .parsed_tiles
            .insert((31, 31), water_tile(vec![flat_water(90.0, 1)]));

        assert_eq!(stream.water_surface_at(-1.0, 1.0), Some(12.0));
        assert_eq!(stream.water_surface_at(-5.0, 1.0), None);
        assert_eq!(stream.water_surface_at(1.0, 1.0), None);
    }

    #[test]
    fn worker_panic_remains_an_error_on_subsequent_polls() {
        struct PanickingReader;

        impl TerrainReader for PanickingReader {
            fn read_map_wdt(&self, _map: &str) -> Result<NativeMapWdt, String> {
                panic!("test map reader panic");
            }

            fn read_tile(
                &self,
                _map: &str,
                _tile: (u32, u32),
            ) -> Result<NativeTerrainTile, String> {
                unreachable!("map reader panics before tile requests");
            }
        }

        let mut stream = StreamedTerrain::with_reader(PanickingReader);
        stream.request_map("test".into(), (0, 0)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let first_error = loop {
            if let Err(error) = stream.poll() {
                break error;
            }
            assert!(Instant::now() < deadline, "worker did not exit");
            std::thread::yield_now();
        };
        assert_eq!(
            first_error,
            "Native terrain worker panicked: test map reader panic"
        );
        assert_eq!(stream.poll(), Err(first_error));
    }

    #[test]
    fn cached_global_wmo_map_is_parsed_without_tile_requests() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut stream = StreamedTerrain::new(data_root);
        stream
            .request_map("stormwindjail".into(), (32, 48))
            .unwrap();
        wait_for(&mut stream, |state| !state.pending_map);
        let state = stream.state();
        assert_eq!(state.global_wmo_fdid, Some(108_631));
        assert_eq!(state.wdt_path.unwrap().file_name().unwrap(), "791060.wdt");
        assert!(state.pending_tiles.is_empty());
        assert!(state.parsed_tiles.is_empty());
        assert!(state.failures.is_empty());
        assert!(stream.map_wdt.is_some());
        // The player's tile, requested once the WDT is parsed, has no ADT on a WMO-only map.
        stream
            .request_map("stormwindjail".into(), (31, 31))
            .unwrap();
        let state = stream.state();
        assert!(state.pending_tiles.is_empty());
        assert!(state.failures.is_empty());
    }

    #[test]
    fn bounded_tiles_are_deduplicated_and_parsed_assets_retained() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let reader = ControlledReader {
            assets: cached_assets(),
            started: started_tx,
            release: Mutex::new(release_rx),
        };
        let mut stream = StreamedTerrain::with_reader(reader);
        stream.request_map("azeroth".into(), (32, 48)).unwrap();
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        stream.request_map("azeroth".into(), (32, 48)).unwrap();
        release_tx.send(()).unwrap();
        wait_for(&mut stream, |state| {
            state.pending_tiles.is_empty()
                && !state.pending_map
                && state.parsed_tiles.len() + state.failures.len() == 9
        });
        let state = stream.state();
        assert_eq!(state.parsed_tiles.len(), 1);
        assert_eq!(state.failures.len(), 8);
        let parsed = &state.parsed_tiles[0];
        assert_eq!(parsed.tile, (32, 48));
        assert_eq!(parsed.root_chunks, 256);
        assert!(parsed.tex_chunk_layers > 0);
        assert!(parsed.obj_doodads > 0);
        assert_eq!(parsed.root_path.file_name().unwrap(), "778027.adt");
        assert_eq!(
            parsed.tex_path.as_ref().unwrap().file_name().unwrap(),
            "778030.adt"
        );
        assert_eq!(
            parsed.obj_path.as_ref().unwrap().file_name().unwrap(),
            "778028.adt"
        );
        assert_eq!(parsed.root_height_grids, 256);
        assert!(parsed.obj_wmos > 0 || parsed.obj_doodads > 0);
        assert!(
            state
                .failures
                .iter()
                .all(|failure| failure.error.contains("missing tile") && failure.tile != (32, 48))
        );
        assert!(state.wdt_flags.is_some());
        assert_eq!(stream.parsed_tiles.len(), 1);
        assert!(stream.parsed_tiles.contains_key(&(32, 48)));
    }

    #[test]
    fn explicit_initial_tiles_load_only_supplied_tiles_then_same_map_adds_one() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let reader = ControlledReader {
            assets: cached_assets(),
            started: started_tx,
            release: Mutex::new(release_rx),
        };
        let mut stream = StreamedTerrain::with_reader(reader);
        stream
            .request_map_tiles("azeroth".into(), (31, 37), &[(31, 36), (31, 37), (31, 36)])
            .unwrap();
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        stream.request_map("azeroth".into(), (31, 36)).unwrap();
        release_tx.send(()).unwrap();
        wait_for(&mut stream, |state| {
            !state.pending_map && state.pending_tiles.is_empty() && state.failures.len() == 2
        });
        let failed_tiles: BTreeSet<_> = stream.state().failures.iter().map(|f| f.tile).collect();
        assert_eq!(failed_tiles, BTreeSet::from([(31, 36), (31, 37)]));
        assert!(stream.state().parsed_tiles.is_empty());

        stream.request_map("azeroth".into(), (31, 38)).unwrap();
        wait_for(&mut stream, |state| {
            state.pending_tiles.is_empty() && state.failures.len() == 3
        });
        let failed_tiles: BTreeSet<_> = stream.state().failures.iter().map(|f| f.tile).collect();
        assert_eq!(failed_tiles, BTreeSet::from([(31, 36), (31, 37), (31, 38)]));
    }

    #[test]
    fn same_map_request_before_wdt_ready_adds_only_requested_tile() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let reader = ControlledReader {
            assets: cached_assets(),
            started: started_tx,
            release: Mutex::new(release_rx),
        };
        let mut stream = StreamedTerrain::with_reader(reader);
        stream
            .request_map_tiles("azeroth".into(), (31, 37), &[(31, 36)])
            .unwrap();
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        stream.request_map("azeroth".into(), (31, 38)).unwrap();
        release_tx.send(()).unwrap();
        wait_for(&mut stream, |state| {
            !state.pending_map && state.pending_tiles.is_empty() && state.failures.len() == 3
        });
        let failed_tiles: BTreeSet<_> = stream.state().failures.iter().map(|f| f.tile).collect();
        assert_eq!(failed_tiles, BTreeSet::from([(31, 36), (31, 37), (31, 38)]));
    }

    #[test]
    fn explicit_initial_tiles_reject_invalid_coordinates_without_changing_map() {
        let mut stream = StreamedTerrain::with_reader(cached_assets());
        assert_eq!(
            stream.request_map_tiles("azeroth".into(), (31, 37), &[(31, 36), (64, 0)]),
            Err("Terrain tile (64, 0) outside 0..64".into())
        );
        assert_eq!(stream.state().map, None);
        assert!(!stream.state().pending_map);
    }

    #[test]
    fn explicit_initial_tiles_skip_global_wmo_map() {
        let mut stream = StreamedTerrain::with_reader(cached_assets());
        stream
            .request_map_tiles("stormwindjail".into(), (32, 48), &[(31, 36), (31, 37)])
            .unwrap();
        wait_for(&mut stream, |state| !state.pending_map);
        let state = stream.state();
        assert!(state.global_wmo_present);
        assert!(state.pending_tiles.is_empty());
        assert!(state.parsed_tiles.is_empty());
        assert!(state.failures.is_empty());
    }

    #[test]
    fn stale_results_do_not_replace_revisited_map_or_reset_state() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let reader = ControlledReader {
            assets: cached_assets(),
            started: started_tx,
            release: Mutex::new(release_rx),
        };
        let mut stream = StreamedTerrain::with_reader(reader);
        stream.request_map("azeroth".into(), (32, 48)).unwrap();
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        stream
            .request_map("stormwindjail".into(), (32, 48))
            .unwrap();
        stream.reset().unwrap();
        assert_eq!(stream.state().map, None);
        stream.request_map("azeroth".into(), (32, 48)).unwrap();
        release_tx.send(()).unwrap();
        release_tx.send(()).unwrap();
        wait_for(&mut stream, |state| {
            state.pending_tiles.is_empty() && !state.pending_map
        });
        assert_eq!(stream.state().map.as_deref(), Some("azeroth"));
        assert!(stream.state().global_wmo_fdid.is_none());
        assert_eq!(stream.state().parsed_tiles.len(), 1);
    }

    #[test]
    fn missing_map_surfaces_read_error_without_scheduling_tiles() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut stream = StreamedTerrain::new(data_root);
        stream
            .request_map("map_that_does_not_exist_999".into(), (0, 0))
            .unwrap();
        wait_for(&mut stream, |state| !state.pending_map);
        let state = stream.state();
        assert!(state.map_error.unwrap().contains("not in listfile"));
        assert!(state.pending_tiles.is_empty());
        assert!(state.parsed_tiles.is_empty());
    }
}
