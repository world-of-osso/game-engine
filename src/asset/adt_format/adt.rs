use std::collections::HashMap;

use super::adt_tex::{AdtWaterData, parse_mh2o};

pub const CHUNK_SIZE: f32 = 100.0 / 3.0;
pub const UNIT_SIZE: f32 = CHUNK_SIZE / 8.0;

const HALF_UNIT: f32 = UNIT_SIZE / 2.0;
const TILE_SIZE: f32 = CHUNK_SIZE * 16.0;
const MCVT_COUNT: usize = 145;

type AdtChunksResult<'a> = Result<(Vec<&'a [u8]>, Option<&'a [u8]>), String>;

#[derive(Clone)]
pub struct ChunkHeightGrid {
    pub index_x: u32,
    pub index_y: u32,
    pub origin_x: f32,
    pub origin_z: f32,
    pub base_y: f32,
    pub heights: [f32; 145],
}

pub(crate) struct McnkData {
    pub index_x: u32,
    pub index_y: u32,
    pub pos: [f32; 3],
    pub heights: [f32; MCVT_COUNT],
    pub normals: [[f32; 3]; MCVT_COUNT],
}

pub(crate) struct ParsedAdtData {
    pub chunks: Vec<McnkData>,
    pub height_grids: Vec<ChunkHeightGrid>,
    pub center_surface: [f32; 3],
    pub chunk_positions: Vec<[f32; 3]>,
    pub water: Option<AdtWaterData>,
    pub water_error: Option<String>,
}

fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_u32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

fn read_f32(data: &[u8], off: usize) -> Result<f32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_f32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

fn read_i8(data: &[u8], off: usize) -> Result<i8, String> {
    data.get(off)
        .copied()
        .map(|b| b as i8)
        .ok_or_else(|| format!("read_i8 out of bounds at {off:#x}"))
}

pub struct ChunkIter<'a> {
    data: &'a [u8],
    off: usize,
}

impl<'a> ChunkIter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, off: 0 }
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = Result<(&'a [u8; 4], &'a [u8]), String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.off + 8 > self.data.len() {
            return None;
        }
        let tag: &[u8; 4] = self.data[self.off..self.off + 4].try_into().unwrap();
        let size = match read_u32(self.data, self.off + 4) {
            Ok(s) => s as usize,
            Err(e) => return Some(Err(e)),
        };
        let payload_start = self.off + 8;
        let payload_end = payload_start + size;
        if payload_end > self.data.len() {
            return Some(Err(format!(
                "chunk {:?} truncated at {:#x}",
                std::str::from_utf8(tag).unwrap_or("????"),
                self.off,
            )));
        }
        self.off = payload_end;
        Some(Ok((tag, &self.data[payload_start..payload_end])))
    }
}

fn parse_mcvt(payload: &[u8]) -> Result<[f32; MCVT_COUNT], String> {
    if payload.len() < MCVT_COUNT * 4 {
        return Err(format!(
            "MCVT too small: {} bytes (need {})",
            payload.len(),
            MCVT_COUNT * 4
        ));
    }
    let mut heights = [0.0f32; MCVT_COUNT];
    for (i, h) in heights.iter_mut().enumerate() {
        *h = read_f32(payload, i * 4)?;
    }
    Ok(heights)
}

fn parse_mcnr(payload: &[u8]) -> Result<[[f32; 3]; MCVT_COUNT], String> {
    if payload.len() < MCVT_COUNT * 3 {
        return Err(format!(
            "MCNR too small: {} bytes (need {})",
            payload.len(),
            MCVT_COUNT * 3
        ));
    }
    let mut normals = [[0.0f32; 3]; MCVT_COUNT];
    for (i, n) in normals.iter_mut().enumerate() {
        let nx = read_i8(payload, i * 3)? as f32 / 127.0;
        let nz = read_i8(payload, i * 3 + 1)? as f32 / 127.0;
        let ny = read_i8(payload, i * 3 + 2)? as f32 / 127.0;
        let mut normal = [ny, nz, -nx];
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if len > 0.0001 {
            normal[0] /= len;
            normal[1] /= len;
            normal[2] /= len;
        } else {
            normal = [0.0, 1.0, 0.0];
        }
        *n = normal;
    }
    Ok(normals)
}

