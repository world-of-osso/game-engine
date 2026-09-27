//! Bevy-free authored ADT chunk height sampling.

use crate::asset::adt_format::adt::{CHUNK_SIZE, ChunkHeightGrid, UNIT_SIZE, vertex_index};

/// Convert Bevy world X/Z to ADT filename (row, column), clamped to the map grid.
pub fn bevy_to_tile_coords(bx: f32, bz: f32) -> (u32, u32) {
    let tile_size = CHUNK_SIZE * 16.0;
    let center = 32.0 * tile_size;
    let row = ((center + bz) / tile_size).floor() as i32;
    let col = ((center - bx) / tile_size).floor() as i32;
    (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
}

/// Try to get height from a single chunk. Returns None if (bx, bz) is outside this chunk.
pub fn sample_chunk_height(g: &ChunkHeightGrid, bx: f32, bz: f32) -> Option<f32> {
    let local_x = g.origin_x - bx;
    let local_z = bz - g.origin_z;
    if !(0.0..CHUNK_SIZE).contains(&local_x) || !(0.0..CHUNK_SIZE).contains(&local_z) {
        return None;
    }
    let col = (local_z / UNIT_SIZE).floor() as usize;
    let row = (local_x / UNIT_SIZE).floor() as usize;
    let col = col.min(7);
    let row = row.min(7);
    let frac_x = (local_z - col as f32 * UNIT_SIZE) / UNIT_SIZE;
    let frac_z = (local_x - row as f32 * UNIT_SIZE) / UNIT_SIZE;
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
