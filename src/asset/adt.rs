use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use super::m2::wow_to_bevy;

pub(crate) const CHUNK_SIZE: f32 = 100.0 / 3.0; // 33.333... yards per MCNK side
pub(crate) const UNIT_SIZE: f32 = CHUNK_SIZE / 8.0; // distance between outer vertices
const HALF_UNIT: f32 = UNIT_SIZE / 2.0;

/// Number of vertices in one MCNK height grid (9×9 outer + 8×8 inner).
const MCVT_COUNT: usize = 145;

/// Per-chunk height data for runtime terrain collision queries.
#[derive(Clone)]
pub struct ChunkHeightGrid {
    pub index_x: u32,
    pub index_y: u32,
    pub origin_x: f32, // Bevy X of chunk corner (= WoW pos[1])
    pub origin_z: f32, // Bevy Z of chunk corner (= -WoW pos[0])
    pub base_y: f32,   // WoW pos[2], heights are relative to this
    pub heights: [f32; 145],
}

#[allow(dead_code)]
pub struct McnkMesh {
    pub mesh: Mesh,
    pub index_x: u32,
    pub index_y: u32,
}

pub struct AdtData {
    pub chunks: Vec<McnkMesh>,
    pub height_grids: Vec<ChunkHeightGrid>,
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    /// Bevy-space position at the terrain center (surface height, not bounding box center).
    pub center_surface: [f32; 3],
    /// Raw WoW [Y, X, Z] position from each MCNK (for water mesh positioning).
    pub chunk_positions: Vec<[f32; 3]>,
    /// Parsed MH2O water data (if present in the ADT).
    pub water: Option<AdtWaterData>,
}

impl AdtData {
    /// Bounding box of the terrain in Bevy world-space.
    pub fn bounds(&self) -> (bevy::math::Vec3, bevy::math::Vec3) {
        (self.bounds_min.into(), self.bounds_max.into())
    }
}

// ── primitive readers ────────────────────────────────────────────────────────

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

fn read_i8(data: &[u8], off: usize) -> Result<i8, String> {
    data.get(off)
        .copied()
        .map(|b| b as i8)
        .ok_or_else(|| format!("read_i8 out of bounds at {off:#x}"))
}

// ── chunk iteration ──────────────────────────────────────────────────────────

/// Iterator over IFF-style chunks: reversed 4CC tag + u32 LE size + payload.
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

// ── MCVT / MCNR parsing ──────────────────────────────────────────────────────

/// Parse MCVT payload → 145 height floats.
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

/// Parse MCNR payload → 145 normals as unit [f32;3].
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
        let ny = read_i8(payload, i * 3 + 1)? as f32 / 127.0;
        let nz = read_i8(payload, i * 3 + 2)? as f32 / 127.0;
        // WoW normals: X-right, Y-forward, Z-up → wow_to_bevy
        *n = wow_to_bevy(nx, ny, nz);
    }
    Ok(normals)
}

// ── MCNK parsing ─────────────────────────────────────────────────────────────

struct McnkData {
    index_x: u32,
    index_y: u32,
    pos: [f32; 3], // WoW world-space corner position
    heights: [f32; MCVT_COUNT],
    normals: [[f32; 3]; MCVT_COUNT],
}

/// Parse a single MCNK chunk payload (128-byte header + sub-chunks).
fn parse_mcnk(payload: &[u8]) -> Result<McnkData, String> {
    if payload.len() < 128 {
        return Err(format!("MCNK payload too small: {} bytes", payload.len()));
    }

    let index_x = read_u32(payload, 0x04)?;
    let index_y = read_u32(payload, 0x08)?;
    let pos = [
        read_f32(payload, 0x68)?,
        read_f32(payload, 0x6c)?,
        read_f32(payload, 0x70)?,
    ];

    let sub = &payload[128..];
    let (heights, normals) = parse_mcnk_subchunks(sub)?;

    Ok(McnkData { index_x, index_y, pos, heights, normals })
}

