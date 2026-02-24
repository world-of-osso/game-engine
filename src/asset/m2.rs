use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

/// Convert WoW coordinate (X-right, Y-forward, Z-up) to Bevy (X-right, Y-up, Z-back).
fn wow_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, z, -y]
}

pub struct M2RenderBatch {
    pub mesh: Mesh,
    /// None = runtime-resolved texture (character/creature skin), use placeholder color.
    pub texture_fdid: Option<u32>,
}

pub struct M2Model {
    pub batches: Vec<M2RenderBatch>,
}

struct M2Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    tex_coords: [f32; 2],
}

struct M2Submesh {
    vertex_start: u16,
    vertex_count: u16,
    triangle_start: u16,
    triangle_count: u16,
}

struct M2TextureUnit {
    submesh_index: u16,
    /// Index into the MD20 textureLookup table.
    texture_id: u16,
}

struct SkinData {
    lookup: Vec<u16>,
    indices: Vec<u16>,
    submeshes: Vec<M2Submesh>,
    batches: Vec<M2TextureUnit>,
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

/// Parse a .skin file: vertex lookup, raw indices, submeshes, and texture batches.
///
/// Skin header (after "SKIN" magic):
///   offset  4: vertex_lookup M2Array
///   offset 12: indices M2Array
///   offset 20: bones M2Array (skipped)
///   offset 28: submeshes M2Array -> M2SkinSection[n] (48 bytes each)
///   offset 36: batches M2Array -> M2Batch[n] (24 bytes each)
fn parse_skin_full(data: &[u8]) -> Result<SkinData, String> {
    if data.len() < 44 || &data[0..4] != b"SKIN" {
        return Err("Invalid skin file (bad magic)".into());
    }
    let lookup_count = read_u32(data, 4)? as usize;
    let lookup_offset = read_u32(data, 8)? as usize;
    let indices_count = read_u32(data, 12)? as usize;
    let indices_offset = read_u32(data, 16)? as usize;
    // bones at offset 20 (skip)
    let sub_count = read_u32(data, 28)? as usize;
    let sub_offset = read_u32(data, 32)? as usize;
    let batch_count = read_u32(data, 36)? as usize;
    let batch_offset = read_u32(data, 40)? as usize;

    let mut lookup = Vec::with_capacity(lookup_count);
    for i in 0..lookup_count {
        lookup.push(read_u16(data, lookup_offset + i * 2)?);
    }

    let mut indices = Vec::with_capacity(indices_count);
    for i in 0..indices_count {
        indices.push(read_u16(data, indices_offset + i * 2)?);
    }

    let submeshes = parse_submeshes(data, sub_offset, sub_count)?;
    let batches = parse_texture_units(data, batch_offset, batch_count)?;

    Ok(SkinData {
        lookup,
        indices,
        submeshes,
        batches,
    })
}

/// Parse M2SkinSection entries (48 bytes each).
/// Layout: meshPartID(u16), level(u16), vertexStart(u16), vertexCount(u16),
///         indexStart(u16), indexCount(u16), ... (bone/bounding data)
fn parse_submeshes(data: &[u8], offset: usize, count: usize) -> Result<Vec<M2Submesh>, String> {
    let mut subs = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 48;
        if base + 48 > data.len() {
            return Err(format!("Submesh {i} out of bounds at {base:#x}"));
        }
        subs.push(M2Submesh {
            vertex_start: read_u16(data, base + 4)?,
            vertex_count: read_u16(data, base + 6)?,
            triangle_start: read_u16(data, base + 8)?,
            triangle_count: read_u16(data, base + 10)?,
        });
    }
    Ok(subs)
}

