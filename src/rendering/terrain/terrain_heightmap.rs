//! Queryable heightmap for terrain collision across multiple tiles.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::asset::adt::{self, CHUNK_SIZE, ChunkHeightGrid, UNIT_SIZE, vertex_index};
use crate::sound_footsteps::{FootstepSurface, classify_surface_from_texture_path};
use crate::terrain_tile::bevy_to_tile_coords;

/// Queryable heightmap for terrain collision across multiple tiles.
#[derive(Resource, Default)]
pub struct TerrainHeightmap {
    /// Per-tile grids: (tile_y, tile_x) → 256 chunk height grids.
    tiles: HashMap<(u32, u32), Vec<Option<ChunkHeightGrid>>>,
    /// Per-tile dominant surface class for each chunk.
    surfaces: HashMap<(u32, u32), Vec<FootstepSurface>>,
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
        self.surfaces.remove(&(tile_y, tile_x));
    }

    /// Look up terrain height at a Bevy-space (x, z) position across all loaded tiles.
    pub fn height_at(&self, bx: f32, bz: f32) -> Option<f32> {
        self.tiles
            .values()
            .flat_map(|grids| grids.iter().flatten())
            .find_map(|g| sample_chunk_height(g, bx, bz))
    }

    pub fn insert_tile_surfaces(&mut self, tile_y: u32, tile_x: u32, tex_data: &adt::AdtTexData) {
        let mut chunk_surfaces = vec![FootstepSurface::Dirt; 256];
        for (idx, chunk) in tex_data.chunk_layers.iter().enumerate().take(256) {
            chunk_surfaces[idx] = dominant_surface_for_chunk(tex_data, chunk);
        }
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
        if let Some(tex_data) = tex_data {
            self.insert_tile_surfaces(tile_y, tile_x, tex_data);
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

    fn chunk_index_at(&self, tile_y: u32, tile_x: u32, bx: f32, bz: f32) -> Option<usize> {
        self.tile_chunks(tile_y, tile_x)?
            .iter()
            .flatten()
            .find(|grid| sample_chunk_height(grid, bx, bz).is_some())
            .map(|grid| (grid.index_y * 16 + grid.index_x) as usize)
    }
}

fn dominant_surface_for_chunk(
    tex_data: &adt::AdtTexData,
    chunk: &adt::ChunkTexLayers,
) -> FootstepSurface {
    let Some(fdid) = dominant_texture_fdid(tex_data, chunk) else {
        return FootstepSurface::Dirt;
    };
    let Some(path) = game_engine::listfile::lookup_fdid(fdid) else {
        return FootstepSurface::Dirt;
    };
    classify_surface_from_texture_path(path)
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
            texture_fdids: vec![1, 2],
            chunk_layers: vec![adt::ChunkTexLayers {
                layers: vec![
                    adt::TextureLayer {
                        texture_index: 0,
                        flags: adt::MclyFlags::default(),
                        effect_id: 0,
                        alpha_map: None,
                    },
                    adt::TextureLayer {
                        texture_index: 1,
                        flags: adt::MclyFlags::default(),
                        effect_id: 0,
                        alpha_map: Some(vec![255; 4096]),
                    },
                ],
            }],
        };

        assert_eq!(dominant_texture_fdid(&tex, &tex.chunk_layers[0]), Some(2));
    }
}
