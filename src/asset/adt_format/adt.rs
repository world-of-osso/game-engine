use std::io::Cursor;

use super::adt_tex::{AdtWaterData, parse_mh2o};
use crate::asset::read_bytes::read_u32;
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

mod lod_chunks;
mod parsing;

use lod_chunks::collect_lod_chunks;
use parsing::{
    collect_adt_chunks, parse_blend_mesh_data, parse_lod_liquids, parse_lod_object_placements,
    parse_lod_object_visibility, parse_mcnk, parse_mfbo, parse_mlhd, parse_mlll, parse_mlnd,
    parse_mlvh, parse_mver, parse_u16_block,
};

#[cfg(test)]
fn parse_mccv(payload: &[u8]) -> Result<[[f32; 4]; MCVT_COUNT], String> {
    parsing::parse_mccv(payload)
}

#[cfg(test)]
fn parse_mclv(payload: &[u8]) -> Result<[[f32; 4]; MCVT_COUNT], String> {
    parsing::parse_mclv(payload)
}

#[cfg(test)]
fn parse_mcnk_subchunks(sub: &[u8], flags: McnkFlags) -> Result<McnkSubchunksResult, String> {
    parsing::parse_mcnk_subchunks(sub, flags)
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
        (origin_x - r * UNIT_SIZE, origin_z + c * UNIT_SIZE)
    } else {
        (
            origin_x - r * UNIT_SIZE - HALF_UNIT,
            origin_z + c * UNIT_SIZE + HALF_UNIT,
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

pub(crate) fn load_adt_parsed(data: &[u8]) -> Result<ParsedAdtData, String> {
    load_adt_inner(data, None)
}

pub(crate) fn load_adt_for_tile_parsed(
    data: &[u8],
    tile_y: u32,
    tile_x: u32,
) -> Result<ParsedAdtData, String> {
    load_adt_inner(data, Some((tile_y, tile_x)))
}

#[cfg(test)]
pub(crate) fn load_adt_raw(data: &[u8]) -> Result<ParsedAdtData, String> {
    load_adt_parsed(data)
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

fn load_adt_inner(data: &[u8], tile_coords: Option<(u32, u32)>) -> Result<ParsedAdtData, String> {
    let root_chunks = collect_adt_chunks(data)?;
    let blend_mesh = parse_blend_mesh_data(&root_chunks)?;
    let flight_bounds = root_chunks.mfbo.map(parse_mfbo).transpose()?;
    let parsed: Vec<McnkData> = root_chunks
        .mcnks
        .into_iter()
        .map(parse_mcnk)
        .collect::<Result<Vec<_>, String>>()?;
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
