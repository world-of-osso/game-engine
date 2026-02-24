use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

/// Convert WoW coordinate (X-right, Y-forward, Z-up) to Bevy (X-right, Y-up, Z-back).
fn wow_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, z, -y]
}

pub struct M2Model {
    pub mesh: Mesh,
    pub texture_ids: Vec<u32>,
}

struct M2Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coords: [f32; 2],
}

struct M2Chunks<'a> {
    md20: &'a [u8],
    txid: Option<&'a [u8]>,
}

/// Read a little-endian u32 from a byte slice at the given offset.
fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_u32 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

/// Read a little-endian f32 from a byte slice at the given offset.
fn read_f32(data: &[u8], off: usize) -> Result<f32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_f32 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

/// Read a little-endian u16 from a byte slice at the given offset.
fn read_u16(data: &[u8], off: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_u16 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

/// Parse top-level chunks, extracting MD21 (MD20 payload) and optional TXID.
fn parse_chunks(data: &[u8]) -> Result<M2Chunks<'_>, String> {
    let mut md20 = None;
    let mut txid = None;
    let mut off = 0;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = read_u32(data, off + 4)? as usize;
        let end = off + 8 + size;
        if end > data.len() {
            let tag_str = std::str::from_utf8(tag).unwrap_or("????");
            return Err(format!("Chunk {tag_str} truncated at offset {off:#x}"));
        }
        match tag {
            b"MD21" => md20 = Some(&data[off + 8..end]),
            b"TXID" => txid = Some(&data[off + 8..end]),
            _ => {}
        }
        off = end;
    }
    Ok(M2Chunks {
        md20: md20.ok_or("No MD21 chunk found")?,
        txid,
    })
}

/// Parse the vertex array from the MD20 blob (M2Array at offset 0x3C).
fn parse_vertices(md20: &[u8]) -> Result<Vec<M2Vertex>, String> {
    if md20.len() < 0x44 {
        return Err("MD20 header too short for vertices".into());
    }
    let count = read_u32(md20, 0x3C)? as usize;
    let offset = read_u32(md20, 0x40)? as usize;

    let mut vertices = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 48;
        if base + 48 > md20.len() {
            return Err(format!("Vertex {i} out of bounds at offset {base:#x}"));
        }
        vertices.push(M2Vertex {
            position: [
                read_f32(md20, base)?,
                read_f32(md20, base + 4)?,
                read_f32(md20, base + 8)?,
            ],
            // skip bone_weights (4 bytes) + bone_indices (4 bytes)
            normal: [
                read_f32(md20, base + 20)?,
                read_f32(md20, base + 24)?,
                read_f32(md20, base + 28)?,
            ],
            tex_coords: [read_f32(md20, base + 32)?, read_f32(md20, base + 36)?],
        });
    }
    Ok(vertices)
}

/// Parse a .skin file: resolve the two-level index indirection.
///
/// Layout: "SKIN" magic, then vertex_lookup M2Array, then indices M2Array.
/// Final index = vertex_lookup[indices[i]]
fn parse_skin(data: &[u8]) -> Result<Vec<u16>, String> {
    if data.len() < 16 || &data[0..4] != b"SKIN" {
        return Err("Invalid skin file (bad magic)".into());
    }
    let lookup_count = read_u32(data, 4)? as usize;
    let lookup_offset = read_u32(data, 8)? as usize;
    let indices_count = read_u32(data, 12)? as usize;
    let indices_offset = read_u32(data, 16)? as usize;

    let mut resolved = Vec::with_capacity(indices_count);
    for i in 0..indices_count {
        let idx = read_u16(data, indices_offset + i * 2)? as usize;
        if idx >= lookup_count {
            return Err(format!("Skin index {idx} out of lookup bounds ({lookup_count})"));
        }
        let global_idx = read_u16(data, lookup_offset + idx * 2)?;
        resolved.push(global_idx);
    }
    Ok(resolved)
}

fn load_skin_indices(m2_path: &Path) -> Option<Vec<u16>> {
    let stem = m2_path.file_stem()?.to_str()?;
    let skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
    let data = std::fs::read(&skin_path).ok()?;
    parse_skin(&data).ok()
}