/// Scan sub-chunks after the MCNK 128-byte header for TVCM and RNCM.
fn parse_mcnk_subchunks(
    sub: &[u8],
) -> Result<([f32; MCVT_COUNT], [[f32; 3]; MCVT_COUNT]), String> {
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
    // Normals are optional; fall back to up-vector if absent.
    let normals = normals.unwrap_or([[0.0, 1.0, 0.0]; MCVT_COUNT]);
    Ok((heights, normals))
}

// ── vertex position computation ───────────────────────────────────────────────

/// Return the index of a vertex in the flat 145-element array.
///
/// The grid has alternating outer (9-wide) and inner (8-wide) rows:
/// row 0: outer (9 verts), row 1: inner (8 verts), row 2: outer, …
/// Grid row `r` maps to array row `r/2` outer or `r/2` inner.
pub(crate) fn vertex_index(grid_row: usize, col: usize) -> usize {
    // Array is laid out as: 9 outer, 8 inner, 9 outer, 8 inner, ..., 9 outer
    // For outer row r_outer (0..=8): base = r_outer * 17, index = base + col
    // For inner row r_inner (0..=7): base = r_inner * 17 + 9, index = base + col
    let r_outer = grid_row / 2;
    if grid_row.is_multiple_of(2) {
        r_outer * 17 + col
    } else {
        r_outer * 17 + 9 + col
    }
}

/// Compute Bevy-space world position for one vertex in the 145-element grid.
///
/// MCNK position is stored as `[Y_wow, X_wow, Z_wow]`.
/// Cols step in -X (from pos[1]), rows step in -Y (from pos[0]).
fn vertex_position(grid_row: usize, col: usize, pos: [f32; 3], heights: &[f32; MCVT_COUNT]) -> [f32; 3] {
    let idx = vertex_index(grid_row, col);
    let r = (grid_row / 2) as f32;
    let c = col as f32;

    let (wx, wy) = if grid_row.is_multiple_of(2) {
        (pos[1] - c * UNIT_SIZE, pos[0] - r * UNIT_SIZE)
    } else {
        (pos[1] - c * UNIT_SIZE - HALF_UNIT, pos[0] - r * UNIT_SIZE - HALF_UNIT)
    };
    wow_to_bevy(wx, wy, pos[2] + heights[idx])
}

// ── mesh building ─────────────────────────────────────────────────────────────

