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
    pub shadow_map: Option<[u8; 512]>,
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

#[cfg(test)]
fn mccv_color_to_shader_color(bgra: [u8; 4]) -> [f32; 4] {
    [
        f32::from(bgra[2]) / 127.0,
        f32::from(bgra[1]) / 127.0,
        f32::from(bgra[0]) / 127.0,
        f32::from(bgra[3]) / 255.0,
    ]
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

fn build_mcnk_geometry(
    chunk: &super::adt_format::adt::McnkData,
    tile_coords: Option<(u32, u32)>,
) -> McnkGeometry {
    let mut positions = Vec::with_capacity(145);
    let mut normals_out = Vec::with_capacity(145);
    let mut uvs = Vec::with_capacity(145);
    let (origin_x, origin_z) = super::adt_format::adt::chunk_origin_bevy(chunk, tile_coords);
    for i in 0..145 {
        let (grid_row, col) = decode_mcnk_vertex_grid(i);
        positions.push(super::adt_format::adt::vertex_position_from_origin(
            grid_row,
            col,
            origin_x,
            origin_z,
            chunk.pos[2],
            &chunk.heights,
        ));
        normals_out.push(chunk.normals[i]);
        uvs.push(terrain_vertex_uv(grid_row, col));
    }
    let holes_high_res = if chunk.flags.high_res_holes {
        chunk.holes_high_res
    } else {
        None
    };
    (
        positions,
        normals_out,
        uvs,
        build_mcnk_indices(chunk.holes_low_res, holes_high_res),
    )
}

fn build_mcnk_indices(holes_low_res: u16, holes_high_res: Option<u64>) -> Vec<u32> {
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
            indices.extend_from_slice(&[tl, tr, center]);
            indices.extend_from_slice(&[tr, br, center]);
            indices.extend_from_slice(&[br, bl, center]);
            indices.extend_from_slice(&[bl, tl, center]);
        }
    }
    indices
}