/// Parse texture types from the MD20 textures M2Array at offset 0x50.
/// Each texture entry is 16 bytes: { type: u32, flags: u32, name: M2Array }.
fn parse_texture_types(md20: &[u8]) -> Result<Vec<u32>, String> {
    if md20.len() < 0x58 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x50)? as usize;
    let offset = read_u32(md20, 0x54)? as usize;
    let mut types = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 16;
        if base + 16 > md20.len() {
            return Err(format!("Texture entry {i} out of bounds at {base:#x}"));
        }
        types.push(read_u32(md20, base)?);
    }
    Ok(types)
}

/// Parse the TXID chunk as an array of u32 FileDataIDs.
fn parse_txid(data: &[u8]) -> Vec<u32> {
    (0..data.len() / 4)
        .filter_map(|i| read_u32(data, i * 4).ok())
        .collect()
}

/// Extract hardcoded (type 0) texture FileDataIDs from MD20 + TXID.
fn extract_texture_ids(md20: &[u8], txid: Option<&[u8]>) -> Result<Vec<u32>, String> {
    let txid_data = match txid {
        Some(data) => data,
        None => return Ok(Vec::new()),
    };
    let types = parse_texture_types(md20)?;
    let fdids = parse_txid(txid_data);
    Ok(types
        .iter()
        .zip(fdids.iter())
        .filter(|(ty, fdid)| **ty == 0 && **fdid != 0)
        .map(|(_, fdid)| *fdid)
        .collect())
}

fn build_mesh(vertices: &[M2Vertex], indices: Vec<u16>) -> Mesh {
    let mut positions = Vec::with_capacity(vertices.len());
    let mut normals = Vec::with_capacity(vertices.len());
    let mut uvs = Vec::with_capacity(vertices.len());

    for v in vertices {
        positions.push(wow_to_bevy(v.position[0], v.position[1], v.position[2]));
        normals.push(wow_to_bevy(v.normal[0], v.normal[1], v.normal[2]));
        uvs.push(v.tex_coords);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U16(indices));
    mesh
}