/// Parse M2Batch entries (24 bytes each).
/// Layout: flags(u16), shader_id(u16), submesh_index(u16), submesh_index2(u16),
///         color_index(i16), render_flags_index(u16), texture_unit_index(u16), mode(u16),
///         texture_id(u16), texture_unit2(u16), transparency_index(u16), texture_animation_id(u16)
fn parse_texture_units(data: &[u8], offset: usize, count: usize) -> Result<Vec<M2TextureUnit>, String> {
    let mut units = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 24;
        if base + 24 > data.len() {
            return Err(format!("TextureUnit {i} out of bounds at {base:#x}"));
        }
        units.push(M2TextureUnit {
            submesh_index: read_u16(data, base + 4)?,
            texture_id: read_u16(data, base + 16)?,
        });
    }
    Ok(units)
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

/// Parse the textureLookup M2Array at MD20 offset 0x80 (array of u16).
fn parse_texture_lookup(md20: &[u8]) -> Result<Vec<u16>, String> {
    if md20.len() < 0x88 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x80)? as usize;
    let offset = read_u32(md20, 0x84)? as usize;
    let mut lookup = Vec::with_capacity(count);
    for i in 0..count {
        lookup.push(read_u16(md20, offset + i * 2)?);
    }
    Ok(lookup)
}

/// Default FDIDs for runtime-resolved character texture types (human male, light skin).
/// Used when the model has no hardcoded FDID for a texture slot.
fn default_fdid_for_type(ty: u32) -> Option<u32> {
    match ty {
        1 => Some(1027767),  // body skin (humanmaleskin00_00_hd)
        2 => Some(1027743),  // underwear (humanmalenakedpelvisskin00_00_hd)
        15 => Some(1043094), // hair/scalp (scalpupperhair00_00_hd)
        16 => Some(1042989), // beard (faciallowerhair00_00_hd)
        _ => None,
    }
}

/// Resolve a batch's texture through the lookup chain:
/// batch.texture_id -> textureLookup[id] -> textures[idx].type -> TXID[idx]
/// For runtime-resolved types (type != 0), falls back to default character textures.
fn resolve_batch_texture(
    unit: &M2TextureUnit,
    tex_lookup: &[u16],
    tex_types: &[u32],
    txid: &[u32],
) -> Option<u32> {
    let tex_idx = *tex_lookup.get(unit.texture_id as usize)? as usize;
    let ty = *tex_types.get(tex_idx)?;
    if ty == 0 {
        let fdid = *txid.get(tex_idx)?;
        if fdid != 0 { return Some(fdid); }
    }
    default_fdid_for_type(ty)
}

/// Resolve raw skin indices through the vertex lookup table.
fn resolve_indices(lookup: &[u16], indices: &[u16]) -> Vec<u16> {
    indices
        .iter()
        .filter_map(|&idx| lookup.get(idx as usize).copied())
        .collect()
}

/// Return the first hardcoded (type 0) texture FDID, if any.
fn first_hardcoded_texture(tex_types: &[u32], txid: &[u32]) -> Option<u32> {
    tex_types
        .iter()
        .zip(txid.iter())
        .find(|(ty, fdid)| **ty == 0 && **fdid != 0)
        .map(|(_, fdid)| *fdid)
}

