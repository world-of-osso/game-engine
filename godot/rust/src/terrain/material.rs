//! Native GPU resources from retained split-ADT data; no world-readiness decisions.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    time::{Duration, Instant},
};

use game_engine_core::{adt, blp};
use godot::{
    classes::{
        ArrayMesh, CollisionShape3D, ConcavePolygonShape3D, Image, ImageTexture, MeshInstance3D,
        Node3D, Shader, ShaderMaterial, StaticBody3D, image,
    },
    prelude::*,
};

use super::{
    assets::NativeTerrainTile, streaming::StreamedTerrain, textures::TerrainLayerTextures,
};
use crate::lighting::TerrainLight;

type Tile = (u32, u32);

/// Main-thread time per frame for building terrain chunk resources; at least one chunk
/// is built each frame.
const TERRAIN_BUDGET: Duration = Duration::from_millis(8);

type BuiltChunk = (
    String,
    Gd<ArrayMesh>,
    Gd<ShaderMaterial>,
    Gd<ConcavePolygonShape3D>,
);

/// A tile whose chunks are being built over several frames.
struct TileBuild {
    tile: Tile,
    chunks: Vec<BuiltChunk>,
    /// Next root chunk to build.
    next: usize,
}

#[derive(Default)]
pub(crate) struct TerrainMaterials {
    root: Option<Gd<Node3D>>,
    attached: BTreeSet<Tile>,
    building: Option<TileBuild>,
    /// Tiles whose GPU resources could not be built; never retried until reset.
    failures: BTreeMap<Tile, String>,
    textures: HashMap<u32, Gd<ImageTexture>>,
    placeholder: Option<Gd<ImageTexture>>,
    shader: Option<Gd<Shader>>,
    materials: Vec<Gd<ShaderMaterial>>,
    light: Option<TerrainLight>,
    water: super::water::WaterMaterials,
}

impl TerrainMaterials {
    /// Terrain chunk materials and liquid materials built so far.
    pub fn material_counts(&self) -> (usize, usize) {
        (self.materials.len(), self.water.material_count())
    }

    pub fn attached_tiles(&self) -> &BTreeSet<(u32, u32)> {
        &self.attached
    }

    pub fn failures(&self) -> &BTreeMap<(u32, u32), String> {
        &self.failures
    }

