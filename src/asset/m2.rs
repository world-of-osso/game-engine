use std::path::Path;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};

/// Convert WoW coordinate (X-right, Y-forward, Z-up) to Bevy (X-right, Y-up, Z-back).
pub fn wow_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, z, -y]
}

/// How to scale a texture overlay before blitting.
#[derive(Clone, Copy)]
pub enum OverlayScale {
    /// No scaling.
    None,
    /// 2x both dimensions (nearest-neighbor).
    Uniform2x,
}

/// A region overlay to composite onto the base texture.
pub struct TextureOverlay {
    pub fdid: u32,
    pub x: u32,
    pub y: u32,
    pub scale: OverlayScale,
}

pub struct M2RenderBatch {
    pub mesh: Mesh,
    /// None = runtime-resolved texture, use placeholder color.
    pub texture_fdid: Option<u32>,
    /// Region overlays composited onto the base texture (e.g. underwear on body skin).
    pub overlays: Vec<TextureOverlay>,
    /// M2Material flags for this batch (0x04 = two-sided, 0x01 = unlit).
    pub render_flags: u16,
    /// Blending mode (0=opaque, 1=alpha_key, 2=alpha, 4=add, etc.).
    pub blend_mode: u16,
}

pub struct M2Model {
    pub batches: Vec<M2RenderBatch>,
    pub bones: Vec<super::m2_anim::M2Bone>,
    pub sequences: Vec<super::m2_anim::M2AnimSequence>,
    pub bone_tracks: Vec<super::m2_anim::BoneAnimTracks>,
    pub global_sequences: Vec<u32>,
}

struct M2Vertex {
    position: [f32; 3],
    bone_weights: [u8; 4],
    bone_indices: [u8; 4],
    normal: [f32; 3],
    tex_coords: [f32; 2],
}

struct M2Submesh {
    mesh_part_id: u16,
    vertex_start: u16,
    vertex_count: u16,
    triangle_start: u32,  // computed from cumulative index counts to avoid u16 overflow
    triangle_count: u16,
}

struct M2TextureUnit {
    submesh_index: u16,
    /// Index into the MD20 textureLookup table.
    texture_id: u16,
    /// Index into the MD20 materials (render flags) table.
    render_flags_index: u16,
}

