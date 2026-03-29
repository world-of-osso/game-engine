use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use super::m2_texture;
#[cfg(test)]
pub use m2_texture::{first_hardcoded_texture, resolve_batch_texture};

/// Convert WoW coordinate (X-right, Y-forward, Z-up) to Bevy (X-right, Y-up, Z-back).
pub fn wow_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, z, -y]
}

/// How to scale a texture overlay before blitting.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlayScale {
    None,
    Uniform2x,
}

/// A region overlay to composite onto the base texture.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TextureOverlay {
    pub fdid: u32,
    pub x: u32,
    pub y: u32,
    pub scale: OverlayScale,
}

#[derive(Clone)]
pub struct M2RenderBatch {
    pub mesh: Mesh,
    pub texture_fdid: Option<u32>,
    pub texture_2_fdid: Option<u32>,
    pub texture_type: Option<u32>,
    pub overlays: Vec<TextureOverlay>,
    pub render_flags: u16,
    pub blend_mode: u16,
    pub transparency: f32,
    pub texture_anim: Option<super::m2_anim::AnimTrack<[f32; 3]>>,
    pub texture_anim_2: Option<super::m2_anim::AnimTrack<[f32; 3]>>,
    pub use_uv_2_1: bool,
    pub use_uv_2_2: bool,
    pub use_env_map_2: bool,
    pub shader_id: u16,
    pub texture_count: u16,
    /// M2 submesh mesh_part_id (geoset group*100 + variant). Used for geoset visibility.
    pub mesh_part_id: u16,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct M2Model {
    pub batches: Vec<M2RenderBatch>,
    pub bones: Vec<super::m2_anim::M2Bone>,
    pub sequences: Vec<super::m2_anim::M2AnimSequence>,
    pub bone_tracks: Vec<super::m2_anim::BoneAnimTracks>,
    pub global_sequences: Vec<u32>,
    pub particle_emitters: Vec<super::m2_particle::M2ParticleEmitter>,
    pub attachments: Vec<super::m2_attach::M2Attachment>,
    pub attachment_lookup: Vec<i16>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ModelCacheKey {
    path: PathBuf,
    skin_fdids: [u32; 3],
}

static M2_MODEL_CACHE: OnceLock<Mutex<HashMap<ModelCacheKey, Result<M2Model, String>>>> =
    OnceLock::new();

#[path = "m2_cache_stats.rs"]
mod cache_stats;
pub use cache_stats::{ModelCacheStats, model_cache_stats};

struct M2Vertex {
    position: [f32; 3],
    bone_weights: [u8; 4],
    bone_indices: [u8; 4],
    normal: [f32; 3],
    tex_coords: [f32; 2],
    tex_coords_2: [f32; 2],
}

struct M2Submesh {
    mesh_part_id: u16,
    vertex_start: u16,
    vertex_count: u16,
    triangle_start: u32,
    triangle_count: u16,
}

pub struct M2TextureUnit {
    pub flags: u8,
    pub priority_plane: i8,
    pub shader_id: u16,
    pub submesh_index: u16,
    pub color_index: i16,
    pub texture_id: u16,
    pub render_flags_index: u16,
    pub material_layer: u16,
    pub texture_count: u16,
    pub texture_coord_index: u16,
    pub transparency_index: u16,
    pub texture_animation_id: u16,
}

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
    ska1: Option<&'a [u8]>,
    txid: Option<&'a [u8]>,
    skid: Option<u32>,
    sfid: Vec<u32>,
}

/// Grouped texture lookup tables passed to batched model building.
pub struct TextureTables<'a> {
    pub tex_lookup: &'a [u16],
    pub tex_types: &'a [u32],
    pub txid: &'a [u32],
    pub skin_fdids: &'a [u32; 3],
}

pub(crate) fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_u32 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

pub(crate) fn read_f32(data: &[u8], off: usize) -> Result<f32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_f32 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

