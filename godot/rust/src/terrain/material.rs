//! Native GPU resources from retained split-ADT data; no world-readiness decisions.

use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

use game_engine_core::adt;
use godot::{
    classes::{
        CollisionShape3D, ConcavePolygonShape3D, ImageTexture, MeshInstance3D, Node3D, Shader,
        ShaderMaterial, StaticBody3D,
    },
    prelude::*,
};

use super::{
    assets::NativeTerrainTile,
    streaming::StreamedTerrain,
    textures::TerrainLayerTextures,
    tile_material::{
        CHUNKS_PER_SIDE, ChunkInputs, LayerArrays, REF_UNTEXTURED, REF_WHITE, TileAssembly,
        hidden_mask,
    },
};
use crate::{lighting::TerrainLight, wmo::portals::HalfSpace};
use glam::Vec3;

type Tile = (u32, u32);

/// Main-thread time per frame for building terrain chunk resources; at least one chunk
/// is built each frame.
const TERRAIN_BUDGET: Duration = Duration::from_millis(8);

/// A built tile's camera-rejection mask (`terrain.gdshader` `chunk_hidden`).
struct TileVisibility {
    texture: Gd<ImageTexture>,
    hidden: [u8; CHUNKS_PER_SIDE * CHUNKS_PER_SIDE],
    /// Mask texel and tile-space bounds of each drawn chunk.
    chunks: Vec<(usize, Vec3, Vec3)>,
}

/// A tile whose chunks are being gathered over several frames.
struct TileBuild {
    tile: Tile,
    assembly: TileAssembly,
    /// Chunk collision bodies, allocated with their chunk so a tile's 256 bodies spread
    /// over the gathering frames instead of one finishing frame (~15 ms per tile).
    collisions: Vec<Gd<StaticBody3D>>,
    bounds: Vec<(usize, Vec3, Vec3)>,
    /// Next root chunk to build.
    next: usize,
    /// The tile's liquid node once built; `Some(None)` when it has no liquid.
    water: Option<Option<Gd<Node3D>>>,
}

impl Drop for TileBuild {
    /// Frees the nodes of a tile that never spawned: failed, reset or no longer parsed.
    fn drop(&mut self) {
        for body in self.collisions.drain(..) {
            body.free();
        }
        if let Some(Some(water)) = self.water.take() {
            water.free();
        }
    }
}

#[derive(Default)]
pub(crate) struct TerrainMaterials {
    root: Option<Gd<Node3D>>,
    attached: BTreeSet<Tile>,
    building: Option<TileBuild>,
    /// Loading bubble tiles, center first; empty after entry.
    priority: Vec<Tile>,
    /// Tiles whose GPU resources could not be built; never retried until reset.
    failures: BTreeMap<Tile, String>,
    layers: LayerArrays,
    shader: Option<Gd<Shader>>,
    /// One per built tile.
    materials: Vec<Gd<ShaderMaterial>>,
    tiles: Vec<TileVisibility>,
    light: Option<TerrainLight>,
    water: super::water::WaterMaterials,
}

impl TerrainMaterials {
    /// Terrain tile materials and liquid materials built so far.
    pub fn material_counts(&self) -> (usize, usize) {
        (self.materials.len(), self.water.material_count())
    }

    pub fn attached_tiles(&self) -> &BTreeSet<(u32, u32)> {
        &self.attached
    }

    pub fn prioritize_tiles(&mut self, tiles: Vec<Tile>) {
        self.priority = tiles;
    }

    pub fn failures(&self) -> &BTreeMap<(u32, u32), String> {
        &self.failures
    }

    /// Reject terrain chunks from the camera while keeping their shadow and collision.
    pub fn cull_outdoor(&mut self, windows: Option<&[Vec<HalfSpace>]>, interiors: &[(Vec3, Vec3)]) {
        let Some(root) = &self.root else {
            return;
        };
        let world = super::objects::affine(root.get_global_transform());
        for tile in &mut self.tiles {
            let mut hidden = tile.hidden;
            for &(texel, min, max) in &tile.chunks {
                let (min, max) = world_box(world, min, max);
                let admitted = super::scenery::outdoor_box_admitted(min, max, windows, interiors);
                hidden[texel] = if admitted { 0 } else { 255 };
            }
            if hidden == tile.hidden {
                continue;
            }
            match hidden_mask(&hidden) {
                Ok(image) => {
                    crate::rebuild_guard::count(crate::rebuild_guard::Rebuild::MaterialRebind, 1);
                    tile.texture.update(&image);
                    tile.hidden = hidden;
                }
                Err(error) => crate::frame_error::report_once(&error),
            }
        }
    }