/// Parsed M2Material entry (4 bytes: flags u16 + blending_mode u16).
struct M2Material {
    flags: u16,
    blend_mode: u16,
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
    skid: Option<u32>,
    sfid: Vec<u32>,
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

/// Parse top-level chunks, extracting MD21 (MD20 payload), optional TXID, and optional SKID.
fn parse_chunks(data: &[u8]) -> Result<M2Chunks<'_>, String> {
    let mut md20 = None;
    let mut txid = None;
    let mut skid = None;
    let mut sfid = Vec::new();
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
            b"SKID" if size >= 4 => skid = Some(read_u32(data, off + 8)?),
            b"SFID" => sfid = parse_sfid(&data[off + 8..end]),
            _ => {}
        }
        off = end;
    }
    Ok(M2Chunks {
        md20: md20.ok_or("No MD21 chunk found")?,
        txid,
        skid,
        sfid,
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
        let bw_slice = md20.get(base + 12..base + 16).ok_or_else(|| format!("Vertex {i} bone_weights out of bounds at {base:#x}"))?;
        let bone_weights: [u8; 4] = bw_slice.try_into().unwrap();
        let bi_slice = md20.get(base + 16..base + 20).ok_or_else(|| format!("Vertex {i} bone_indices out of bounds at {base:#x}"))?;
        let bone_indices: [u8; 4] = bi_slice.try_into().unwrap();
        vertices.push(M2Vertex {
            position: [
                read_f32(md20, base)?,
                read_f32(md20, base + 4)?,
                read_f32(md20, base + 8)?,
            ],
            bone_weights,
            bone_indices,
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
    // bones at offset 20 (not used — vertex bone indices are already global)
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
    let mut cumulative_index = 0u32;
    for i in 0..count {
        let base = offset + i * 48;
        if base + 48 > data.len() {
            return Err(format!("Submesh {i} out of bounds at {base:#x}"));
        }
        let index_count = read_u16(data, base + 10)?;
        subs.push(M2Submesh {
            mesh_part_id: read_u16(data, base)?,
            vertex_start: read_u16(data, base + 4)?,
            vertex_count: read_u16(data, base + 6)?,
            triangle_start: cumulative_index,
            triangle_count: index_count,
        });
        cumulative_index += index_count as u32;
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
            render_flags_index: read_u16(data, base + 10)?,
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

/// Parse M2Material entries (flags u16 + blend_mode u16) at MD20 offset 0x70.
fn parse_materials(md20: &[u8]) -> Result<Vec<M2Material>, String> {
    if md20.len() < 0x78 { return Ok(Vec::new()); }
    let count = read_u32(md20, 0x70)? as usize;
    let offset = read_u32(md20, 0x74)? as usize;
    let mut mats = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 4;
        mats.push(M2Material { flags: read_u16(md20, base)?, blend_mode: read_u16(md20, base + 2)? });
    }
    Ok(mats)
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
/// HD models use higher-resolution textures with different layouts.
fn default_fdid_for_type(ty: u32, is_hd: bool) -> Option<u32> {
    match (ty, is_hd) {
        (1, true) => Some(1027767),   // body skin HD (humanmaleskin00_00_hd, 1024x512)
        (1, false) => Some(120191),   // body skin SD (humanmaleskin00_00, 512x512)
        (19, _) => Some(3484643),     // eye color (eyes00_00, default human male)
        _ => None,
    }
}

/// HD face texture — base for type-6 head atlas + composited onto body atlas FACE_UPPER region.
const HD_FACE_FDID: u32 = 1027494; // humanmalefaceupper00_00_hd, 512x512

/// HD scalp hair overlay — composited on top of face texture for type-6 geosets.
/// From DB2: ChrCustomizationMaterial TargetID=11 (ScalpUpperHair), Peasant style + color 0.
const HD_SCALP_HAIR_FDID: u32 = 1043094; // scalpupperhair00_00_hd, 512x512

/// SD underwear overlay (layout 153, 512x512 body texture).
const UNDERWEAR_SD_FDID: u32 = 120181; // humanmalenakedpelvisskin00_00, 256x128
const UNDERWEAR_SD_POS: (u32, u32) = (256, 192); // TORSO_UPPER region

/// HD underwear overlay (1024x512 body texture, same region coords as SD).
const UNDERWEAR_HD_FDID: u32 = 1027743; // humanmalenakedpelvisskin00_00_hd, 256x128
const UNDERWEAR_HD_POS: (u32, u32) = (256, 192); // TORSO_UPPER region (same as SD)

// Standard-def scalp/hair textures (need 2x scale to match 512x512 body).
// Region coords from CharComponentTextureSections layout 153.
const SCALP_UPPER_FDID: u32 = 120233; // scalpupperhair00_00, 128x32 → 2x = 256x64
const SCALP_UPPER_REGION: (u32, u32) = (0, 320); // FACE_UPPER
const SCALP_LOWER_FDID: u32 = 119383; // faciallowerhair00_00, 128x64 → 2x = 256x128
const SCALP_LOWER_REGION: (u32, u32) = (0, 384); // FACE_LOWER

/// Return body skin overlays: underwear + scalp hair textures.
/// HD models have separate face geometry with dedicated textures, so scalp overlays
/// are only composited for legacy models (which bake scalp into the body texture).
/// HD body texture is 1024x512 (2x wider, same height) — underwear needs Width2x scaling.
fn body_skin_overlays(
    unit: &M2TextureUnit,
    tex_lookup: &[u16],
    tex_types: &[u32],
    is_hd: bool,
) -> Vec<TextureOverlay> {
    let Some(&lookup_val) = tex_lookup.get(unit.texture_id as usize) else {
        return Vec::new();
    };
    let ty = tex_types.get(lookup_val as usize).copied().unwrap_or(0);
    if ty != 1 {
        return Vec::new();
    }
    if is_hd {
        let (x, y) = UNDERWEAR_HD_POS;
        return vec![
            // Face texture composited onto body atlas right half (FACE_UPPER region)
            TextureOverlay { fdid: HD_FACE_FDID, x: 512, y: 0, scale: OverlayScale::None },
            TextureOverlay { fdid: UNDERWEAR_HD_FDID, x, y, scale: OverlayScale::None },
        ];
    }
    let (x, y) = UNDERWEAR_SD_POS;
    vec![
        TextureOverlay { fdid: UNDERWEAR_SD_FDID, x, y, scale: OverlayScale::None },
        TextureOverlay { fdid: SCALP_UPPER_FDID, x: SCALP_UPPER_REGION.0, y: SCALP_UPPER_REGION.1, scale: OverlayScale::Uniform2x },
        TextureOverlay { fdid: SCALP_LOWER_FDID, x: SCALP_LOWER_REGION.0, y: SCALP_LOWER_REGION.1, scale: OverlayScale::Uniform2x },
    ]
}

/// batch.texture_id -> textureLookup -> textures[].type -> TXID[]. Type!=0 uses defaults.
fn resolve_batch_texture(
    unit: &M2TextureUnit,
    tex_lookup: &[u16],
    tex_types: &[u32],
    txid: &[u32],
    is_hd: bool,
) -> Option<u32> {
    let tex_idx = *tex_lookup.get(unit.texture_id as usize)? as usize;
    let ty = *tex_types.get(tex_idx)?;
    if ty == 0 {
        let fdid = *txid.get(tex_idx)?;
        if fdid != 0 {
            return Some(fdid);
        }
    }
    default_fdid_for_type(ty, is_hd)
}

/// Get the texture type for a batch (through the lookup chain).
fn batch_texture_type(unit: &M2TextureUnit, tex_lookup: &[u16], tex_types: &[u32]) -> Option<u32> {
    let tex_idx = *tex_lookup.get(unit.texture_id as usize)? as usize;
    tex_types.get(tex_idx).copied()
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

struct VertexBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    joint_indices: Vec<[u16; 4]>,
    joint_weights: Vec<[f32; 4]>,
}

/// Collect vertex attribute buffers for the submesh vertex range via the lookup table.
fn collect_submesh_vertices(
    vertices: &[M2Vertex],
    lookup: &[u16],
    vstart: usize,
    vcount: usize,
) -> VertexBuffers {
    let mut buf = VertexBuffers {
        positions: Vec::with_capacity(vcount),
        normals: Vec::with_capacity(vcount),
        uvs: Vec::with_capacity(vcount),
        joint_indices: Vec::with_capacity(vcount),
        joint_weights: Vec::with_capacity(vcount),
    };
    for i in 0..vcount {
        let global_idx = lookup.get(vstart + i).copied().unwrap_or(0) as usize;
        let Some(v) = vertices.get(global_idx) else { continue };
        buf.positions.push(wow_to_bevy(v.position[0], v.position[1], v.position[2]));
        buf.normals.push(wow_to_bevy(v.normal[0], v.normal[1], v.normal[2]));
        buf.uvs.push(v.tex_coords);
        buf.joint_indices.push([
            v.bone_indices[0] as u16, v.bone_indices[1] as u16,
            v.bone_indices[2] as u16, v.bone_indices[3] as u16,
        ]);
        buf.joint_weights.push([
            v.bone_weights[0] as f32 / 255.0, v.bone_weights[1] as f32 / 255.0,
            v.bone_weights[2] as f32 / 255.0, v.bone_weights[3] as f32 / 255.0,
        ]);
    }
    buf
}
/// Remap raw skin indices to submesh-local indices (relative to vstart).
fn remap_submesh_indices(indices: &[u16], tstart: usize, tcount: usize, vstart: usize) -> Vec<u16> {
    (0..tcount)
        .filter_map(|j| indices.get(tstart + j))
        .map(|&idx| (idx as usize).saturating_sub(vstart) as u16)
        .collect()
}

/// Build a Bevy Mesh for one submesh: compact vertex buffer + remapped indices.
fn build_batch_mesh(
    vertices: &[M2Vertex],
    lookup: &[u16],
    indices: &[u16],
    sub: &M2Submesh,
    has_bones: bool,
) -> Mesh {
    let vstart = sub.vertex_start as usize;
    let buf = collect_submesh_vertices(vertices, lookup, vstart, sub.vertex_count as usize);
    let local_indices = remap_submesh_indices(
        indices, sub.triangle_start as usize, sub.triangle_count as usize, vstart,
    );

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, buf.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, buf.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, buf.uvs);
    if has_bones {
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(buf.joint_indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, buf.joint_weights);
    }
    mesh.insert_indices(Indices::U16(local_indices));
    mesh
}

/// Build a Bevy Mesh from all vertices + resolved index list (fallback path).
fn build_mesh(vertices: &[M2Vertex], indices: Vec<u16>) -> Mesh {
    let identity_lookup: Vec<u16> = (0..vertices.len() as u16).collect();
    let sub = M2Submesh {
        mesh_part_id: 0, vertex_start: 0,
        vertex_count: vertices.len() as u16,
        triangle_start: 0, triangle_count: indices.len() as u16,
    };
    build_batch_mesh(vertices, &identity_lookup, &indices, &sub, true)
}

struct SkelData {
    bones: Vec<super::m2_anim::M2Bone>,
    sequences: Vec<super::m2_anim::M2AnimSequence>,
    bone_tracks: Vec<super::m2_anim::BoneAnimTracks>,
    global_sequences: Vec<u32>,
}

/// Load animation data from an external .skel file (SKS1 + SKB1 chunks).
///
/// .skel is chunked binary (same `[tag 4B][size 4B][data]` format as M2).
/// SKS1 contains sequences + global sequences. SKB1 contains bones + animation tracks.
/// All M2Array offsets within each chunk are relative to the chunk data start.
fn load_skel_data(skel_path: &Path) -> Result<SkelData, String> {
    let data = std::fs::read(skel_path)
        .map_err(|e| format!("Failed to read .skel file: {e}"))?;
    let mut result = SkelData {
        bones: Vec::new(),
        sequences: Vec::new(),
        bone_tracks: Vec::new(),
        global_sequences: Vec::new(),
    };
    let mut off = 0;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = read_u32(&data, off + 4)? as usize;
        let end = off + 8 + size;
        if end > data.len() {
            break;
        }
        let chunk = &data[off + 8..end];
        match tag {
            b"SKS1" if chunk.len() >= 24 => {
                parse_sks1_chunk(chunk, &mut result)?;
            }
            b"SKB1" if chunk.len() >= 16 => {
                parse_skb1_chunk(chunk, &mut result)?;
            }
            _ => {}
        }
        off = end;
    }
    Ok(result)
}

/// Parse SKS1 chunk: global_loops M2Array at 0x00, sequences M2Array at 0x08.
fn parse_sks1_chunk(chunk: &[u8], result: &mut SkelData) -> Result<(), String> {
    let gl_count = read_u32(chunk, 0)? as usize;
    let gl_offset = read_u32(chunk, 4)? as usize;
    result.global_sequences =
        super::m2_anim::parse_global_sequences_at(chunk, gl_offset, gl_count)?;

    let seq_count = read_u32(chunk, 8)? as usize;
    let seq_offset = read_u32(chunk, 12)? as usize;
    result.sequences = super::m2_anim::parse_sequences_at(chunk, seq_offset, seq_count)?;
    Ok(())
}

/// Parse SKB1 chunk: bones M2Array at 0x00 (bones + animation tracks share the same data).
fn parse_skb1_chunk(chunk: &[u8], result: &mut SkelData) -> Result<(), String> {
    let bone_count = read_u32(chunk, 0)? as usize;
    let bone_offset = read_u32(chunk, 4)? as usize;
    result.bones = super::m2_anim::parse_bones_at(chunk, bone_offset, bone_count)?;
    result.bone_tracks =
        super::m2_anim::parse_bone_animations_at(chunk, bone_offset, bone_count)?;
    Ok(())
}

/// Fallback: load bones/sequences/tracks/global_sequences from the MD20 blob.
fn load_anim_from_md20(md20: &[u8]) -> SkelData {
    SkelData {
        bones: super::m2_anim::parse_bones(md20).unwrap_or_default(),
        sequences: super::m2_anim::parse_sequences(md20).unwrap_or_default(),
        bone_tracks: super::m2_anim::parse_bone_animations(md20).unwrap_or_default(),
        global_sequences: super::m2_anim::parse_global_sequences(md20).unwrap_or_default(),
    }
}

/// Parse SFID chunk: array of u32 skin file FDIDs.
fn parse_sfid(data: &[u8]) -> Vec<u32> {
    data.chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

/// Load skin data, trying SFID FDID first, then name-based fallback.
fn load_skin_data(m2_path: &Path, sfid: &[u32]) -> Option<SkinData> {
    // Try SFID FDID (LOD 0) first — works for FDID-named files
    if let Some(&fdid) = sfid.first() {
        let skin_path = m2_path.with_file_name(format!("{fdid}.skin"));
        if let Ok(data) = std::fs::read(&skin_path) {
            return parse_skin_full(&data).ok();
        }
    }
    // Fallback: name-based (e.g. humanmale_hd00.skin)
    let stem = m2_path.file_stem()?.to_str()?;
    let skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
    let data = std::fs::read(&skin_path).ok()?;
    parse_skin_full(&data).ok()
}

/// Default geoset visibility: base skin + bald cap + hair style + default variants + ears.
/// Group 0: geoset 0 (body), 1 (bald cap closes head), 5 (hair style on top).
/// Groups 1-3: facial hair variant 2 (102, 202, 302 — ties/accessories).
/// Groups 4+: first variant (x01) is default per group.
fn default_geoset_visible(mesh_part_id: u16) -> bool {
    let group = mesh_part_id / 100;
    let variant = mesh_part_id % 100;
    match group {
        // Group 0: body (0), bald cap (1), hair style (5)
        0 => matches!(mesh_part_id, 0 | 1 | 5),
        // Groups 1-3: facial hair — variant 2 is default (accessories/ties)
        1..=3 => variant == 2,
        // Group 7: ears — variants 1 and 2 both visible
        7 => matches!(variant, 1 | 2),
        // Group 32: face — all variants visible (nose, mouth, cheeks are separate)
        32 => variant >= 1,
        // All other groups: variant 1 is default
        _ => variant == 1,
    }
}

fn resolve_batch_fdid_and_overlays(
    unit: &M2TextureUnit,
    tex_lookup: &[u16],
    tex_types: &[u32],
    txid: &[u32],
    is_hd: bool,
) -> (Option<u32>, Vec<TextureOverlay>) {
    let tex_type = batch_texture_type(unit, tex_lookup, tex_types);
    let mut fdid = resolve_batch_texture(unit, tex_lookup, tex_types, txid, is_hd);
    if is_hd && fdid.is_none() && tex_type == Some(6) {
        fdid = Some(HD_SCALP_HAIR_FDID);
    }
    let mut overlays = body_skin_overlays(unit, tex_lookup, tex_types, is_hd);
    (fdid, overlays)
}

fn build_batched_model(
    vertices: &[M2Vertex],
    skin: &SkinData,
    materials: &[M2Material],
    tex_lookup: &[u16],
    tex_types: &[u32],
    txid: &[u32],
    has_bones: bool,
    is_hd: bool,
) -> Result<Vec<M2RenderBatch>, String> {
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
        let mesh = build_batch_mesh(vertices, &skin.lookup, &skin.indices, sub, has_bones);
        let (texture_fdid, overlays) = resolve_batch_fdid_and_overlays(unit, tex_lookup, tex_types, txid, is_hd);
        let mat = materials.get(unit.render_flags_index as usize);
        let render_flags = mat.map(|m| m.flags).unwrap_or(0);
        let blend_mode = mat.map(|m| m.blend_mode).unwrap_or(0);
        batches.push(M2RenderBatch { mesh, texture_fdid, overlays, render_flags, blend_mode });
    }
    Ok(batches)
}

/// Load animation data from .skel (if SKID chunk present) or from MD20 inline.
fn load_anim_data(path: &Path, chunks: &M2Chunks<'_>) -> SkelData {
    if chunks.skid.is_some() {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let skel_path = path.with_file_name(format!("{stem}.skel"));
        match load_skel_data(&skel_path) {
            Ok(s) => return s,
            Err(e) => eprintln!("Failed to load .skel: {e}"),
        }
    }
    load_anim_from_md20(chunks.md20)
}

/// Load an M2 model file (chunked MD21 format) and return per-batch meshes + textures.
pub fn load_m2(path: &Path) -> Result<M2Model, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read M2 file: {e}"))?;
    let chunks = parse_chunks(&data)?;
    let vertices = parse_vertices(chunks.md20)?;
    let tex_types = parse_texture_types(chunks.md20)?;
    let tex_lookup = parse_texture_lookup(chunks.md20)?;
    let txid = chunks.txid.map(parse_txid).unwrap_or_default();
    let anim = load_anim_data(path, &chunks);
    let skin = load_skin_data(path, &chunks.sfid);
    let materials = parse_materials(chunks.md20)?;

    let batches = if let Some(ref skin) = skin
        && !skin.submeshes.is_empty()
        && !skin.batches.is_empty()
    {
        build_batched_model(&vertices, skin, &materials, &tex_lookup, &tex_types, &txid, !anim.bones.is_empty(), chunks.skid.is_some())?
    } else {
        let indices = match skin { Some(s) => resolve_indices(&s.lookup, &s.indices), None => (0..vertices.len() as u16).collect() };
        let fdid = first_hardcoded_texture(&tex_types, &txid);
        vec![M2RenderBatch { mesh: build_mesh(&vertices, indices), texture_fdid: fdid, overlays: Vec::new(), render_flags: 0, blend_mode: 0 }]
    };

    Ok(M2Model {
        batches,
        bones: anim.bones,
        sequences: anim.sequences,
        bone_tracks: anim.bone_tracks,
        global_sequences: anim.global_sequences,
    })
}

#[cfg(test)]
#[path = "m2_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "m2_debug_tests.rs"]
mod debug_tests;

#[cfg(test)]
#[path = "m2_jaw_debug_tests.rs"]
mod jaw_debug_tests;
