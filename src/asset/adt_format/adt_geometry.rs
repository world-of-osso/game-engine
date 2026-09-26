//! Bevy-free MCNK mesh geometry in the Bevy world coordinate basis.
use super::adt::{self, vertex_index};

pub struct Geometry {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Original Bevy triangle winding. Adapt winding in the native renderer.
    pub indices: Vec<u32>,
}

pub struct GeometryChunk<'a> {
    pub index_x: u32,
    pub index_y: u32,
    pub position: [f32; 3],
    pub heights: &'a [f32; 145],
    pub normals: &'a [[f32; 3]; 145],
    pub holes_low_res: u16,
    pub holes_high_res: Option<u64>,
}

fn decode_mcnk_vertex_grid(index: usize) -> (usize, usize) {
    let pair = index / 17;
    let rem = index % 17;
    if rem < 9 {
        (pair * 2, rem)
    } else {
        (pair * 2 + 1, rem - 9)
    }
}

fn terrain_vertex_uv(grid_row: usize, col: usize) -> [f32; 2] {
    if grid_row.is_multiple_of(2) {
        [col as f32 / 8.0, (grid_row / 2) as f32 / 8.0]
    } else {
        [
            (col as f32 + 0.5) / 8.0,
            ((grid_row / 2) as f32 + 0.5) / 8.0,
        ]
    }
}

type McnkVertexData = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>);

fn collect_mcnk_vertices(
    chunk: &GeometryChunk<'_>,
    tile_coords: Option<(u32, u32)>,
) -> McnkVertexData {
    let mut positions = Vec::with_capacity(145);
    let mut normals = Vec::with_capacity(145);
    let mut uvs = Vec::with_capacity(145);
    let (origin_x, origin_z) =
        adt::chunk_origin_from_parts(chunk.index_x, chunk.index_y, chunk.position, tile_coords);

    for i in 0..145 {
        let (grid_row, col) = decode_mcnk_vertex_grid(i);
        positions.push(adt::vertex_position_from_origin(
            grid_row,
            col,
            origin_x,
            origin_z,
            chunk.position[2],
            chunk.heights,
        ));
        normals.push(chunk.normals[i]);
        uvs.push(terrain_vertex_uv(grid_row, col));
    }

    (positions, normals, uvs)
}

pub fn build_mcnk_geometry(chunk: GeometryChunk<'_>, tile_coords: Option<(u32, u32)>) -> Geometry {
    let (positions, normals, uvs) = collect_mcnk_vertices(&chunk, tile_coords);
    Geometry {
        positions,
        normals,
        uvs,
        indices: build_mcnk_indices(chunk.holes_low_res, chunk.holes_high_res),
    }
}

pub(crate) fn build_mcnk_indices(holes_low_res: u16, holes_high_res: Option<u64>) -> Vec<u32> {
    let mut indices = Vec::with_capacity(8 * 8 * 4 * 3);
    for qr in 0..8usize {
        for qc in 0..8usize {
            if terrain_hole_at(holes_low_res, holes_high_res, qc, qr) {
                continue;
            }
            let tl = vertex_index(qr * 2, qc) as u32;
            let tr = vertex_index(qr * 2, qc + 1) as u32;
            let bl = vertex_index(qr * 2 + 2, qc) as u32;
            let br = vertex_index(qr * 2 + 2, qc + 1) as u32;
            let center = vertex_index(qr * 2 + 1, qc) as u32;
            indices.extend_from_slice(&[tl, center, tr]);
            indices.extend_from_slice(&[tr, center, br]);
            indices.extend_from_slice(&[br, center, bl]);
            indices.extend_from_slice(&[bl, center, tl]);
        }
    }
    indices
}

pub(crate) fn terrain_hole_at(
    holes_low_res: u16,
    holes_high_res: Option<u64>,
    col: usize,
    row: usize,
) -> bool {
    if let Some(mask) = holes_high_res {
        return high_res_hole_at(mask, col, row);
    }
    low_res_hole_at(holes_low_res, col / 2, row / 2)
}

pub(crate) fn low_res_hole_at(holes_low_res: u16, col: usize, row: usize) -> bool {
    if col >= 4 || row >= 4 {
        return false;
    }
    let bit = row * 4 + col;
    ((holes_low_res >> bit) & 1) != 0
}

pub(crate) fn high_res_hole_at(holes_high_res: u64, col: usize, row: usize) -> bool {
    if col >= 8 || row >= 8 {
        return false;
    }
    let bit = row * 8 + col;
    ((holes_high_res >> bit) & 1) != 0
}
