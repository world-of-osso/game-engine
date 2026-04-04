use std::collections::HashMap;
use std::io::Cursor;
use std::mem::size_of;

use super::adt_tex::{AdtWaterData, parse_mh2o};
use crate::asset::read_bytes::{read_f32, read_i8, read_u32};
use binrw::BinRead;

pub const CHUNK_SIZE: f32 = 100.0 / 3.0;
pub const UNIT_SIZE: f32 = CHUNK_SIZE / 8.0;

const HALF_UNIT: f32 = UNIT_SIZE / 2.0;
const TILE_SIZE: f32 = CHUNK_SIZE * 16.0;
const MCVT_COUNT: usize = 145;
const MCCV_BYTES_PER_VERTEX: usize = 4;
const MCLV_BYTES_PER_VERTEX: usize = 4;
const MCSH_BYTES: usize = 512;
const MCSE_BYTES_PER_EMITTER: usize = 28;
const MCNK_FLAG_HAS_MCSH: u32 = 0x1;
const MCNK_FLAG_IMPASS: u32 = 0x2;
const MCNK_FLAG_HAS_MCCV: u32 = 0x40;
const MCNK_FLAG_DO_NOT_FIX_ALPHA_MAP: u32 = 0x8000;
const MCNK_FLAG_HIGH_RES_HOLES: u32 = 0x10000;

type AdtChunksResult<'a> = Result<(Vec<&'a [u8]>, Option<&'a [u8]>), String>;
type McnkSubchunksResult = (
    [f32; MCVT_COUNT],
    [[f32; 3]; MCVT_COUNT],
    [[f32; 4]; MCVT_COUNT],
    Option<[[f32; 4]; MCVT_COUNT]>,
    Option<[u8; MCSH_BYTES]>,
    Vec<SoundEmitter>,
);

#[derive(Clone)]
pub struct ChunkHeightGrid {
    pub index_x: u32,
    pub index_y: u32,
    pub origin_x: f32,
    pub origin_z: f32,
    pub base_y: f32,
    pub heights: [f32; 145],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct McnkFlags {
    pub has_mcsh: bool,
    pub impass: bool,
    pub has_mccv: bool,
    pub do_not_fix_alpha_map: bool,
    pub high_res_holes: bool,
}

impl McnkFlags {
    fn from_bits(bits: u32) -> Self {
        Self {
            has_mcsh: (bits & MCNK_FLAG_HAS_MCSH) != 0,
            impass: (bits & MCNK_FLAG_IMPASS) != 0,
            has_mccv: (bits & MCNK_FLAG_HAS_MCCV) != 0,
            do_not_fix_alpha_map: (bits & MCNK_FLAG_DO_NOT_FIX_ALPHA_MAP) != 0,
            high_res_holes: (bits & MCNK_FLAG_HIGH_RES_HOLES) != 0,
        }
    }
}

pub(crate) struct McnkData {
    pub index_x: u32,
    pub index_y: u32,
    pub pos: [f32; 3],
    pub flags: McnkFlags,
    pub area_id: u32,
    pub shadow_map: Option<[u8; MCSH_BYTES]>,
    pub vertex_lighting: Option<[[f32; 4]; MCVT_COUNT]>,
    pub sound_emitters: Vec<SoundEmitter>,
    pub holes_low_res: u16,
    pub holes_high_res: Option<u64>,
    pub heights: [f32; MCVT_COUNT],
    pub normals: [[f32; 3]; MCVT_COUNT],
    pub vertex_colors: [[f32; 4]; MCVT_COUNT],
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq)]
#[br(little)]
pub(crate) struct SoundEmitter {
    pub sound_entry_id: u32,
    pub position: [f32; 3],
    pub size_min: [f32; 3],
}

#[derive(BinRead)]
#[br(little)]
struct McnkHeader {
    flags: u32,
    index_x: u32,
    index_y: u32,
    _n_layers: u32,
    _n_doodad_refs: u32,
    _holes_high_res: u64,
    _ofs_mcvt: u32,
    _ofs_mcnr: u32,
    _ofs_mcly: u32,
    _ofs_mcrf: u32,
    _ofs_mcal: u32,
    _size_mcal: u32,
    _ofs_mcsh: u32,
    _size_mcsh: u32,
    _area_id: u32,
    _n_map_obj_refs: u32,
    _holes_low_res: u16,
    _unknown_but_used: u16,
    _low_quality_texture_map: u64,
    _no_effect_doodad: u64,
    _unknown_tail: [u8; 16],
    pos_y: f32,
    pos_x: f32,
    pos_z: f32,
}

