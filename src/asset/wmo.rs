use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use super::adt::ChunkIter;
use super::m2::wow_to_bevy;

// ── primitive readers ────────────────────────────────────────────────────────

fn read_u8(data: &[u8], off: usize) -> Result<u8, String> {
    data.get(off)
        .copied()
        .ok_or_else(|| format!("read_u8 out of bounds at {off:#x}"))
}

fn read_u16(data: &[u8], off: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_u16 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

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

// ── types ───────────────────────────────────────────────────────────────────

pub struct WmoRootData {
    pub n_groups: u32,
    pub materials: Vec<WmoMaterialDef>,
}

pub struct WmoMaterialDef {
    pub texture_fdid: u32,
    pub flags: u32,
    pub blend_mode: u32,
}

pub struct WmoGroupData {
    pub batches: Vec<WmoGroupBatch>,
}

pub struct WmoGroupBatch {
    pub mesh: Mesh,
    pub material_index: u16,
}

// ── root file parsing ───────────────────────────────────────────────────────

/// Parse a WMO root file and return header info + materials.
pub fn load_wmo_root(data: &[u8]) -> Result<WmoRootData, String> {
    let mut n_groups = 0u32;
    let mut materials = Vec::new();

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"DHOM" => {
                // MOHD: 64-byte header
                if payload.len() < 64 {
                    return Err(format!("MOHD too small: {} bytes", payload.len()));
                }
                n_groups = read_u32(payload, 4)?;
            }
            b"TMOM" => {
                // MOMT: 64 bytes per material
                materials = parse_momt(payload)?;
            }
            _ => {}
        }
    }

    Ok(WmoRootData { n_groups, materials })
}

/// Parse MOMT chunk: 64 bytes per material entry.
fn parse_momt(data: &[u8]) -> Result<Vec<WmoMaterialDef>, String> {
    let count = data.len() / 64;
    let mut mats = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 64;
        let flags = read_u32(data, base)?;
        let blend_mode = read_u32(data, base + 8)?;
        let texture_fdid = read_u32(data, base + 0x0C)?;
        mats.push(WmoMaterialDef { texture_fdid, flags, blend_mode });
    }
    Ok(mats)
}

// ── group file parsing ──────────────────────────────────────────────────────

/// Size of the MOGP header before sub-chunks begin.
const MOGP_HEADER_SIZE: usize = 68;

/// Parse a WMO group file and return meshes per render batch.
pub fn load_wmo_group(data: &[u8]) -> Result<WmoGroupData, String> {
    let mogp_payload = find_mogp(data)?;
    if mogp_payload.len() < MOGP_HEADER_SIZE {
        return Err(format!("MOGP payload too small: {} bytes", mogp_payload.len()));
    }

    let sub_chunks = &mogp_payload[MOGP_HEADER_SIZE..];
    let raw = parse_group_subchunks(sub_chunks)?;
    build_group_batches(&raw)
}

/// Find the MOGP chunk payload in a group file.
fn find_mogp(data: &[u8]) -> Result<&[u8], String> {
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        if tag == b"PGOM" {
            return Ok(payload);
        }
    }
    Err("No MOGP chunk found in WMO group file".to_string())
}

struct RawGroupData {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u16>,
    batches: Vec<RawBatch>,
}

struct RawBatch {
    start_index: u32,
    count: u16,
    min_index: u16,
    max_index: u16,
    material_id: u16,
}

/// Parse MOVT, MOVI, MONR, MOTV, MOBA sub-chunks from MOGP payload.
fn parse_group_subchunks(data: &[u8]) -> Result<RawGroupData, String> {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let mut batches = Vec::new();

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"TVOM" => vertices = parse_vec3_array(payload)?,
            b"RNOM" => normals = parse_vec3_array(payload)?,
            b"VTOM" => uvs = parse_vec2_array(payload)?,
            b"IVOM" => indices = parse_u16_array(payload),
            b"ABOM" => batches = parse_moba(payload)?,
            _ => {}
        }
    }

    if vertices.is_empty() {
        return Err("WMO group missing MOVT (vertices)".to_string());
    }
    if indices.is_empty() {
        return Err("WMO group missing MOVI (indices)".to_string());
    }

    Ok(RawGroupData { vertices, normals, uvs, indices, batches })
}

/// Parse array of [f32; 3] from chunk payload.
fn parse_vec3_array(data: &[u8]) -> Result<Vec<[f32; 3]>, String> {
    let count = data.len() / 12;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 12;
        out.push([
            read_f32(data, base)?,
            read_f32(data, base + 4)?,
            read_f32(data, base + 8)?,
        ]);
    }
    Ok(out)
}

/// Parse array of [f32; 2] from chunk payload.
fn parse_vec2_array(data: &[u8]) -> Result<Vec<[f32; 2]>, String> {
    let count = data.len() / 8;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 8;
        out.push([read_f32(data, base)?, read_f32(data, base + 4)?]);
    }
    Ok(out)
}

