//! Queryable heightmap for terrain collision across multiple tiles.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::asset::adt::{self, ChunkHeightGrid};
#[cfg(test)]
use crate::asset::adt::{CHUNK_SIZE, UNIT_SIZE, vertex_index};
use crate::rendering::ground_effects::{self, GroundEffectEntry};
pub(crate) use crate::rendering::terrain_height_data::sample_chunk_height;
use crate::rendering::terrain_height_data::{
    WaterLayerSurface as BorrowedWaterLayerSurface, layer_has_water,
    sample_water_layer_height as sample_shared_water_layer_height,
};
#[cfg(test)]
use crate::rendering::terrain_surface_data::dominant_texture_fdid;
use crate::rendering::terrain_surface_data::{
    dominant_effect_id, dominant_surface_for_chunk_with_resolver,
};
use crate::sound_footsteps::FootstepSurface;
use crate::terrain_tile::bevy_to_tile_coords;

#[cfg(test)]
const WATER_STEP: f32 = CHUNK_SIZE / 8.0;

#[derive(Clone)]
struct WaterLayerSurface {
    chunk_origin_wow_x: f32,
    chunk_origin_wow_y: f32,
    min_height: f32,
    x_offset: u8,
    y_offset: u8,
    width: u8,
    height: u8,
    exists: [u8; 8],
    vertex_heights: Vec<f32>,
}