pub(crate) fn read_u16(data: &[u8], off: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_u16 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

fn parse_chunks(data: &[u8]) -> Result<M2Chunks<'_>, String> {
    let mut md20 = None;
    let mut ska1 = None;
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
            b"SKA1" => ska1 = Some(&data[off + 8..end]),
            b"TXID" => txid = Some(&data[off + 8..end]),
            b"SKID" if size >= 4 => skid = Some(read_u32(data, off + 8)?),
            b"SFID" => sfid = parse_sfid(&data[off + 8..end]),
            _ => {}
        }
        off = end;
    }
    Ok(M2Chunks {
        md20: md20.ok_or("No MD21 chunk found")?,
        ska1,
        txid,
        skid,
        sfid,
    })
}

fn parse_one_vertex(md20: &[u8], i: usize, base: usize) -> Result<M2Vertex, String> {
    let bw = md20
        .get(base + 12..base + 16)
        .ok_or_else(|| format!("Vertex {i} bone_weights out of bounds at {base:#x}"))?;
    let bi = md20
        .get(base + 16..base + 20)
        .ok_or_else(|| format!("Vertex {i} bone_indices out of bounds at {base:#x}"))?;
    Ok(M2Vertex {
        position: [
            read_f32(md20, base)?,
            read_f32(md20, base + 4)?,
            read_f32(md20, base + 8)?,
        ],
        bone_weights: bw.try_into().unwrap(),
        bone_indices: bi.try_into().unwrap(),
        normal: [
            read_f32(md20, base + 20)?,
            read_f32(md20, base + 24)?,
            read_f32(md20, base + 28)?,
        ],
        tex_coords: [read_f32(md20, base + 32)?, read_f32(md20, base + 36)?],
        tex_coords_2: [read_f32(md20, base + 40)?, read_f32(md20, base + 44)?],
    })
}

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
        vertices.push(parse_one_vertex(md20, i, base)?);
    }
    Ok(vertices)
}

fn parse_skin_full(data: &[u8]) -> Result<SkinData, String> {
    if data.len() < 44 || &data[0..4] != b"SKIN" {
        return Err("Invalid skin file (bad magic)".into());
    }
    let lookup = parse_u16_array(data, 4)?;
    let indices = parse_u16_array(data, 12)?;
    let sub_count = read_u32(data, 28)? as usize;
    let sub_offset = read_u32(data, 32)? as usize;
    let batch_count = read_u32(data, 36)? as usize;
    let batch_offset = read_u32(data, 40)? as usize;
    let submeshes = parse_submeshes(data, sub_offset, sub_count)?;
    let batches = parse_texture_units(data, batch_offset, batch_count)?;
    Ok(SkinData {
        lookup,
        indices,
        submeshes,
        batches,
    })
}

fn parse_u16_array(data: &[u8], header_off: usize) -> Result<Vec<u16>, String> {
    let count = read_u32(data, header_off)? as usize;
    let offset = read_u32(data, header_off + 4)? as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        out.push(read_u16(data, offset + i * 2)?);
    }
    Ok(out)
}

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

fn parse_texture_units(
    data: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<M2TextureUnit>, String> {
    let mut units = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 24;
        if base + 24 > data.len() {
            return Err(format!("TextureUnit {i} out of bounds at {base:#x}"));
        }
        units.push(M2TextureUnit {
            flags: data[base],
            priority_plane: data[base + 1] as i8,
            shader_id: read_u16(data, base + 2)?,
            submesh_index: read_u16(data, base + 4)?,
            color_index: read_u16(data, base + 8)? as i16,
            render_flags_index: read_u16(data, base + 10)?,
            material_layer: read_u16(data, base + 12)?,
            texture_count: read_u16(data, base + 14)?,
            texture_id: read_u16(data, base + 16)?,
            texture_coord_index: read_u16(data, base + 18)?,
            transparency_index: read_u16(data, base + 20)?,
            texture_animation_id: read_u16(data, base + 22)?,
        });
    }
    Ok(units)
}

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

