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
    pub alpha_map: Option<Vec<u8>>,
}

pub struct ChunkTexLayers {
    pub layers: Vec<TextureLayer>,
}

pub struct AdtTexData {
    pub texture_fdids: Vec<u32>,
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
struct Mh2oChunkHeader {
    instance_offset: u32,
    layer_count: u32,
    _attributes_offset: u32,
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
        out
    } else {
        return Err(format!(
            "MCAL uncompressed layer {layer_idx}: need ≥2048 bytes but only {} remain at offset {offset_in_mcal:#x}",
            raw.len()
        ));
    };
    Ok(Some(data))
}

fn build_texture_layers(mcly: &[u8], mcal: &[u8]) -> Result<Vec<TextureLayer>, String> {
    let layer_count = mcly.len() / size_of::<MclyEntry>();
    let mut layers = Vec::with_capacity(layer_count);
    for i in 0..layer_count {
        let base = i * size_of::<MclyEntry>();
        let entry: MclyEntry = parse_binrw_value(mcly, base, "MCLY entry")?;
        let alpha_map = read_layer_alpha_map(entry.flags, entry.offset_in_mcal as usize, mcal, i)?;
        layers.push(TextureLayer {
            texture_index: entry.texture_index,
            flags: MclyFlags { raw: entry.flags },
            effect_id: entry._effect_id,
            alpha_map,
        });
    }
    Ok(layers)
}

fn parse_tex0_mcnk(payload: &[u8]) -> Result<ChunkTexLayers, String> {
    let mut mcly_payload: Option<&[u8]> = None;
    let mut mcal_payload: Option<&[u8]> = None;
    for chunk in ChunkIter::new(payload) {
        let (tag, data) = chunk?;
        match tag {
            b"YLCM" => mcly_payload = Some(data),
            b"LACM" => mcal_payload = Some(data),
            _ => {}
        }
    }
    let mcly = mcly_payload.unwrap_or(&[]);
    let mcal = mcal_payload.unwrap_or(&[]);
    Ok(ChunkTexLayers {
        layers: build_texture_layers(mcly, mcal)?,
    })
}

pub fn load_adt_tex0(data: &[u8]) -> Result<AdtTexData, String> {
    let mut texture_fdids: Vec<u32> = Vec::new();
    let mut chunk_layers: Vec<ChunkTexLayers> = Vec::with_capacity(256);
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"DIDM" => {
                let count = payload.len() / 4;
                texture_fdids.reserve(count);
                for i in 0..count {
                    texture_fdids.push(read_u32(payload, i * 4)?);
                }
            }
            b"KNCM" => chunk_layers.push(parse_tex0_mcnk(payload)?),
            _ => {}
        }
    }
    if chunk_layers.is_empty() {
        return Err("No KNCM chunks found in _tex0.adt file".to_string());
    }
    Ok(AdtTexData {
        texture_fdids,
        chunk_layers,
    })
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
}

pub struct ChunkWater {
    pub layers: Vec<WaterLayer>,
}

pub struct AdtWaterData {
    pub chunks: Vec<ChunkWater>,
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

fn parse_liquid_instance(payload: &[u8], off: usize) -> Result<WaterLayer, String> {
    if off + size_of::<LiquidInstanceHeader>() > payload.len() {
        return Err(format!(
            "SLiquidInstance out of bounds at {off:#x} (payload len {:#x})",
            payload.len()
        ));
    }
    let header: LiquidInstanceHeader = parse_binrw_value(payload, off, "SLiquidInstance")?;
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
        vertex_heights: read_vertex_heights(
            payload,
            header.vertex_offset as usize,
            header.width,
            header.height,
        )?,
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

        if header.instance_offset == 0 || header.layer_count == 0 {
            chunks.push(ChunkWater { layers: Vec::new() });
            continue;
        }

        let mut layers = Vec::with_capacity(header.layer_count as usize);
        for layer_idx in 0..header.layer_count as usize {
            layers.push(parse_liquid_instance(
                payload,
                header.instance_offset as usize + layer_idx * size_of::<LiquidInstanceHeader>(),
            )?);
        }
        chunks.push(ChunkWater { layers });
    }
    Ok(AdtWaterData { chunks })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ROTATION_BITS: u32 = 3;
    const TEST_SPEED_BITS: u32 = 5 << 3;
    const TEST_SECOND_SPEED_BITS: u32 = 2 << 3;
    const TEST_SECOND_ROTATION_BITS: u32 = 1;
    const TEST_ANIMATED_REFLECTIVE_FLAGS: u32 = MCLY_FLAG_USE_CUBE_MAP_REFLECTION
        | MCLY_FLAG_ALPHA_COMPRESSED
        | MCLY_FLAG_USE_ALPHA_MAP
        | MCLY_FLAG_OVERBRIGHT
        | MCLY_FLAG_ANIMATION_ENABLED
        | TEST_SPEED_BITS
        | TEST_ROTATION_BITS;
    const TEST_OVERBRIGHT_REFLECTIVE_FLAGS: u32 =
        MCLY_FLAG_USE_CUBE_MAP_REFLECTION | MCLY_FLAG_OVERBRIGHT | MCLY_FLAG_ANIMATION_ENABLED;

