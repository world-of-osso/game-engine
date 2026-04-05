use std::mem::size_of;

use crate::asset::read_bytes::read_f32;
use binrw::BinRead;

use super::parse_binrw_value;

#[derive(BinRead)]
#[br(little)]
pub(super) struct LiquidInstanceHeader {
    liquid_type: u16,
    liquid_object: u16,
    min_height: f32,
    max_height: f32,
    x_offset: u8,
    y_offset: u8,
    width: u8,
    height: u8,
    exists_offset: u32,
    vertex_offset: u32,
}

#[derive(BinRead)]
#[br(little)]
struct HeightUvVertex {
    height: f32,
    u: u16,
    v: u16,
}

#[derive(BinRead)]
#[br(little)]
#[repr(C, packed)]
struct HeightUvDepthVertex {
    height: f32,
    u: u16,
    v: u16,
    depth: u8,
}

#[derive(BinRead)]
#[br(little)]
pub(super) struct Mh2oChunkHeader {
    instance_offset: u32,
    layer_count: u32,
    attributes_offset: u32,
}

#[derive(BinRead)]
#[br(little)]
pub(super) struct Mh2oAttributes {
    pub(super) fishable: u64,
    pub(super) deep: u64,
}

pub struct WaterLayer {
    pub liquid_type: u16,
    pub liquid_object: u16,
    pub min_height: f32,
    pub max_height: f32,
    pub x_offset: u8,
    pub y_offset: u8,
    pub width: u8,
    pub height: u8,
    pub exists: [u8; 8],
    pub vertex_heights: Vec<f32>,
    pub vertex_uvs: Vec<[f32; 2]>,
    pub vertex_depths: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WaterAttributes {
    pub fishable: u64,
    pub deep: u64,
}

impl WaterAttributes {
    const TILE_SIZE: usize = 8;

    pub fn is_fishable(&self, x: usize, y: usize) -> bool {
        water_attribute_bit(self.fishable, x, y)
    }

