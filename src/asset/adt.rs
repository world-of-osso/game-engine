use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

pub use super::adt_tex::{
    AdtTexData, AdtWaterData, ChunkTexLayers, TextureLayer, build_water_mesh, load_adt_tex0,
    parse_mh2o,
};
use super::m2::wow_to_bevy;

pub(crate) const CHUNK_SIZE: f32 = 100.0 / 3.0; // 33.333... yards per MCNK side

type McnkGeometry = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>);
type AdtChunksResult<'a> = Result<(Vec<&'a [u8]>, Option<&'a [u8]>), String>;
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
    /// Bevy-space position at the terrain center (surface height, not bounding box center).
    pub center_surface: [f32; 3],
    /// Raw WoW [Y, X, Z] position from each MCNK (for water mesh positioning).
    pub chunk_positions: Vec<[f32; 3]>,
    /// Parsed MH2O water data (if present in the ADT).
    pub water: Option<AdtWaterData>,
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
        // MCNR bytes are stored as [X, Z, Y] (not [X, Y, Z])
        let nx = read_i8(payload, i * 3)? as f32 / 127.0; // X_wow
        let nz = read_i8(payload, i * 3 + 1)? as f32 / 127.0; // Z_wow
        let ny = read_i8(payload, i * 3 + 2)? as f32 / 127.0; // Y_wow
        // wow_to_bevy expects (X_wow, Y_wow, Z_wow)
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
    let (heights, normals) = parse_mcnk_subchunks(&payload[128..])?;
    Ok(McnkData {
        index_x,
        index_y,
        pos,
        heights,
        normals,
    })
}

/// Scan sub-chunks after the MCNK 128-byte header for TVCM and RNCM.
fn parse_mcnk_subchunks(sub: &[u8]) -> Result<([f32; MCVT_COUNT], [[f32; 3]; MCVT_COUNT]), String> {
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
    let normals = normals.unwrap_or([[0.0, 1.0, 0.0]; MCVT_COUNT]);
    Ok((heights, normals))
}

// ── vertex position computation ───────────────────────────────────────────────

/// Return the index of a vertex in the flat 145-element array.
pub(crate) fn vertex_index(grid_row: usize, col: usize) -> usize {
    let r_outer = grid_row / 2;
    if grid_row.is_multiple_of(2) {
        r_outer * 17 + col
    } else {
        r_outer * 17 + 9 + col
    }
}

/// Compute Bevy-space world position for one vertex in the 145-element grid.
fn vertex_position(
    grid_row: usize,
    col: usize,
    pos: [f32; 3],
    heights: &[f32; MCVT_COUNT],
) -> [f32; 3] {
    let idx = vertex_index(grid_row, col);
    let r = (grid_row / 2) as f32;
    let c = col as f32;
    let (wx, wy) = if grid_row.is_multiple_of(2) {
        (pos[1] - c * UNIT_SIZE, pos[0] - r * UNIT_SIZE)
    } else {
        (
            pos[1] - c * UNIT_SIZE - HALF_UNIT,
            pos[0] - r * UNIT_SIZE - HALF_UNIT,
        )
    };
    wow_to_bevy(wx, wy, pos[2] + heights[idx])
}

// ── mesh building ─────────────────────────────────────────────────────────────

fn build_mcnk_geometry(chunk: &McnkData) -> McnkGeometry {
    let mut positions = Vec::with_capacity(MCVT_COUNT);
    let mut normals_out = Vec::with_capacity(MCVT_COUNT);
    let mut uvs = Vec::with_capacity(MCVT_COUNT);
    for i in 0..MCVT_COUNT {
        let pair = i / 17;
        let rem = i % 17;
        let (grid_row, col) = if rem < 9 {
            (pair * 2, rem)
        } else {
            (pair * 2 + 1, rem - 9)
        };
        positions.push(vertex_position(grid_row, col, chunk.pos, &chunk.heights));
        normals_out.push(chunk.normals[i]);
        let uv = if grid_row.is_multiple_of(2) {
            [col as f32 / 8.0, (grid_row / 2) as f32 / 8.0]
        } else {
            [
                (col as f32 + 0.5) / 8.0,
                ((grid_row / 2) as f32 + 0.5) / 8.0,
            ]
        };
        uvs.push(uv);
    }
    let indices = build_mcnk_indices();
    (positions, normals_out, uvs, indices)
}