    /// Build GPU resources with the loading budget while priority tiles are set,
    /// otherwise `TERRAIN_BUDGET` per frame. Build the
    /// tiles the map request started from first; a tile attaches once all its chunks
    /// are built.
    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        terrain: &StreamedTerrain,
    ) -> Result<(), String> {
        let budget = if self.priority.is_empty() {
            TERRAIN_BUDGET
        } else {
            super::LOADING_RESOURCE_BUDGET
        };
        let deadline = Instant::now() + budget;
        while Instant::now() < deadline {
            let Some(tile) = self.next_tile(terrain) else {
                break;
            };
            let node = match self.build_tile(tile, &terrain.parsed_tiles[&tile], deadline) {
                Ok(Some(node)) => node,
                Ok(None) => break,
                Err(error) => {
                    // Like a tile that fails to parse, one unbuildable tile is reported and
                    // left out; readiness decides whether the player's tile blocks entry.
                    self.building = None;
                    let error = format!("Terrain ({}, {}): {error}", tile.0, tile.1);
                    godot_error!("{error}");
                    self.failures.insert(tile, error);
                    continue;
                }
            };
            let root = self.root.get_or_insert_with(|| {
                let mut root = Node3D::new_alloc();
                root.set_name("WorldTerrain");
                parent.add_child(&root);
                root
            });
            root.add_child(&node);
            self.attached.insert(tile);
        }
        // A layer array grew: every built material still samples the replaced one.
        if self.layers.take_replaced() {
            for material in &mut self.materials {
                crate::rebuild_guard::count(crate::rebuild_guard::Rebuild::MaterialRebind, 1);
                self.layers.bind(material);
            }
        }
        self.water.sample_clock(parent)
    }

    /// The tile being built while it is still parsed, else the next unbuilt one.
    fn next_tile(&mut self, terrain: &StreamedTerrain) -> Option<Tile> {
        let unbuilt = |tile: &Tile| {
            terrain.parsed_tiles.contains_key(tile)
                && !self.attached.contains(tile)
                && !self.failures.contains_key(tile)
        };
        if let Some(build) = &self.building {
            if unbuilt(&build.tile) {
                return Some(build.tile);
            }
            self.building = None;
        }
        let nearby = self.priority.iter().copied().find(|tile| unbuilt(tile));
        if nearby.is_some() {
            return nearby;
        }
        let initial = terrain
            .initial_tiles()
            .iter()
            .copied()
            .find(|tile| unbuilt(tile));
        initial.or_else(|| {
            terrain
                .parsed_tiles
                .keys()
                .copied()
                .find(|tile| unbuilt(tile))
        })
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.attached.clear();
        self.building = None;
        self.priority.clear();
        self.failures.clear();
        self.layers = LayerArrays::default();
        self.materials.clear();
        self.tiles.clear();
        self.light = None;
        self.water = super::water::WaterMaterials::default();
    }

    pub fn update_lighting(&mut self, light: TerrainLight) {
        if self.light.is_none() {
            for material in &mut self.materials {
                light.bind(material);
            }
        }
        self.water.update_lighting(&light);
        self.light = Some(light);
    }

    fn shader(&mut self) -> Result<Gd<Shader>, String> {
        if let Some(shader) = &self.shader {
            return Ok(shader.clone());
        }
        let shader = crate::shader_warmup::load_shader("res://shaders/terrain.gdshader")?;
        self.shader = Some(shader.clone());
        Ok(shader)
    }

    /// Gather more of `tile`'s chunks until the frame's deadline;
    /// return the tile node once every chunk is gathered.
    fn build_tile(
        &mut self,
        tile: Tile,
        parsed: &NativeTerrainTile,
        deadline: Instant,
    ) -> Result<Option<Gd<Node3D>>, String> {
        let tex = parsed.tex.as_ref().ok_or("Missing texture companion")?;
        if tex.chunk_layers.len() != parsed.root.chunks.len() {
            return Err(format!(
                "Root/texture chunk counts differ: {}/{}",
                parsed.root.chunks.len(),
                tex.chunk_layers.len()
            ));
        }
        let build = self.building.take().unwrap_or_else(|| TileBuild {
            tile,
            assembly: TileAssembly::default(),
            collisions: Vec::new(),
            bounds: Vec::new(),
            next: 0,
            water: None,
        });
        let gathered = build.next == parsed.root.chunks.len();
        let Some(mut build) = self.build_tile_chunks(tile, build, parsed, tex, deadline)? else {
            return Ok(None);
        };
        // The liquid build and the tile's GPU build are several milliseconds each: each
        // opens its own frame's budget instead of running past the deadline the gathering
        // or the other already used.
        if !gathered {
            self.building = Some(build);
            return Ok(None);
        }
        if build.water.is_none() {
            let _span = crate::profile::span(|| "terrain.water".to_owned());
            build.water = Some(self.water.build(&parsed.root, &parsed.liquid_materials)?);
            self.building = Some(build);
            return Ok(None);
        }
        self.spawn_tile(tile, build).map(Some)
    }

    fn build_tile_chunks(
        &mut self,
        tile: Tile,
        mut build: TileBuild,
        parsed: &NativeTerrainTile,
        tex: &adt::AdtTexData,
        deadline: Instant,
    ) -> Result<Option<TileBuild>, String> {
        // Texture chunks are stored by encounter order, not by root chunk coordinates.
        let chunks = parsed.root.chunks.iter().zip(&tex.chunk_layers);
        for (chunk, layers) in chunks.skip(build.next) {
            if Instant::now() >= deadline {
                self.building = Some(build);
                return Ok(None);
            }
            build.next += 1;
            let geometry = adt::chunk_geometry(chunk, Some(tile));
            if geometry.indices.is_empty() {
                continue;
            }
            let inputs = self.chunk_inputs(parsed, &layers.layers)?;
            let index = (chunk.index_x, chunk.index_y);
            build
                .assembly
                .add_chunk(index, &geometry, &chunk.vertex_colors, &inputs)?;
            build.bounds.push((
                chunk.index_y as usize * CHUNKS_PER_SIDE + chunk.index_x as usize,
                geometry_min(&geometry),
                geometry_max(&geometry),
            ));
            build.collisions.push(spawn_collision(
                &format!("Chunk{}_{}", chunk.index_x, chunk.index_y),
                &collision_from_geometry(&geometry),
            ));
        }
        Ok(Some(build))
    }

    fn spawn_tile(&mut self, tile: Tile, mut build: TileBuild) -> Result<Gd<Node3D>, String> {
        let hidden = [0; CHUNKS_PER_SIDE * CHUNKS_PER_SIDE];
        let mask = ImageTexture::create_from_image(&hidden_mask(&hidden)?)
            .ok_or("Godot rejected the terrain chunk mask texture")?;
        let built = if build.assembly.is_empty() {
            None
        } else {
            let shader = self.shader()?;
            let (mesh, mut material) =
                std::mem::take(&mut build.assembly).build(&shader, &self.layers, &mask)?;
            if let Some(light) = &self.light {
                light.bind(&mut material);
            }
            Some((mesh, material))
        };
        // Allocate the root only after all fallible resource construction; on an error
        // `TileBuild` frees its collision bodies and liquid.
        let mut root = Node3D::new_alloc();
        root.set_name(&format!("Tile{}_{}", tile.0, tile.1));
        if let Some(water) = build.water.take().flatten() {
            root.add_child(&water);
        }
        if let Some((mesh, material)) = built {
            let mut instance = MeshInstance3D::new_alloc();
            instance.set_name("Terrain");
            instance.set_mesh(&mesh);
            instance.set_surface_override_material(0, &material);
            root.add_child(&instance);
            self.materials.push(material);
            self.tiles.push(TileVisibility {
                texture: mask,
                hidden,
                chunks: std::mem::take(&mut build.bounds),
            });
        }
        for body in build.collisions.drain(..) {
            root.add_child(&body);
        }
        Ok(root)
    }

    /// `terrain.gdshader` inputs of one chunk; its layer textures join the shared arrays.
    fn chunk_inputs(
        &mut self,
        parsed: &NativeTerrainTile,
        layers: &[adt::TextureLayer],
    ) -> Result<ChunkInputs, String> {
        let inputs = ChunkMaterialInputs::new(parsed, layers)?;
        let (mut ground, mut height) = ([REF_WHITE; 4], [REF_WHITE; 4]);
        if inputs.textures.is_empty() {
            // Original untextured chunk: every ground and height slot samples one gray texel.
            ground = [REF_UNTEXTURED; 4];
            height = [REF_UNTEXTURED; 4];
        }
        for (slot, images) in inputs.textures.iter().enumerate() {
            ground[slot] = self
                .layers
                .reference(images.diffuse_fdid, &images.diffuse)?;
            if let Some(texture) = &images.height {
                height[slot] = self.layers.reference(texture.fdid, &texture.image)?;
            }
        }
        Ok(ChunkInputs {
            config: inputs.config,
            layer_params: inputs.layer_params,
            animation: inputs.animation,
            ground,
            height,
            alpha: inputs.alpha,
        })
    }
}