/// Build a Bevy Mesh for one submesh: compact vertex buffer + remapped indices.
fn build_batch_mesh(
    vertices: &[M2Vertex],
    lookup: &[u16],
    indices: &[u16],
    sub: &M2Submesh,
) -> Mesh {
    let vstart = sub.vertex_start as usize;
    let vcount = sub.vertex_count as usize;
    let tstart = sub.triangle_start as usize;
    let tcount = sub.triangle_count as usize;

    let mut positions = Vec::with_capacity(vcount);
    let mut normals = Vec::with_capacity(vcount);
    let mut uvs = Vec::with_capacity(vcount);

    for i in 0..vcount {
        let global_idx = lookup.get(vstart + i).copied().unwrap_or(0) as usize;
        if let Some(v) = vertices.get(global_idx) {
            positions.push(wow_to_bevy(v.position[0], v.position[1], v.position[2]));
            normals.push(wow_to_bevy(v.normal[0], v.normal[1], v.normal[2]));
            uvs.push(v.tex_coords);
        }
    }

    let mut local_indices = Vec::with_capacity(tcount);
    for j in 0..tcount {
        if let Some(&idx) = indices.get(tstart + j) {
            local_indices.push((idx as usize).saturating_sub(vstart) as u16);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U16(local_indices));
    mesh
}

/// Build a Bevy Mesh from all vertices + resolved index list (fallback path).
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

fn load_skin_data(m2_path: &Path) -> Option<SkinData> {
    let stem = m2_path.file_stem()?.to_str()?;
    let skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
    let data = std::fs::read(&skin_path).ok()?;
    parse_skin_full(&data).ok()
}

/// Build per-batch meshes from skin submesh/batch data.
fn build_batched_model(
    vertices: &[M2Vertex],
    skin: &SkinData,
    tex_lookup: &[u16],
    tex_types: &[u32],
    txid: &[u32],
) -> Result<M2Model, String> {
    let mut batches = Vec::with_capacity(skin.batches.len());
    for unit in &skin.batches {
        let sub_idx = unit.submesh_index as usize;
        if sub_idx >= skin.submeshes.len() {
            return Err(format!(
                "Batch submesh_index {sub_idx} >= submesh count {}",
                skin.submeshes.len()
            ));
        }
        let sub = &skin.submeshes[sub_idx];
        let mesh = build_batch_mesh(vertices, &skin.lookup, &skin.indices, sub);
        let texture_fdid = resolve_batch_texture(unit, tex_lookup, tex_types, txid);
        batches.push(M2RenderBatch { mesh, texture_fdid });
    }
    Ok(M2Model { batches })
}

/// Load an M2 model file (chunked MD21 format) and return per-batch meshes + textures.
pub fn load_m2(path: &Path) -> Result<M2Model, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read M2 file: {e}"))?;
    let chunks = parse_chunks(&data)?;
    let vertices = parse_vertices(chunks.md20)?;
    let tex_types = parse_texture_types(chunks.md20)?;
    let tex_lookup = parse_texture_lookup(chunks.md20)?;
    let txid = chunks.txid.map(parse_txid).unwrap_or_default();

    let skin = load_skin_data(path);

    if let Some(ref skin) = skin
        && !skin.submeshes.is_empty()
        && !skin.batches.is_empty()
    {
        return build_batched_model(&vertices, skin, &tex_lookup, &tex_types, &txid);
    }

    // Fallback: single mesh, first hardcoded texture
    let indices = match skin {
        Some(skin) => resolve_indices(&skin.lookup, &skin.indices),
        None => (0..vertices.len() as u16).collect(),
    };
    let fdid = first_hardcoded_texture(&tex_types, &txid);
    Ok(M2Model {
        batches: vec![M2RenderBatch {
            mesh: build_mesh(&vertices, indices),
            texture_fdid: fdid,
        }],
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
        // bone_weights + bone_indices (8 bytes of zeros -- already zeroed)
        // normal at +20: [0, 1, 0]
        md20[base + 24..base + 28].copy_from_slice(&1.0f32.to_le_bytes());
        // tex_coords at +32: [0.5, 0.5]
        md20[base + 32..base + 36].copy_from_slice(&0.5f32.to_le_bytes());
        md20[base + 36..base + 40].copy_from_slice(&0.5f32.to_le_bytes());

        md20
    }

    /// Build a minimal skin file with full header (44 bytes) + data sections.
    fn build_skin(
        lookup: &[u16],
        indices: &[u16],
        submeshes: &[(u16, u16, u16, u16)],
        batches: &[(u16, u16)],
    ) -> Vec<u8> {
        let header_size: u32 = 44;
        let lookup_offset = header_size;
        let indices_offset = lookup_offset + (lookup.len() as u32) * 2;
        let sub_offset = indices_offset + (indices.len() as u32) * 2;
        let batch_offset = sub_offset + (submeshes.len() as u32) * 48;
        let total = batch_offset + (batches.len() as u32) * 24;

        let mut skin = vec![0u8; total as usize];
        skin[0..4].copy_from_slice(b"SKIN");

        // vertex_lookup M2Array
        skin[4..8].copy_from_slice(&(lookup.len() as u32).to_le_bytes());
        skin[8..12].copy_from_slice(&lookup_offset.to_le_bytes());
        // indices M2Array
        skin[12..16].copy_from_slice(&(indices.len() as u32).to_le_bytes());
        skin[16..20].copy_from_slice(&indices_offset.to_le_bytes());
        // bones M2Array (empty)
        // submeshes M2Array
        skin[28..32].copy_from_slice(&(submeshes.len() as u32).to_le_bytes());
        skin[32..36].copy_from_slice(&sub_offset.to_le_bytes());
        // batches M2Array
        skin[36..40].copy_from_slice(&(batches.len() as u32).to_le_bytes());
        skin[40..44].copy_from_slice(&batch_offset.to_le_bytes());

        // Write lookup data
        for (i, &v) in lookup.iter().enumerate() {
            let off = lookup_offset as usize + i * 2;
            skin[off..off + 2].copy_from_slice(&v.to_le_bytes());
        }
        // Write indices data
        for (i, &v) in indices.iter().enumerate() {
            let off = indices_offset as usize + i * 2;
            skin[off..off + 2].copy_from_slice(&v.to_le_bytes());
        }
        // Write submeshes (48 bytes each, fields at +4,+6,+8,+10)
        for (i, &(vs, vc, ts, tc)) in submeshes.iter().enumerate() {
            let base = sub_offset as usize + i * 48;
            skin[base + 4..base + 6].copy_from_slice(&vs.to_le_bytes());
            skin[base + 6..base + 8].copy_from_slice(&vc.to_le_bytes());
            skin[base + 8..base + 10].copy_from_slice(&ts.to_le_bytes());
            skin[base + 10..base + 12].copy_from_slice(&tc.to_le_bytes());
        }
        // Write batches (24 bytes each, submesh_index at +4, texture_id at +16)
        for (i, &(sub_idx, tex_id)) in batches.iter().enumerate() {
            let base = batch_offset as usize + i * 24;
            skin[base + 4..base + 6].copy_from_slice(&sub_idx.to_le_bytes());
            skin[base + 16..base + 18].copy_from_slice(&tex_id.to_le_bytes());
        }

        skin
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
    fn first_hardcoded_texture_filters_type_0() {
        let types = vec![0, 1];
        let txid = vec![100, 200];
        assert_eq!(first_hardcoded_texture(&types, &txid), Some(100));
    }

    #[test]
    fn first_hardcoded_texture_none_when_empty() {
        assert_eq!(first_hardcoded_texture(&[], &[]), None);
        assert_eq!(first_hardcoded_texture(&[1], &[100]), None);
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
    fn parse_skin_full_resolves_indices() {
        let skin = build_skin(&[10, 20, 30], &[2, 0, 1], &[], &[]);
        let data = parse_skin_full(&skin).unwrap();
        assert_eq!(data.lookup, vec![10, 20, 30]);
        assert_eq!(data.indices, vec![2, 0, 1]);
        assert!(data.submeshes.is_empty());
        assert!(data.batches.is_empty());

        let resolved = resolve_indices(&data.lookup, &data.indices);
        assert_eq!(resolved, vec![30, 10, 20]);
    }

    #[test]
    fn parse_skin_full_with_submeshes_and_batches() {
        // 4 lookup entries, 6 indices (2 triangles), 1 submesh, 1 batch
        let skin = build_skin(
            &[0, 1, 2, 3],
            &[0, 1, 2, 2, 3, 0],
            &[(0, 4, 0, 6)],   // vertex_start=0, count=4, tri_start=0, tri_count=6
            &[(0, 0)],         // submesh_index=0, texture_id=0
        );
        let data = parse_skin_full(&skin).unwrap();
        assert_eq!(data.submeshes.len(), 1);
        assert_eq!(data.submeshes[0].vertex_start, 0);
        assert_eq!(data.submeshes[0].vertex_count, 4);
        assert_eq!(data.submeshes[0].triangle_start, 0);
        assert_eq!(data.submeshes[0].triangle_count, 6);
        assert_eq!(data.batches.len(), 1);
        assert_eq!(data.batches[0].submesh_index, 0);
        assert_eq!(data.batches[0].texture_id, 0);
    }

    #[test]
    fn resolve_batch_texture_chain() {
        let tex_lookup = vec![0, 1];
        let tex_types = vec![0, 1];
        let txid = vec![100, 200];

        // texture_id=0 -> lookup[0]=0 -> type=0 -> FDID 100
        let unit0 = M2TextureUnit { submesh_index: 0, texture_id: 0 };
        assert_eq!(resolve_batch_texture(&unit0, &tex_lookup, &tex_types, &txid), Some(100));

        // texture_id=1 -> lookup[1]=1 -> type=1 (runtime) -> default body skin FDID
        let unit1 = M2TextureUnit { submesh_index: 0, texture_id: 1 };
        assert_eq!(resolve_batch_texture(&unit1, &tex_lookup, &tex_types, &txid), Some(1027767));

        // unknown runtime type -> None
        let tex_types_unk = vec![0, 99];
        let unit2 = M2TextureUnit { submesh_index: 0, texture_id: 1 };
        assert_eq!(resolve_batch_texture(&unit2, &tex_lookup, &tex_types_unk, &txid), None);
    }

    #[test]
    fn wow_to_bevy_transform() {
        let [x, y, z] = wow_to_bevy(1.0, 2.0, 3.0);
        assert_eq!(x, 1.0);
        assert_eq!(y, 3.0); // z -> y
        assert_eq!(z, -2.0); // -y -> z
    }

    #[test]
    fn debug_humanmale_textures() {
        let path = std::path::Path::new("data/models/humanmale.m2");
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(_) => return,
        };

        let chunks = parse_chunks(&data).expect("parse_chunks failed");
        let tex_types = parse_texture_types(chunks.md20).expect("parse_texture_types failed");
        let tex_lookup = parse_texture_lookup(chunks.md20).expect("parse_texture_lookup failed");
        let txid = chunks.txid.map(parse_txid).unwrap_or_default();

        let skin_path = path.with_file_name("humanmale00.skin");
        let skin = std::fs::read(&skin_path)
            .ok()
            .and_then(|d| parse_skin_full(&d).ok());

        println!("=== TXID FDIDs ({} entries) ===", txid.len());
        for (i, fdid) in txid.iter().enumerate() {
            println!("  TXID[{i}] = {fdid}");
        }

        println!("\n=== Texture Types ({} entries) ===", tex_types.len());
        for (i, ty) in tex_types.iter().enumerate() {
            println!("  textures[{i}].type = {ty}");
        }

        println!("\n=== textureLookup ({} entries) ===", tex_lookup.len());
        for (i, v) in tex_lookup.iter().enumerate() {
            println!("  textureLookup[{i}] = {v}");
        }

        match skin {
            None => println!("\n=== Skin: not found or failed to parse ==="),
            Some(ref skin) => {
                println!("\n=== Skin Batches ({} entries) ===", skin.batches.len());
                for (i, batch) in skin.batches.iter().enumerate() {
                    let tex_idx = tex_lookup
                        .get(batch.texture_id as usize)
                        .copied()
                        .map(|v| v as usize);
                    let ty = tex_idx.and_then(|idx| tex_types.get(idx)).copied();
                    let fdid = tex_idx.and_then(|idx| txid.get(idx)).copied();
                    println!(
                        "  batch[{i}]: submesh_index={}, texture_id={} -> textureLookup[{}]={} -> type={} -> FDID={}",
                        batch.submesh_index,
                        batch.texture_id,
                        batch.texture_id,
                        tex_idx.map(|v| v.to_string()).unwrap_or("OOB".into()),
                        ty.map(|v| v.to_string()).unwrap_or("OOB".into()),
                        fdid.map(|v| v.to_string()).unwrap_or("OOB".into()),
                    );
                }
            }
        }
    }

}