    #[test]
    fn mcly_flags_decode_animation_and_reflection_bits() {
        let flags = MclyFlags {
            raw: TEST_ANIMATED_REFLECTIVE_FLAGS,
        };

        assert_eq!(flags.animation_rotation(), 3);
        assert_eq!(flags.animation_speed(), 5);
        assert!(flags.animation_enabled());
        assert!(flags.overbright());
        assert!(flags.use_alpha_map());
        assert!(flags.alpha_compressed());
        assert!(flags.use_cube_map_reflection());
    }

    #[test]
    fn build_texture_layers_exposes_mcly_flags_and_effect_id() {
        let mcly = mcly_entry_payload(
            7,
            TEST_OVERBRIGHT_REFLECTIVE_FLAGS | TEST_SECOND_SPEED_BITS | TEST_SECOND_ROTATION_BITS,
            0,
            99,
        );

        let layers = build_texture_layers(&mcly, &[]).expect("expected MCLY layer to parse");
        let layer = &layers[0];

        assert_eq!(layer.texture_index, 7);
        assert_eq!(layer.effect_id, 99);
        assert_eq!(layer.flags.animation_rotation(), 1);
        assert_eq!(layer.flags.animation_speed(), 2);
        assert!(layer.flags.animation_enabled());
        assert!(layer.flags.overbright());
        assert!(layer.flags.use_cube_map_reflection());
        assert_eq!(layer.alpha_map, None);
    }

    #[test]
    fn load_adt_tex0_preserves_parsed_mcly_flags_per_chunk() {
        let mut payload = Vec::new();
        append_subchunk(&mut payload, b"DIDM", 3u32.to_le_bytes().to_vec());
        append_subchunk(
            &mut payload,
            b"KNCM",
            tex0_mcnk_payload(
                mcly_entry_payload(0, MCLY_FLAG_USE_ALPHA_MAP, 0, 11),
                vec![0x7F; 4096],
            ),
        );

        let parsed = load_adt_tex0(&payload).expect("expected _tex0 payload to parse");
        let layer = &parsed.chunk_layers[0].layers[0];

        assert_eq!(parsed.texture_fdids, vec![3]);
        assert!(layer.flags.use_alpha_map());
        assert_eq!(layer.effect_id, 11);
        assert_eq!(layer.alpha_map.as_ref().map(Vec::len), Some(4096));
    }

    fn mcly_entry_payload(
        texture_index: u32,
        flags: u32,
        offset_in_mcal: u32,
        effect_id: u32,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&texture_index.to_le_bytes());
        payload.extend_from_slice(&flags.to_le_bytes());
        payload.extend_from_slice(&offset_in_mcal.to_le_bytes());
        payload.extend_from_slice(&effect_id.to_le_bytes());
        payload
    }

    fn tex0_mcnk_payload(mcly: Vec<u8>, mcal: Vec<u8>) -> Vec<u8> {
        let mut payload = Vec::new();
        append_subchunk(&mut payload, b"YLCM", mcly);
        append_subchunk(&mut payload, b"LACM", mcal);
        payload
    }

    fn append_subchunk(payload: &mut Vec<u8>, tag: &[u8; 4], chunk_payload: Vec<u8>) {
        payload.extend_from_slice(tag);
        payload.extend_from_slice(&(chunk_payload.len() as u32).to_le_bytes());
        payload.extend_from_slice(&chunk_payload);
    }
}