/// Queryable heightmap for terrain collision across multiple tiles.
#[derive(Resource, Default)]
pub struct TerrainHeightmap {
    /// Per-tile grids: (tile_y, tile_x) → 256 chunk height grids.
    tiles: HashMap<(u32, u32), Vec<Option<ChunkHeightGrid>>>,
    /// Per-tile dominant ground effect metadata for each chunk.
    effects: HashMap<(u32, u32), Vec<Option<GroundEffectEntry>>>,
    /// Per-tile dominant surface class for each chunk.
    surfaces: HashMap<(u32, u32), Vec<FootstepSurface>>,
    /// Per-tile water layers for cheap swim/depth queries.
    water_layers: HashMap<(u32, u32), Vec<WaterLayerSurface>>,
    /// Per-tile MCNK AreaTable ids, indexed like `tiles`.
    areas: HashMap<(u32, u32), Vec<u32>>,
    /// The map is one WMO (WDT MPHD flag 0x1): it has no terrain, only WMO floors.
    wmo_only: bool,
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
        let mut areas = vec![0; 256];
        for chunk in &adt_data.chunks {
            let idx = (chunk.index_y * 16 + chunk.index_x) as usize;
            if idx < 256 {
                areas[idx] = chunk.area_id;
            }
        }
        self.areas.insert((tile_y, tile_x), areas);
    }

    /// Get all loaded tile coordinate keys.
    pub fn tile_keys(&self) -> impl Iterator<Item = &(u32, u32)> {
        self.tiles.keys()
    }

    /// Get chunk grids for a specific tile.
    pub fn tile_chunks(&self, tile_y: u32, tile_x: u32) -> Option<&Vec<Option<ChunkHeightGrid>>> {
        self.tiles.get(&(tile_y, tile_x))
    }

    /// Whether the tile containing Bevy-space (x, z) has registered heights.
    /// Mark the map WMO-only: ground comes from WMO floors alone.
    pub fn set_wmo_only(&mut self) {
        self.wmo_only = true;
    }

    pub fn is_wmo_only(&self) -> bool {
        self.wmo_only
    }

    pub fn has_tile_at(&self, bx: f32, bz: f32) -> bool {
        self.tiles.contains_key(&bevy_to_tile_coords(bx, bz))
    }

    /// Remove height grids for a tile.
    pub fn remove_tile(&mut self, tile_y: u32, tile_x: u32) {
        self.tiles.remove(&(tile_y, tile_x));
        self.effects.remove(&(tile_y, tile_x));
        self.surfaces.remove(&(tile_y, tile_x));
        self.water_layers.remove(&(tile_y, tile_x));
        self.areas.remove(&(tile_y, tile_x));
    }

    /// AreaTable id of the terrain chunk under a Bevy-space (x, z) position.
    pub fn area_id_at(&self, bx: f32, bz: f32) -> Option<u32> {
        let (tile_y, tile_x) = bevy_to_tile_coords(bx, bz);
        let chunk_idx = self.chunk_index_at(tile_y, tile_x, bx, bz)?;
        self.areas
            .get(&(tile_y, tile_x))
            .and_then(|areas| areas.get(chunk_idx))
            .copied()
            .filter(|&id| id != 0)
    }

    /// Look up terrain height at a Bevy-space (x, z) position across all loaded tiles.
    pub fn height_at(&self, bx: f32, bz: f32) -> Option<f32> {
        self.tiles
            .values()
            .flat_map(|grids| grids.iter().flatten())
            .find_map(|g| sample_chunk_height(g, bx, bz))
    }

    pub fn water_surface_at(&self, bx: f32, bz: f32) -> Option<f32> {
        let (tile_y, tile_x) = bevy_to_tile_coords(bx, bz);
        self.water_layers
            .get(&(tile_y, tile_x))
            .into_iter()
            .flat_map(|layers| layers.iter())
            .filter_map(|layer| sample_water_layer_height(layer, bx, bz))
            .max_by(f32::total_cmp)
    }

    pub fn insert_tile_surfaces(&mut self, tile_y: u32, tile_x: u32, tex_data: &adt::AdtTexData) {
        let mut chunk_effects = vec![None; 256];
        let mut chunk_surfaces = vec![FootstepSurface::Dirt; 256];
        for (idx, chunk) in tex_data.chunk_layers.iter().enumerate().take(256) {
            let effect = dominant_ground_effect_for_chunk(chunk);
            chunk_effects[idx] = effect;
            chunk_surfaces[idx] = dominant_surface_for_chunk(tex_data, chunk, effect);
        }
        self.effects.insert((tile_y, tile_x), chunk_effects);
        self.surfaces.insert((tile_y, tile_x), chunk_surfaces);
    }

    pub fn register_tile(
        &mut self,
        tile_y: u32,
        tile_x: u32,
        adt_data: &adt::AdtData,
        tex_data: Option<&adt::AdtTexData>,
    ) {
        self.insert_tile(tile_y, tile_x, adt_data);
        self.insert_tile_water(tile_y, tile_x, adt_data);
        if let Some(tex_data) = tex_data {
            self.insert_tile_surfaces(tile_y, tile_x, tex_data);
        }
    }

    fn insert_tile_water(&mut self, tile_y: u32, tile_x: u32, adt_data: &adt::AdtData) {
        let Some(water) = adt_data.water.as_ref() else {
            self.water_layers.remove(&(tile_y, tile_x));
            return;
        };
        let mut layers = Vec::new();
        for (chunk_index, chunk) in water.chunks.iter().enumerate() {
            let Some(chunk_pos) = adt_data.chunk_positions.get(chunk_index) else {
                continue;
            };
            for layer in &chunk.layers {
                if !layer_has_water(layer) {
                    continue;
                }
                layers.push(WaterLayerSurface {
                    chunk_origin_wow_x: chunk_pos[1],
                    chunk_origin_wow_y: chunk_pos[0],
                    min_height: layer.min_height,
                    x_offset: layer.x_offset,
                    y_offset: layer.y_offset,
                    width: layer.width,
                    height: layer.height,
                    exists: layer.exists,
                    vertex_heights: layer.vertex_heights.clone(),
                });
            }
        }
        if layers.is_empty() {
            self.water_layers.remove(&(tile_y, tile_x));
        } else {
            self.water_layers.insert((tile_y, tile_x), layers);
        }
    }

    pub fn surface_at(&self, bx: f32, bz: f32) -> Option<FootstepSurface> {
        let (tile_y, tile_x) = bevy_to_tile_coords(bx, bz);
        let chunk_idx = self.chunk_index_at(tile_y, tile_x, bx, bz)?;
        self.surfaces
            .get(&(tile_y, tile_x))
            .and_then(|surfaces| surfaces.get(chunk_idx))
            .copied()
    }

    pub fn ground_effect_at(&self, bx: f32, bz: f32) -> Option<GroundEffectEntry> {
        let (tile_y, tile_x) = bevy_to_tile_coords(bx, bz);
        let chunk_idx = self.chunk_index_at(tile_y, tile_x, bx, bz)?;
        self.effects
            .get(&(tile_y, tile_x))
            .and_then(|effects| effects.get(chunk_idx))
            .copied()
            .flatten()
    }

    fn chunk_index_at(&self, tile_y: u32, tile_x: u32, bx: f32, bz: f32) -> Option<usize> {
        self.tile_chunks(tile_y, tile_x)?
            .iter()
            .flatten()
            .find(|grid| sample_chunk_height(grid, bx, bz).is_some())
            .map(|grid| (grid.index_y * 16 + grid.index_x) as usize)
    }
}

fn sample_water_layer_height(layer: &WaterLayerSurface, bx: f32, bz: f32) -> Option<f32> {
    let borrowed = BorrowedWaterLayerSurface {
        chunk_origin_wow_x: layer.chunk_origin_wow_x,
        chunk_origin_wow_y: layer.chunk_origin_wow_y,
        min_height: layer.min_height,
        x_offset: layer.x_offset,
        y_offset: layer.y_offset,
        width: layer.width,
        height: layer.height,
        exists: layer.exists,
        vertex_heights: &layer.vertex_heights,
    };
    sample_shared_water_layer_height(&borrowed, bx, bz)
}

