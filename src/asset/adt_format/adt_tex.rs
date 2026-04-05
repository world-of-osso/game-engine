//! ADT _tex0.adt and MH2O water parsing.

use std::io::Cursor;
use std::mem::size_of;

use crate::asset::read_bytes::{read_f32, read_u32};
use binrw::BinRead;

use super::adt::ChunkIter;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MclyFlags {
    pub raw: u32,
}

impl MclyFlags {
    pub fn animation_rotation(&self) -> u8 {
        (self.raw & 0x7) as u8
    }

    pub fn animation_speed(&self) -> u8 {
        ((self.raw >> 3) & 0x7) as u8
    }

    pub fn animation_enabled(&self) -> bool {
        (self.raw & MCLY_FLAG_ANIMATION_ENABLED) != 0
    }

    pub fn overbright(&self) -> bool {
        (self.raw & MCLY_FLAG_OVERBRIGHT) != 0
    }

    pub fn use_alpha_map(&self) -> bool {
        (self.raw & MCLY_FLAG_USE_ALPHA_MAP) != 0
    }

    pub fn alpha_compressed(&self) -> bool {
        (self.raw & MCLY_FLAG_ALPHA_COMPRESSED) != 0
    }

    pub fn use_cube_map_reflection(&self) -> bool {
        (self.raw & MCLY_FLAG_USE_CUBE_MAP_REFLECTION) != 0
    }
}

pub struct TextureLayer {
    pub texture_index: u32,
    pub flags: MclyFlags,
    pub effect_id: u32,
    pub material_id: u8,
    pub alpha_map: Option<Vec<u8>>,
}