fn parse_materials(md20: &[u8]) -> Result<Vec<M2Material>, String> {
    if md20.len() < 0x78 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x70)? as usize;
    let offset = read_u32(md20, 0x74)? as usize;
    let mut mats = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 4;
        mats.push(M2Material {
            flags: read_u16(md20, base)?,
            blend_mode: read_u16(md20, base + 2)?,
        });
    }
    Ok(mats)
}

fn parse_txid(data: &[u8]) -> Vec<u32> {
    (0..data.len() / 4)
        .filter_map(|i| read_u32(data, i * 4).ok())
        .collect()
}

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

fn parse_texture_unit_lookup(md20: &[u8]) -> Result<Vec<i16>, String> {
    if md20.len() < 0x90 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x88)? as usize;
    let offset = read_u32(md20, 0x8C)? as usize;
    let mut lookup = Vec::with_capacity(count);
    for i in 0..count {
        let off = offset + i * 2;
        let bytes: [u8; 2] = md20
            .get(off..off + 2)
            .ok_or_else(|| format!("texture unit lookup {i} out of bounds at {off:#x}"))?
            .try_into()
            .unwrap();
        lookup.push(i16::from_le_bytes(bytes));
    }
    Ok(lookup)
}

fn parse_i16_lookup(md20: &[u8], header_off: usize, label: &str) -> Result<Vec<i16>, String> {
    if md20.len() < header_off + 8 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, header_off)? as usize;
    let offset = read_u32(md20, header_off + 4)? as usize;
    let mut lookup = Vec::with_capacity(count);
    for i in 0..count {
        let off = offset + i * 2;
        let bytes: [u8; 2] = md20
            .get(off..off + 2)
            .ok_or_else(|| format!("{label} lookup {i} out of bounds at {off:#x}"))?
            .try_into()
            .unwrap();
        lookup.push(i16::from_le_bytes(bytes));
    }
    Ok(lookup)
}

fn parse_transparency_lookup(md20: &[u8]) -> Result<Vec<i16>, String> {
    parse_i16_lookup(md20, 0x90, "transparency")
}

fn parse_uv_animation_lookup(md20: &[u8]) -> Result<Vec<i16>, String> {
    parse_i16_lookup(md20, 0x98, "uv animation")
}

fn resolve_indices(lookup: &[u16], indices: &[u16]) -> Vec<u16> {
    indices
        .iter()
        .filter_map(|&idx| lookup.get(idx as usize).copied())
        .collect()
}

// --- Mesh building ---

struct VertexBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    uvs2: Vec<[f32; 2]>,
    joint_indices: Vec<[u16; 4]>,
    joint_weights: Vec<[f32; 4]>,
}

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
        uvs2: Vec::with_capacity(vcount),
        joint_indices: Vec::with_capacity(vcount),
        joint_weights: Vec::with_capacity(vcount),
    };
    for i in 0..vcount {
        let global_idx = lookup.get(vstart + i).copied().unwrap_or(0) as usize;
        let Some(v) = vertices.get(global_idx) else {
            continue;
        };
        buf.positions
            .push(wow_to_bevy(v.position[0], v.position[1], v.position[2]));
        buf.normals
            .push(wow_to_bevy(v.normal[0], v.normal[1], v.normal[2]));
        buf.uvs.push(v.tex_coords);
        buf.uvs2.push(v.tex_coords_2);
        buf.joint_indices.push([
            v.bone_indices[0] as u16,
            v.bone_indices[1] as u16,
            v.bone_indices[2] as u16,
            v.bone_indices[3] as u16,
        ]);
        buf.joint_weights.push([
            v.bone_weights[0] as f32 / 255.0,
            v.bone_weights[1] as f32 / 255.0,
            v.bone_weights[2] as f32 / 255.0,
            v.bone_weights[3] as f32 / 255.0,
        ]);
    }
    buf
}