fn dominant_surface_for_chunk(
    tex_data: &adt::AdtTexData,
    chunk: &adt::ChunkTexLayers,
    effect: Option<GroundEffectEntry>,
) -> FootstepSurface {
    if let Some(effect) = effect
        && let Some(surface) = ground_effects::resolve_ground_effect_surface(effect.effect_id)
    {
        return surface;
    }
    dominant_surface_for_chunk_with_resolver(
        tex_data,
        chunk,
        |_| None,
        game_engine::listfile::lookup_fdid,
    )
}

fn dominant_ground_effect_for_chunk(chunk: &adt::ChunkTexLayers) -> Option<GroundEffectEntry> {
    dominant_ground_effect_for_chunk_with_resolver(chunk, ground_effects::resolve_ground_effect)
}

fn dominant_ground_effect_for_chunk_with_resolver(
    chunk: &adt::ChunkTexLayers,
    resolve_ground_effect: impl Fn(u32) -> Option<GroundEffectEntry>,
) -> Option<GroundEffectEntry> {
    dominant_effect_id(chunk).and_then(resolve_ground_effect)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_adt(
        height_grids: Vec<adt::ChunkHeightGrid>,
        water: Option<adt::AdtWaterData>,
    ) -> adt::AdtData {
        adt::AdtData {
            chunks: Vec::new(),
            blend_mesh: None,
            flight_bounds: None,
            height_grids,
            center_surface: [0.0, 0.0, 0.0],
            chunk_positions: vec![[0.0, 0.0, 0.0]; 256],
            water,
            water_error: None,
        }
    }

    #[test]
    fn terrain_axis_water_sampler_uses_authored_offsets() {
        let layer = WaterLayerSurface {
            chunk_origin_wow_x: 100.0,
            chunk_origin_wow_y: -200.0,
            min_height: 7.0,
            x_offset: 5,
            y_offset: 2,
            width: 1,
            height: 1,
            exists: [1, 0, 0, 0, 0, 0, 0, 0],
            vertex_heights: vec![7.0; 4],
        };
        let actual =
            sample_water_layer_height(&layer, 100.0 - 2.5 * WATER_STEP, 200.0 + 5.5 * WATER_STEP);
        assert_eq!(actual, Some(7.0));
        assert_eq!(
            sample_water_layer_height(&layer, 100.0 - 5.5 * WATER_STEP, 200.0 + 2.5 * WATER_STEP),
            None
        );
    }

    #[test]
    fn terrain_axis_samples_authored_rows_columns_and_center_vertices() {
        let mut heights = [0.0; 145];
        for row in 0..=8 {
            for col in 0..=8 {
                heights[vertex_index(row * 2, col)] = 10.0 * row as f32 + col as f32;
            }
            if row < 8 {
                for col in 0..8 {
                    heights[vertex_index(row * 2 + 1, col)] =
                        10.0 * (row as f32 + 0.5) + col as f32 + 0.5 + 3.0;
                }
            }
        }
        let grid = ChunkHeightGrid {
            index_x: 0,
            index_y: 0,
            origin_x: 100.0,
            origin_z: 200.0,
            base_y: 50.0,
            heights,
        };
        for (row, col, expected) in [(2.0, 5.0, 75.0), (2.5, 5.5, 83.5)] {
            let actual =
                sample_chunk_height(&grid, 100.0 - row * UNIT_SIZE, 200.0 + col * UNIT_SIZE)
                    .unwrap();
            assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
        }
    }

    #[test]
    fn client_heightmap_covers_server_default_spawn() {
        let data = std::fs::read("data/terrain/azeroth_32_48.adt")
            .expect("expected test ADT data/terrain/azeroth_32_48.adt");
        let adt = adt::load_adt(&data).expect("expected ADT to parse");
        let mut heightmap = TerrainHeightmap::default();
        heightmap.insert_tile(32, 48, &adt);

        let [bx, expected_y, bz] = crate::asset::m2::wow_to_bevy(-8949.0, -132.0, 83.0);
        let terrain_y = heightmap
            .height_at(bx, bz)
            .expect("server default spawn should land on loaded client terrain");

        assert!(
            (terrain_y - expected_y).abs() < 10.0,
            "expected terrain near saved spawn height, got terrain_y={terrain_y} expected_y={expected_y}"
        );
    }

    #[test]
    fn dominant_texture_prefers_highest_alpha_layer() {
        let tex = adt::AdtTexData {
            map_flags: adt::MphdFlags::default(),
            texture_amplifier: None,
            texture_fdids: vec![1, 2],
            height_texture_fdids: Vec::new(),
            texture_flags: Vec::new(),
            texture_params: Vec::new(),
            chunk_layers: vec![adt::ChunkTexLayers {
                layers: vec![
                    adt::TextureLayer {
                        texture_index: 0,
                        flags: adt::MclyFlags::default(),
                        effect_id: 0,
                        material_id: 0,
                        alpha_map: None,
                    },
                    adt::TextureLayer {
                        texture_index: 1,
                        flags: adt::MclyFlags::default(),
                        effect_id: 0,
                        material_id: 0,
                        alpha_map: Some(vec![255; 4096]),
                    },
                ],
            }],
        };

        assert_eq!(dominant_texture_fdid(&tex, &tex.chunk_layers[0]), Some(2));
    }

    #[test]
    fn dominant_effect_prefers_highest_alpha_layer() {
        let chunk = adt::ChunkTexLayers {
            layers: vec![
                adt::TextureLayer {
                    texture_index: 0,
                    flags: adt::MclyFlags::default(),
                    effect_id: 5,
                    material_id: 0,
                    alpha_map: None,
                },
                adt::TextureLayer {
                    texture_index: 1,
                    flags: adt::MclyFlags::default(),
                    effect_id: 9,
                    material_id: 0,
                    alpha_map: Some(vec![255; 4096]),
                },
            ],
        };

        assert_eq!(dominant_effect_id(&chunk), Some(9));
    }

    #[test]
    fn dominant_surface_uses_effect_id_override_before_texture_path() {
        let tex = adt::AdtTexData {
            map_flags: adt::MphdFlags::default(),
            texture_amplifier: None,
            texture_fdids: vec![1],
            height_texture_fdids: Vec::new(),
            texture_flags: Vec::new(),
            texture_params: Vec::new(),
            chunk_layers: vec![adt::ChunkTexLayers {
                layers: vec![adt::TextureLayer {
                    texture_index: 0,
                    flags: adt::MclyFlags::default(),
                    effect_id: 42,
                    material_id: 0,
                    alpha_map: None,
                }],
            }],
        };

        let surface = dominant_surface_for_chunk_with_resolver(
            &tex,
            &tex.chunk_layers[0],
            |effect_id| (effect_id == 42).then_some(FootstepSurface::Stone),
            |_| None,
        );

        assert_eq!(surface, FootstepSurface::Stone);
    }

    #[test]
    fn dominant_ground_effect_resolves_from_highest_weight_effect_id() {
        let chunk = adt::ChunkTexLayers {
            layers: vec![
                adt::TextureLayer {
                    texture_index: 0,
                    flags: adt::MclyFlags::default(),
                    effect_id: 7,
                    material_id: 0,
                    alpha_map: Some(vec![16; 4096]),
                },
                adt::TextureLayer {
                    texture_index: 1,
                    flags: adt::MclyFlags::default(),
                    effect_id: 9,
                    material_id: 0,
                    alpha_map: Some(vec![255; 4096]),
                },
            ],
        };

        let entry = dominant_ground_effect_for_chunk_with_resolver(&chunk, |effect_id| {
            (effect_id == 9).then_some(GroundEffectEntry {
                effect_id,
                density: 12,
                terrain_sound_id: 3,
            })
        });

        assert_eq!(
            entry,
            Some(GroundEffectEntry {
                effect_id: 9,
                density: 12,
                terrain_sound_id: 3,
            })
        );
    }

    #[test]
    fn water_surface_query_returns_layer_height_inside_existing_quad() {
        let sample_x = -WATER_STEP * 0.25;
        let sample_z = WATER_STEP * 0.25;
        let (tile_y, tile_x) = bevy_to_tile_coords(sample_x, sample_z);
        let adt = empty_adt(
            Vec::new(),
            Some(adt::AdtWaterData {
                chunks: (0..256)
                    .map(|index| adt::ChunkWater {
                        layers: if index == 0 {
                            vec![adt::WaterLayer {
                                liquid_type: 0,
                                liquid_object: 0,
                                min_height: 5.0,
                                max_height: 5.0,
                                x_offset: 0,
                                y_offset: 0,
                                width: 1,
                                height: 1,
                                exists: [1, 0, 0, 0, 0, 0, 0, 0],
                                vertex_heights: vec![5.0, 5.0, 5.0, 5.0],
                                vertex_uvs: Vec::new(),
                                vertex_depths: Vec::new(),
                                object_vertex_bytes: Vec::new(),
                            }]
                        } else {
                            Vec::new()
                        },
                        attributes: None,
                    })
                    .collect(),
            }),
        );
        let mut heightmap = TerrainHeightmap::default();

        heightmap.register_tile(tile_y, tile_x, &adt, None);

        assert_eq!(heightmap.water_surface_at(sample_x, sample_z), Some(5.0));
    }
}