pub(crate) struct ParsedAdtData {
    pub chunks: Vec<McnkData>,
    pub height_grids: Vec<ChunkHeightGrid>,
    pub center_surface: [f32; 3],
    pub chunk_positions: Vec<[f32; 3]>,
    pub water: Option<AdtWaterData>,
    pub water_error: Option<String>,
}

fn parse_binrw_value<T>(data: &[u8], offset: usize, label: &str) -> Result<T, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let end = offset
        .checked_add(size_of::<T>())
        .ok_or_else(|| format!("{label} end offset overflow"))?;
    let slice = data
        .get(offset..end)
        .ok_or_else(|| format!("{label} out of bounds at {offset:#x}"))?;
    T::read_le(&mut Cursor::new(slice))
        .map_err(|err| format!("{label} parse failed at {offset:#x}: {err}"))
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

fn parse_mccv(payload: &[u8]) -> Result<[[f32; 4]; MCVT_COUNT], String> {
    let expected_len = MCVT_COUNT * MCCV_BYTES_PER_VERTEX;
    if payload.len() < expected_len {
        return Err(format!(
            "MCCV too small: {} bytes (need {})",
            payload.len(),
            expected_len
        ));
    }
    let mut colors = [[1.0f32; 4]; MCVT_COUNT];
    for (i, color) in colors.iter_mut().enumerate() {
        let base = i * MCCV_BYTES_PER_VERTEX;
        let blue = payload[base];
        let green = payload[base + 1];
        let red = payload[base + 2];
        let alpha = payload[base + 3];
        *color = [
            f32::from(red) / 127.0,
            f32::from(green) / 127.0,
            f32::from(blue) / 127.0,
            f32::from(alpha) / 255.0,
        ];
    }
    Ok(colors)
}

fn parse_mclv(payload: &[u8]) -> Result<[[f32; 4]; MCVT_COUNT], String> {
    let expected_len = MCVT_COUNT * MCLV_BYTES_PER_VERTEX;
    if payload.len() < expected_len {
        return Err(format!(
            "MCLV too small: {} bytes (need {})",
            payload.len(),
            expected_len
        ));
    }

    let mut colors = [[1.0f32; 4]; MCVT_COUNT];
    for (i, color) in colors.iter_mut().enumerate() {
        let base = i * MCLV_BYTES_PER_VERTEX;
        let blue = payload[base];
        let green = payload[base + 1];
        let red = payload[base + 2];
        *color = [
            f32::from(red) / 128.0,
            f32::from(green) / 128.0,
            f32::from(blue) / 128.0,
            1.0,
        ];
    }
    Ok(colors)
}

fn parse_mcsh(payload: &[u8]) -> Result<[u8; MCSH_BYTES], String> {
    if payload.len() < MCSH_BYTES {
        return Err(format!(
            "MCSH too small: {} bytes (need {})",
            payload.len(),
            MCSH_BYTES
        ));
    }

    let mut shadow_map = [0; MCSH_BYTES];
    shadow_map.copy_from_slice(&payload[..MCSH_BYTES]);
    Ok(shadow_map)
}

fn parse_mcse(payload: &[u8]) -> Result<Vec<SoundEmitter>, String> {
    if !payload.len().is_multiple_of(MCSE_BYTES_PER_EMITTER) {
        return Err(format!(
            "MCSE size must be a multiple of {MCSE_BYTES_PER_EMITTER} bytes: {} bytes",
            payload.len()
        ));
    }

    let mut emitters = Vec::with_capacity(payload.len() / MCSE_BYTES_PER_EMITTER);
    let mut cursor = Cursor::new(payload);
    while (cursor.position() as usize) < payload.len() {
        emitters.push(
            SoundEmitter::read_le(&mut cursor)
                .map_err(|err| format!("MCSE emitter parse failed: {err}"))?,
        );
    }
    Ok(emitters)
}

