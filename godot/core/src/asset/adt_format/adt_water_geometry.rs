//! Bevy-free MH2O water mesh geometry in the Bevy world coordinate basis.
use super::adt::CHUNK_SIZE;
use super::adt_tex::WaterLayer;

pub struct WaterGeometry {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// RGB is white; alpha is authored vertex depth normalized to 0..1.
    pub colors: Vec<[f32; 4]>,
    /// Original Bevy triangle winding. Adapt winding in the native renderer.
    pub indices: Vec<u32>,
}

struct WaterMeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
}

const WATER_STEP: f32 = CHUNK_SIZE / 8.0;

fn quad_exists(layer: &WaterLayer, row: usize, col: usize) -> bool {
    row < 8 && col < 8 && ((layer.exists[row] >> col) & 1 != 0)
}

fn water_height(layer: &WaterLayer, vert_row: usize, vert_col: usize) -> f32 {
    if layer.vertex_heights.is_empty() {
        return layer.min_height;
    }
    let w = layer.width as usize + 1;
    layer
        .vertex_heights
        .get(vert_row * w + vert_col)
        .copied()
        .unwrap_or(layer.min_height)
}

fn water_depth(layer: &WaterLayer, vert_row: usize, vert_col: usize) -> f32 {
    if layer.vertex_depths.is_empty() {
        return 1.0;
    }
    let w = layer.width as usize + 1;
    let depth = layer
        .vertex_depths
        .get(vert_row * w + vert_col)
        .copied()
        .unwrap_or(u8::MAX);
    f32::from(depth) / 255.0
}

fn emit_water_quad(
    chunk_pos: [f32; 3],
    layer: &WaterLayer,
    row: usize,
    col: usize,
    buffers: &mut WaterMeshBuffers,
) {
    let abs_row = layer.y_offset as usize + row;
    let abs_col = layer.x_offset as usize + col;
    for (dr, dc) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let r = abs_row + dr;
        let c = abs_col + dc;
        let wz = water_height(layer, row + dr, col + dc);
        let depth = water_depth(layer, row + dr, col + dc);
        let wx = chunk_pos[1] - r as f32 * WATER_STEP;
        let wy = chunk_pos[0] - c as f32 * WATER_STEP;
        buffers.positions.push([wx, wz, -wy]);
        buffers.normals.push([0.0, 1.0, 0.0]);
        buffers.uvs.push([c as f32 / 8.0, r as f32 / 8.0]);
        buffers.colors.push([1.0, 1.0, 1.0, depth]);
    }
}

fn create_water_mesh_buffers(max_quads: usize) -> WaterMeshBuffers {
    WaterMeshBuffers {
        positions: Vec::with_capacity(max_quads * 4),
        normals: Vec::with_capacity(max_quads * 4),
        uvs: Vec::with_capacity(max_quads * 4),
        colors: Vec::with_capacity(max_quads * 4),
    }
}

fn append_water_quad(
    chunk_pos: [f32; 3],
    layer: &WaterLayer,
    row: usize,
    col: usize,
    buffers: &mut WaterMeshBuffers,
    indices: &mut Vec<u32>,
) {
    let base_idx = buffers.positions.len() as u32;
    emit_water_quad(chunk_pos, layer, row, col, buffers);
    indices.extend_from_slice(&[
        base_idx,
        base_idx + 2,
        base_idx + 1,
        base_idx + 2,
        base_idx + 3,
        base_idx + 1,
    ]);
}

pub fn build_water_geometry(chunk_pos: [f32; 3], layer: &WaterLayer) -> WaterGeometry {
    let w = layer.width as usize;
    let h = layer.height as usize;
    let max_quads = w * h;
    let mut buffers = create_water_mesh_buffers(max_quads);
    let mut indices = Vec::with_capacity(max_quads * 6);
    for row in 0..h {
        for col in 0..w {
            if !quad_exists(layer, row, col) {
                continue;
            }
            append_water_quad(chunk_pos, layer, row, col, &mut buffers, &mut indices);
        }
    }
    WaterGeometry {
        positions: buffers.positions,
        normals: buffers.normals,
        uvs: buffers.uvs,
        colors: buffers.colors,
        indices,
    }
}