fn parse_mcnk(payload: &[u8]) -> Result<McnkData, String> {
    if payload.len() < 128 {
        return Err(format!("MCNK payload too small: {} bytes", payload.len()));
    }
    let index_x = read_u32(payload, 0x04)?;
    let index_y = read_u32(payload, 0x08)?;
    let pos = [
        read_f32(payload, 0x6c)?,
        read_f32(payload, 0x68)?,
        read_f32(payload, 0x70)?,
    ];
    let (heights, normals) = parse_mcnk_subchunks(&payload[128..])?;
    Ok(McnkData {
        index_x,
        index_y,
        pos,
        heights,
        normals,
    })
}

fn parse_mcnk_subchunks(sub: &[u8]) -> Result<([f32; MCVT_COUNT], [[f32; 3]; MCVT_COUNT]), String> {
    let mut heights = None;
    let mut normals = None;
    for chunk in ChunkIter::new(sub) {
        let (tag, payload) = chunk?;
        match tag {
            b"TVCM" => heights = Some(parse_mcvt(payload)?),
            b"RNCM" => normals = Some(parse_mcnr(payload)?),
            _ => {}
        }
        if heights.is_some() && normals.is_some() {
            break;
        }
    }
    let heights = heights.ok_or("MCNK missing TVCM sub-chunk")?;
    let normals = normals.unwrap_or([[0.0, 1.0, 0.0]; MCVT_COUNT]);
    Ok((heights, normals))
}

pub fn vertex_index(grid_row: usize, col: usize) -> usize {
    let r_outer = grid_row / 2;
    if grid_row.is_multiple_of(2) {
        r_outer * 17 + col
    } else {
        r_outer * 17 + 9 + col
    }
}

pub(crate) fn vertex_position_from_origin(
    grid_row: usize,
    col: usize,
    origin_x: f32,
    origin_z: f32,
    base_y: f32,
    heights: &[f32; MCVT_COUNT],
) -> [f32; 3] {
    let idx = vertex_index(grid_row, col);
    let r = (grid_row / 2) as f32;
    let c = col as f32;
    let (bx, bz) = if grid_row.is_multiple_of(2) {
        (origin_x - c * UNIT_SIZE, origin_z + r * UNIT_SIZE)
    } else {
        (
            origin_x - c * UNIT_SIZE - HALF_UNIT,
            origin_z + r * UNIT_SIZE + HALF_UNIT,
        )
    };
    [bx, base_y + heights[idx], bz]
}

fn tile_origin_bevy(tile_y: u32, tile_x: u32) -> (f32, f32) {
    let center = 32.0 * TILE_SIZE;
    let origin_x = center - tile_x as f32 * TILE_SIZE;
    let origin_z = tile_y as f32 * TILE_SIZE - center;
    (origin_x, origin_z)
}

pub(crate) fn chunk_origin_bevy(chunk: &McnkData, tile_coords: Option<(u32, u32)>) -> (f32, f32) {
    if let Some((tile_y, tile_x)) = tile_coords {
        let (tile_origin_x, tile_origin_z) = tile_origin_bevy(tile_y, tile_x);
        (
            tile_origin_x - chunk.index_y as f32 * CHUNK_SIZE,
            tile_origin_z + chunk.index_x as f32 * CHUNK_SIZE,
        )
    } else {
        (chunk.pos[1], -chunk.pos[0])
    }
}

fn build_height_grids(
    parsed: &[McnkData],
    tile_coords: Option<(u32, u32)>,
) -> Vec<ChunkHeightGrid> {
    parsed
        .iter()
        .map(|d| ChunkHeightGrid {
            index_x: d.index_x,
            index_y: d.index_y,
            origin_x: chunk_origin_bevy(d, tile_coords).0,
            origin_z: chunk_origin_bevy(d, tile_coords).1,
            base_y: d.pos[2],
            heights: d.heights,
        })
        .collect()
}

const SEAM_SMOOTHING_RINGS: &[(usize, f32)] = &[];