fn parse_mcnk(payload: &[u8]) -> Result<McnkData, String> {
    if payload.len() < size_of::<McnkHeader>() {
        return Err(format!("MCNK payload too small: {} bytes", payload.len()));
    }
    let header: McnkHeader = parse_binrw_value(payload, 0, "MCNK header")?;
    let flags = McnkFlags::from_bits(header.flags);
    let pos = [header.pos_x, header.pos_y, header.pos_z];
    let (heights, normals, vertex_colors, vertex_lighting, shadow_map, sound_emitters) =
        parse_mcnk_subchunks(&payload[128..], flags)?;
    Ok(McnkData {
        index_x: header.index_x,
        index_y: header.index_y,
        pos,
        flags,
        area_id: header._area_id,
        shadow_map,
        vertex_lighting,
        sound_emitters,
        holes_low_res: header._holes_low_res,
        holes_high_res: flags.high_res_holes.then_some(header._holes_high_res),
        heights,
        normals,
        vertex_colors,
    })
}

fn parse_mcnk_subchunks(sub: &[u8], flags: McnkFlags) -> Result<McnkSubchunksResult, String> {
    let mut heights = None;
    let mut normals = None;
    let mut vertex_colors = None;
    let mut vertex_lighting = None;
    let mut shadow_map = None;
    let mut sound_emitters = None;
    for chunk in ChunkIter::new(sub) {
        let (tag, payload) = chunk?;
        match tag {
            b"TVCM" => heights = Some(parse_mcvt(payload)?),
            b"RNCM" => normals = Some(parse_mcnr(payload)?),
            b"VCCM" => vertex_colors = Some(parse_mccv(payload)?),
            b"VLCM" => vertex_lighting = Some(parse_mclv(payload)?),
            b"HSCM" => shadow_map = Some(parse_mcsh(payload)?),
            b"MCSE" => sound_emitters = Some(parse_mcse(payload)?),
            _ => {}
        }
    }
    let heights = heights.ok_or("MCNK missing TVCM sub-chunk")?;
    let normals = normals.unwrap_or([[0.0, 1.0, 0.0]; MCVT_COUNT]);
    let vertex_colors = resolve_mcnk_vertex_colors(vertex_colors, flags)?;
    let shadow_map = resolve_mcnk_shadow_map(shadow_map, flags)?;
    let sound_emitters = sound_emitters.unwrap_or_default();
    Ok((
        heights,
        normals,
        vertex_colors,
        vertex_lighting,
        shadow_map,
        sound_emitters,
    ))
}

fn resolve_mcnk_vertex_colors(
    vertex_colors: Option<[[f32; 4]; MCVT_COUNT]>,
    flags: McnkFlags,
) -> Result<[[f32; 4]; MCVT_COUNT], String> {
    match vertex_colors {
        Some(colors) => Ok(colors),
        None if !flags.has_mccv => Ok([[1.0, 1.0, 1.0, 1.0]; MCVT_COUNT]),
        None => Err("MCNK flagged with MCCV but missing VCCM sub-chunk".to_string()),
    }
}