    /// Build the GPU resources of parsed tiles within `TERRAIN_BUDGET` per frame, the
    /// tiles the map request started from first; a tile attaches once all its chunks
    /// are built.
    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        terrain: &StreamedTerrain,
    ) -> Result<(), String> {
        let started = Instant::now();
        while started.elapsed() < TERRAIN_BUDGET {
            let Some(tile) = self.next_tile(terrain) else {
                break;
            };
            let node = match self.build_tile(tile, &terrain.parsed_tiles[&tile], started) {
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
        self.failures.clear();
        self.textures.clear();
        self.placeholder = None;
        self.materials.clear();
        self.light = None;
        self.water = super::water::WaterMaterials::default();
    }

    pub fn update_lighting(&mut self, light: TerrainLight) {
        for material in &mut self.materials {
            light.bind(material);
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

    /// Build more of `tile`'s chunks until the frame's budget, counted from `started`, is
    /// spent; the tile node once every chunk is built.
    fn build_tile(
        &mut self,
        tile: Tile,
        parsed: &NativeTerrainTile,
        started: Instant,
    ) -> Result<Option<Gd<Node3D>>, String> {
        let tex = parsed.tex.as_ref().ok_or("Missing texture companion")?;
        if tex.chunk_layers.len() != parsed.root.chunks.len() {
            return Err(format!(
                "Root/texture chunk counts differ: {}/{}",
                parsed.root.chunks.len(),
                tex.chunk_layers.len()
            ));
        }
        let shader = self.shader()?;
        let build = self.building.take().unwrap_or(TileBuild {
            tile,
            chunks: Vec::new(),
            next: 0,
        });
        let Some(build) = self.build_tile_chunks(tile, build, parsed, tex, &shader, started)?
        else {
            return Ok(None);
        };
        self.spawn_tile(tile, parsed, build).map(Some)
    }

    fn build_tile_chunks(
        &mut self,
        tile: Tile,
        mut build: TileBuild,
        parsed: &NativeTerrainTile,
        tex: &adt::AdtTexData,
        shader: &Gd<Shader>,
        started: Instant,
    ) -> Result<Option<TileBuild>, String> {
        // Texture chunks are stored by encounter order, not by root chunk coordinates.
        let chunks = parsed.root.chunks.iter().zip(&tex.chunk_layers);
        for (chunk, layers) in chunks.skip(build.next) {
            if started.elapsed() >= TERRAIN_BUDGET {
                self.building = Some(build);
                return Ok(None);
            }
            build.next += 1;
            let geometry = adt::chunk_geometry(chunk, Some(tile));
            if geometry.indices.is_empty() {
                continue;
            }
            let material = self.build_material(parsed, &layers.layers, shader)?;
            let collision = collision_from_geometry(&geometry);
            let mesh = super::build_mesh(geometry, &chunk.vertex_colors);
            build.chunks.push((
                format!("Chunk{}_{}", chunk.index_x, chunk.index_y),
                mesh,
                material,
                collision,
            ));
        }
        Ok(Some(build))
    }

    fn spawn_tile(
        &mut self,
        tile: Tile,
        parsed: &NativeTerrainTile,
        build: TileBuild,
    ) -> Result<Gd<Node3D>, String> {
        let span = crate::profile::span(|| "terrain.water".to_owned());
        let water = self.water.build(&parsed.root, &parsed.liquid_materials)?;
        drop(span);
        // Allocate manual-lifetime nodes only after all fallible resource construction.
        let mut root = Node3D::new_alloc();
        root.set_name(&format!("Tile{}_{}", tile.0, tile.1));
        if let Some(water) = water {
            root.add_child(&water);
        }
        for (name, mesh, material, collision) in build.chunks {
            root.add_child(&spawn_chunk(&name, &mesh, &material, &collision));
            self.materials.push(material);
        }
        Ok(root)
    }

    fn build_material(
        &mut self,
        parsed: &NativeTerrainTile,
        layers: &[adt::TextureLayer],
        shader: &Gd<Shader>,
    ) -> Result<Gd<ShaderMaterial>, String> {
        let inputs = ChunkMaterialInputs::new(parsed, layers)?;
        let span = crate::profile::span(|| "terrain.set_shader".to_owned());
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(shader);
        drop(span);
        material.set_shader_parameter("config", &vector4(inputs.config).to_variant());
        self.bind_layer_textures(&mut material, &inputs.textures)?;
        for slot in 0..4 {
            material.set_shader_parameter(
                &format!("layer_params_{slot}"),
                &vector4(inputs.layer_params[slot]).to_variant(),
            );
            material.set_shader_parameter(
                &format!("animation_params_{slot}"),
                &vector4(inputs.animation[slot]).to_variant(),
            );
        }
        let alpha = texture_from_rgba(64, 64, &inputs.alpha)?;
        material.set_shader_parameter("alpha_packed", &alpha.to_variant());
        if let Some(light) = &self.light {
            light.bind(&mut material);
        }
        Ok(material)
    }

    fn bind_layer_textures(
        &mut self,
        material: &mut Gd<ShaderMaterial>,
        textures: &[&TerrainLayerTextures],
    ) -> Result<(), String> {
        if textures.is_empty() {
            // Original untextured chunk: every ground and height slot samples one gray texel.
            let placeholder = self.placeholder()?;
            for slot in 0..4 {
                material.set_shader_parameter(&format!("ground_{slot}"), &placeholder.to_variant());
                material.set_shader_parameter(&format!("height_{slot}"), &placeholder.to_variant());
            }
            return Ok(());
        }
        for (slot, images) in textures.iter().enumerate() {
            let diffuse = self.texture(images.diffuse_fdid, &images.diffuse)?;
            material.set_shader_parameter(&format!("ground_{slot}"), &diffuse.to_variant());
            if let Some(height) = &images.height {
                let texture = self.texture(height.fdid, &height.image)?;
                material.set_shader_parameter(&format!("height_{slot}"), &texture.to_variant());
            }
        }
        Ok(())
    }

    fn placeholder(&mut self) -> Result<Gd<ImageTexture>, String> {
        if let Some(texture) = &self.placeholder {
            return Ok(texture.clone());
        }
        let texture = texture_from_rgba(1, 1, &UNTEXTURED_GROUND_RGBA)?;
        self.placeholder = Some(texture.clone());
        Ok(texture)
    }

    fn texture(&mut self, fdid: u32, pixels: &blp::RgbaImage) -> Result<Gd<ImageTexture>, String> {
        if let Some(texture) = self.textures.get(&fdid) {
            return Ok(texture.clone());
        }
        let texture = texture_from_rgba(pixels.width, pixels.height, &pixels.pixels)
            .map_err(|error| format!("Texture FDID {fdid}: {error}"))?;
        self.textures.insert(fdid, texture.clone());
        Ok(texture)
    }
}

/// The original renderer's color placeholder for chunks without MCLY layers.
const UNTEXTURED_GROUND_RGBA: [u8; 4] = [128, 128, 128, 255];

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

fn spawn_chunk(
    name: &str,
    mesh: &Gd<ArrayMesh>,
    material: &Gd<ShaderMaterial>,
    collision: &Gd<ConcavePolygonShape3D>,
) -> Gd<MeshInstance3D> {
    let mut instance = MeshInstance3D::new_alloc();
    instance.set_name(name);
    instance.set_mesh(mesh);
    instance.set_surface_override_material(0, material);
    let mut body = StaticBody3D::new_alloc();
    body.set_name("TerrainCollision");
    let mut shape = CollisionShape3D::new_alloc();
    shape.set_name("Shape");
    shape.set_shape(collision);
    body.add_child(&shape);
    instance.add_child(&body);
    instance
}

fn vector4(values: [f32; 4]) -> Vector4 {
    Vector4::new(values[0], values[1], values[2], values[3])
}

fn texture_from_rgba(width: u32, height: u32, pixels: &[u8]) -> Result<Gd<ImageTexture>, String> {
    let image = Image::create_from_data(
        width as i32,
        height as i32,
        false,
        image::Format::RGBA8,
        &PackedByteArray::from(pixels),
    )
    .ok_or_else(|| format!("Godot rejected {width}x{height} terrain image"))?;
    ImageTexture::create_from_image(&image).ok_or_else(|| "Godot rejected terrain texture".into())
}

#[cfg(test)]
mod tests {
    use game_engine_core::warband_scene_data::{
        read_authored_catalog, supplemental_terrain_tile_coords,
    };

    use super::*;
    use crate::terrain::assets::{cached_assets, test_data_root};

    /// Every authored tile a campsite streams, as the character-select background requests it.
    fn campsite_tiles(scene_id: u32) -> Vec<NativeTerrainTile> {
        let catalog = read_authored_catalog(&test_data_root()).expect("authored campsites");
        let scene = catalog
            .scenes
            .iter()
            .find(|scene| scene.id == scene_id)
            .expect("authored scene");
        let assets = cached_assets();
        let mut tiles = supplemental_terrain_tile_coords(scene);
        tiles.push(scene.tile_coords());
        tiles
            .into_iter()
            .map(|(y, x)| {
                assets
                    .read_tile(&scene.map_name(), y, x)
                    .expect("cached campsite tile")
            })
            .collect()
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
