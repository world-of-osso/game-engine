//! Queryable heightmap for terrain collision across multiple tiles.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::asset::adt::{self, CHUNK_SIZE, ChunkHeightGrid, UNIT_SIZE, vertex_index};
use crate::rendering::ground_effects::{self, GroundEffectEntry};
use crate::sound_footsteps::{FootstepSurface, classify_surface_from_texture_path};
use crate::terrain_tile::bevy_to_tile_coords;

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
        self.effects.remove(&(tile_y, tile_x));
        self.surfaces.remove(&(tile_y, tile_x));
        self.water_layers.remove(&(tile_y, tile_x));
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

fn layer_has_water(layer: &adt::WaterLayer) -> bool {
    (0..layer.height as usize).any(|row| {
        (0..layer.width as usize)
            .any(|col| row < 8 && col < 8 && ((layer.exists[row] >> col) & 1 != 0))
    })
}

fn sample_water_layer_height(layer: &WaterLayerSurface, bx: f32, bz: f32) -> Option<f32> {
    if layer.width == 0 || layer.height == 0 {
        return None;
    }
    let wow_x = bx;
    let wow_y = -bz;
    let abs_col_f = (layer.chunk_origin_wow_x - wow_x) / WATER_STEP;
    let abs_row_f = (layer.chunk_origin_wow_y - wow_y) / WATER_STEP;
    let x_min = f32::from(layer.x_offset);
    let y_min = f32::from(layer.y_offset);
    let x_max = x_min + f32::from(layer.width);
    let y_max = y_min + f32::from(layer.height);
    if abs_col_f < x_min || abs_col_f >= x_max || abs_row_f < y_min || abs_row_f >= y_max {
        return None;
    }

    let abs_col = abs_col_f.floor() as usize;
    let abs_row = abs_row_f.floor() as usize;
    let col = abs_col.checked_sub(layer.x_offset as usize)?;
    let row = abs_row.checked_sub(layer.y_offset as usize)?;
    if !layer_quad_exists(layer, row, col) {
        return None;
    }

    let fx = abs_col_f - abs_col as f32;
    let fz = abs_row_f - abs_row as f32;
    Some(interpolate_water_height(layer, row, col, fx, fz))
}

fn layer_quad_exists(layer: &WaterLayerSurface, row: usize, col: usize) -> bool {
    row < 8 && col < 8 && ((layer.exists[row] >> col) & 1 != 0)
}

fn interpolate_water_height(
    layer: &WaterLayerSurface,
    row: usize,
    col: usize,
    fx: f32,
    fz: f32,
) -> f32 {
    let top =
        water_vertex_height(layer, row, col).lerp(water_vertex_height(layer, row, col + 1), fx);
    let bottom = water_vertex_height(layer, row + 1, col)
        .lerp(water_vertex_height(layer, row + 1, col + 1), fx);
    top.lerp(bottom, fz)
}

