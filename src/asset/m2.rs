use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

/// Convert WoW coordinate (X-right, Y-forward, Z-up) to Bevy (X-right, Y-up, Z-back).
fn wow_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, z, -y]
}

/// A region overlay to composite onto the base texture.
pub struct TextureOverlay {
    pub fdid: u32,
    pub x: u32,
    pub y: u32,
}

pub struct M2RenderBatch {
    pub mesh: Mesh,
    /// None = runtime-resolved texture, use placeholder color.
    pub texture_fdid: Option<u32>,
    /// Region overlays composited onto the base texture (e.g. underwear on body skin).
    pub overlays: Vec<TextureOverlay>,
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
    mesh_part_id: u16,
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
            mesh_part_id: read_u16(data, base)?,
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
        1 => Some(120191), // body skin (humanmaleskin00_00, 512x512)
        _ => None,
    }
}

/// Default underwear overlay for body skin (type 1) batches.
/// Region coords from CharComponentTextureSections layout 153 (512x512).
const UNDERWEAR_FDID: u32 = 120181; // humanmalenakedpelvisskin00_00, 256x128
const UNDERWEAR_REGION: (u32, u32, u32, u32) = (256, 192, 256, 128); // LEG_UPPER

// Scalp/hair textures (HD full-body overlays, 512x512 with alpha).
// These are blitted at (0,0) — only the face/scalp region has pixel data.
const SCALP_UPPER_FDID: u32 = 1043094; // scalpupperhair00_00_hd, 512x512
const SCALP_LOWER_FDID: u32 = 1042989; // faciallowerhair00_00_hd, 512x512

/// Return body skin overlays: underwear + scalp hair textures.
fn body_skin_overlays(
    unit: &M2TextureUnit,
    tex_lookup: &[u16],
    tex_types: &[u32],
) -> Vec<TextureOverlay> {
    let Some(&lookup_val) = tex_lookup.get(unit.texture_id as usize) else {
        return Vec::new();
    };
    let ty = tex_types.get(lookup_val as usize).copied().unwrap_or(0);
    if ty != 1 {
        return Vec::new();
    }
    let (x, y, _, _) = UNDERWEAR_REGION;
    vec![
        TextureOverlay { fdid: UNDERWEAR_FDID, x, y },
        TextureOverlay { fdid: SCALP_UPPER_FDID, x: 0, y: 0 },
        TextureOverlay { fdid: SCALP_LOWER_FDID, x: 0, y: 0 },
    ]
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
        if fdid != 0 {
            return Some(fdid);
        }
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

/// Default geoset visibility: base skin + a hair style + x01 variant per group + ears override.
/// Group 0: geoset 0 (base skin) + geoset 5 (hair style placeholder, 1 is bald).
fn default_geoset_visible(mesh_part_id: u16) -> bool {
    if mesh_part_id == 0 || mesh_part_id == 5 {
        return true;
    }
    // Groups 1+: first variant (x01) is default for each group
    if mesh_part_id > 100 && mesh_part_id % 100 == 1 {
        return true;
    }
    // CharacterDefaultsGeosetModifier: ears default to variant 2
    mesh_part_id == 702
}

/// Build per-batch meshes from skin submesh/batch data, filtering by geoset visibility.
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
        if !default_geoset_visible(sub.mesh_part_id) {
            continue;
        }
        let mesh = build_batch_mesh(vertices, &skin.lookup, &skin.indices, sub);
        let texture_fdid = resolve_batch_texture(unit, tex_lookup, tex_types, txid);
        let overlays = body_skin_overlays(unit, tex_lookup, tex_types);
        batches.push(M2RenderBatch { mesh, texture_fdid, overlays });
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
            overlays: Vec::new(),
        }],
    })
}

#[cfg(test)]
#[path = "m2_tests.rs"]
mod tests;