fn remap_submesh_indices(indices: &[u16], tstart: usize, tcount: usize, vstart: usize) -> Vec<u16> {
    (0..tcount)
        .filter_map(|j| indices.get(tstart + j))
        .map(|&idx| (idx as usize).saturating_sub(vstart) as u16)
        .collect()
}

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
        indices,
        sub.triangle_start as usize,
        sub.triangle_count as usize,
        vstart,
    );
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, buf.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, buf.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, buf.uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, buf.uvs2);
    if has_bones {
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(buf.joint_indices),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, buf.joint_weights);
    }
    mesh.insert_indices(Indices::U16(local_indices));
    mesh
}

fn build_mesh(vertices: &[M2Vertex], indices: Vec<u16>) -> Mesh {
    let identity_lookup: Vec<u16> = (0..vertices.len() as u16).collect();
    let sub = M2Submesh {
        mesh_part_id: 0,
        vertex_start: 0,
        vertex_count: vertices.len() as u16,
        triangle_start: 0,
        triangle_count: indices.len() as u16,
    };
    build_batch_mesh(vertices, &identity_lookup, &indices, &sub, true)
}

// --- Animation / skeleton loading ---

struct SkelData {
    bones: Vec<super::m2_anim::M2Bone>,
    sequences: Vec<super::m2_anim::M2AnimSequence>,
    bone_tracks: Vec<super::m2_anim::BoneAnimTracks>,
    global_sequences: Vec<u32>,
}

fn load_skel_data(skel_path: &Path) -> Result<SkelData, String> {
    let data = std::fs::read(skel_path).map_err(|e| format!("Failed to read .skel file: {e}"))?;
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
            b"SKS1" if chunk.len() >= 24 => parse_sks1_chunk(chunk, &mut result)?,
            b"SKB1" if chunk.len() >= 16 => parse_skb1_chunk(chunk, &mut result)?,
            _ => {}
        }
        off = end;
    }
    Ok(result)
}

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

fn parse_skb1_chunk(chunk: &[u8], result: &mut SkelData) -> Result<(), String> {
    let bone_count = read_u32(chunk, 0)? as usize;
    let bone_offset = read_u32(chunk, 4)? as usize;
    result.bones = super::m2_anim::parse_bones_at(chunk, bone_offset, bone_count)?;
    result.bone_tracks = super::m2_anim::parse_bone_animations_at(chunk, bone_offset, bone_count)?;
    Ok(())
}

fn load_anim_from_md20(md20: &[u8]) -> SkelData {
    SkelData {
        bones: super::m2_anim::parse_bones(md20).unwrap_or_default(),
        sequences: super::m2_anim::parse_sequences(md20).unwrap_or_default(),
        bone_tracks: super::m2_anim::parse_bone_animations(md20).unwrap_or_default(),
        global_sequences: super::m2_anim::parse_global_sequences(md20).unwrap_or_default(),
    }
}

fn load_anim_data(path: &Path, chunks: &M2Chunks<'_>) -> SkelData {
    if let Some(skel_fdid) = chunks.skid {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let skel_path = path.with_file_name(format!("{stem}.skel"));
        super::casc_resolver::ensure_file_at_path(skel_fdid, &skel_path);
        match load_skel_data(&skel_path) {
            Ok(s) => return s,
            Err(e) => eprintln!("Failed to load .skel: {e}"),
        }
    }
    load_anim_from_md20(chunks.md20)
}