fn stitch_chunk_edges(parsed: &mut [McnkData]) {
    let indices: HashMap<(u32, u32), usize> = parsed
        .iter()
        .enumerate()
        .map(|(i, chunk)| ((chunk.index_x, chunk.index_y), i))
        .collect();

    for i in 0..parsed.len() {
        let (index_x, index_y) = (parsed[i].index_x, parsed[i].index_y);
        if let Some(&neighbor) = indices.get(&(index_x, index_y + 1)) {
            stitch_vertical_border(parsed, i, neighbor);
        }
        if let Some(&neighbor) = indices.get(&(index_x + 1, index_y)) {
            stitch_horizontal_border(parsed, i, neighbor);
        }
    }

    stitch_chunk_corners(parsed, &indices);
}

fn stitch_horizontal_border(parsed: &mut [McnkData], top: usize, bottom: usize) {
    let (top_chunk, bottom_chunk) = split_two_mut(parsed, top, bottom);
    for col in 0..=8 {
        let stitched = average_height_pair(
            top_chunk.pos[2],
            &mut top_chunk.heights,
            vertex_index(16, col),
            bottom_chunk.pos[2],
            &mut bottom_chunk.heights,
            vertex_index(0, col),
        );
        smooth_horizontal_interior(top_chunk, bottom_chunk, col, stitched);
    }
}

fn stitch_vertical_border(parsed: &mut [McnkData], left: usize, right: usize) {
    let (left_chunk, right_chunk) = split_two_mut(parsed, left, right);
    for row in 0..=8 {
        let stitched = average_height_pair(
            left_chunk.pos[2],
            &mut left_chunk.heights,
            vertex_index(row * 2, 8),
            right_chunk.pos[2],
            &mut right_chunk.heights,
            vertex_index(row * 2, 0),
        );
        smooth_vertical_interior(left_chunk, right_chunk, row, stitched);
    }
}

fn stitch_chunk_corners(parsed: &mut [McnkData], indices: &HashMap<(u32, u32), usize>) {
    for (&(index_x, index_y), &top_left) in indices {
        let Some(&top_right) = indices.get(&(index_x, index_y + 1)) else {
            continue;
        };
        let Some(&bottom_left) = indices.get(&(index_x + 1, index_y)) else {
            continue;
        };
        let Some(&bottom_right) = indices.get(&(index_x + 1, index_y + 1)) else {
            continue;
        };
        average_height_quad(
            parsed,
            [
                (top_left, vertex_index(16, 8)),
                (top_right, vertex_index(16, 0)),
                (bottom_left, vertex_index(0, 8)),
                (bottom_right, vertex_index(0, 0)),
            ],
        );
    }
}

fn split_two_mut<T>(slice: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert!(a != b, "indices must be distinct");
    if a < b {
        let (left, right) = slice.split_at_mut(b);
        (&mut left[a], &mut right[0])
    } else {
        let (left, right) = slice.split_at_mut(a);
        (&mut right[0], &mut left[b])
    }
}

fn average_height_pair(
    base_a: f32,
    heights_a: &mut [f32; MCVT_COUNT],
    idx_a: usize,
    base_b: f32,
    heights_b: &mut [f32; MCVT_COUNT],
    idx_b: usize,
) -> f32 {
    let absolute_a = base_a + heights_a[idx_a];
    let absolute_b = base_b + heights_b[idx_b];
    let avg = (absolute_a + absolute_b) * 0.5;
    heights_a[idx_a] = avg - base_a;
    heights_b[idx_b] = avg - base_b;
    avg
}

fn average_height_quad(parsed: &mut [McnkData], vertices: [(usize, usize); 4]) {
    let average = vertices
        .iter()
        .map(|&(chunk_idx, vertex_idx)| {
            parsed[chunk_idx].pos[2] + parsed[chunk_idx].heights[vertex_idx]
        })
        .sum::<f32>()
        / vertices.len() as f32;

    for (chunk_idx, vertex_idx) in vertices {
        let chunk = &mut parsed[chunk_idx];
        chunk.heights[vertex_idx] = average - chunk.pos[2];
    }
}

