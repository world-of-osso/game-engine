//! Queryable heightmap for terrain collision across multiple tiles.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::asset::adt::{self, CHUNK_SIZE, ChunkHeightGrid, UNIT_SIZE, vertex_index};

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
        self.tiles
            .values()
            .flat_map(|grids| grids.iter().flatten())
            .find_map(|g| sample_chunk_height(g, bx, bz))
    }
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
}