pub struct ChunkTexLayers {
    pub layers: Vec<TextureLayer>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextureParams {
    pub flags: u32,
    pub height_scale: f32,
    pub height_offset: f32,
}

pub struct AdtTexData {
    pub texture_amplifier: Option<u32>,
    pub texture_fdids: Vec<u32>,
    pub height_texture_fdids: Vec<u32>,
    pub texture_flags: Vec<u32>,
    pub texture_params: Vec<TextureParams>,
    pub chunk_layers: Vec<ChunkTexLayers>,
}

#[derive(BinRead)]
#[br(little)]
struct MclyEntry {
    texture_index: u32,
    flags: u32,
    offset_in_mcal: u32,
    _effect_id: u32,
}

#[derive(BinRead)]
#[br(little)]
struct LiquidInstanceHeader {
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
struct Mh2oChunkHeader {
    instance_offset: u32,
    layer_count: u32,
    attributes_offset: u32,
}

#[derive(BinRead)]
#[br(little)]
struct Mh2oAttributes {
    fishable: u64,
    deep: u64,
}

#[derive(BinRead)]
#[br(little)]
struct RawTextureParams {
    flags: u32,
    height_scale: f32,
    height_offset: f32,
    _padding: u32,
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

fn rle_fill(
    out: &mut Vec<u8>,
    src: &[u8],
    i: &mut usize,
    count: usize,
    limit: usize,
) -> Result<(), String> {
    if *i >= src.len() {
        return Err("MCAL RLE fill: missing value byte".to_string());
    }
    let value = src[*i];
    *i += 1;
    for _ in 0..count {
        if out.len() < limit {
            out.push(value);
        }
    }
    Ok(())
}

fn rle_copy(
    out: &mut Vec<u8>,
    src: &[u8],
    i: &mut usize,
    count: usize,
    limit: usize,
) -> Result<(), String> {
    let end = *i + count;
    if end > src.len() {
        return Err(format!(
            "MCAL RLE copy: need {count} bytes at {:#x} but only {} remain",
            *i,
            src.len() - *i
        ));
    }
    for &b in &src[*i..end] {
        if out.len() < limit {
            out.push(b);
        }
    }
    *i = end;
    Ok(())
}

fn decompress_mcal_rle(src: &[u8]) -> Result<Vec<u8>, String> {
    const EXPECTED: usize = 4096;
    let mut out = Vec::with_capacity(EXPECTED);
    let mut i = 0;
    while out.len() < EXPECTED {
        if i >= src.len() {
            return Err(format!(
                "MCAL RLE underrun: only {} of {} bytes produced",
                out.len(),
                EXPECTED
            ));
        }
        let header = src[i];
        i += 1;
        let fill = (header & 0x80) != 0;
        let count = (header & 0x7f) as usize;
        if fill {
            rle_fill(&mut out, src, &mut i, count, EXPECTED)?;
        } else {
            rle_copy(&mut out, src, &mut i, count, EXPECTED)?;
        }
    }
    Ok(out)
}

const MCLY_FLAG_ANIMATION_ENABLED: u32 = 0x40;
const MCLY_FLAG_OVERBRIGHT: u32 = 0x80;
const MCLY_FLAG_USE_ALPHA_MAP: u32 = 0x100;
const MCLY_FLAG_ALPHA_COMPRESSED: u32 = 0x200;
const MCLY_FLAG_USE_CUBE_MAP_REFLECTION: u32 = 0x400;

fn read_layer_alpha_map(
    flags: u32,
    offset_in_mcal: usize,
    mcal: &[u8],
    layer_idx: usize,
    do_not_fix_alpha_map: bool,
) -> Result<Option<Vec<u8>>, String> {
    if (flags & MCLY_FLAG_USE_ALPHA_MAP) == 0 {
        return Ok(None);
    }
    let raw = &mcal[offset_in_mcal..];
    let data = if (flags & MCLY_FLAG_ALPHA_COMPRESSED) != 0 {
        decompress_mcal_rle(raw)?
    } else if raw.len() >= 4096 {
        raw[..4096].to_vec()
    } else if raw.len() >= 2048 {
        let mut out = vec![0u8; 4096];
        for i in 0..2048 {
            let v = raw[i];
            out[i * 2] = (v & 0x0F) * 17;
            out[i * 2 + 1] = (v >> 4) * 17;
        }
        if !do_not_fix_alpha_map {
            fix_alpha_map_edges(&mut out);
        }
        out
    } else {
        return Err(format!(
            "MCAL uncompressed layer {layer_idx}: need ≥2048 bytes but only {} remain at offset {offset_in_mcal:#x}",
            raw.len()
        ));
    };
    Ok(Some(data))
}

fn fix_alpha_map_edges(alpha_map: &mut [u8]) {
    const ALPHA_MAP_SIZE: usize = 64;

    for row in 0..ALPHA_MAP_SIZE {
        let base = row * ALPHA_MAP_SIZE;
        alpha_map[base + 63] = alpha_map[base + 62];
    }
    for col in 0..ALPHA_MAP_SIZE {
        alpha_map[63 * ALPHA_MAP_SIZE + col] = alpha_map[62 * ALPHA_MAP_SIZE + col];
    }
}

fn build_texture_layers(
    mcly: &[u8],
    mcal: &[u8],
    mcmt: Option<[u8; 4]>,
    do_not_fix_alpha_map: bool,
) -> Result<Vec<TextureLayer>, String> {
    let layer_count = mcly.len() / size_of::<MclyEntry>();
    let mut layers = Vec::with_capacity(layer_count);
    for i in 0..layer_count {
        let base = i * size_of::<MclyEntry>();
        let entry: MclyEntry = parse_binrw_value(mcly, base, "MCLY entry")?;
        let alpha_map = read_layer_alpha_map(
            entry.flags,
            entry.offset_in_mcal as usize,
            mcal,
            i,
            do_not_fix_alpha_map,
        )?;
        layers.push(TextureLayer {
            texture_index: entry.texture_index,
            flags: MclyFlags { raw: entry.flags },
            effect_id: entry._effect_id,
            material_id: mcmt.and_then(|ids| ids.get(i).copied()).unwrap_or(0),
            alpha_map,
        });
    }
    Ok(layers)
}

fn parse_tex0_mcnk(payload: &[u8], do_not_fix_alpha_map: bool) -> Result<ChunkTexLayers, String> {
    let mut mcly_payload: Option<&[u8]> = None;
    let mut mcal_payload: Option<&[u8]> = None;
    let mut mcmt_payload: Option<[u8; 4]> = None;
    for chunk in ChunkIter::new(payload) {
        let (tag, data) = chunk?;
        match tag {
            b"YLCM" => mcly_payload = Some(data),
            b"LACM" => mcal_payload = Some(data),
            b"TMCM" => mcmt_payload = Some(parse_mcmt(data)?),
            _ => {}
        }
    }
    let mcly = mcly_payload.unwrap_or(&[]);
    let mcal = mcal_payload.unwrap_or(&[]);
    Ok(ChunkTexLayers {
        layers: build_texture_layers(mcly, mcal, mcmt_payload, do_not_fix_alpha_map)?,
    })
}

fn parse_mcmt(payload: &[u8]) -> Result<[u8; 4], String> {
    let values: [u8; 4] = payload
        .get(..4)
        .ok_or_else(|| "MCMT too small: need 4 bytes".to_string())?
        .try_into()
        .map_err(|_| "MCMT must be exactly 4 bytes".to_string())?;
    Ok(values)
}

fn parse_u32_chunk(payload: &[u8], label: &str) -> Result<Vec<u32>, String> {
    if !payload.len().is_multiple_of(size_of::<u32>()) {
        return Err(format!(
            "{label} size must be a multiple of {} bytes: {} bytes",
            size_of::<u32>(),
            payload.len()
        ));
    }

    let mut values = Vec::with_capacity(payload.len() / size_of::<u32>());
    for i in 0..(payload.len() / size_of::<u32>()) {
        values.push(read_u32(payload, i * size_of::<u32>())?);
    }
    Ok(values)
}

fn parse_texture_params(payload: &[u8]) -> Result<Vec<TextureParams>, String> {
    if !payload.len().is_multiple_of(size_of::<RawTextureParams>()) {
        return Err(format!(
            "MTXP size must be a multiple of {} bytes: {} bytes",
            size_of::<RawTextureParams>(),
            payload.len()
        ));
    }

    let mut params = Vec::with_capacity(payload.len() / size_of::<RawTextureParams>());
    for i in 0..(payload.len() / size_of::<RawTextureParams>()) {
        let entry: RawTextureParams =
            parse_binrw_value(payload, i * size_of::<RawTextureParams>(), "MTXP entry")?;
        params.push(TextureParams {
            flags: entry.flags,
            height_scale: entry.height_scale,
            height_offset: entry.height_offset,
        });
    }
    Ok(params)
}

pub fn load_adt_tex0(data: &[u8]) -> Result<AdtTexData, String> {
    load_adt_tex0_with_chunk_alpha_flags(data, &[])
}

pub fn load_adt_tex0_with_chunk_alpha_flags(
    data: &[u8],
    do_not_fix_alpha_map: &[bool],
) -> Result<AdtTexData, String> {
    let mut texture_amplifier: Option<u32> = None;
    let mut texture_fdids: Vec<u32> = Vec::new();
    let mut height_texture_fdids: Vec<u32> = Vec::new();
    let mut texture_flags: Vec<u32> = Vec::new();
    let mut texture_params: Vec<TextureParams> = Vec::new();
    let mut chunk_layers: Vec<ChunkTexLayers> = Vec::with_capacity(256);
    let mut chunk_index = 0usize;
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"PMAM" => texture_amplifier = Some(parse_mamp(payload)?),
            b"DIDM" => texture_fdids = parse_u32_chunk(payload, "MDID")?,
            b"DIHM" => height_texture_fdids = parse_u32_chunk(payload, "MHID")?,
            b"FXTM" => texture_flags = parse_u32_chunk(payload, "MTXF")?,
            b"PXTM" => texture_params = parse_texture_params(payload)?,
            b"KNCM" => {
                let chunk_do_not_fix_alpha = do_not_fix_alpha_map
                    .get(chunk_index)
                    .copied()
                    .unwrap_or(false);
                chunk_layers.push(parse_tex0_mcnk(payload, chunk_do_not_fix_alpha)?);
                chunk_index += 1;
            }
            _ => {}
        }
    }
    if chunk_layers.is_empty() {
        return Err("No KNCM chunks found in _tex0.adt file".to_string());
    }
    Ok(AdtTexData {
        texture_amplifier,
        texture_fdids,
        height_texture_fdids,
        texture_flags,
        texture_params,
        chunk_layers,
    })
}

fn parse_mamp(payload: &[u8]) -> Result<u32, String> {
    if payload.len() != size_of::<u32>() {
        return Err(format!(
            "MAMP size must be exactly {} bytes: {} bytes",
            size_of::<u32>(),
            payload.len()
        ));
    }
    read_u32(payload, 0)
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
    const HEADER_SIZE: usize = CHUNK_COUNT * 12;
    if payload.len() < HEADER_SIZE {
        return Err(format!(
            "MH2O chunk too small: {} bytes (need header {HEADER_SIZE})",
            payload.len()
        ));
    }

    let mut chunks = Vec::with_capacity(CHUNK_COUNT);
    for i in 0..CHUNK_COUNT {
        let base = i * size_of::<Mh2oChunkHeader>();
        let header: Mh2oChunkHeader = parse_binrw_value(payload, base, "MH2O chunk header")?;
        let attributes = if header.attributes_offset == 0 {
            None
        } else {
            Some(parse_water_attributes(
                payload,
                header.attributes_offset as usize,
            )?)
        };

        if header.instance_offset == 0 || header.layer_count == 0 {
            chunks.push(ChunkWater {
                layers: Vec::new(),
                attributes,
            });
            continue;
        }

        let mut layers = Vec::with_capacity(header.layer_count as usize);
        for layer_idx in 0..header.layer_count as usize {
            layers.push(parse_liquid_instance(
                payload,
                header.instance_offset as usize + layer_idx * size_of::<LiquidInstanceHeader>(),
            )?);
        }
        chunks.push(ChunkWater { layers, attributes });
    }
    Ok(AdtWaterData { chunks })
}

#[cfg(test)]
#[path = "adt_tex_tests.rs"]
mod tests;
