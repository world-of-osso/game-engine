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
    /// A LiquidObject instance's raw vertex bytes, whose format its LiquidMaterial names.
    pub object_vertex_bytes: Vec<u8>,
}

/// `liquid_object_or_lvf` from this value on is a `LiquidObject` ID, below it a vertex
/// format (WebWowViewerCpp LiquidInstance.cpp `createAdtVertexData`).
pub const FIRST_LIQUID_OBJECT: u16 = 42;

impl WaterLayer {
    /// Re-reads a LiquidObject instance's vertices in its material's vertex format `lvf`.
    pub fn decode_object_vertices(&mut self, lvf: u8) -> Result<(), String> {
        if self.object_vertex_bytes.is_empty() {
            return Ok(());
        }
        (self.vertex_heights, self.vertex_uvs, self.vertex_depths) =
            decode_liquid_vertices(&self.object_vertex_bytes, self.width, self.height, lvf)?;
        Ok(())
    }
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

pub type WaterVertexData = (Vec<f32>, Vec<[f32; 2]>, Vec<u8>);

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
    // wowdev MH2O / WebWowViewerCpp LiquidInstance.cpp:264-265: one bit stream over the
    // layer's cells, bit `y * width + x`, in (width * height + 7) / 8 bytes. Rows of
    // `exists` hold the layer-relative columns.
    let (w, h) = (width as usize, height as usize);
    let size = (w * h).div_ceil(8);
    let Some(bits) = payload.get(offset..offset + size) else {
        return Err(format!(
            "MH2O exists bitmask out of bounds: offset {offset:#x}, need {size} bytes"
        ));
    };
    for (y, row) in exists.iter_mut().enumerate().take(h) {
        for x in 0..w {
            let index = y * w + x;
            if (bits[index / 8] >> (index % 8)) & 1 != 0 {
                *row |= 1 << x;
            }
        }
    }
    Ok(exists)
}

fn vertex_count(width: u8, height: u8) -> usize {
    (width as usize + 1) * (height as usize + 1)
}

/// `count` little-endian heights at the start of `data`.
fn read_heights(data: &[u8], count: usize) -> Result<Vec<f32>, String> {
    let bytes = data.get(..count * 4).ok_or_else(|| {
        format!(
            "MH2O heightmap needs {} bytes, has {}",
            count * 4,
            data.len()
        )
    })?;
    (0..count).map(|index| read_f32(bytes, index * 4)).collect()
}

fn read_uvs(data: &[u8], count: usize) -> Result<Vec<[f32; 2]>, String> {
    let bytes = data
        .get(..count * 4)
        .ok_or_else(|| format!("MH2O UV map needs {} bytes, has {}", count * 4, data.len()))?;
    Ok(bytes
        .chunks_exact(4)
        .map(|uv| {
            let u = u16::from_le_bytes([uv[0], uv[1]]);
            let v = u16::from_le_bytes([uv[2], uv[3]]);
            [f32::from(u) / 255.0, f32::from(v) / 255.0]
        })
        .collect())
}

fn read_depths(data: &[u8], count: usize) -> Result<Vec<u8>, String> {
    data.get(..count)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| format!("MH2O depthmap needs {count} bytes, has {}", data.len()))
}

/// wowdev MH2O `LiquidVertexFormat` arrays, one after another: LVF 0 heights then depths,
/// LVF 1 heights then UVs, LVF 2 depths, LVF 3 heights, UVs, then depths.
pub fn decode_liquid_vertices(
    data: &[u8],
    width: u8,
    height: u8,
    lvf: u8,
) -> Result<WaterVertexData, String> {
    let count = vertex_count(width, height);
    let uses_heights = lvf != 2;
    let heights = if uses_heights {
        read_heights(data, count)?
    } else {
        Vec::new()
    };
    let after_heights = if uses_heights {
        &data[count * 4..]
    } else {
        data
    };
    let uvs = if matches!(lvf, 1 | 3) {
        read_uvs(after_heights, count)?
    } else {
        Vec::new()
    };
    let after_uvs = &after_heights[uvs.len() * 4..];
    let depths = match lvf {
        0 | 2 | 3 => read_depths(after_uvs, count)?,
        1 => Vec::new(),
        other => {
            return Err(format!(
                "MH2O liquid vertex format {other} is not supported"
            ));
        }
    };
    Ok((heights, uvs, depths))
}