/// Shader inputs for one MCNK chunk, resolved before any Godot resource exists.
struct ChunkMaterialInputs<'a> {
    config: [f32; 4],
    /// Decoded images per used slot; empty for a chunk with no authored layers.
    textures: Vec<&'a TerrainLayerTextures>,
    layer_params: [[f32; 4]; 4],
    animation: [[f32; 4]; 4],
    alpha: Vec<u8>,
}

impl<'a> ChunkMaterialInputs<'a> {
    fn new(parsed: &'a NativeTerrainTile, layers: &[adt::TextureLayer]) -> Result<Self, String> {
        let tex = parsed.tex.as_ref().ok_or("Missing texture companion")?;
        // Preserve the original renderer's four texture slots, including campsite
        // chunks with a fifth MCLY record; do not reject the entire authored tile.
        let layers = &layers[..layers.len().min(4)];
        let textures = layers
            .iter()
            .map(|layer| {
                parsed
                    .textures
                    .get(&layer.texture_index)
                    .ok_or_else(|| format!("Texture index {} was not decoded", layer.texture_index))
            })
            .collect::<Result<Vec<_>, _>>()?;
        // A chunk without MCLY layers renders untextured with default map settings,
        // matching the original renderer's fallback material.
        let (blend, amplifier) = if layers.is_empty() {
            (adt::TerrainBlendMode::Layered, None)
        } else {
            (
                adt::terrain_blend_mode(tex.map_flags),
                tex.texture_amplifier,
            )
        };
        let mut has_height = [false; 4];
        for (slot, images) in textures.iter().enumerate() {
            has_height[slot] = images.height.is_some();
        }
        Ok(Self {
            config: [
                layers.len() as f32,
                blend as u32 as f32,
                adt::terrain_texture_repeat(amplifier),
                0.0,
            ],
            textures,
            layer_params: adt::texture_layer_params(&tex.texture_params, layers, has_height),
            animation: adt::terrain_layer_animation_params(layers),
            alpha: adt::pack_alpha_map_bytes(layers),
        })
    }
}

