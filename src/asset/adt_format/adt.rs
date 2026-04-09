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
const MCBB_BYTES_PER_BATCH: usize = 20;
const MCDD_BYTES: usize = 64;
const MBMH_BYTES_PER_HEADER: usize = 28;
const MBBB_BYTES_PER_BOUND: usize = 28;
const MBNV_BYTES_PER_VERTEX: usize = 44;
const MCNK_FLAG_HAS_MCSH: u32 = 0x1;
const MCNK_FLAG_IMPASS: u32 = 0x2;
const MCNK_FLAG_HAS_MCCV: u32 = 0x40;
const MCNK_FLAG_DO_NOT_FIX_ALPHA_MAP: u32 = 0x8000;
const MCNK_FLAG_HIGH_RES_HOLES: u32 = 0x10000;

type AdtChunksResult<'a> = Result<AdtRootChunks<'a>, String>;
type McnkSubchunksResult = (
    [f32; MCVT_COUNT],
    [[f32; 3]; MCVT_COUNT],
    [[f32; 4]; MCVT_COUNT],
    Option<[[f32; 4]; MCVT_COUNT]>,
    Option<[u8; MCSH_BYTES]>,
    Vec<SoundEmitter>,
    Vec<BlendBatch>,
    Option<[u8; MCDD_BYTES]>,
);

struct McnkSubchunkAccum {
    heights: Option<[f32; MCVT_COUNT]>,
    normals: Option<[[f32; 3]; MCVT_COUNT]>,
    vertex_colors: Option<[[f32; 4]; MCVT_COUNT]>,
    vertex_lighting: Option<[[f32; 4]; MCVT_COUNT]>,
    shadow_map: Option<[u8; MCSH_BYTES]>,
    sound_emitters: Option<Vec<SoundEmitter>>,
    blend_batches: Option<Vec<BlendBatch>>,
    detail_doodad_disable: Option<[u8; MCDD_BYTES]>,
}

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
    pub blend_batches: Vec<BlendBatch>,
    pub detail_doodad_disable: Option<[u8; MCDD_BYTES]>,
    pub holes_low_res: u16,
    pub holes_high_res: Option<u64>,
    pub heights: [f32; MCVT_COUNT],
    pub normals: [[f32; 3]; MCVT_COUNT],
    pub vertex_colors: [[f32; 4]; MCVT_COUNT],
}

