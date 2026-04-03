use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

pub use super::adt_format::adt::{CHUNK_SIZE, ChunkHeightGrid, ChunkIter, UNIT_SIZE, vertex_index};
pub use super::adt_format::adt_tex::{
    AdtTexData, AdtWaterData, ChunkTexLayers, ChunkWater, TextureLayer, WaterLayer, load_adt_tex0,
    parse_mh2o,
};
use super::m2::wow_to_bevy;

pub struct McnkMesh {
    pub mesh: Mesh,
    pub index_x: u32,
    pub index_y: u32,
}

pub struct AdtData {
    pub chunks: Vec<McnkMesh>,
    pub height_grids: Vec<ChunkHeightGrid>,
    pub center_surface: [f32; 3],
    pub chunk_positions: Vec<[f32; 3]>,
    pub water: Option<AdtWaterData>,
    pub water_error: Option<String>,
}

type McnkGeometry = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>);
type WaterGeometry = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>);

fn build_mcnk_geometry(
    chunk: &super::adt_format::adt::McnkData,
    tile_coords: Option<(u32, u32)>,
) -> McnkGeometry {
    let mut positions = Vec::with_capacity(145);
    let mut normals_out = Vec::with_capacity(145);
    let mut uvs = Vec::with_capacity(145);
    let (origin_x, origin_z) = super::adt_format::adt::chunk_origin_bevy(chunk, tile_coords);
    for i in 0..145 {
        let pair = i / 17;
        let rem = i % 17;
        let (grid_row, col) = if rem < 9 {
            (pair * 2, rem)
        } else {
            (pair * 2 + 1, rem - 9)
        };
        positions.push(super::adt_format::adt::vertex_position_from_origin(
            grid_row,
            col,
            origin_x,
            origin_z,
            chunk.pos[2],
            &chunk.heights,
        ));
        normals_out.push(chunk.normals[i]);
        let uv = if grid_row.is_multiple_of(2) {
            [col as f32 / 8.0, (grid_row / 2) as f32 / 8.0]
        } else {
            [
                (col as f32 + 0.5) / 8.0,
                ((grid_row / 2) as f32 + 0.5) / 8.0,
            ]
        };
        uvs.push(uv);
    }
    (positions, normals_out, uvs, build_mcnk_indices())
}

fn build_mcnk_indices() -> Vec<u32> {
    let mut indices = Vec::with_capacity(8 * 8 * 4 * 3);
    for qr in 0..8usize {
        for qc in 0..8usize {
            let tl = vertex_index(qr * 2, qc) as u32;
            let tr = vertex_index(qr * 2, qc + 1) as u32;
            let bl = vertex_index(qr * 2 + 2, qc) as u32;
            let br = vertex_index(qr * 2 + 2, qc + 1) as u32;
            let center = vertex_index(qr * 2 + 1, qc) as u32;
            indices.extend_from_slice(&[tl, tr, center]);
            indices.extend_from_slice(&[tr, br, center]);
            indices.extend_from_slice(&[br, bl, center]);
            indices.extend_from_slice(&[bl, tl, center]);
        }
    }
    indices
}

fn build_mcnk_mesh(
    chunk: &super::adt_format::adt::McnkData,
    tile_coords: Option<(u32, u32)>,
) -> Mesh {
    let (positions, normals, uvs, indices) = build_mcnk_geometry(chunk, tile_coords);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn build_chunks(
    parsed: &[super::adt_format::adt::McnkData],
    tile_coords: Option<(u32, u32)>,
) -> Vec<McnkMesh> {
    parsed
        .iter()
        .map(|chunk| McnkMesh {
            mesh: build_mcnk_mesh(chunk, tile_coords),
            index_x: chunk.index_x,
            index_y: chunk.index_y,
        })
        .collect()
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

fn emit_water_quad(
    chunk_pos: [f32; 3],
    layer: &WaterLayer,
    row: usize,
    col: usize,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
) {
    let abs_row = layer.y_offset as usize + row;
    let abs_col = layer.x_offset as usize + col;
    for (dr, dc) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let r = abs_row + dr;
        let c = abs_col + dc;
        let wz = water_height(layer, row + dr, col + dc);
        let wx = chunk_pos[1] - c as f32 * WATER_STEP;
        let wy = chunk_pos[0] - r as f32 * WATER_STEP;
        positions.push(wow_to_bevy(wx, wy, wz));
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([c as f32 / 8.0, r as f32 / 8.0]);
    }
}

fn build_water_geometry(chunk_pos: [f32; 3], layer: &WaterLayer) -> WaterGeometry {
    let w = layer.width as usize;
    let h = layer.height as usize;
    let max_quads = w * h;
    let mut positions = Vec::with_capacity(max_quads * 4);
    let mut normals = Vec::with_capacity(max_quads * 4);
    let mut uvs = Vec::with_capacity(max_quads * 4);
    let mut indices = Vec::with_capacity(max_quads * 6);
    for row in 0..h {
        for col in 0..w {
            if !quad_exists(layer, row, col) {
                continue;
            }
            let base_idx = positions.len() as u32;
            emit_water_quad(
                chunk_pos,
                layer,
                row,
                col,
                &mut positions,
                &mut normals,
                &mut uvs,
            );
            indices.extend_from_slice(&[
                base_idx,
                base_idx + 1,
                base_idx + 2,
                base_idx + 2,
                base_idx + 1,
                base_idx + 3,
            ]);
        }
    }
    (positions, normals, uvs, indices)
}

pub fn build_water_mesh(chunk_pos: [f32; 3], layer: &WaterLayer) -> Mesh {
    let (positions, normals, uvs, indices) = build_water_geometry(chunk_pos, layer);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn load_adt(data: &[u8]) -> Result<AdtData, String> {
    load_adt_inner(super::adt_format::adt::load_adt_parsed(data)?, None)
}

pub fn load_adt_for_tile(data: &[u8], tile_y: u32, tile_x: u32) -> Result<AdtData, String> {
    load_adt_inner(
        super::adt_format::adt::load_adt_for_tile_parsed(data, tile_y, tile_x)?,
        Some((tile_y, tile_x)),
    )
}

#[cfg(test)]
pub(crate) fn load_adt_raw(data: &[u8]) -> Result<AdtData, String> {
    load_adt_inner(super::adt_format::adt::load_adt_raw(data)?, None)
}

fn load_adt_inner(
    parsed: super::adt_format::adt::ParsedAdtData,
    tile_coords: Option<(u32, u32)>,
) -> Result<AdtData, String> {
    Ok(AdtData {
        chunks: build_chunks(&parsed.chunks, tile_coords),
        height_grids: parsed.height_grids,
        center_surface: parsed.center_surface,
        chunk_positions: parsed.chunk_positions,
        water: parsed.water,
        water_error: parsed.water_error,
    })
}