fn world_box(transform: glam::Affine3A, min: Vec3, max: Vec3) -> (Vec3, Vec3) {
    let mut world_min = Vec3::splat(f32::INFINITY);
    let mut world_max = Vec3::splat(f32::NEG_INFINITY);
    for bits in 0..8 {
        let point = Vec3::new(
            if bits & 1 == 0 { min.x } else { max.x },
            if bits & 2 == 0 { min.y } else { max.y },
            if bits & 4 == 0 { min.z } else { max.z },
        );
        let point = transform.transform_point3(point);
        world_min = world_min.min(point);
        world_max = world_max.max(point);
    }
    (world_min, world_max)
}

fn collision_from_geometry(geometry: &adt::Geometry) -> Gd<ConcavePolygonShape3D> {
    let faces: Vec<Vector3> = geometry
        .indices
        .chunks_exact(3)
        .flat_map(|triangle| [triangle[0], triangle[2], triangle[1]])
        .map(|index| Vector3::from_array(geometry.positions[index as usize]))
        .collect();
    let mut shape = ConcavePolygonShape3D::new_gd();
    shape.set_faces(&PackedVector3Array::from(faces.as_slice()));
    shape
}

fn spawn_collision(name: &str, collision: &Gd<ConcavePolygonShape3D>) -> Gd<StaticBody3D> {
    let mut body = StaticBody3D::new_alloc();
    body.set_name(name);
    let mut shape = CollisionShape3D::new_alloc();
    shape.set_name("Shape");
    shape.set_shape(collision);
    body.add_child(&shape);
    body
}