    pub fn is_deep(&self, x: usize, y: usize) -> bool {
        water_attribute_bit(self.deep, x, y)
    }
}

pub struct ChunkWater {
    pub layers: Vec<WaterLayer>,
    pub attributes: Option<WaterAttributes>,
}

pub struct AdtWaterData {
    pub chunks: Vec<ChunkWater>,
}

type WaterVertexData = (Vec<f32>, Vec<[f32; 2]>, Vec<u8>);

fn water_attribute_bit(mask: u64, x: usize, y: usize) -> bool {
    if x >= WaterAttributes::TILE_SIZE || y >= WaterAttributes::TILE_SIZE {
        return false;
    }
    let bit_index = y * WaterAttributes::TILE_SIZE + x;
    ((mask >> bit_index) & 1) != 0
}

fn read_exists_bitmask(
    payload: &[u8],
    offset: usize,
    width: u8,
    height: u8,
) -> Result<[u8; 8], String> {
    let mut exists = [0u8; 8];
    if offset == 0 {
        let mask = (1u16.wrapping_shl(width as u32) - 1) as u8;
        for slot in exists.iter_mut().take(height as usize) {
            *slot = mask;
        }
        return Ok(exists);
    }
    let h = height as usize;
    if offset + h > payload.len() {
        return Err(format!(
            "MH2O exists bitmask out of bounds: offset {offset:#x}, need {h} bytes"
        ));
    }
    exists[..h].copy_from_slice(&payload[offset..offset + h]);
    Ok(exists)
}

fn read_vertex_heights(
    payload: &[u8],
    offset: usize,
    width: u8,
    height: u8,
) -> Result<Vec<f32>, String> {
    if offset == 0 {
        return Ok(Vec::new());
    }
    let count = (width as usize + 1) * (height as usize + 1);
    let byte_len = count * 4;
    if offset + byte_len > payload.len() {
        return Err(format!(
            "MH2O vertex data out of bounds: offset {offset:#x}, need {byte_len} bytes"
        ));
    }
    let mut heights = Vec::with_capacity(count);
    for i in 0..count {
        heights.push(read_f32(payload, offset + i * 4)?);
    }
    Ok(heights)
}

fn read_height_uv_vertices(
    payload: &[u8],
    offset: usize,
    width: u8,
    height: u8,
) -> Result<(Vec<f32>, Vec<[f32; 2]>), String> {
    if offset == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let count = (width as usize + 1) * (height as usize + 1);
    let byte_len = count * size_of::<HeightUvVertex>();
    if offset + byte_len > payload.len() {
        return Err(format!(
            "MH2O LVF1 vertex data out of bounds: offset {offset:#x}, need {byte_len} bytes"
        ));
    }

    let mut heights = Vec::with_capacity(count);
    let mut uvs = Vec::with_capacity(count);
    for i in 0..count {
        let vertex: HeightUvVertex = parse_binrw_value(
            payload,
            offset + i * size_of::<HeightUvVertex>(),
            "MH2O LVF1 vertex",
        )?;
        heights.push(vertex.height);
        uvs.push([f32::from(vertex.u) / 255.0, f32::from(vertex.v) / 255.0]);
    }
    Ok((heights, uvs))
}

fn read_depth_only_vertices(
    payload: &[u8],
    offset: usize,
    width: u8,
    height: u8,
) -> Result<Vec<u8>, String> {
    if offset == 0 {
        return Ok(Vec::new());
    }
    let count = (width as usize + 1) * (height as usize + 1);
    if offset + count > payload.len() {
        return Err(format!(
            "MH2O LVF2 vertex data out of bounds: offset {offset:#x}, need {count} bytes"
        ));
    }
    Ok(payload[offset..offset + count].to_vec())
}

fn read_height_uv_depth_vertices(
    payload: &[u8],
    offset: usize,
    width: u8,
    height: u8,
) -> Result<WaterVertexData, String> {
    if offset == 0 {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    }
    let count = (width as usize + 1) * (height as usize + 1);
    let byte_len = count * size_of::<HeightUvDepthVertex>();
    if offset + byte_len > payload.len() {
        return Err(format!(
            "MH2O LVF3 vertex data out of bounds: offset {offset:#x}, need {byte_len} bytes"
        ));
    }

    let mut heights = Vec::with_capacity(count);
    let mut uvs = Vec::with_capacity(count);
    let mut depths = Vec::with_capacity(count);
    for i in 0..count {
        let vertex: HeightUvDepthVertex = parse_binrw_value(
            payload,
            offset + i * size_of::<HeightUvDepthVertex>(),
            "MH2O LVF3 vertex",
        )?;
        heights.push(vertex.height);
        uvs.push([f32::from(vertex.u) / 255.0, f32::from(vertex.v) / 255.0]);
        depths.push(vertex.depth);
    }
    Ok((heights, uvs, depths))
}

fn read_vertex_data(
    payload: &[u8],
    offset: usize,
    width: u8,
    height: u8,
    liquid_object: u16,
) -> Result<WaterVertexData, String> {
    if liquid_object == 1 {
        let (heights, uvs) = read_height_uv_vertices(payload, offset, width, height)?;
        return Ok((heights, uvs, Vec::new()));
    }
    if liquid_object == 2 {
        return Ok((
            Vec::new(),
            Vec::new(),
            read_depth_only_vertices(payload, offset, width, height)?,
        ));
    }
    if liquid_object == 3 {
        return read_height_uv_depth_vertices(payload, offset, width, height);
    }

    Ok((
        read_vertex_heights(payload, offset, width, height)?,
        Vec::new(),
        Vec::new(),
    ))
}

fn parse_liquid_instance(payload: &[u8], off: usize) -> Result<WaterLayer, String> {
    if off + size_of::<LiquidInstanceHeader>() > payload.len() {
        return Err(format!(
            "SLiquidInstance out of bounds at {off:#x} (payload len {:#x})",
            payload.len()
        ));
    }
    let header: LiquidInstanceHeader = parse_binrw_value(payload, off, "SLiquidInstance")?;
    let (vertex_heights, vertex_uvs, vertex_depths) = read_vertex_data(
        payload,
        header.vertex_offset as usize,
        header.width,
        header.height,
        header.liquid_object,
    )?;
    Ok(WaterLayer {
        liquid_type: header.liquid_type,
        liquid_object: header.liquid_object,
        min_height: header.min_height,
        max_height: header.max_height,
        x_offset: header.x_offset,
        y_offset: header.y_offset,
        width: header.width,
        height: header.height,
        exists: read_exists_bitmask(
            payload,
            header.exists_offset as usize,
            header.width,
            header.height,
        )?,
        vertex_heights,
        vertex_uvs,
        vertex_depths,
    })
}

fn parse_water_attributes(payload: &[u8], offset: usize) -> Result<WaterAttributes, String> {
    let attributes: Mh2oAttributes = parse_binrw_value(payload, offset, "MH2O attributes")?;
    Ok(WaterAttributes {
        fishable: attributes.fishable,
        deep: attributes.deep,
    })
}

pub fn parse_mh2o(payload: &[u8]) -> Result<AdtWaterData, String> {
    const CHUNK_COUNT: usize = 256;
    validate_mh2o_header_size(payload, CHUNK_COUNT)?;

    let mut chunks = Vec::with_capacity(CHUNK_COUNT);
    for i in 0..CHUNK_COUNT {
        chunks.push(parse_mh2o_chunk(payload, i)?);
    }
    Ok(AdtWaterData { chunks })
}

fn validate_mh2o_header_size(payload: &[u8], chunk_count: usize) -> Result<(), String> {
    let header_size = chunk_count * size_of::<Mh2oChunkHeader>();
    if payload.len() < header_size {
        return Err(format!(
            "MH2O chunk too small: {} bytes (need header {header_size})",
            payload.len()
        ));
    }
    Ok(())
}

fn parse_mh2o_chunk(payload: &[u8], chunk_index: usize) -> Result<ChunkWater, String> {
    let header = read_mh2o_chunk_header(payload, chunk_index)?;
    let attributes = read_mh2o_attributes(payload, &header)?;
    let layers = if header.instance_offset == 0 || header.layer_count == 0 {
        Vec::new()
    } else {
        parse_mh2o_layers(payload, &header)?
    };

    Ok(ChunkWater { layers, attributes })
}

fn read_mh2o_chunk_header(payload: &[u8], chunk_index: usize) -> Result<Mh2oChunkHeader, String> {
    let base = chunk_index * size_of::<Mh2oChunkHeader>();
    parse_binrw_value(payload, base, "MH2O chunk header")
}

fn read_mh2o_attributes(
    payload: &[u8],
    header: &Mh2oChunkHeader,
) -> Result<Option<WaterAttributes>, String> {
    if header.attributes_offset == 0 {
        return Ok(None);
    }

    parse_water_attributes(payload, header.attributes_offset as usize).map(Some)
}

fn parse_mh2o_layers(payload: &[u8], header: &Mh2oChunkHeader) -> Result<Vec<WaterLayer>, String> {
    let mut layers = Vec::with_capacity(header.layer_count as usize);
    for layer_idx in 0..header.layer_count as usize {
        let offset =
            header.instance_offset as usize + layer_idx * size_of::<LiquidInstanceHeader>();
        layers.push(parse_liquid_instance(payload, offset)?);
    }
    Ok(layers)
}