/// Parse array of u16 from chunk payload.
fn parse_u16_array(data: &[u8]) -> Vec<u16> {
    data.chunks_exact(2)
        .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

/// Parse MOBA chunk: 24 bytes per render batch.
fn parse_moba(data: &[u8]) -> Result<Vec<RawBatch>, String> {
    let count = data.len() / 24;
    let mut batches = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 24;
        let material_id_large = read_u16(data, base + 0x0A)?;
        let start_index = read_u32(data, base + 0x0C)?;
        let count = read_u16(data, base + 0x10)?;
        let min_index = read_u16(data, base + 0x12)?;
        let max_index = read_u16(data, base + 0x14)?;
        let material_id_small = read_u8(data, base + 0x17)?;
        let material_id = if material_id_small == 0xFF {
            material_id_large
        } else {
            material_id_small as u16
        };
        batches.push(RawBatch { start_index, count, min_index, max_index, material_id });
    }
    Ok(batches)
}

// ── mesh building ───────────────────────────────────────────────────────────

/// Build Bevy meshes from raw group data, one mesh per render batch.
fn build_group_batches(raw: &RawGroupData) -> Result<WmoGroupData, String> {
    if raw.batches.is_empty() {
        let mesh = build_whole_group_mesh(raw);
        return Ok(WmoGroupData {
            batches: vec![WmoGroupBatch { mesh, material_index: 0 }],
        });
    }

    let mut out = Vec::with_capacity(raw.batches.len());
    for batch in &raw.batches {
        let mesh = build_batch_mesh(raw, batch);
        out.push(WmoGroupBatch { mesh, material_index: batch.material_id });
    }
    Ok(WmoGroupData { batches: out })
}

/// Build a Bevy mesh for the entire group (fallback when no MOBA batches).
fn build_whole_group_mesh(raw: &RawGroupData) -> Mesh {
    let positions: Vec<[f32; 3]> = raw.vertices.iter()
        .map(|v| wow_to_bevy(v[0], v[1], v[2]))
        .collect();
    let normals = convert_normals(&raw.normals, positions.len());
    let uvs = convert_uvs(&raw.uvs, positions.len());
    let indices: Vec<u32> = raw.indices.iter().map(|&i| i as u32).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Build a Bevy mesh for one render batch, slicing and re-indexing vertices.
fn build_batch_mesh(raw: &RawGroupData, batch: &RawBatch) -> Mesh {
    let vmin = batch.min_index as usize;
    let vmax = (batch.max_index as usize).min(raw.vertices.len().saturating_sub(1));
    let vert_count = vmax - vmin + 1;

    let positions: Vec<[f32; 3]> = raw.vertices[vmin..=vmax]
        .iter()
        .map(|v| wow_to_bevy(v[0], v[1], v[2]))
        .collect();
    let normals = if raw.normals.len() > vmax {
        raw.normals[vmin..=vmax]
            .iter()
            .map(|n| wow_to_bevy(n[0], n[1], n[2]))
            .collect()
    } else {
        vec![[0.0, 1.0, 0.0]; vert_count]
    };
    let uvs = if raw.uvs.len() > vmax {
        raw.uvs[vmin..=vmax].to_vec()
    } else {
        vec![[0.0, 0.0]; vert_count]
    };

    let idx_start = batch.start_index as usize;
    let idx_end = (idx_start + batch.count as usize).min(raw.indices.len());
    let indices: Vec<u32> = raw.indices[idx_start..idx_end]
        .iter()
        .map(|&i| (i - batch.min_index) as u32)
        .collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn convert_normals(src: &[[f32; 3]], expected: usize) -> Vec<[f32; 3]> {
    if src.len() == expected {
        src.iter().map(|n| wow_to_bevy(n[0], n[1], n[2])).collect()
    } else {
        vec![[0.0, 1.0, 0.0]; expected]
    }
}

fn convert_uvs(src: &[[f32; 2]], expected: usize) -> Vec<[f32; 2]> {
    if src.len() == expected {
        src.to_vec()
    } else {
        vec![[0.0, 0.0]; expected]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_momt_entry_size() {
        // 64 bytes of zeros should parse as one material with zero fields
        let data = vec![0u8; 64];
        let mats = parse_momt(&data).unwrap();
        assert_eq!(mats.len(), 1);
        assert_eq!(mats[0].texture_fdid, 0);
    }

    #[test]
    fn parse_moba_entry_size() {
        // 24 bytes of zeros should parse as one batch
        let data = vec![0u8; 24];
        let batches = parse_moba(&data).unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].material_id, 0);
    }

    #[test]
    fn parse_elwynn_bridge_root() {
        let data = std::fs::read("data/models/108121.wmo")
            .expect("missing test asset: elwynnwidebridge root");
        let root = load_wmo_root(&data).unwrap();
        assert!(root.n_groups > 0, "should have at least one group");
        eprintln!(
            "elwynnwidebridge: {} groups, {} materials",
            root.n_groups, root.materials.len(),
        );
        for (i, m) in root.materials.iter().enumerate() {
            eprintln!("  mat {i}: texture_fdid={} flags={:#x} blend={}", m.texture_fdid, m.flags, m.blend_mode);
        }
    }

    #[test]
    fn parse_elwynn_bridge_group() {
        let data = std::fs::read("data/models/108122.wmo")
            .expect("missing test asset: elwynnwidebridge group 0");
        let group = load_wmo_group(&data).unwrap();
        assert!(!group.batches.is_empty(), "should have at least one batch");
        eprintln!("elwynnwidebridge group 0: {} batches", group.batches.len());
        for (i, b) in group.batches.iter().enumerate() {
            let vc = b.mesh.count_vertices();
            eprintln!("  batch {i}: material_index={} vertices={vc}", b.material_index);
        }
    }
}