fn parse_sfid(data: &[u8]) -> Vec<u32> {
    data.chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn load_skin_data(m2_path: &Path, sfid: &[u32]) -> Option<SkinData> {
    let stem = m2_path.file_stem()?.to_str()?;
    if let Some(&fdid) = sfid.first() {
        let canonical_skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
        if let Some(resolved_path) = super::casc_resolver::ensure_file_at_path(fdid, &canonical_skin_path)
            && let Ok(data) = std::fs::read(&resolved_path)
        {
            return parse_skin_full(&data).ok();
        }
        let numeric_skin_path = m2_path.with_file_name(format!("{fdid}.skin"));
        if let Some(resolved_path) = super::casc_resolver::ensure_file_at_path(fdid, &numeric_skin_path)
            && let Ok(data) = std::fs::read(&resolved_path)
        {
            return parse_skin_full(&data).ok();
        }
    }
    let skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
    let data = std::fs::read(&skin_path).ok()?;
    parse_skin_full(&data).ok()
}

/// Resolve the primary companion `.skin` file for an M2, extracting it from CASC if needed.
pub fn ensure_primary_skin_path(m2_path: &Path) -> Option<PathBuf> {
    let data = std::fs::read(m2_path).ok()?;
    let chunks = parse_chunks(&data).ok()?;
    let stem = m2_path.file_stem()?.to_str()?;
    if let Some(&fdid) = chunks.sfid.first() {
        let canonical_skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
        if let Some(path) = super::casc_resolver::ensure_file_at_path(fdid, &canonical_skin_path) {
            return Some(path);
        }
        let numeric_skin_path = m2_path.with_file_name(format!("{fdid}.skin"));
        return super::casc_resolver::ensure_file_at_path(fdid, &numeric_skin_path);
    }
    let skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
    skin_path.exists().then_some(skin_path)
}

/// Default geoset visibility for initial model display.
pub fn default_geoset_visible(mesh_part_id: u16) -> bool {
    let group = mesh_part_id / 100;
    let variant = mesh_part_id % 100;
    match group {
        0 => matches!(mesh_part_id, 0 | 1 | 5 | 16 | 17 | 27..=33),
        1..=3 => variant == 2,
        7 => matches!(variant, 1 | 2),
        32 => variant >= 1,
        _ => variant == 1,
    }
}

// --- Batch building (includes ALL submeshes for runtime geoset control) ---

#[allow(clippy::too_many_arguments)]
fn build_one_batch(
    vertices: &[M2Vertex],
    skin: &SkinData,
    materials: &[M2Material],
    tex: &TextureTables<'_>,
    color_tracks: &[super::m2_anim::ColorAnimTracks],
    transparencies: &[super::m2_anim::AnimTrack<i16>],
    transparency_lookup: &[i16],
    texture_animations: &[super::m2_anim::TextureAnimTracks],
    uv_animation_lookup: &[i16],
    texture_unit_lookup: &[i16],
    has_bones: bool,
    is_hd: bool,
    unit: &M2TextureUnit,
) -> Result<Option<M2RenderBatch>, String> {
    let sub_idx = unit.submesh_index as usize;
    if sub_idx >= skin.submeshes.len() {
        return Err(format!(
            "Batch submesh_index {sub_idx} >= submesh count {}",
            skin.submeshes.len()
        ));
    }
    let sub = &skin.submeshes[sub_idx];
    let mesh = build_batch_mesh(vertices, &skin.lookup, &skin.indices, sub, has_bones);
    let texture_type = m2_texture::batch_texture_type(unit, tex.tex_lookup, tex.tex_types);
    let (texture_fdid, mut texture_2_fdid, overlays) =
        m2_texture::resolve_batch_fdid_and_overlays(unit, tex, is_hd);
    if unit.shader_id & 0x7fff == 0 {
        texture_2_fdid = None;
    }
    let mat = materials.get(unit.render_flags_index as usize);
    let transparency_track = transparency_lookup
        .get(unit.transparency_index as usize)
        .copied()
        .unwrap_or(unit.transparency_index as i16);
    let transparency = transparencies
        .get(transparency_track.max(0) as usize)
        .and_then(|track| super::m2_anim::evaluate_i16_track(track, 0, 0))
        .map(|value| (value as f32 / 32767.0).clamp(0.0, 1.0))
        .unwrap_or(1.0);
    let color_opacity = usize::try_from(unit.color_index)
        .ok()
        .and_then(|idx| color_tracks.get(idx))
        .and_then(|tracks| super::m2_anim::evaluate_i16_track(&tracks.opacity, 0, 0))
        .map(|value| (value as f32 / 32767.0).clamp(0.0, 1.0))
        .unwrap_or(1.0);
    let transparency = transparency * color_opacity;
    if transparency <= 0.0 {
        return Ok(None);
    }
    let texture_anim = uv_animation_lookup
        .get(unit.texture_animation_id as usize)
        .copied()
        .and_then(|idx| usize::try_from(idx).ok())
        .and_then(|idx| texture_animations.get(idx))
        .map(|tracks| tracks.translation.clone());
    let texture_anim_2 = if unit.texture_count > 1 {
        uv_animation_lookup
            .get(unit.texture_animation_id.saturating_add(1) as usize)
            .copied()
            .and_then(|idx| usize::try_from(idx).ok())
            .and_then(|idx| texture_animations.get(idx))
            .map(|tracks| tracks.translation.clone())
    } else {
        None
    };
    // Retail chunked models sometimes omit the global texture-unit lookup table.
    // Fall back to UV1 only when it is meaningful, and infer environment maps
    // from known reflection textures when the lookup metadata is absent.
    let use_uv_2_1 = texture_unit_lookup
        .get(unit.texture_coord_index as usize)
        .copied()
        == Some(1);
    let (use_uv_2_2, use_env_map_2) = if unit.texture_count > 1 {
        if texture_unit_lookup.is_empty() {
            (
                mesh_has_meaningful_uv1(&mesh),
                texture_looks_like_environment_map(texture_2_fdid),
            )
        } else {
            let lookup = texture_unit_lookup
                .get(unit.texture_coord_index.saturating_add(1) as usize)
                .copied();
            (lookup == Some(1), lookup == Some(-1))
        }
    } else {
        (false, false)
    };
    Ok(Some(M2RenderBatch {
        mesh,
        texture_fdid,
        texture_2_fdid,
        texture_type,
        overlays,
        render_flags: mat.map(|m| m.flags).unwrap_or(0),
        blend_mode: mat.map(|m| m.blend_mode).unwrap_or(0),
        transparency,
        texture_anim,
        texture_anim_2,
        use_uv_2_1,
        use_uv_2_2,
        use_env_map_2,
        shader_id: unit.shader_id,
        texture_count: unit.texture_count,
        mesh_part_id: sub.mesh_part_id,
    }))
}

fn texture_looks_like_environment_map(fdid: Option<u32>) -> bool {
    let Some(path) = fdid.and_then(game_engine::listfile::lookup_fdid) else {
        return false;
    };
    let lower = path.to_ascii_lowercase();
    lower.contains("armorreflect") || lower.contains("_reflect") || lower.contains("envmap")
}

#[allow(clippy::too_many_arguments)]
fn build_batched_model(
    vertices: &[M2Vertex],
    skin: &SkinData,
    materials: &[M2Material],
    tex: &TextureTables<'_>,
    color_tracks: &[super::m2_anim::ColorAnimTracks],
    transparencies: &[super::m2_anim::AnimTrack<i16>],
    transparency_lookup: &[i16],
    texture_animations: &[super::m2_anim::TextureAnimTracks],
    uv_animation_lookup: &[i16],
    texture_unit_lookup: &[i16],
    has_bones: bool,
    is_hd: bool,
) -> Result<Vec<M2RenderBatch>, String> {
    let mut batches = Vec::with_capacity(skin.batches.len());
    for unit in &skin.batches {
        let batch = build_one_batch(
            vertices,
            skin,
            materials,
            tex,
            color_tracks,
            transparencies,
            transparency_lookup,
            texture_animations,
            uv_animation_lookup,
            texture_unit_lookup,
            has_bones,
            is_hd,
            unit,
        )?;
        if let Some(batch) = batch {
            batches.push(batch);
        }
    }
    Ok(batches)
}

pub(crate) fn mesh_has_meaningful_uv1(mesh: &Mesh) -> bool {
    let Some(VertexAttributeValues::Float32x2(uv0)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else {
        return false;
    };
    let Some(VertexAttributeValues::Float32x2(uv1)) = mesh.attribute(Mesh::ATTRIBUTE_UV_1) else {
        return false;
    };
    let mut min_u = f32::INFINITY;
    let mut max_u = f32::NEG_INFINITY;
    let mut min_v = f32::INFINITY;
    let mut max_v = f32::NEG_INFINITY;
    let mut differs_from_uv0 = false;

    for (a, b) in uv0.iter().zip(uv1.iter()) {
        min_u = min_u.min(b[0]);
        max_u = max_u.max(b[0]);
        min_v = min_v.min(b[1]);
        max_v = max_v.max(b[1]);
        differs_from_uv0 |= (a[0] - b[0]).abs() > 0.0001 || (a[1] - b[1]).abs() > 0.0001;
    }

    let uv1_varies = (max_u - min_u) > 0.0001 || (max_v - min_v) > 0.0001;
    differs_from_uv0 && uv1_varies
}

fn build_fallback_batch(
    vertices: &[M2Vertex],
    skin: Option<SkinData>,
    tex_types: &[u32],
    txid: &[u32],
) -> Result<Vec<M2RenderBatch>, String> {
    let indices = match skin {
        Some(s) => resolve_indices(&s.lookup, &s.indices),
        None => (0..vertices.len() as u16).collect(),
    };
    let fdid = m2_texture::first_hardcoded_texture(tex_types, txid);
    Ok(vec![M2RenderBatch {
        mesh: build_mesh(vertices, indices),
        texture_fdid: fdid,
        texture_2_fdid: None,
        texture_type: None,
        overlays: Vec::new(),
        render_flags: 0,
        blend_mode: 0,
        transparency: 1.0,
        texture_anim: None,
        texture_anim_2: None,
        use_uv_2_1: false,
        use_uv_2_2: false,
        use_env_map_2: false,
        shader_id: 0,
        texture_count: 1,
        mesh_part_id: 0,
    }])
}

fn build_render_batches(
    md20: &[u8],
    path: &Path,
    chunks: &M2Chunks<'_>,
    txid: &[u32],
    has_bones: bool,
    skin_fdids: &[u32; 3],
) -> Result<Vec<M2RenderBatch>, String> {
    let vertices = parse_vertices(md20)?;
    let tex_types = parse_texture_types(md20)?;
    let tex_lookup = parse_texture_lookup(md20)?;
    let texture_unit_lookup = parse_texture_unit_lookup(md20)?;
    let materials = parse_materials(md20)?;
    let color_tracks = super::m2_anim::parse_color_tracks(md20)?;
    let transparencies = super::m2_anim::parse_transparency_tracks(md20)?;
    let transparency_lookup = parse_transparency_lookup(md20)?;
    let texture_animations = super::m2_anim::parse_texture_animations(md20)?;
    let uv_animation_lookup = parse_uv_animation_lookup(md20)?;
    let skin = load_skin_data(path, &chunks.sfid);
    if !chunks.sfid.is_empty() && skin.is_none() {
        return Err(format!(
            "Missing external skin for {} (SFID {:?})",
            path.display(),
            chunks.sfid
        ));
    }
    let tex = TextureTables {
        tex_lookup: &tex_lookup,
        tex_types: &tex_types,
        txid,
        skin_fdids,
    };
    if let Some(ref skin) = skin
        && !skin.submeshes.is_empty()
        && !skin.batches.is_empty()
    {
        build_batched_model(
            &vertices,
            skin,
            &materials,
            &tex,
            &color_tracks,
            &transparencies,
            &transparency_lookup,
            &texture_animations,
            &uv_animation_lookup,
            &texture_unit_lookup,
            has_bones,
            chunks.skid.is_some(),
        )
    } else {
        build_fallback_batch(&vertices, skin, &tex_types, txid)
    }
}

fn load_attachment_data(chunks: &M2Chunks<'_>) -> (Vec<super::m2_attach::M2Attachment>, Vec<i16>) {
    let attachments = chunks
        .ska1
        .map(super::m2_attach::parse_ska1_attachments)
        .transpose()
        .unwrap_or_default()
        .filter(|parsed| !parsed.is_empty())
        .unwrap_or_else(|| super::m2_attach::parse_attachments(chunks.md20).unwrap_or_default());
    let attachment_lookup = chunks
        .ska1
        .map(super::m2_attach::parse_ska1_attachment_lookup)
        .transpose()
        .unwrap_or_default()
        .filter(|parsed| !parsed.is_empty())
        .unwrap_or_else(|| {
            super::m2_attach::parse_attachment_lookup(chunks.md20).unwrap_or_default()
        });
    (attachments, attachment_lookup)
}

fn find_chunk<'a>(data: &'a [u8], needle: &[u8; 4]) -> Option<&'a [u8]> {
    let mut off = 0;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = read_u32(data, off + 4).ok()? as usize;
        let end = off + 8 + size;
        if end > data.len() {
            return None;
        }
        if tag == needle {
            return Some(&data[off + 8..end]);
        }
        off = end;
    }
    None
}