fn terrain_hole_at(
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

fn low_res_hole_at(holes_low_res: u16, col: usize, row: usize) -> bool {
    if col >= 4 || row >= 4 {
        return false;
    }
    let bit = row * 4 + col;
    ((holes_low_res >> bit) & 1) != 0
}

fn high_res_hole_at(holes_high_res: u64, col: usize, row: usize) -> bool {
    if col >= 8 || row >= 8 {
        return false;
    }
    let bit = row * 8 + col;
    ((holes_high_res >> bit) & 1) != 0
}

fn build_mcnk_mesh(
    chunk: &super::adt_format::adt::McnkData,
    tile_coords: Option<(u32, u32)>,
) -> Mesh {
    let (positions, normals, uvs, indices) = build_mcnk_geometry(chunk, tile_coords);
    let colors = combine_mcnk_vertex_colors(chunk.vertex_colors, chunk.vertex_lighting);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn combine_mcnk_vertex_colors<const N: usize>(
    vertex_colors: [[f32; 4]; N],
    vertex_lighting: Option<[[f32; 4]; N]>,
) -> Vec<[f32; 4]> {
    match vertex_lighting {
        Some(vertex_lighting) => vertex_colors
            .into_iter()
            .zip(vertex_lighting)
            .map(|(base, lighting)| {
                [
                    base[0] * lighting[0],
                    base[1] * lighting[1],
                    base[2] * lighting[2],
                    base[3],
                ]
            })
            .collect(),
        None => vertex_colors.to_vec(),
    }
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
            shadow_map: chunk.shadow_map,
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

#[cfg(test)]
mod tests {
    use bevy::mesh::{Mesh, VertexAttributeValues};

    use super::{build_mcnk_indices, build_mcnk_mesh, mccv_color_to_shader_color};

    const FULL_LOW_RES_HOLE_MASK: u16 = u16::MAX;

    #[test]
    fn mcnk_indices_emit_all_quads_without_holes() {
        let indices = build_mcnk_indices(0, None);
        assert_eq!(indices.len(), 8 * 8 * 4 * 3);
    }

    #[test]
    fn mcnk_indices_skip_all_quads_in_low_res_hole_block() {
        let indices = build_mcnk_indices(1 << 5, None);
        assert_eq!(indices.len(), (8 * 8 - 4) * 4 * 3);
        assert_eq!(indices.chunks_exact(3).len(), (8 * 8 - 4) * 4);

        let skipped_cells = [(2usize, 2usize), (3, 2), (2, 3), (3, 3)];
        for (quad_col, quad_row) in skipped_cells {
            let base = quad_index_base(quad_row, quad_col);
            assert!(!indices.contains(&base.center));
        }

        let preserved = quad_index_base(1, 2);
        assert!(indices.contains(&preserved.center));
    }

    #[test]
    fn mcnk_indices_skip_only_targeted_high_res_hole_quad() {
        let indices = build_mcnk_indices(FULL_LOW_RES_HOLE_MASK, Some(1 << (3 * 8 + 2)));
        assert_eq!(indices.len(), (8 * 8 - 1) * 4 * 3);

        let skipped = quad_index_base(3, 2);
        assert!(!indices.contains(&skipped.center));

        let preserved = quad_index_base(3, 3);
        assert!(indices.contains(&preserved.center));
    }

    #[test]
    fn mccv_color_conversion_uses_bgra_order_and_neutral_center() {
        assert_eq!(
            mccv_color_to_shader_color([0x7F, 0x7F, 0x7F, 0xFF]),
            [1.0, 1.0, 1.0, 1.0]
        );
        assert_eq!(
            mccv_color_to_shader_color([0x00, 0x00, 0x00, 0xFF]),
            [0.0, 0.0, 0.0, 1.0]
        );

        let bright_red = mccv_color_to_shader_color([0x00, 0x00, 0xFF, 0x80]);
        assert!((bright_red[0] - (255.0 / 127.0)).abs() < f32::EPSILON);
        assert_eq!(bright_red[1], 0.0);
        assert_eq!(bright_red[2], 0.0);
        assert_eq!(bright_red[3], 128.0 / 255.0);
    }

    #[test]
    fn mcnk_mesh_emits_vertex_color_attribute_from_mccv() {
        let chunk = super::super::adt_format::adt::McnkData {
            index_x: 0,
            index_y: 0,
            pos: [0.0, 0.0, 0.0],
            flags: super::super::adt_format::adt::McnkFlags::default(),
            shadow_map: None,
            vertex_lighting: None,
            holes_low_res: 0,
            holes_high_res: None,
            heights: [0.0; 145],
            normals: [[0.0, 1.0, 0.0]; 145],
            vertex_colors: [[1.0, 1.0, 1.0, 1.0]; 145],
        };

        let mesh = build_mcnk_mesh(&chunk, None);
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("expected terrain mesh vertex colors");
        };
        assert_eq!(colors.len(), 145);
        assert_eq!(colors[0], [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn mcnk_mesh_multiplies_vertex_lighting_into_vertex_colors() {
        let chunk = super::super::adt_format::adt::McnkData {
            index_x: 0,
            index_y: 0,
            pos: [0.0, 0.0, 0.0],
            flags: super::super::adt_format::adt::McnkFlags::default(),
            shadow_map: None,
            vertex_lighting: Some([[1.5, 0.5, 0.25, 1.0]; 145]),
            holes_low_res: 0,
            holes_high_res: None,
            heights: [0.0; 145],
            normals: [[0.0, 1.0, 0.0]; 145],
            vertex_colors: [[0.5, 0.5, 0.5, 0.75]; 145],
        };

        let mesh = build_mcnk_mesh(&chunk, None);
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("expected terrain mesh vertex colors");
        };
        assert_eq!(colors[0], [0.75, 0.25, 0.125, 0.75]);
    }

    struct QuadIndices {
        center: u32,
    }

    fn quad_index_base(quad_row: usize, quad_col: usize) -> QuadIndices {
        QuadIndices {
            center: super::vertex_index(quad_row * 2 + 1, quad_col) as u32,
        }
    }
}