/// Load an M2 model file (chunked MD21 format) and return mesh + texture IDs.
pub fn load_m2(path: &Path) -> Result<M2Model, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read M2 file: {e}"))?;
    let chunks = parse_chunks(&data)?;
    let vertices = parse_vertices(chunks.md20)?;
    let texture_ids = extract_texture_ids(chunks.md20, chunks.txid)?;

    let indices = load_skin_indices(path)
        .unwrap_or_else(|| (0..vertices.len() as u16).collect());

    Ok(M2Model {
        mesh: build_mesh(&vertices, indices),
        texture_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal MD21 chunked file with the given MD20 blob.
    fn wrap_md21(md20: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"MD21");
        data.extend_from_slice(&(md20.len() as u32).to_le_bytes());
        data.extend_from_slice(md20);
        data
    }

    /// Build a minimal MD20 blob with a single vertex at the given position.
    fn minimal_md20(pos: [f32; 3]) -> Vec<u8> {
        // MD20 header: 0x44 bytes minimum (up to end of num_skin_profiles)
        // Vertices M2Array at 0x3C: count=1, offset pointing after header
        let vertex_offset: u32 = 0x48; // just past the header
        let mut md20 = vec![0u8; vertex_offset as usize + 48];

        // Magic
        md20[0..4].copy_from_slice(b"MD20");
        // version = 264 (WotLK)
        md20[4..8].copy_from_slice(&264u32.to_le_bytes());
        // vertices M2Array at 0x3C
        md20[0x3C..0x40].copy_from_slice(&1u32.to_le_bytes()); // count
        md20[0x40..0x44].copy_from_slice(&vertex_offset.to_le_bytes()); // offset

        // Write one 48-byte vertex at vertex_offset
        let base = vertex_offset as usize;
        // position
        md20[base..base + 4].copy_from_slice(&pos[0].to_le_bytes());
        md20[base + 4..base + 8].copy_from_slice(&pos[1].to_le_bytes());
        md20[base + 8..base + 12].copy_from_slice(&pos[2].to_le_bytes());
        // bone_weights + bone_indices (8 bytes of zeros — already zeroed)
        // normal at +20: [0, 1, 0]
        md20[base + 24..base + 28].copy_from_slice(&1.0f32.to_le_bytes());
        // tex_coords at +32: [0.5, 0.5]
        md20[base + 32..base + 36].copy_from_slice(&0.5f32.to_le_bytes());
        md20[base + 36..base + 40].copy_from_slice(&0.5f32.to_le_bytes());

        md20
    }

    #[test]
    fn parse_chunks_finds_md21() {
        let md20 = minimal_md20([1.0, 2.0, 3.0]);
        let data = wrap_md21(&md20);
        let chunks = parse_chunks(&data).unwrap();
        assert_eq!(chunks.md20, &md20);
        assert!(chunks.txid.is_none());
    }

    #[test]
    fn parse_chunks_captures_txid() {
        let md20 = minimal_md20([0.0, 0.0, 0.0]);
        let txid_data: Vec<u8> = [42u32, 99u32]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let mut data = Vec::new();
        data.extend_from_slice(b"TXID");
        data.extend_from_slice(&(txid_data.len() as u32).to_le_bytes());
        data.extend_from_slice(&txid_data);
        data.extend_from_slice(&wrap_md21(&md20));

        let chunks = parse_chunks(&data).unwrap();
        assert_eq!(chunks.md20, &md20);
        assert_eq!(chunks.txid.unwrap(), &txid_data);
    }

    #[test]
    fn parse_txid_reads_fdids() {
        let data: Vec<u8> = [42u32, 99u32, 0u32]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        assert_eq!(parse_txid(&data), vec![42, 99, 0]);
    }

    #[test]
    fn extract_texture_ids_filters_type_0() {
        // MD20 with textures M2Array at 0x50: 2 entries at offset 0x60
        let tex_offset: u32 = 0x60;
        let mut md20 = vec![0u8; tex_offset as usize + 32];
        md20[0..4].copy_from_slice(b"MD20");
        md20[4..8].copy_from_slice(&274u32.to_le_bytes());
        md20[0x50..0x54].copy_from_slice(&2u32.to_le_bytes());
        md20[0x54..0x58].copy_from_slice(&tex_offset.to_le_bytes());
        // Entry 0: type=0 (hardcoded)
        let b = tex_offset as usize;
        md20[b..b + 4].copy_from_slice(&0u32.to_le_bytes());
        // Entry 1: type=1 (character skin)
        md20[b + 16..b + 20].copy_from_slice(&1u32.to_le_bytes());

        let txid: Vec<u8> = [100u32, 200u32]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let ids = extract_texture_ids(&md20, Some(&txid)).unwrap();
        assert_eq!(ids, vec![100]);
    }

    #[test]
    fn extract_texture_ids_empty_without_txid() {
        let md20 = minimal_md20([0.0, 0.0, 0.0]);
        let ids = extract_texture_ids(&md20, None).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn parse_vertices_single() {
        let md20 = minimal_md20([1.0, 2.0, 3.0]);
        let verts = parse_vertices(&md20).unwrap();
        assert_eq!(verts.len(), 1);
        assert_eq!(verts[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(verts[0].normal, [0.0, 1.0, 0.0]);
        assert_eq!(verts[0].tex_coords, [0.5, 0.5]);
    }

    #[test]
    fn parse_skin_resolves_indices() {
        // Build a minimal skin: 3 lookup entries, 3 indices
        let mut skin = Vec::new();
        skin.extend_from_slice(b"SKIN");
        // vertex_lookup: count=3, offset=20
        skin.extend_from_slice(&3u32.to_le_bytes());
        skin.extend_from_slice(&20u32.to_le_bytes());
        // indices: count=3, offset=26
        skin.extend_from_slice(&3u32.to_le_bytes());
        skin.extend_from_slice(&26u32.to_le_bytes());
        // vertex_lookup data at offset 20: [10, 20, 30]
        skin.extend_from_slice(&10u16.to_le_bytes());
        skin.extend_from_slice(&20u16.to_le_bytes());
        skin.extend_from_slice(&30u16.to_le_bytes());
        // indices data at offset 26: [2, 0, 1]
        skin.extend_from_slice(&2u16.to_le_bytes());
        skin.extend_from_slice(&0u16.to_le_bytes());
        skin.extend_from_slice(&1u16.to_le_bytes());

        let resolved = parse_skin(&skin).unwrap();
        assert_eq!(resolved, vec![30, 10, 20]);
    }

    #[test]
    fn wow_to_bevy_transform() {
        let [x, y, z] = wow_to_bevy(1.0, 2.0, 3.0);
        assert_eq!(x, 1.0);
        assert_eq!(y, 3.0); // z -> y
        assert_eq!(z, -2.0); // -y -> z
    }
}