fn load_skel_attachment_data(skel_path: &Path) -> Result<(Vec<super::m2_attach::M2Attachment>, Vec<i16>), String> {
    let data = std::fs::read(skel_path).map_err(|e| format!("Failed to read .skel file: {e}"))?;
    let Some(ska1) = find_chunk(&data, b"SKA1") else {
        return Ok((Vec::new(), Vec::new()));
    };
    Ok((
        super::m2_attach::parse_ska1_attachments(ska1)?,
        super::m2_attach::parse_ska1_attachment_lookup(ska1)?,
    ))
}

fn load_model_attachment_data(path: &Path, chunks: &M2Chunks<'_>) -> (Vec<super::m2_attach::M2Attachment>, Vec<i16>) {
    if !path.starts_with("data/models") {
        return load_attachment_data(chunks);
    }
    if let Some(skel_fdid) = chunks.skid {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let skel_path = path.with_file_name(format!("{stem}.skel"));
        super::casc_resolver::ensure_file_at_path(skel_fdid, &skel_path);
        if let Ok((attachments, attachment_lookup)) = load_skel_attachment_data(&skel_path)
            && (!attachments.is_empty() || !attachment_lookup.is_empty())
        {
            return (attachments, attachment_lookup);
        }
    }
    load_attachment_data(chunks)
}