fn geometry_min(geometry: &adt::Geometry) -> Vec3 {
    geometry
        .positions
        .iter()
        .fold(Vec3::splat(f32::INFINITY), |min, p| {
            min.min(Vec3::from_array(*p))
        })
}

fn geometry_max(geometry: &adt::Geometry) -> Vec3 {
    geometry
        .positions
        .iter()
        .fold(Vec3::splat(f32::NEG_INFINITY), |max, p| {
            max.max(Vec3::from_array(*p))
        })
}

#[cfg(test)]
mod tests {
    use game_engine_core::warband_scene_data::read_authored_catalog;

    use super::*;
    use crate::terrain::assets::{cached_assets, test_data_root};

    #[test]
    fn outdoor_terrain_bounds_follow_world_transform() {
        let transform = glam::Affine3A::from_translation(Vec3::new(10.0, 3.0, -2.0));
        let (min, max) = world_box(transform, Vec3::ZERO, Vec3::new(2.0, 1.0, 3.0));
        assert_eq!(min, Vec3::new(10.0, 3.0, -2.0));
        assert_eq!(max, Vec3::new(12.0, 4.0, 1.0));
    }

    /// Every authored tile a campsite streams, as the character-select background requests it.
    fn campsite_tiles(scene_id: u32) -> Vec<NativeTerrainTile> {
        let catalog = read_authored_catalog(&test_data_root()).expect("authored campsites");
        let scene = catalog
            .scenes
            .iter()
            .find(|scene| scene.id == scene_id)
            .expect("authored scene");
        let assets = cached_assets();
        let (y, x) = scene.tile_coords();
        vec![
            assets
                .read_tile(&scene.map_name(), y, x)
                .expect("cached campsite tile"),
        ]
    }

    fn assert_untextured_chunks_use_original_fallback(scene_id: u32) {
        let mut untextured = 0;
        for tile in campsite_tiles(scene_id) {
            let tex = tile.tex.as_ref().expect("texture companion");
            for layers in &tex.chunk_layers {
                let inputs = ChunkMaterialInputs::new(&tile, &layers.layers)
                    .unwrap_or_else(|error| panic!("scene {scene_id}: {error}"));
                if !layers.layers.is_empty() {
                    continue;
                }
                untextured += 1;
                assert_eq!(inputs.config, [0.0, 0.0, 8.0, 0.0]);
                assert!(inputs.textures.is_empty());
                // Original DEFAULT_LAYER_PARAMS: no height scale, unit offset, 1x brightness.
                assert_eq!(inputs.layer_params, [[0.0, 1.0, 0.0, 1.0]; 4]);
                assert_eq!(inputs.animation, [[0.0; 4]; 4]);
                assert!(inputs.alpha.chunks_exact(4).all(|px| px == [0, 0, 0, 255]));
            }
        }
        assert!(untextured > 0, "scene {scene_id} has no MCLY-less chunk");
    }

    #[test]
    fn cultists_quay_untextured_chunks_build_original_fallback_material() {
        assert_untextured_chunks_use_original_fallback(5);
    }

    #[test]
    fn gallagio_grand_gallery_untextured_chunks_build_original_fallback_material() {
        assert_untextured_chunks_use_original_fallback(25);
    }
}