pub struct BlendMeshData {
    pub headers: Vec<BlendMeshHeader>,
    pub bounds: Vec<BlendMeshBounds>,
    pub vertices: Vec<BlendMeshVertex>,
    pub indices: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightBounds {
    pub min_heights: [i16; 9],
    pub max_heights: [i16; 9],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodHeader {
    pub flags: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodLevel {
    pub vertex_step: f32,
    pub payload: [u32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LodQuadTreeNode {
    pub words16: [u16; 4],
    pub words32: [u32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedLodData {
    pub version: u32,
    pub header: LodHeader,
    pub heights: Vec<f32>,
    pub levels: Vec<LodLevel>,
    pub nodes: Vec<LodQuadTreeNode>,
    pub indices: Vec<u16>,
    pub skirt_indices: Vec<u16>,
    pub liquid_directory: Option<LodLiquidDirectory>,
    pub liquids: Vec<LodLiquidPatch>,
    pub m2_placements: Vec<LodObjectPlacement>,
    pub m2_visibility: Vec<LodObjectVisibility>,
    pub wmo_placements: Vec<LodObjectPlacement>,
    pub wmo_visibility: Vec<LodObjectVisibility>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LodLiquidDirectory {
    pub raw: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LodLiquidPatchHeader {
    pub words: [u32; 6],
}

#[derive(Clone, Debug, PartialEq)]
pub struct LodLiquidPatch {
    pub header: LodLiquidPatchHeader,
    pub indices: Vec<u16>,
    pub vertices: Vec<[f32; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodObjectPlacement {
    pub id: u32,
    pub asset_id: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: f32,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodObjectVisibility {
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq)]
#[br(little)]
pub struct SoundEmitter {
    pub sound_entry_id: u32,
    pub position: [f32; 3],
    pub size_min: [f32; 3],
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq)]
#[br(little)]
pub struct BlendBatch {
    pub mbmh_index: u32,
    pub index_count: u32,
    pub index_first: u32,
    pub vertex_count: u32,
    pub vertex_first: u32,
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq)]
#[br(little)]
pub struct BlendMeshHeader {
    pub map_object_id: u32,
    pub texture_id: u32,
    pub unknown: u32,
    pub index_count: u32,
    pub vertex_count: u32,
    pub index_start: u32,
    pub vertex_start: u32,
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq)]
#[br(little)]
pub struct BlendMeshBounds {
    pub map_object_id: u32,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, Copy, BinRead, PartialEq)]
#[br(little)]
pub struct BlendMeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [[u8; 4]; 3],
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
    pub blend_mesh: Option<BlendMeshData>,
    pub flight_bounds: Option<FlightBounds>,
    pub height_grids: Vec<ChunkHeightGrid>,
    pub center_surface: [f32; 3],
    pub chunk_positions: Vec<[f32; 3]>,
    pub water: Option<AdtWaterData>,
    pub water_error: Option<String>,
}

struct AdtRootChunks<'a> {
    mcnks: Vec<&'a [u8]>,
    mh2o: Option<&'a [u8]>,
    mfbo: Option<&'a [u8]>,
    mbmh: Option<&'a [u8]>,
    mbbb: Option<&'a [u8]>,
    mbnv: Option<&'a [u8]>,
    mbmi: Option<&'a [u8]>,
}

struct LodRootChunks<'a> {
    mver: Option<&'a [u8]>,
    mlhd: Option<&'a [u8]>,
    mlvh: Option<&'a [u8]>,
    mlll: Option<&'a [u8]>,
    mlnd: Option<&'a [u8]>,
    mlvi: Option<&'a [u8]>,
    mlsi: Option<&'a [u8]>,
    mlld: Option<&'a [u8]>,
    mldd: Option<&'a [u8]>,
    mldx: Option<&'a [u8]>,
    mlmd: Option<&'a [u8]>,
    mlmx: Option<&'a [u8]>,
    liquid_groups: Vec<LodLiquidChunkGroup<'a>>,
}

struct LodLiquidChunkGroup<'a> {
    header: &'a [u8],
    indices: &'a [u8],
    vertices: &'a [u8],
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

fn fix_shadow_map_edges(shadow_map: &mut [u8; MCSH_BYTES]) {
    const SHADOW_MAP_SIDE: usize = 64;

    for row in 0..SHADOW_MAP_SIDE {
        let last_source_col = shadow_map_bit(shadow_map, row, 62);
        set_shadow_map_bit(shadow_map, row, 63, last_source_col);
    }
    for col in 0..SHADOW_MAP_SIDE {
        let last_source_row = shadow_map_bit(shadow_map, 62, col);
        set_shadow_map_bit(shadow_map, 63, col, last_source_row);
    }
}

fn shadow_map_bit(shadow_map: &[u8; MCSH_BYTES], row: usize, col: usize) -> bool {
    let byte_index = row * 8 + col / 8;
    let mask = 1 << (7 - (col % 8));
    (shadow_map[byte_index] & mask) != 0
}

fn set_shadow_map_bit(shadow_map: &mut [u8; MCSH_BYTES], row: usize, col: usize, value: bool) {
    let byte_index = row * 8 + col / 8;
    let mask = 1 << (7 - (col % 8));
    if value {
        shadow_map[byte_index] |= mask;
    } else {
        shadow_map[byte_index] &= !mask;
    }
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

fn parse_mcbb(payload: &[u8]) -> Result<Vec<BlendBatch>, String> {
    if !payload.len().is_multiple_of(MCBB_BYTES_PER_BATCH) {
        return Err(format!(
            "MCBB size must be a multiple of {MCBB_BYTES_PER_BATCH} bytes: {} bytes",
            payload.len()
        ));
    }

    let mut batches = Vec::with_capacity(payload.len() / MCBB_BYTES_PER_BATCH);
    let mut cursor = Cursor::new(payload);
    while (cursor.position() as usize) < payload.len() {
        batches.push(
            BlendBatch::read_le(&mut cursor)
                .map_err(|err| format!("MCBB batch parse failed: {err}"))?,
        );
    }
    Ok(batches)
}

fn parse_mcdd(payload: &[u8]) -> Result<[u8; MCDD_BYTES], String> {
    if payload.len() < MCDD_BYTES {
        return Err(format!(
            "MCDD too small: {} bytes (need {})",
            payload.len(),
            MCDD_BYTES
        ));
    }

    let mut disable = [0; MCDD_BYTES];
    disable.copy_from_slice(&payload[..MCDD_BYTES]);
    Ok(disable)
}

fn parse_blend_mesh_headers(payload: &[u8]) -> Result<Vec<BlendMeshHeader>, String> {
    parse_binrw_array(payload, MBMH_BYTES_PER_HEADER, "MBMH header")
}

fn parse_blend_mesh_bounds(payload: &[u8]) -> Result<Vec<BlendMeshBounds>, String> {
    parse_binrw_array(payload, MBBB_BYTES_PER_BOUND, "MBBB bounds")
}

fn parse_blend_mesh_vertices(payload: &[u8]) -> Result<Vec<BlendMeshVertex>, String> {
    parse_binrw_array(payload, MBNV_BYTES_PER_VERTEX, "MBNV vertex")
}

fn parse_blend_mesh_indices(payload: &[u8]) -> Result<Vec<u16>, String> {
    if !payload.len().is_multiple_of(size_of::<u16>()) {
        return Err(format!(
            "MBMI size must be a multiple of {} bytes: {} bytes",
            size_of::<u16>(),
            payload.len()
        ));
    }

    Ok(payload
        .chunks_exact(size_of::<u16>())
        .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

fn parse_binrw_array<T>(payload: &[u8], entry_size: usize, label: &str) -> Result<Vec<T>, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    if !payload.len().is_multiple_of(entry_size) {
        return Err(format!(
            "{label} array size must be a multiple of {entry_size} bytes: {} bytes",
            payload.len()
        ));
    }

    let mut entries = Vec::with_capacity(payload.len() / entry_size);
    let mut cursor = Cursor::new(payload);
    while (cursor.position() as usize) < payload.len() {
        entries
            .push(T::read_le(&mut cursor).map_err(|err| format!("{label} parse failed: {err}"))?);
    }
    Ok(entries)
}

fn parse_blend_mesh_data(root_chunks: &AdtRootChunks<'_>) -> Result<Option<BlendMeshData>, String> {
    let has_any_blend_mesh_chunk = root_chunks.mbmh.is_some()
        || root_chunks.mbbb.is_some()
        || root_chunks.mbnv.is_some()
        || root_chunks.mbmi.is_some();
    if !has_any_blend_mesh_chunk {
        return Ok(None);
    }

    let headers = root_chunks
        .mbmh
        .ok_or("ADT has blend mesh chunks but is missing HMBM (MBMH)".to_string())
        .and_then(parse_blend_mesh_headers)?;
    let vertices = root_chunks
        .mbnv
        .ok_or("ADT has blend mesh chunks but is missing VNBM (MBNV)".to_string())
        .and_then(parse_blend_mesh_vertices)?;
    let indices = root_chunks
        .mbmi
        .ok_or("ADT has blend mesh chunks but is missing IMBM (MBMI)".to_string())
        .and_then(parse_blend_mesh_indices)?;
    let bounds = match root_chunks.mbbb {
        Some(payload) => parse_blend_mesh_bounds(payload)?,
        None => Vec::new(),
    };

    Ok(Some(BlendMeshData {
        headers,
        bounds,
        vertices,
        indices,
    }))
}

fn parse_mfbo(payload: &[u8]) -> Result<FlightBounds, String> {
    const MFBO_ENTRIES: usize = 18;
    const MFBO_BYTES: usize = MFBO_ENTRIES * size_of::<i16>();

    if payload.len() < MFBO_BYTES {
        return Err(format!(
            "MFBO too small: {} bytes (need {})",
            payload.len(),
            MFBO_BYTES
        ));
    }

    let mut values = [0i16; MFBO_ENTRIES];
    for (index, value) in values.iter_mut().enumerate() {
        let base = index * size_of::<i16>();
        *value = i16::from_le_bytes(payload[base..base + size_of::<i16>()].try_into().unwrap());
    }

    let mut min_heights = [0i16; 9];
    let mut max_heights = [0i16; 9];
    min_heights.copy_from_slice(&values[..9]);
    max_heights.copy_from_slice(&values[9..]);
    Ok(FlightBounds {
        min_heights,
        max_heights,
    })
}

fn parse_mver(payload: &[u8]) -> Result<u32, String> {
    read_u32(payload, 0).map_err(|err| format!("MVER parse failed: {err}"))
}

fn parse_mlhd(payload: &[u8]) -> Result<LodHeader, String> {
    if payload.len() < 28 {
        return Err(format!("MLHD too small: {} bytes (need 28)", payload.len()));
    }

    Ok(LodHeader {
        flags: read_u32(payload, 0)?,
        bounds_min: [
            read_f32(payload, 4)?,
            read_f32(payload, 12)?,
            read_f32(payload, 20)?,
        ],
        bounds_max: [
            read_f32(payload, 8)?,
            read_f32(payload, 16)?,
            read_f32(payload, 24)?,
        ],
    })
}

fn parse_mlvh(payload: &[u8]) -> Result<Vec<f32>, String> {
    if !payload.len().is_multiple_of(size_of::<f32>()) {
        return Err(format!(
            "MLVH size {} is not a multiple of {}",
            payload.len(),
            size_of::<f32>()
        ));
    }

    (0..payload.len())
        .step_by(size_of::<f32>())
        .map(|offset| read_f32(payload, offset))
        .collect()
}

fn parse_mlll(payload: &[u8]) -> Result<Vec<LodLevel>, String> {
    const MLLL_RECORD_BYTES: usize = 20;
    if !payload.len().is_multiple_of(MLLL_RECORD_BYTES) {
        return Err(format!(
            "MLLL size {} is not a multiple of {}",
            payload.len(),
            MLLL_RECORD_BYTES
        ));
    }

    (0..payload.len())
        .step_by(MLLL_RECORD_BYTES)
        .map(|offset| {
            Ok(LodLevel {
                vertex_step: read_f32(payload, offset)?,
                payload: [
                    read_u32(payload, offset + 4)?,
                    read_u32(payload, offset + 8)?,
                    read_u32(payload, offset + 12)?,
                    read_u32(payload, offset + 16)?,
                ],
            })
        })
        .collect()
}

fn parse_mlnd(payload: &[u8]) -> Result<Vec<LodQuadTreeNode>, String> {
    const MLND_RECORD_BYTES: usize = 20;
    if !payload.len().is_multiple_of(MLND_RECORD_BYTES) {
        return Err(format!(
            "MLND size {} is not a multiple of {}",
            payload.len(),
            MLND_RECORD_BYTES
        ));
    }

    (0..payload.len())
        .step_by(MLND_RECORD_BYTES)
        .map(|offset| {
            Ok(LodQuadTreeNode {
                words16: [
                    u16::from_le_bytes(payload[offset..offset + 2].try_into().unwrap()),
                    u16::from_le_bytes(payload[offset + 2..offset + 4].try_into().unwrap()),
                    u16::from_le_bytes(payload[offset + 4..offset + 6].try_into().unwrap()),
                    u16::from_le_bytes(payload[offset + 6..offset + 8].try_into().unwrap()),
                ],
                words32: [
                    read_u32(payload, offset + 8)?,
                    read_u32(payload, offset + 12)?,
                    read_u32(payload, offset + 16)?,
                ],
            })
        })
        .collect()
}

fn parse_u16_block(payload: &[u8], label: &str) -> Result<Vec<u16>, String> {
    if !payload.len().is_multiple_of(size_of::<u16>()) {
        return Err(format!(
            "{label} size {} is not a multiple of {}",
            payload.len(),
            size_of::<u16>()
        ));
    }

    (0..payload.len())
        .step_by(size_of::<u16>())
        .map(|offset| {
            Ok(u16::from_le_bytes(
                payload[offset..offset + size_of::<u16>()]
                    .try_into()
                    .unwrap(),
            ))
        })
        .collect()
}

fn parse_mlln(payload: &[u8]) -> Result<LodLiquidPatchHeader, String> {
    const MLLN_WORDS: usize = 6;
    const MLLN_BYTES: usize = MLLN_WORDS * size_of::<u32>();
    if payload.len() < MLLN_BYTES {
        return Err(format!(
            "MLLN too small: {} bytes (need {})",
            payload.len(),
            MLLN_BYTES
        ));
    }

    let mut words = [0u32; MLLN_WORDS];
    for (index, value) in words.iter_mut().enumerate() {
        *value = read_u32(payload, index * size_of::<u32>())?;
    }
    Ok(LodLiquidPatchHeader { words })
}

fn parse_mllv(payload: &[u8]) -> Result<Vec<[f32; 3]>, String> {
    const MLLV_VERTEX_BYTES: usize = 3 * size_of::<f32>();
    if !payload.len().is_multiple_of(MLLV_VERTEX_BYTES) {
        return Err(format!(
            "MLLV size {} is not a multiple of {}",
            payload.len(),
            MLLV_VERTEX_BYTES
        ));
    }

    (0..payload.len())
        .step_by(MLLV_VERTEX_BYTES)
        .map(|offset| {
            Ok([
                read_f32(payload, offset)?,
                read_f32(payload, offset + size_of::<f32>())?,
                read_f32(payload, offset + size_of::<f32>() * 2)?,
            ])
        })
        .collect()
}

fn parse_lod_liquids(groups: Vec<LodLiquidChunkGroup<'_>>) -> Result<Vec<LodLiquidPatch>, String> {
    groups
        .into_iter()
        .map(|group| {
            Ok(LodLiquidPatch {
                header: parse_mlln(group.header)?,
                indices: parse_u16_block(group.indices, "MLLI")?,
                vertices: parse_mllv(group.vertices)?,
            })
        })
        .collect()
}

fn parse_lod_object_placements(
    payload: Option<&[u8]>,
    label: &str,
) -> Result<Vec<LodObjectPlacement>, String> {
    const LOD_OBJECT_PLACEMENT_BYTES: usize = 40;
    let Some(payload) = payload else {
        return Ok(Vec::new());
    };
    if !payload.len().is_multiple_of(LOD_OBJECT_PLACEMENT_BYTES) {
        return Err(format!(
            "{label} size {} is not a multiple of {}",
            payload.len(),
            LOD_OBJECT_PLACEMENT_BYTES
        ));
    }

    (0..payload.len())
        .step_by(LOD_OBJECT_PLACEMENT_BYTES)
        .map(|offset| {
            Ok(LodObjectPlacement {
                id: read_u32(payload, offset)?,
                asset_id: read_u32(payload, offset + 4)?,
                position: [
                    read_f32(payload, offset + 8)?,
                    read_f32(payload, offset + 12)?,
                    read_f32(payload, offset + 16)?,
                ],
                rotation: [
                    read_f32(payload, offset + 20)?,
                    read_f32(payload, offset + 24)?,
                    read_f32(payload, offset + 28)?,
                ],
                scale: read_f32(payload, offset + 32)?,
                flags: read_u32(payload, offset + 36)?,
            })
        })
        .collect()
}

fn parse_lod_object_visibility(
    payload: Option<&[u8]>,
    label: &str,
) -> Result<Vec<LodObjectVisibility>, String> {
    const LOD_OBJECT_VISIBILITY_BYTES: usize = 28;
    let Some(payload) = payload else {
        return Ok(Vec::new());
    };
    if !payload.len().is_multiple_of(LOD_OBJECT_VISIBILITY_BYTES) {
        return Err(format!(
            "{label} size {} is not a multiple of {}",
            payload.len(),
            LOD_OBJECT_VISIBILITY_BYTES
        ));
    }

    (0..payload.len())
        .step_by(LOD_OBJECT_VISIBILITY_BYTES)
        .map(|offset| {
            Ok(LodObjectVisibility {
                bounds_min: [
                    read_f32(payload, offset)?,
                    read_f32(payload, offset + 4)?,
                    read_f32(payload, offset + 8)?,
                ],
                bounds_max: [
                    read_f32(payload, offset + 12)?,
                    read_f32(payload, offset + 16)?,
                    read_f32(payload, offset + 20)?,
                ],
                radius: read_f32(payload, offset + 24)?,
            })
        })
        .collect()
}

fn parse_mcnk(payload: &[u8]) -> Result<McnkData, String> {
    if payload.len() < size_of::<McnkHeader>() {
        return Err(format!("MCNK payload too small: {} bytes", payload.len()));
    }
    let header: McnkHeader = parse_binrw_value(payload, 0, "MCNK header")?;
    let flags = McnkFlags::from_bits(header.flags);
    let subchunks = parse_mcnk_subchunks(&payload[128..], flags)?;
    Ok(build_mcnk_data(header, flags, subchunks))
}

fn build_mcnk_data(
    header: McnkHeader,
    flags: McnkFlags,
    subchunks: McnkSubchunksResult,
) -> McnkData {
    let (
        heights,
        normals,
        vertex_colors,
        vertex_lighting,
        shadow_map,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
    ) = subchunks;
    McnkData {
        index_x: header.index_x,
        index_y: header.index_y,
        pos: [header.pos_x, header.pos_y, header.pos_z],
        flags,
        area_id: header._area_id,
        shadow_map,
        vertex_lighting,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
        holes_low_res: header._holes_low_res,
        holes_high_res: flags.high_res_holes.then_some(header._holes_high_res),
        heights,
        normals,
        vertex_colors,
    }
}

fn parse_mcnk_subchunks(sub: &[u8], flags: McnkFlags) -> Result<McnkSubchunksResult, String> {
    let mut accum = empty_mcnk_subchunk_accum();
    apply_mcnk_subchunk_stream(&mut accum, sub)?;
    finalize_mcnk_subchunks(accum, flags)
}

fn empty_mcnk_subchunk_accum() -> McnkSubchunkAccum {
    McnkSubchunkAccum {
        heights: None,
        normals: None,
        vertex_colors: None,
        vertex_lighting: None,
        shadow_map: None,
        sound_emitters: None,
        blend_batches: None,
        detail_doodad_disable: None,
    }
}

fn apply_mcnk_subchunk_stream(accum: &mut McnkSubchunkAccum, sub: &[u8]) -> Result<(), String> {
    for chunk in ChunkIter::new(sub) {
        let (tag, payload) = chunk?;
        apply_mcnk_subchunk(accum, tag, payload)?;
    }
    Ok(())
}

fn apply_mcnk_subchunk(
    accum: &mut McnkSubchunkAccum,
    tag: &[u8; 4],
    payload: &[u8],
) -> Result<(), String> {
    match tag {
        b"TVCM" => accum.heights = Some(parse_mcvt(payload)?),
        b"RNCM" => accum.normals = Some(parse_mcnr(payload)?),
        b"VCCM" => accum.vertex_colors = Some(parse_mccv(payload)?),
        b"VLCM" => accum.vertex_lighting = Some(parse_mclv(payload)?),
        b"HSCM" => accum.shadow_map = Some(parse_mcsh(payload)?),
        b"MCSE" => accum.sound_emitters = Some(parse_mcse(payload)?),
        b"BBCM" => accum.blend_batches = Some(parse_mcbb(payload)?),
        b"DDCM" => accum.detail_doodad_disable = Some(parse_mcdd(payload)?),
        _ => {}
    }
    Ok(())
}

fn finalize_mcnk_subchunks(
    accum: McnkSubchunkAccum,
    flags: McnkFlags,
) -> Result<McnkSubchunksResult, String> {
    let heights = accum.heights.ok_or("MCNK missing TVCM sub-chunk")?;
    let normals = accum.normals.unwrap_or([[0.0, 1.0, 0.0]; MCVT_COUNT]);
    let vertex_colors = resolve_mcnk_vertex_colors(accum.vertex_colors, flags)?;
    let shadow_map = resolve_mcnk_shadow_map(accum.shadow_map, flags)?;
    Ok((
        heights,
        normals,
        vertex_colors,
        accum.vertex_lighting,
        shadow_map,
        accum.sound_emitters.unwrap_or_default(),
        accum.blend_batches.unwrap_or_default(),
        accum.detail_doodad_disable,
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
        Some(mut shadow_map) => {
            if !flags.do_not_fix_alpha_map {
                fix_shadow_map_edges(&mut shadow_map);
            }
            Ok(Some(shadow_map))
        }
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
    let mut root_chunks = AdtRootChunks {
        mcnks: Vec::with_capacity(256),
        mh2o: None,
        mfbo: None,
        mbmh: None,
        mbbb: None,
        mbnv: None,
        mbmi: None,
    };
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"KNCM" => root_chunks.mcnks.push(payload),
            b"O2HM" => root_chunks.mh2o = Some(payload),
            b"OFBM" => root_chunks.mfbo = Some(payload),
            b"HMBM" => root_chunks.mbmh = Some(payload),
            b"BBBM" => root_chunks.mbbb = Some(payload),
            b"VNBM" => root_chunks.mbnv = Some(payload),
            b"IMBM" => root_chunks.mbmi = Some(payload),
            _ => {}
        }
    }
    if root_chunks.mcnks.is_empty() {
        return Err("No KNCM (MCNK) chunks found in ADT file".to_string());
    }
    Ok(root_chunks)
}

fn collect_lod_chunks(data: &[u8]) -> Result<LodRootChunks<'_>, String> {
    let mut root_chunks = LodRootChunks {
        mver: None,
        mlhd: None,
        mlvh: None,
        mlll: None,
        mlnd: None,
        mlvi: None,
        mlsi: None,
        mlld: None,
        mldd: None,
        mldx: None,
        mlmd: None,
        mlmx: None,
        liquid_groups: Vec::new(),
    };
    let mut pending_liquid_header = None;
    let mut pending_liquid_indices = None;

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"REVM" => root_chunks.mver = Some(payload),
            b"DHLM" => root_chunks.mlhd = Some(payload),
            b"HVLM" => root_chunks.mlvh = Some(payload),
            b"LLLM" => root_chunks.mlll = Some(payload),
            b"DNLM" => root_chunks.mlnd = Some(payload),
            b"IVLM" => root_chunks.mlvi = Some(payload),
            b"ISLM" => root_chunks.mlsi = Some(payload),
            b"DLLM" => root_chunks.mlld = Some(payload),
            b"DDLM" => root_chunks.mldd = Some(payload),
            b"XDLM" => root_chunks.mldx = Some(payload),
            b"DMLM" => root_chunks.mlmd = Some(payload),
            b"XMLM" => root_chunks.mlmx = Some(payload),
            b"NLLM" => {
                pending_liquid_header = Some(payload);
                pending_liquid_indices = None;
            }
            b"ILLM" => {
                if pending_liquid_header.is_some() {
                    pending_liquid_indices = Some(payload);
                }
            }
            b"VLLM" => {
                let Some(header) = pending_liquid_header.take() else {
                    return Err("VLLM encountered before NLLM in _lod.adt file".to_string());
                };
                let Some(indices) = pending_liquid_indices.take() else {
                    return Err("VLLM encountered before ILLM in _lod.adt file".to_string());
                };
                root_chunks.liquid_groups.push(LodLiquidChunkGroup {
                    header,
                    indices,
                    vertices: payload,
                });
            }
            _ => {}
        }
    }

    if pending_liquid_header.is_some() || pending_liquid_indices.is_some() {
        return Err("Incomplete MLLN/MLLI/MLLV liquid group in _lod.adt file".to_string());
    }

    if root_chunks.mver.is_none() {
        return Err("No REVM (MVER) chunk found in _lod.adt file".to_string());
    }
    if root_chunks.mlhd.is_none() {
        return Err("No DHLM (MLHD) chunk found in _lod.adt file".to_string());
    }
    if root_chunks.mlvh.is_none() {
        return Err("No HVLM (MLVH) chunk found in _lod.adt file".to_string());
    }
    if root_chunks.mlll.is_none() {
        return Err("No LLLM (MLLL) chunk found in _lod.adt file".to_string());
    }
    if root_chunks.mlnd.is_none() {
        return Err("No DNLM (MLND) chunk found in _lod.adt file".to_string());
    }
    if root_chunks.mlvi.is_none() {
        return Err("No IVLM (MLVI) chunk found in _lod.adt file".to_string());
    }
    if root_chunks.mlsi.is_none() {
        return Err("No ISLM (MLSI) chunk found in _lod.adt file".to_string());
    }

    Ok(root_chunks)
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
    let root_chunks = collect_adt_chunks(data)?;
    let blend_mesh = parse_blend_mesh_data(&root_chunks)?;
    let flight_bounds = root_chunks.mfbo.map(parse_mfbo).transpose()?;
    let mut parsed: Vec<McnkData> = root_chunks
        .mcnks
        .into_iter()
        .map(parse_mcnk)
        .collect::<Result<Vec<_>, String>>()?;
    if stitch {
        stitch_chunk_edges(&mut parsed);
    }
    let center_surface = center_surface_position(&parsed, tile_coords);
    let chunk_positions = parsed.iter().map(|d| d.pos).collect();
    let height_grids = build_height_grids(&parsed, tile_coords);
    let (water, water_error) = match root_chunks.mh2o {
        Some(payload) => match parse_mh2o(payload) {
            Ok(water) => (Some(water), None),
            Err(err) => (None, Some(err)),
        },
        None => (None, None),
    };
    Ok(ParsedAdtData {
        chunks: parsed,
        blend_mesh,
        flight_bounds,
        height_grids,
        center_surface,
        chunk_positions,
        water,
        water_error,
    })
}

pub(crate) fn load_lod_adt(data: &[u8]) -> Result<ParsedLodData, String> {
    let root_chunks = collect_lod_chunks(data)?;

    Ok(ParsedLodData {
        version: parse_mver(root_chunks.mver.unwrap())?,
        header: parse_mlhd(root_chunks.mlhd.unwrap())?,
        heights: parse_mlvh(root_chunks.mlvh.unwrap())?,
        levels: parse_mlll(root_chunks.mlll.unwrap())?,
        nodes: parse_mlnd(root_chunks.mlnd.unwrap())?,
        indices: parse_u16_block(root_chunks.mlvi.unwrap(), "MLVI")?,
        skirt_indices: parse_u16_block(root_chunks.mlsi.unwrap(), "MLSI")?,
        liquid_directory: root_chunks.mlld.map(|payload| LodLiquidDirectory {
            raw: payload.to_vec(),
        }),
        liquids: parse_lod_liquids(root_chunks.liquid_groups)?,
        m2_placements: parse_lod_object_placements(root_chunks.mldd, "MLDD")?,
        m2_visibility: parse_lod_object_visibility(root_chunks.mldx, "MLDX")?,
        wmo_placements: parse_lod_object_placements(root_chunks.mlmd, "MLMD")?,
        wmo_visibility: parse_lod_object_visibility(root_chunks.mlmx, "MLMX")?,
    })
}

#[cfg(test)]
#[path = "adt_tests/mod.rs"]
mod tests;
