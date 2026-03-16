//! ADT _tex0.adt and MH2O water parsing.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use super::adt::{CHUNK_SIZE, ChunkIter};
use super::m2::wow_to_bevy;

// ── _tex0.adt types ───────────────────────────────────────────────────────────

/// One texture layer within a MCNK chunk from the _tex0.adt file.
pub struct TextureLayer {
    pub texture_index: u32,
    pub _flags: u32,
    /// 4096 bytes (64×64 8-bit alpha values). None for base layer (layer 0,
    /// which has no use_alpha_map flag set).
    pub alpha_map: Option<Vec<u8>>,
}

/// Texture layers for one MCNK chunk from the _tex0.adt file.
pub struct ChunkTexLayers {
    pub layers: Vec<TextureLayer>,
}

/// Parsed data from a _tex0.adt split file.
pub struct AdtTexData {
    /// FileDataIDs for diffuse/specular textures (from DIDM / MDID chunk).
    pub texture_fdids: Vec<u32>,
    /// Per-chunk texture layer data; one entry per MCNK (256 total).
    pub chunk_layers: Vec<ChunkTexLayers>,
}

// ── primitive readers ─────────────────────────────────────────────────────────

fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_u32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

fn read_u16(data: &[u8], off: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_u16 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

fn read_f32(data: &[u8], off: usize) -> Result<f32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_f32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

// ── MCAL RLE decompression ────────────────────────────────────────────────────

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

// ── _tex0.adt MCNK subchunk parsing ──────────────────────────────────────────

const MCLY_FLAG_USE_ALPHA_MAP: u32 = 0x100;
const MCLY_FLAG_ALPHA_COMPRESSED: u32 = 0x200;

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
        // 4-bit packed alpha: each byte contains two 4-bit values, scaled to 0–255.
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
    let layer_count = mcly.len() / 16;
    let mut layers = Vec::with_capacity(layer_count);
    for i in 0..layer_count {
        let base = i * 16;
        let texture_index = read_u32(mcly, base)?;
        let flags = read_u32(mcly, base + 0x04)?;
        let offset_in_mcal = read_u32(mcly, base + 0x08)? as usize;
        let alpha_map = read_layer_alpha_map(flags, offset_in_mcal, mcal, i)?;
        layers.push(TextureLayer {
            texture_index,
            _flags: flags,
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

/// Parse a `_tex0.adt` split file and return texture FDIDs and per-chunk layer data.
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

// ── MH2O water types ────────────────────────────────────────────────────────

#[allow(dead_code)]
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

// ── MH2O parsing ────────────────────────────────────────────────────────────

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
    if off + 24 > payload.len() {
        return Err(format!(
            "SLiquidInstance out of bounds at {off:#x} (payload len {:#x})",
            payload.len()
        ));
    }
    let liquid_type = read_u16(payload, off)?;
    let liquid_object = read_u16(payload, off + 2)?;
    let min_height = read_f32(payload, off + 4)?;
    let max_height = read_f32(payload, off + 8)?;
    let x_offset = payload[off + 12];
    let y_offset = payload[off + 13];
    let width = payload[off + 14];
    let height = payload[off + 15];
    let offset_exists = read_u32(payload, off + 16)? as usize;
    let offset_vertex = read_u32(payload, off + 20)? as usize;
    let exists = read_exists_bitmask(payload, offset_exists, width, height)?;
    let vertex_heights = read_vertex_heights(payload, offset_vertex, width, height)?;
    Ok(WaterLayer {
        liquid_type,
        liquid_object,
        min_height,
        max_height,
        x_offset,
        y_offset,
        width,
        height,
        exists,
        vertex_heights,
    })
}

pub fn parse_mh2o(payload: &[u8]) -> Result<AdtWaterData, String> {
    const HEADER_SIZE: usize = 256 * 12;
    if payload.len() < HEADER_SIZE {
        return Err(format!(
            "MH2O payload too small: {} bytes (need {HEADER_SIZE})",
            payload.len()
        ));
    }
    let mut chunks = Vec::with_capacity(256);
    for i in 0..256 {
        let entry_off = i * 12;
        let offset_info = read_u32(payload, entry_off)? as usize;
        let layer_count = read_u32(payload, entry_off + 4)?;
        if layer_count == 0 {
            chunks.push(ChunkWater { layers: Vec::new() });
            continue;
        }
        let mut layers = Vec::with_capacity(layer_count as usize);
        for li in 0..layer_count as usize {
            layers.push(parse_liquid_instance(payload, offset_info + li * 24)?);
        }
        chunks.push(ChunkWater { layers });
    }
    Ok(AdtWaterData { chunks })
}

// ── water mesh building ─────────────────────────────────────────────────────

const WATER_STEP: f32 = CHUNK_SIZE / 8.0;

fn quad_exists(layer: &WaterLayer, row: usize, col: usize) -> bool {
    if row >= 8 || col >= 8 {
        return false;
    }
    (layer.exists[row] >> col) & 1 != 0
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

type WaterGeometry = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>);

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
