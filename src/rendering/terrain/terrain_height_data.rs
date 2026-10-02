//! Bevy-free authored ADT chunk height sampling.

use crate::asset::adt_format::adt::{CHUNK_SIZE, ChunkHeightGrid, UNIT_SIZE, vertex_index};
use crate::asset::adt_format::adt_tex::WaterLayer;
use glam::FloatExt;

const WATER_STEP: f32 = CHUNK_SIZE / 8.0;

/// A borrowed authored water layer at its MCNK origin. Vertex data stays in the parsed tile.
pub struct WaterLayerSurface<'a> {
    pub chunk_origin_wow_x: f32,
    pub chunk_origin_wow_y: f32,
    pub min_height: f32,
    pub x_offset: u8,
    pub y_offset: u8,
    pub width: u8,
    pub height: u8,
    pub exists: [u8; 8],
    pub vertex_heights: &'a [f32],
}

impl<'a> WaterLayerSurface<'a> {
    pub fn from_layer(layer: &'a WaterLayer, chunk_position: [f32; 3]) -> Self {
        Self {
            chunk_origin_wow_x: chunk_position[1],
            chunk_origin_wow_y: chunk_position[0],
            min_height: layer.min_height,
            x_offset: layer.x_offset,
            y_offset: layer.y_offset,
            width: layer.width,
            height: layer.height,
            exists: layer.exists,
            vertex_heights: &layer.vertex_heights,
        }
    }
}

/// Whether any authored quad in this layer is present.
pub fn layer_has_water(layer: &WaterLayer) -> bool {
    (0..layer.height as usize).any(|row| {
        (0..layer.width as usize)
            .any(|col| row < 8 && col < 8 && ((layer.exists[row] >> col) & 1 != 0))
    })
}

/// Sample a water layer at Bevy-space (x, z). Layer edges are half-open.
pub fn sample_water_layer_height(layer: &WaterLayerSurface<'_>, bx: f32, bz: f32) -> Option<f32> {
    if layer.width == 0 || layer.height == 0 {
        return None;
    }
    let wow_x = bx;
    let wow_y = -bz;
    let abs_col_f = (layer.chunk_origin_wow_y - wow_y) / WATER_STEP;
    let abs_row_f = (layer.chunk_origin_wow_x - wow_x) / WATER_STEP;
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

fn layer_quad_exists(layer: &WaterLayerSurface<'_>, row: usize, col: usize) -> bool {
    row < 8 && col < 8 && ((layer.exists[row] >> col) & 1 != 0)
}

fn interpolate_water_height(
    layer: &WaterLayerSurface<'_>,
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

fn water_vertex_height(layer: &WaterLayerSurface<'_>, row: usize, col: usize) -> f32 {
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

/// Convert Bevy world X/Z to ADT filename (row, column), clamped to the map grid.
pub fn bevy_to_tile_coords(bx: f32, bz: f32) -> (u32, u32) {
    let tile_size = CHUNK_SIZE * 16.0;
    let center = 32.0 * tile_size;
    let row = ((center + bz) / tile_size).floor() as i32;
    let col = ((center - bx) / tile_size).floor() as i32;
    (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
}

/// Tile `(row, col)` and chunk `(index_x, index_y)` (MCNK `IndexX` along +z, `IndexY`
/// along -x) containing Bevy (x, z), by index arithmetic from the world coordinate as
/// TrinityCore `GridMap::getHeight` locates a cell (`CENTER_GRID_ID - coord /
/// SIZE_OF_GRIDS`). Authored MCNK positions leave sub-millimetre gaps between
/// neighbouring chunks (azeroth 31_48 ends at z -0.00068), so containment tests miss
/// positions on those edges.
pub fn bevy_to_chunk_coords(bx: f32, bz: f32) -> ((u32, u32), (u32, u32)) {
    let tile_size = CHUNK_SIZE * 16.0;
    let center = 32.0 * tile_size;
    let along = |offset: f32| {
        let tiles = offset / tile_size;
        let tile = tiles.floor().clamp(0.0, 63.0);
        let chunk = ((tiles - tile) * 16.0).floor().clamp(0.0, 15.0);
        (tile as u32, chunk as u32)
    };
    let (row, index_x) = along(center + bz);
    let (col, index_y) = along(center - bx);
    ((row, col), (index_x, index_y))
}

/// Height of Bevy (x, z) in the chunk that index arithmetic places it in
/// (`bevy_to_chunk_coords`), clamped onto the chunk's edges.
pub fn sample_located_chunk_height(g: &ChunkHeightGrid, bx: f32, bz: f32) -> f32 {
    let local_x = (g.origin_x - bx).clamp(0.0, CHUNK_SIZE);
    let local_z = (bz - g.origin_z).clamp(0.0, CHUNK_SIZE);
    let col = ((local_z / UNIT_SIZE).floor() as usize).min(7);
    let row = ((local_x / UNIT_SIZE).floor() as usize).min(7);
    let frac_x = (local_z - col as f32 * UNIT_SIZE) / UNIT_SIZE;
    let frac_z = (local_x - row as f32 * UNIT_SIZE) / UNIT_SIZE;
    interpolate_quad_height(g, row, col, frac_x, frac_z)
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