fn build_mcnk_indices() -> Vec<u32> {
    let mut indices = Vec::with_capacity(8 * 8 * 4 * 3);
    for qr in 0..8usize {
        for qc in 0..8usize {
            let tl = vertex_index(qr * 2, qc) as u32;
            let tr = vertex_index(qr * 2, qc + 1) as u32;
            let bl = vertex_index(qr * 2 + 2, qc) as u32;
            let br = vertex_index(qr * 2 + 2, qc + 1) as u32;
            let center = vertex_index(qr * 2 + 1, qc) as u32;
            indices.extend_from_slice(&[tl, tr, center]);
            indices.extend_from_slice(&[tr, br, center]);
            indices.extend_from_slice(&[br, bl, center]);
            indices.extend_from_slice(&[bl, tl, center]);
        }
    }
    indices
}

fn build_mcnk_mesh(chunk: &McnkData) -> Mesh {
    let (positions, normals, uvs, indices) = build_mcnk_geometry(chunk);
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

// ── top-level parser ──────────────────────────────────────────────────────────

fn build_height_grids(parsed: &[McnkData]) -> Vec<ChunkHeightGrid> {
    parsed
        .iter()
        .map(|d| ChunkHeightGrid {
            index_x: d.index_x,
            index_y: d.index_y,
            origin_x: d.pos[1],
            origin_z: -d.pos[0],
            base_y: d.pos[2],
            heights: d.heights,
        })
        .collect()
}

fn build_chunks(parsed: &[McnkData]) -> Vec<McnkMesh> {
    parsed
        .iter()
        .map(|d| McnkMesh {
            mesh: build_mcnk_mesh(d),
            index_x: d.index_x,
            index_y: d.index_y,
        })
        .collect()
}

fn center_surface_position(chunks: &[McnkData]) -> [f32; 3] {
    let center_chunk = chunks
        .iter()
        .find(|c| c.index_x == 8 && c.index_y == 8)
        .unwrap_or(&chunks[chunks.len() / 2]);
    vertex_position(9, 4, center_chunk.pos, &center_chunk.heights)
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

/// Parse an ADT file and return all 256 MCNK terrain meshes.
pub fn load_adt(data: &[u8]) -> Result<AdtData, String> {
    let (mcnk_payloads, mh2o_payload) = collect_adt_chunks(data)?;
    let parsed: Vec<McnkData> = mcnk_payloads
        .into_iter()
        .map(parse_mcnk)
        .collect::<Result<Vec<_>, String>>()?;
    let center_surface = center_surface_position(&parsed);
    let chunk_positions = parsed.iter().map(|d| d.pos).collect();
    let height_grids = build_height_grids(&parsed);
    let chunks = build_chunks(&parsed);
    let water = mh2o_payload.map(parse_mh2o).transpose()?;
    Ok(AdtData {
        chunks,
        height_grids,
        center_surface,
        chunk_positions,
        water,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcnr_normal_swizzle() {
        let mut payload = vec![0u8; MCVT_COUNT * 3];
        payload[0] = 0;
        payload[1] = 127;
        payload[2] = 0;
        let normals = parse_mcnr(&payload).unwrap();
        let n = normals[0];
        assert!((n[0]).abs() < 0.01, "X should be ~0, got {}", n[0]);
        assert!((n[1] - 1.0).abs() < 0.01, "Y should be ~1, got {}", n[1]);
        assert!((n[2]).abs() < 0.01, "Z should be ~0, got {}", n[2]);
    }

    #[test]
    fn mcnr_tilted_normal() {
        let mut payload = vec![0u8; MCVT_COUNT * 3];
        payload[0] = 90;
        payload[1] = 90;
        payload[2] = 0;
        let normals = parse_mcnr(&payload).unwrap();
        let n = normals[0];
        let expected_xz = 90.0 / 127.0;
        assert!(
            (n[0] - expected_xz).abs() < 0.01,
            "X should be ~{expected_xz}, got {}",
            n[0]
        );
        assert!(
            (n[1] - expected_xz).abs() < 0.01,
            "Y should be ~{expected_xz}, got {}",
            n[1]
        );
        assert!((n[2]).abs() < 0.01, "Z should be ~0, got {}", n[2]);
    }
}