fn resolve_mcnk_shadow_map(
    shadow_map: Option<[u8; MCSH_BYTES]>,
    flags: McnkFlags,
) -> Result<Option<[u8; MCSH_BYTES]>, String> {
    match shadow_map {
        Some(shadow_map) => Ok(Some(shadow_map)),
        None if !flags.has_mcsh => Ok(None),
        None => Err("MCNK flagged with MCSH but missing HSCM sub-chunk".to_string()),
    }
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

#[cfg(test)]
mod tests {
    use super::{
        MCNK_FLAG_DO_NOT_FIX_ALPHA_MAP, MCNK_FLAG_HAS_MCCV, MCNK_FLAG_HAS_MCSH,
        MCNK_FLAG_HIGH_RES_HOLES, MCNK_FLAG_IMPASS, MCVT_COUNT, McnkFlags, parse_mccv, parse_mclv,
        parse_mcnk, parse_mcnk_subchunks,
    };

    #[test]
    fn parse_mccv_reads_bgra_and_maps_neutral_to_one() {
        let mut payload = vec![0x7F; MCVT_COUNT * 4];
        for i in 0..MCVT_COUNT {
            payload[i * 4 + 3] = 0xFF;
        }
        payload[0..4].copy_from_slice(&[0x20, 0x40, 0x60, 0x80]);

        let colors = parse_mccv(&payload).expect("expected MCCV colors");
        assert_eq!(colors.len(), MCVT_COUNT);
        assert_eq!(colors[1], [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(colors[0][0], 0x60 as f32 / 127.0);
        assert_eq!(colors[0][1], 0x40 as f32 / 127.0);
        assert_eq!(colors[0][2], 0x20 as f32 / 127.0);
        assert_eq!(colors[0][3], 0x80 as f32 / 255.0);
    }

    #[test]
    fn mcnk_flags_decode_named_bits() {
        let flags = McnkFlags::from_bits(
            MCNK_FLAG_HAS_MCSH
                | MCNK_FLAG_IMPASS
                | MCNK_FLAG_HAS_MCCV
                | MCNK_FLAG_DO_NOT_FIX_ALPHA_MAP
                | MCNK_FLAG_HIGH_RES_HOLES,
        );

        assert!(flags.has_mcsh);
        assert!(flags.impass);
        assert!(flags.has_mccv);
        assert!(flags.do_not_fix_alpha_map);
        assert!(flags.high_res_holes);
    }

    #[test]
    fn parse_mcnk_subchunks_requires_mccv_when_flagged() {
        let payload = mcnk_subchunks_payload(false, false, false);

        let err = parse_mcnk_subchunks(
            &payload,
            McnkFlags {
                has_mcsh: false,
                impass: false,
                has_mccv: true,
                do_not_fix_alpha_map: false,
                high_res_holes: false,
            },
        )
        .expect_err("expected missing MCCV to be rejected");

        assert!(err.contains("flagged with MCCV"));
    }

    #[test]
    fn parse_mcnk_subchunks_defaults_vertex_colors_when_mccv_not_flagged() {
        let payload = mcnk_subchunks_payload(false, false, false);

        let (_, _, colors, vertex_lighting, shadow_map, sound_emitters) =
            parse_mcnk_subchunks(&payload, McnkFlags::default())
                .expect("expected missing optional MCCV to default");

        assert_eq!(colors[0], [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(colors[MCVT_COUNT - 1], [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(vertex_lighting, None);
        assert_eq!(shadow_map, None);
        assert!(sound_emitters.is_empty());
    }

    #[test]
    fn parse_mcnk_reads_area_id_from_header() {
        let mut payload = vec![0; 128];
        payload[0..4].copy_from_slice(&0u32.to_le_bytes());
        payload[4..8].copy_from_slice(&3u32.to_le_bytes());
        payload[8..12].copy_from_slice(&7u32.to_le_bytes());
        payload[60..64].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        append_subchunk(&mut payload, b"TVCM", vec![0; MCVT_COUNT * 4]);
        append_subchunk(&mut payload, b"RNCM", vec![0; MCVT_COUNT * 3]);

        let chunk = parse_mcnk(&payload).expect("expected MCNK header to parse");
        assert_eq!(chunk.area_id, 0x1234_5678);
    }

    #[test]
    fn parse_mclv_reads_bgra_and_uses_neutral_center() {
        let mut payload = vec![0; MCVT_COUNT * 4];
        payload[0..4].copy_from_slice(&[0x40, 0x60, 0x80, 0xFF]);

        let colors = parse_mclv(&payload).expect("expected MCLV colors");
        assert_eq!(colors.len(), MCVT_COUNT);
        assert_eq!(colors[0], [128.0 / 128.0, 96.0 / 128.0, 64.0 / 128.0, 1.0]);
        assert_eq!(colors[1], [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn parse_mcnk_subchunks_reads_mcsh_when_flagged() {
        let payload = mcnk_subchunks_payload(true, false, false);

        let (_, _, _, vertex_lighting, shadow_map, sound_emitters) = parse_mcnk_subchunks(
            &payload,
            McnkFlags {
                has_mcsh: true,
                impass: false,
                has_mccv: false,
                do_not_fix_alpha_map: false,
                high_res_holes: false,
            },
        )
        .expect("expected MCSH shadow map");

        let shadow_map = shadow_map.expect("expected parsed shadow map");
        assert_eq!(shadow_map[0], 0b1000_0000);
        assert_eq!(shadow_map[1], 0b0100_0000);
        assert_eq!(vertex_lighting, None);
        assert!(sound_emitters.is_empty());
    }

    #[test]
    fn parse_mcnk_subchunks_reads_mcse_emitters() {
        let payload = mcnk_subchunks_payload(false, false, true);

        let (_, _, _, vertex_lighting, shadow_map, sound_emitters) =
            parse_mcnk_subchunks(&payload, McnkFlags::default()).expect("expected MCSE emitters");

        assert_eq!(vertex_lighting, None);
        assert_eq!(shadow_map, None);
        assert_eq!(sound_emitters.len(), 2);
        assert_eq!(sound_emitters[0].sound_entry_id, 42);
        assert_eq!(sound_emitters[0].position, [100.0, 200.0, 300.0]);
        assert_eq!(sound_emitters[0].size_min, [10.0, 20.0, 30.0]);
        assert_eq!(sound_emitters[1].sound_entry_id, 7);
        assert_eq!(sound_emitters[1].position, [1.0, 2.0, 3.0]);
        assert_eq!(sound_emitters[1].size_min, [4.0, 5.0, 6.0]);
    }

    #[test]
    fn parse_mcnk_subchunks_requires_mcsh_when_flagged() {
        let payload = mcnk_subchunks_payload(false, false, false);

        let err = parse_mcnk_subchunks(
            &payload,
            McnkFlags {
                has_mcsh: true,
                impass: false,
                has_mccv: false,
                do_not_fix_alpha_map: false,
                high_res_holes: false,
            },
        )
        .expect_err("expected missing MCSH to be rejected");

        assert!(err.contains("flagged with MCSH"));
    }

    #[test]
    fn parse_mcnk_subchunks_reads_mclv_even_when_it_is_not_first() {
        let payload = mcnk_subchunks_payload(false, true, false);

        let (_, _, _, vertex_lighting, shadow_map, sound_emitters) =
            parse_mcnk_subchunks(&payload, McnkFlags::default())
                .expect("expected vertex lighting to be parsed");

        let vertex_lighting = vertex_lighting.expect("expected parsed MCLV");
        assert_eq!(vertex_lighting[0], [1.0, 0.5, 0.0, 1.0]);
        assert_eq!(shadow_map, None);
        assert!(sound_emitters.is_empty());
    }

    fn mcnk_subchunks_payload(
        include_mcsh: bool,
        include_mclv: bool,
        include_mcse: bool,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        append_subchunk(&mut payload, b"TVCM", vec![0; MCVT_COUNT * 4]);
        append_subchunk(&mut payload, b"RNCM", vec![0; MCVT_COUNT * 3]);
        if include_mcsh {
            let mut shadow_map = vec![0; 512];
            shadow_map[0] = 0b1000_0000;
            shadow_map[1] = 0b0100_0000;
            append_subchunk(&mut payload, b"HSCM", shadow_map);
        }
        if include_mcsh {
            append_subchunk(&mut payload, b"VCCM", vec![0x7F; MCVT_COUNT * 4]);
        }
        if include_mclv {
            let mut vertex_lighting = vec![0; MCVT_COUNT * 4];
            vertex_lighting[0..4].copy_from_slice(&[0x00, 0x40, 0x80, 0xFF]);
            append_subchunk(&mut payload, b"VLCM", vertex_lighting);
        }
        if include_mcse {
            let mut sound_emitters = Vec::new();
            sound_emitters.extend_from_slice(&42u32.to_le_bytes());
            sound_emitters.extend_from_slice(&100.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&200.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&300.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&10.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&20.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&30.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&7u32.to_le_bytes());
            sound_emitters.extend_from_slice(&1.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&2.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&3.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&4.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&5.0f32.to_le_bytes());
            sound_emitters.extend_from_slice(&6.0f32.to_le_bytes());
            append_subchunk(&mut payload, b"MCSE", sound_emitters);
        }
        payload
    }

    fn append_subchunk(payload: &mut Vec<u8>, tag: &[u8; 4], chunk_payload: Vec<u8>) {
        payload.extend_from_slice(tag);
        payload.extend_from_slice(&(chunk_payload.len() as u32).to_le_bytes());
        payload.extend_from_slice(&chunk_payload);
    }
}