fn water_vertex_height(layer: &WaterLayerSurface, row: usize, col: usize) -> f32 {
    if layer.vertex_heights.is_empty() {
        return layer.min_height;
    }
    let width = layer.width as usize + 1;
    layer
        .vertex_heights
        .get(row * width + col)
        .copied()
        .unwrap_or(layer.min_height)
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
    dominant_surface_for_chunk_with_resolver(tex_data, chunk, |_| None)
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

fn dominant_surface_for_chunk_with_resolver(
    tex_data: &adt::AdtTexData,
    chunk: &adt::ChunkTexLayers,
    resolve_effect_surface: impl Fn(u32) -> Option<FootstepSurface>,
) -> FootstepSurface {
    if let Some(effect_id) = dominant_effect_id(chunk)
        && let Some(surface) = resolve_effect_surface(effect_id)
    {
        return surface;
    }
    let Some(fdid) = dominant_texture_fdid(tex_data, chunk) else {
        return FootstepSurface::Dirt;
    };
    let Some(path) = game_engine::listfile::lookup_fdid(fdid) else {
        return FootstepSurface::Dirt;
    };
    classify_surface_from_texture_path(path)
}

fn dominant_effect_id(chunk: &adt::ChunkTexLayers) -> Option<u32> {
    let mut best = None;
    let mut best_weight = 0u64;
    for (layer_idx, layer) in chunk.layers.iter().enumerate() {
        if layer.effect_id == 0 {
            continue;
        }
        let weight = if layer_idx == 0 {
            1_000_000
        } else {
            layer
                .alpha_map
                .as_ref()
                .map(|alpha| alpha.iter().map(|v| u64::from(*v)).sum())
                .unwrap_or_default()
        };
        if weight >= best_weight {
            best = Some(layer.effect_id);
            best_weight = weight;
        }
    }
    best
}

fn dominant_texture_fdid(tex_data: &adt::AdtTexData, chunk: &adt::ChunkTexLayers) -> Option<u32> {
    let mut best = None;
    let mut best_weight = 0u64;
    for (layer_idx, layer) in chunk.layers.iter().enumerate() {
        let fdid = tex_data
            .texture_fdids
            .get(layer.texture_index as usize)
            .copied()?;
        let weight = if layer_idx == 0 {
            1_000_000
        } else {
            layer
                .alpha_map
                .as_ref()
                .map(|alpha| alpha.iter().map(|v| u64::from(*v)).sum())
                .unwrap_or_default()
        };
        if weight >= best_weight {
            best = Some(fdid);
            best_weight = weight;
        }
    }
    best
}

/// Try to get height from a single chunk. Returns None if (bx, bz) is outside this chunk.
pub(crate) fn sample_chunk_height(g: &ChunkHeightGrid, bx: f32, bz: f32) -> Option<f32> {
    let local_x = g.origin_x - bx;
    let local_z = bz - g.origin_z;
    if !(0.0..CHUNK_SIZE).contains(&local_x) || !(0.0..CHUNK_SIZE).contains(&local_z) {
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
fn interpolate_quad_height(g: &ChunkHeightGrid, row: usize, col: usize, fx: f32, fz: f32) -> f32 {
    let h = |idx: usize| g.base_y + g.heights[idx];
    let tl = h(vertex_index(row * 2, col));
    let tr = h(vertex_index(row * 2, col + 1));
    let bl = h(vertex_index(row * 2 + 2, col));
    let br = h(vertex_index(row * 2 + 2, col + 1));
    let center = h(vertex_index(row * 2 + 1, col));

    let dx = fx - 0.5;
    let dz = fz - 0.5;
    let (ha, hb, ax, az, bxx, bz) = if dz.abs() >= dx.abs() {
        if dz < 0.0 {
            (tl, tr, 0.0, 0.0, 1.0, 0.0)
        } else {
            (br, bl, 1.0, 1.0, 0.0, 1.0)
        }
    } else if dx > 0.0 {
        (tr, br, 1.0, 0.0, 1.0, 1.0)
    } else {
        (bl, tl, 0.0, 1.0, 0.0, 0.0)
    };
    barycentric_height(fx, fz, [ax, az, ha], [bxx, bz, hb], [0.5, 0.5, center])
}

/// Barycentric interpolation of height at (px, pz) within triangle (A, B, C).
/// Each vertex is `[x, z, height]`.
fn barycentric_height(px: f32, pz: f32, a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let [ax, az, ha] = a;
    let [bx, bz, hb] = b;
    let [cx, cz, hc] = c;
    let det = (bz - cz) * (ax - cx) + (cx - bx) * (az - cz);
    if det.abs() < 1e-10 {
        return (ha + hb + hc) / 3.0;
    }
    let wa = ((bz - cz) * (px - cx) + (cx - bx) * (pz - cz)) / det;
    let wb = ((cz - az) * (px - cx) + (ax - cx) * (pz - cz)) / det;
    let wc = 1.0 - wa - wb;
    wa * ha + wb * hb + wc * hc
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

        let surface =
            dominant_surface_for_chunk_with_resolver(&tex, &tex.chunk_layers[0], |effect_id| {
                (effect_id == 42).then_some(FootstepSurface::Stone)
            });

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