/// Build one Bevy mesh from a parsed MCNK.
fn build_mcnk_mesh(chunk: &McnkData) -> Mesh {
    let (positions, normals, uvs, indices) = build_mcnk_geometry(chunk);

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Compute positions, normals, UVs, and triangle indices for the 145-vertex diamond grid.
fn build_mcnk_geometry(
    chunk: &McnkData,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
    // Flatten the 145 vertices into sequential arrays (indexed by vertex_index).
    let mut positions = Vec::with_capacity(MCVT_COUNT);
    let mut normals_out = Vec::with_capacity(MCVT_COUNT);
    let mut uvs = Vec::with_capacity(MCVT_COUNT);
    for i in 0..MCVT_COUNT {
        // Reconstruct grid_row / col from flat index i.
        // Layout: [9, 8, 9, 8, ..., 9] with 17 per outer+inner pair.
        let pair = i / 17;
        let rem = i % 17;
        let (grid_row, col) = if rem < 9 {
            (pair * 2, rem)
        } else {
            (pair * 2 + 1, rem - 9)
        };
        positions.push(vertex_position(grid_row, col, chunk.pos, &chunk.heights));
        normals_out.push(chunk.normals[i]);
        // UV coordinates: [0.0, 1.0] across the chunk
        let uv = if grid_row.is_multiple_of(2) {
            // outer vertex
            [col as f32 / 8.0, (grid_row / 2) as f32 / 8.0]
        } else {
            // inner vertex
            [(col as f32 + 0.5) / 8.0, ((grid_row / 2) as f32 + 0.5) / 8.0]
        };
        uvs.push(uv);
    }

    let indices = build_mcnk_indices();
    (positions, normals_out, uvs, indices)
}

/// Build the triangle index list for the 8×8 quad grid (256 quads × 4 triangles = 1024 tris).
fn build_mcnk_indices() -> Vec<u32> {
    let mut indices = Vec::with_capacity(8 * 8 * 4 * 3);
    for qr in 0..8usize {
        for qc in 0..8usize {
            // Outer corners of this quad (even grid rows).
            let tl = vertex_index(qr * 2, qc) as u32;
            let tr = vertex_index(qr * 2, qc + 1) as u32;
            let bl = vertex_index(qr * 2 + 2, qc) as u32;
            let br = vertex_index(qr * 2 + 2, qc + 1) as u32;
            // Center inner vertex (odd grid row).
            let center = vertex_index(qr * 2 + 1, qc) as u32;

            // 4 triangles fanning from center:
            // Top
            indices.extend_from_slice(&[tl, tr, center]);
            // Right
            indices.extend_from_slice(&[tr, br, center]);
            // Bottom
            indices.extend_from_slice(&[br, bl, center]);
            // Left
            indices.extend_from_slice(&[bl, tl, center]);
        }
    }
    indices
}

// ── top-level parser ──────────────────────────────────────────────────────────

/// Parse an ADT file and return all 256 MCNK terrain meshes.
pub fn load_adt(data: &[u8]) -> Result<AdtData, String> {
    let (mcnk_payloads, mh2o_payload) = collect_adt_chunks(data)?;
    let parsed: Vec<McnkData> = mcnk_payloads
        .into_iter()
        .map(parse_mcnk)
        .collect::<Result<Vec<_>, String>>()?;

    let (bounds_min, bounds_max) = compute_bounds(&parsed);
    let center_surface = center_surface_position(&parsed);
    let chunk_positions = parsed.iter().map(|d| d.pos).collect();
    let height_grids = parsed
        .iter()
        .map(|d| ChunkHeightGrid {
            index_x: d.index_x,
            index_y: d.index_y,
            origin_x: d.pos[1],
            origin_z: -d.pos[0],
            base_y: d.pos[2],
            heights: d.heights,
        })
        .collect();
    let chunks = parsed
        .iter()
        .map(|d| McnkMesh { mesh: build_mcnk_mesh(d), index_x: d.index_x, index_y: d.index_y })
        .collect();
    let water = mh2o_payload.map(|p| parse_mh2o(p)).transpose()?;

    Ok(AdtData { chunks, height_grids, bounds_min, bounds_max, center_surface, chunk_positions, water })
}

/// Compute Bevy-space bounding box from MCNK corner positions + height extremes.
fn compute_bounds(chunks: &[McnkData]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for c in chunks {
        let h_min = c.heights.iter().copied().fold(f32::MAX, f32::min);
        let h_max = c.heights.iter().copied().fold(f32::MIN, f32::max);
        // pos = [Y, X, Z]; cols step in -X (pos[1]), rows step in -Y (pos[0])
        for &(wx, wy, wz) in &[
            (c.pos[1], c.pos[0], c.pos[2] + h_min),
            (c.pos[1] - CHUNK_SIZE, c.pos[0] - CHUNK_SIZE, c.pos[2] + h_max),
        ] {
            let [bx, by, bz] = wow_to_bevy(wx, wy, wz);
            min[0] = min[0].min(bx);
            min[1] = min[1].min(by);
            min[2] = min[2].min(bz);
            max[0] = max[0].max(bx);
            max[1] = max[1].max(by);
            max[2] = max[2].max(bz);
        }
    }
    (min, max)
}

/// Compute Bevy-space position at the terrain center (actual surface height).
/// Uses the center vertex of the center MCNK chunk (8,8 in the 16×16 grid).
fn center_surface_position(chunks: &[McnkData]) -> [f32; 3] {
    let center_chunk = chunks
        .iter()
        .find(|c| c.index_x == 8 && c.index_y == 8)
        .unwrap_or(&chunks[chunks.len() / 2]);

    // Center vertex: inner row 4, col 4 → flat index 4*17 + 9 + 4 = 81
    vertex_position(9, 4, center_chunk.pos, &center_chunk.heights)
}

/// Collect KNCM (MCNK) payloads and optionally the O2HM (MH2O) payload.
fn collect_adt_chunks(data: &[u8]) -> Result<(Vec<&[u8]>, Option<&[u8]>), String> {
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

// ── _tex0.adt types ───────────────────────────────────────────────────────────

/// One texture layer within a MCNK chunk from the _tex0.adt file.
pub struct TextureLayer {
    pub texture_index: u32,
    pub flags: u32,
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

// ── MCAL RLE decompression ────────────────────────────────────────────────────

/// Apply one RLE fill run: repeat `value` up to `count` times into `out`.
fn rle_fill(out: &mut Vec<u8>, src: &[u8], i: &mut usize, count: usize, limit: usize) -> Result<(), String> {
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

/// Apply one RLE copy run: copy next `count` bytes from `src` into `out`.
fn rle_copy(out: &mut Vec<u8>, src: &[u8], i: &mut usize, count: usize, limit: usize) -> Result<(), String> {
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

/// Decompress MCAL RLE data into exactly 4096 bytes.
///
/// Each byte header: bit 7 = mode, bits 0–6 = count.
///   mode 0 (copy):  read next `count` bytes literally.
///   mode 1 (fill):  read next 1 byte, repeat it `count` times.
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

/// Read the alpha map for one MCLY layer from the MCAL blob.
///
/// Returns `None` for the base layer (no `use_alpha_map` flag).
fn read_layer_alpha_map(flags: u32, offset_in_mcal: usize, mcal: &[u8], layer_idx: usize) -> Result<Option<Vec<u8>>, String> {
    if (flags & MCLY_FLAG_USE_ALPHA_MAP) == 0 {
        return Ok(None);
    }
    let raw = &mcal[offset_in_mcal..];
    let data = if (flags & MCLY_FLAG_ALPHA_COMPRESSED) != 0 {
        decompress_mcal_rle(raw)?
    } else {
        if raw.len() < 4096 {
            return Err(format!(
                "MCAL uncompressed layer {layer_idx}: need 4096 bytes but only {} remain at offset {offset_in_mcal:#x}",
                raw.len()
            ));
        }
        raw[..4096].to_vec()
    };
    Ok(Some(data))
}

/// Build texture layers from MCLY and MCAL payloads.
fn build_texture_layers(mcly: &[u8], mcal: &[u8]) -> Result<Vec<TextureLayer>, String> {
    let layer_count = mcly.len() / 16;
    let mut layers = Vec::with_capacity(layer_count);
    for i in 0..layer_count {
        let base = i * 16;
        let texture_index = read_u32(mcly, base)?;
        let flags = read_u32(mcly, base + 0x04)?;
        let offset_in_mcal = read_u32(mcly, base + 0x08)? as usize;
        let alpha_map = read_layer_alpha_map(flags, offset_in_mcal, mcal, i)?;
        layers.push(TextureLayer { texture_index, flags, alpha_map });
    }
    Ok(layers)
}

/// Parse YLCM (MCLY) + LACM (MCAL) subchunks from a _tex0 MCNK payload.
///
/// Unlike root .adt MCNK payloads, _tex0 MCNK payloads have NO 128-byte header —
/// subchunks begin immediately.
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
    let layers = build_texture_layers(mcly, mcal)?;
    Ok(ChunkTexLayers { layers })
}

// ── _tex0.adt top-level parser ────────────────────────────────────────────────

/// Parse a `_tex0.adt` split file and return texture FDIDs and per-chunk layer data.
pub fn load_adt_tex0(data: &[u8]) -> Result<AdtTexData, String> {
    let mut texture_fdids: Vec<u32> = Vec::new();
    let mut chunk_layers: Vec<ChunkTexLayers> = Vec::with_capacity(256);

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"DIDM" => {
                // MDID: array of u32 FileDataIDs for diffuse textures.
                let count = payload.len() / 4;
                texture_fdids.reserve(count);
                for i in 0..count {
                    texture_fdids.push(read_u32(payload, i * 4)?);
                }
            }
            b"KNCM" => {
                // MCNK: no 128-byte header in _tex0, subchunks start immediately.
                chunk_layers.push(parse_tex0_mcnk(payload)?);
            }
            _ => {}
        }
    }

    if chunk_layers.is_empty() {
        return Err("No KNCM chunks found in _tex0.adt file".to_string());
    }

    Ok(AdtTexData { texture_fdids, chunk_layers })
}

// ── MH2O water types ────────────────────────────────────────────────────────

pub struct WaterLayer {
    pub liquid_type: u16,
    pub liquid_object: u16,
    pub min_height: f32,
    pub max_height: f32,
    pub x_offset: u8,
    pub y_offset: u8,
    pub width: u8,
    pub height: u8,
    /// Bitmask per row; MSB = col 0.
    pub exists: [u8; 8],
    /// `(width+1)*(height+1)` floats, or empty if flat (use `min_height`).
    pub vertex_heights: Vec<f32>,
}

pub struct ChunkWater {
    pub layers: Vec<WaterLayer>,
}

pub struct AdtWaterData {
    /// 256 entries, one per MCNK chunk.
    pub chunks: Vec<ChunkWater>,
}

// ── MH2O parsing ────────────────────────────────────────────────────────────

/// Parse the MH2O chunk payload into per-chunk water data.
///
/// Header: 256 × 12-byte entries (`offset_info`, `layer_count`, `offset_render`).
/// For entries with layers, `SLiquidInstance` (24 bytes) lives at `offset_info`.
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
            let layer = parse_liquid_instance(payload, offset_info + li * 24)?;
            layers.push(layer);
        }
        chunks.push(ChunkWater { layers });
    }
    Ok(AdtWaterData { chunks })
}