fn load_m2_uncached(path: &Path, skin_fdids: &[u32; 3]) -> Result<M2Model, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read M2 file: {e}"))?;
    let chunks = parse_chunks(&data)?;
    let txid = chunks.txid.map(parse_txid).unwrap_or_default();
    let anim = load_anim_data(path, &chunks);
    let batches = build_render_batches(
        chunks.md20,
        path,
        &chunks,
        &txid,
        !anim.bones.is_empty(),
        skin_fdids,
    )?;
    let mut particles = super::m2_particle::parse_particle_emitters(chunks.md20);
    super::m2_particle::resolve_texture_fdids(&mut particles, &txid);
    let (attachments, attachment_lookup) = load_model_attachment_data(path, &chunks);
    Ok(M2Model {
        batches,
        bones: anim.bones,
        sequences: anim.sequences,
        bone_tracks: anim.bone_tracks,
        global_sequences: anim.global_sequences,
        particle_emitters: particles,
        attachments,
        attachment_lookup,
    })
}

/// Load an M2 model file (chunked MD21 format) and return per-batch meshes + textures.
pub fn load_m2(path: &Path, skin_fdids: &[u32; 3]) -> Result<M2Model, String> {
    let key = ModelCacheKey {
        path: path.to_path_buf(),
        skin_fdids: *skin_fdids,
    };
    let cache = M2_MODEL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache.lock().unwrap().get(&key).cloned() {
        return cached;
    }
    let loaded = load_m2_uncached(path, skin_fdids);
    cache.lock().unwrap().insert(key, loaded.clone());
    loaded
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

#[cfg(test)]
#[path = "m2_runtime_head_tests.rs"]
mod runtime_head_tests;
