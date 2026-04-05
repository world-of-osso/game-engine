//! ADT _tex0.adt and MH2O water parsing.

use std::io::Cursor;
use std::mem::size_of;

use crate::asset::read_bytes::read_u32;
use binrw::BinRead;

use super::adt::ChunkIter;
#[path = "adt_tex_water.rs"]
mod adt_tex_water;
pub use adt_tex_water::{AdtWaterData, ChunkWater, WaterAttributes, WaterLayer, parse_mh2o};
#[cfg(test)]
use adt_tex_water::{LiquidInstanceHeader, Mh2oAttributes, Mh2oChunkHeader};

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

struct AdtTexAccumulator {
    texture_amplifier: Option<u32>,
    texture_fdids: Vec<u32>,
    height_texture_fdids: Vec<u32>,
    texture_flags: Vec<u32>,
    texture_params: Vec<TextureParams>,
    chunk_layers: Vec<ChunkTexLayers>,
    chunk_index: usize,
}

impl AdtTexAccumulator {
    fn new() -> Self {
        Self {
            texture_amplifier: None,
            texture_fdids: Vec::new(),
            height_texture_fdids: Vec::new(),
            texture_flags: Vec::new(),
            texture_params: Vec::new(),
            chunk_layers: Vec::with_capacity(256),
            chunk_index: 0,
        }
    }

    fn finish(self) -> Result<AdtTexData, String> {
        if self.chunk_layers.is_empty() {
            return Err("No KNCM chunks found in _tex0.adt file".to_string());
        }

        Ok(AdtTexData {
            texture_amplifier: self.texture_amplifier,
            texture_fdids: self.texture_fdids,
            height_texture_fdids: self.height_texture_fdids,
            texture_flags: self.texture_flags,
            texture_params: self.texture_params,
            chunk_layers: self.chunk_layers,
        })
    }
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
    let mut accum = AdtTexAccumulator::new();
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        parse_tex0_chunk(&mut accum, tag, payload, do_not_fix_alpha_map)?;
    }
    accum.finish()
}

fn parse_tex0_chunk(
    accum: &mut AdtTexAccumulator,
    tag: &[u8; 4],
    payload: &[u8],
    do_not_fix_alpha_map: &[bool],
) -> Result<(), String> {
    match tag {
        b"PMAM" => accum.texture_amplifier = Some(parse_mamp(payload)?),
        b"DIDM" => accum.texture_fdids = parse_u32_chunk(payload, "MDID")?,
        b"DIHM" => accum.height_texture_fdids = parse_u32_chunk(payload, "MHID")?,
        b"FXTM" => accum.texture_flags = parse_u32_chunk(payload, "MTXF")?,
        b"PXTM" => accum.texture_params = parse_texture_params(payload)?,
        b"KNCM" => parse_tex0_mcnk_chunk(accum, payload, do_not_fix_alpha_map)?,
        _ => {}
    }
    Ok(())
}

fn parse_tex0_mcnk_chunk(
    accum: &mut AdtTexAccumulator,
    payload: &[u8],
    do_not_fix_alpha_map: &[bool],
) -> Result<(), String> {
    let chunk_do_not_fix_alpha = do_not_fix_alpha_map
        .get(accum.chunk_index)
        .copied()
        .unwrap_or(false);
    accum
        .chunk_layers
        .push(parse_tex0_mcnk(payload, chunk_do_not_fix_alpha)?);
    accum.chunk_index += 1;
    Ok(())
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

#[cfg(test)]
#[path = "adt_tex_tests.rs"]
mod tests;