/// Parse one `SLiquidInstance` (24 bytes) at the given offset within the MH2O payload.
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

/// Read the exists bitmask: `height` bytes, each with `width` bits (MSB = col 0).
fn read_exists_bitmask(
    payload: &[u8],
    offset: usize,
    width: u8,
    height: u8,
) -> Result<[u8; 8], String> {
    let mut exists = [0u8; 8];
    if offset == 0 {
        // No bitmask pointer → all quads exist.
        for row in 0..height as usize {
            // Set `width` bits from MSB.
            exists[row] = 0xFF << (8 - width);
        }
        return Ok(exists);
    }
    let h = height as usize;
    if offset + h > payload.len() {
        return Err(format!(
            "MH2O exists bitmask out of bounds: offset {offset:#x}, need {h} bytes"
        ));
    }
    for row in 0..h {
        exists[row] = payload[offset + row];
    }
    Ok(exists)
}

/// Read vertex height data: `(width+1)*(height+1)` f32 values, or empty if flat.
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

// ── water mesh building ─────────────────────────────────────────────────────

const WATER_STEP: f32 = CHUNK_SIZE / 8.0;

/// Build a Bevy mesh for one water layer within a single MCNK chunk.
///
/// `chunk_pos` is the raw WoW `[Y, X, Z]` from the MCNK header (offset 0x68).
pub fn build_water_mesh(chunk_pos: [f32; 3], layer: &WaterLayer) -> Mesh {
    let (positions, normals, uvs, indices) = build_water_geometry(chunk_pos, layer);

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Compute positions, normals, UVs, and indices for a water layer's quad grid.
fn build_water_geometry(
    chunk_pos: [f32; 3],
    layer: &WaterLayer,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
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
            // Two triangles: TL=0, TR=1, BL=2, BR=3
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

/// Check if a quad at (row, col) within the water layer exists.
fn quad_exists(layer: &WaterLayer, row: usize, col: usize) -> bool {
    if row >= 8 || col >= 8 {
        return false;
    }
    let byte = layer.exists[row];
    // MSB = col 0: bit (7 - col)
    (byte >> (7 - col)) & 1 != 0
}

/// Emit the 4 vertices for one water quad at local (row, col).
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

    // Four corners: TL, TR, BL, BR
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

/// Get the water height at a vertex within the layer grid.
fn water_height(layer: &WaterLayer, vert_row: usize, vert_col: usize) -> f32 {
    if layer.vertex_heights.is_empty() {
        return layer.min_height;
    }
    let w = layer.width as usize + 1;
    let idx = vert_row * w + vert_col;
    layer.vertex_heights.get(idx).copied().unwrap_or(layer.min_height)
}