/// The vertex bytes of an instance, up to the largest format's size (LVF 3: 9 bytes per
/// vertex); `None` without vertex data.
fn vertex_bytes<'a>(payload: &'a [u8], header: &LiquidInstanceHeader) -> Option<&'a [u8]> {
    let offset = header.vertex_offset as usize;
    if offset == 0 {
        return None;
    }
    let end = payload
        .len()
        .min(offset + vertex_count(header.width, header.height) * 9);
    Some(payload.get(offset..end).unwrap_or(&[]))
}

fn parse_liquid_instance(payload: &[u8], off: usize) -> Result<WaterLayer, String> {
    let header = read_liquid_instance_header(payload, off)?;
    let vertex_data = read_liquid_vertex_data(payload, &header)?;
    let exists = read_liquid_exists_bitmask(payload, &header)?;
    Ok(build_water_layer(header, exists, vertex_data))
}

fn read_liquid_instance_header(
    payload: &[u8],
    offset: usize,
) -> Result<LiquidInstanceHeader, String> {
    if offset + size_of::<LiquidInstanceHeader>() > payload.len() {
        return Err(format!(
            "SLiquidInstance out of bounds at {offset:#x} (payload len {:#x})",
            payload.len()
        ));
    }
    parse_binrw_value(payload, offset, "SLiquidInstance")
}

/// `liquid_object_or_lvf` below [`FIRST_LIQUID_OBJECT`] is the vertex format. A LiquidObject's
/// format comes from its LiquidMaterial, unknown here: its bytes are kept for
/// [`WaterLayer::decode_object_vertices`], and are provisionally read as LVF 0 (the original
/// parser's reading) only when they fit that format.
fn read_liquid_vertex_data(
    payload: &[u8],
    header: &LiquidInstanceHeader,
) -> Result<(WaterVertexData, Vec<u8>), String> {
    let Some(data) = vertex_bytes(payload, header) else {
        return Ok(((Vec::new(), Vec::new(), Vec::new()), Vec::new()));
    };
    if header.liquid_object < FIRST_LIQUID_OBJECT {
        let lvf = header.liquid_object as u8;
        return Ok((
            decode_liquid_vertices(data, header.width, header.height, lvf)?,
            Vec::new(),
        ));
    }
    let provisional =
        decode_liquid_vertices(data, header.width, header.height, 0).unwrap_or_default();
    Ok((provisional, data.to_vec()))
}

fn read_liquid_exists_bitmask(
    payload: &[u8],
    header: &LiquidInstanceHeader,
) -> Result<[u8; 8], String> {
    read_exists_bitmask(
        payload,
        header.exists_offset as usize,
        header.width,
        header.height,
    )
}

fn build_water_layer(
    header: LiquidInstanceHeader,
    exists: [u8; 8],
    (vertex_data, object_vertex_bytes): (WaterVertexData, Vec<u8>),
) -> WaterLayer {
    let (vertex_heights, vertex_uvs, vertex_depths) = vertex_data;
    WaterLayer {
        liquid_type: header.liquid_type,
        liquid_object: header.liquid_object,
        min_height: header.min_height,
        max_height: header.max_height,
        x_offset: header.x_offset,
        y_offset: header.y_offset,
        width: header.width,
        height: header.height,
        exists,
        vertex_heights,
        vertex_uvs,
        vertex_depths,
        object_vertex_bytes,
    }
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