fn smooth_horizontal_interior(
    top_chunk: &mut McnkData,
    bottom_chunk: &mut McnkData,
    col: usize,
    stitched_height: f32,
) {
    if col == 0 || col == 8 {
        return;
    }
    for &(ring, weight) in SEAM_SMOOTHING_RINGS {
        if ring > 8 {
            break;
        }
        blend_absolute_height(
            top_chunk,
            vertex_index(16 - ring * 2, col),
            stitched_height,
            weight,
        );
        blend_absolute_height(
            bottom_chunk,
            vertex_index(ring * 2, col),
            stitched_height,
            weight,
        );
    }
}

fn smooth_vertical_interior(
    left_chunk: &mut McnkData,
    right_chunk: &mut McnkData,
    row: usize,
    stitched_height: f32,
) {
    if row == 0 || row == 8 {
        return;
    }
    for &(ring, weight) in SEAM_SMOOTHING_RINGS {
        if ring > 8 {
            break;
        }
        blend_absolute_height(
            left_chunk,
            vertex_index(row * 2, 8 - ring),
            stitched_height,
            weight,
        );
        blend_absolute_height(
            right_chunk,
            vertex_index(row * 2, ring),
            stitched_height,
            weight,
        );
    }
}

fn blend_absolute_height(chunk: &mut McnkData, vertex_idx: usize, target_height: f32, weight: f32) {
    let current = chunk.pos[2] + chunk.heights[vertex_idx];
    let blended = current + (target_height - current) * weight;
    chunk.heights[vertex_idx] = blended - chunk.pos[2];
}

fn center_surface_position(chunks: &[McnkData], tile_coords: Option<(u32, u32)>) -> [f32; 3] {
    let center_chunk = chunks
        .iter()
        .find(|c| c.index_x == 8 && c.index_y == 8)
        .unwrap_or(&chunks[chunks.len() / 2]);
    let (origin_x, origin_z) = chunk_origin_bevy(center_chunk, tile_coords);
    vertex_position_from_origin(
        9,
        4,
        origin_x,
        origin_z,
        center_chunk.pos[2],
        &center_chunk.heights,
    )
}

fn collect_adt_chunks(data: &[u8]) -> AdtChunksResult<'_> {
    let mut mcnks = Vec::with_capacity(256);
    let mut mh2o = None;
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"KNCM" => mcnks.push(payload),
            b"O2HM" => mh2o = Some(payload),
            _ => {}
        }
    }
    if mcnks.is_empty() {
        return Err("No KNCM (MCNK) chunks found in ADT file".to_string());
    }
    Ok((mcnks, mh2o))
}

pub(crate) fn load_adt_parsed(data: &[u8]) -> Result<ParsedAdtData, String> {
    load_adt_inner(data, true, None)
}

pub(crate) fn load_adt_for_tile_parsed(
    data: &[u8],
    tile_y: u32,
    tile_x: u32,
) -> Result<ParsedAdtData, String> {
    load_adt_inner(data, true, Some((tile_y, tile_x)))
}

#[cfg(test)]
pub(crate) fn load_adt_raw(data: &[u8]) -> Result<ParsedAdtData, String> {
    load_adt_inner(data, false, None)
}

fn load_adt_inner(
    data: &[u8],
    stitch: bool,
    tile_coords: Option<(u32, u32)>,
) -> Result<ParsedAdtData, String> {
    let (mcnk_payloads, mh2o_payload) = collect_adt_chunks(data)?;
    let mut parsed: Vec<McnkData> = mcnk_payloads
        .into_iter()
        .map(parse_mcnk)
        .collect::<Result<Vec<_>, String>>()?;
    if stitch {
        stitch_chunk_edges(&mut parsed);
    }
    let center_surface = center_surface_position(&parsed, tile_coords);
    let chunk_positions = parsed.iter().map(|d| d.pos).collect();
    let height_grids = build_height_grids(&parsed, tile_coords);
    let (water, water_error) = match mh2o_payload {
        Some(payload) => match parse_mh2o(payload) {
            Ok(water) => (Some(water), None),
            Err(err) => (None, Some(err)),
        },
        None => (None, None),
    };
    Ok(ParsedAdtData {
        chunks: parsed,
        height_grids,
        center_surface,
        chunk_positions,
        water,
        water_error,
    })
}
