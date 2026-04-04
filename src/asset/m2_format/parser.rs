use std::io::Cursor;
use std::mem::size_of;
use std::path::{Path, PathBuf};

use crate::asset::read_bytes::read_u32;

use binrw::BinRead;

const CHUNK_HEADER_SIZE: usize = 8;
const SKIN_HEADER_SIZE: usize = 44;
const U16_ARRAY_HEADER_STRIDE: usize = 4;
const M2_ARRAY_HEADER_SIZE: usize = 8;
const LOOKUP_ENTRY_SIZE_U32: usize = 4;
const MD20_TRANSPARENCY_LOOKUP_COUNT_OFFSET: usize = 0x90;
const MD20_UV_ANIMATION_LOOKUP_COUNT_OFFSET: usize = 0x98;

#[derive(BinRead)]
#[br(little)]
pub(crate) struct M2Vertex {
    pub position: [f32; 3],
    pub bone_weights: [u8; 4],
    pub bone_indices: [u8; 4],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub tex_coords_2: [f32; 2],
}

pub(crate) struct M2Submesh {
    pub mesh_part_id: u16,
    pub vertex_start: u16,
    pub vertex_count: u16,
    pub triangle_start: u32,
    pub triangle_count: u16,
}

#[derive(BinRead)]
#[br(little)]
struct RawM2Submesh {
    mesh_part_id: u16,
    _level: u16,
    vertex_start: u16,
    vertex_count: u16,
    _triangle_start: u16,
    triangle_count: u16,
    _bone_count: u16,
    _bone_combo_index: u16,
    _bone_influences: u16,
    _root_bone: u16,
    _center_mass: [f32; 3],
    _sort_center_position: [f32; 3],
    _sort_radius: f32,
}

#[derive(BinRead)]
#[br(little)]
pub(crate) struct M2Material {
    pub flags: u16,
    pub blend_mode: u16,
}

pub struct M2TextureUnit {
    pub flags: u8,
    pub priority_plane: i8,
    pub shader_id: u16,
    pub submesh_index: u16,
    pub color_index: i16,
    pub render_flags_index: u16,
    pub material_layer: u16,
    pub texture_count: u16,
    pub texture_id: u16,
    pub texture_coord_index: u16,
    pub transparency_index: u16,
    pub texture_animation_id: u16,
}

#[derive(BinRead)]
#[br(little)]
struct RawM2TextureUnit {
    flags: u8,
    priority_plane: i8,
    shader_id: u16,
    submesh_index: u16,
    _submesh_index2: u16,
    color_index: i16,
    render_flags_index: u16,
    material_layer: u16,
    texture_count: u16,
    texture_id: u16,
    texture_coord_index: u16,
    transparency_index: u16,
    texture_animation_id: u16,
}

#[derive(BinRead)]
#[br(little)]
struct M2TextureTypeEntry {
    texture_type: u32,
    _flags: u32,
    _filename_length: u32,
    _filename_offset: u32,
}

#[derive(BinRead)]
#[br(little)]
struct SkinHeader {
    _magic: [u8; 4],
    lookup_count: u32,
    lookup_offset: u32,
    indices_count: u32,
    indices_offset: u32,
    _bone_indices_count: u32,
    _bone_indices_offset: u32,
    submesh_count: u32,
    submesh_offset: u32,
    batch_count: u32,
    batch_offset: u32,
}

pub(crate) struct SkinData {
    pub lookup: Vec<u16>,
    pub indices: Vec<u16>,
    pub submeshes: Vec<M2Submesh>,
    pub batches: Vec<M2TextureUnit>,
}

pub(crate) struct M2Chunks<'a> {
    pub md20: &'a [u8],
    pub ska1: Option<&'a [u8]>,
    pub txid: Option<&'a [u8]>,
    pub skid: Option<u32>,
    pub sfid: Vec<u32>,
}

pub struct TextureTables<'a> {
    pub tex_lookup: &'a [u16],
    pub tex_types: &'a [u32],
    pub txid: &'a [u32],
    pub skin_fdids: &'a [u32; 3],
}

pub(crate) struct SkelData {
    pub bones: Vec<super::m2_anim::M2Bone>,
    pub sequences: Vec<super::m2_anim::M2AnimSequence>,
    pub bone_tracks: Vec<super::m2_anim::BoneAnimTracks>,
    pub global_sequences: Vec<u32>,
}

fn parse_binrw_entries<T>(
    data: &[u8],
    offset: usize,
    count: usize,
    label: &str,
) -> Result<Vec<T>, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let byte_len = count
        .checked_mul(size_of::<T>())
        .ok_or_else(|| format!("{label} byte length overflow"))?;
    let end = offset
        .checked_add(byte_len)
        .ok_or_else(|| format!("{label} end offset overflow"))?;
    let slice = data
        .get(offset..end)
        .ok_or_else(|| format!("{label} data out of bounds at {offset:#x}"))?;
    let mut cursor = Cursor::new(slice);
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        entries.push(T::read_le(&mut cursor).map_err(|err| {
            format!(
                "{label} {i} parse failed at {:#x}: {err}",
                offset + i * size_of::<T>()
            )
        })?);
    }
    Ok(entries)
}

fn read_m2_array<T>(md20: &[u8], header_off: usize, label: &str) -> Result<Vec<T>, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    if md20.len() < header_off + M2_ARRAY_HEADER_SIZE {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, header_off)? as usize;
    let offset = read_u32(md20, header_off + U16_ARRAY_HEADER_STRIDE)? as usize;
    parse_binrw_entries(md20, offset, count, label)
}

pub(crate) fn parse_chunks(data: &[u8]) -> Result<M2Chunks<'_>, String> {
    let mut md20 = None;
    let mut ska1 = None;
    let mut txid = None;
    let mut skid = None;
    let mut sfid = Vec::new();
    let mut off = 0;
    while off + CHUNK_HEADER_SIZE <= data.len() {
        let tag = &data[off..off + 4];
        let size = read_u32(data, off + 4)? as usize;
        let end = off + CHUNK_HEADER_SIZE + size;
        if end > data.len() {
            let tag_str = std::str::from_utf8(tag).unwrap_or("????");
            return Err(format!("Chunk {tag_str} truncated at offset {off:#x}"));
        }
        match tag {
            b"MD21" => md20 = Some(&data[off + CHUNK_HEADER_SIZE..end]),
            b"SKA1" => ska1 = Some(&data[off + CHUNK_HEADER_SIZE..end]),
            b"TXID" => txid = Some(&data[off + CHUNK_HEADER_SIZE..end]),
            b"SKID" if size >= 4 => skid = Some(read_u32(data, off + CHUNK_HEADER_SIZE)?),
            b"SFID" => sfid = parse_sfid(&data[off + CHUNK_HEADER_SIZE..end]),
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

pub(crate) fn parse_vertices(md20: &[u8]) -> Result<Vec<M2Vertex>, String> {
    if md20.len() < super::MD20_VERTICES_DATA_OFFSET + 4 {
        return Err("MD20 header too short for vertices".into());
    }
    read_m2_array(md20, super::MD20_VERTICES_COUNT_OFFSET, "vertex")
}

pub(crate) fn parse_skin_full(data: &[u8]) -> Result<SkinData, String> {
    if data.len() < SKIN_HEADER_SIZE || &data[0..4] != b"SKIN" {
        return Err("Invalid skin file (bad magic)".into());
    }
    let header: SkinHeader = SkinHeader::read_le(&mut Cursor::new(&data[..SKIN_HEADER_SIZE]))
        .map_err(|err| format!("SKIN header parse failed: {err}"))?;
    let lookup = parse_u16_entries(
        data,
        header.lookup_offset as usize,
        header.lookup_count as usize,
    )?;
    let indices = parse_u16_entries(
        data,
        header.indices_offset as usize,
        header.indices_count as usize,
    )?;
    let submeshes = parse_submeshes(
        data,
        header.submesh_offset as usize,
        header.submesh_count as usize,
    )?;
    let batches = parse_texture_units(
        data,
        header.batch_offset as usize,
        header.batch_count as usize,
    )?;
    Ok(SkinData {
        lookup,
        indices,
        submeshes,
        batches,
    })
}

fn parse_u16_entries(data: &[u8], offset: usize, count: usize) -> Result<Vec<u16>, String> {
    parse_binrw_entries(data, offset, count, "u16 array")
}

fn parse_submeshes(data: &[u8], offset: usize, count: usize) -> Result<Vec<M2Submesh>, String> {
    let raw = parse_binrw_entries::<RawM2Submesh>(data, offset, count, "submesh")?;
    let mut subs = Vec::with_capacity(raw.len());
    let mut cumulative_index = 0u32;
    for submesh in raw {
        subs.push(M2Submesh {
            mesh_part_id: submesh.mesh_part_id,
            vertex_start: submesh.vertex_start,
            vertex_count: submesh.vertex_count,
            triangle_start: cumulative_index,
            triangle_count: submesh.triangle_count,
        });
        cumulative_index += submesh.triangle_count as u32;
    }
    Ok(subs)
}

fn parse_texture_units(
    data: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<M2TextureUnit>, String> {
    Ok(
        parse_binrw_entries::<RawM2TextureUnit>(data, offset, count, "texture unit")?
            .into_iter()
            .map(|unit| M2TextureUnit {
                flags: unit.flags,
                priority_plane: unit.priority_plane,
                shader_id: unit.shader_id,
                submesh_index: unit.submesh_index,
                color_index: unit.color_index,
                render_flags_index: unit.render_flags_index,
                material_layer: unit.material_layer,
                texture_count: unit.texture_count,
                texture_id: unit.texture_id,
                texture_coord_index: unit.texture_coord_index,
                transparency_index: unit.transparency_index,
                texture_animation_id: unit.texture_animation_id,
            })
            .collect(),
    )
}

pub(crate) fn parse_texture_types(md20: &[u8]) -> Result<Vec<u32>, String> {
    Ok(read_m2_array::<M2TextureTypeEntry>(
        md20,
        super::MD20_TEXTURES_COUNT_OFFSET,
        "texture type",
    )?
    .into_iter()
    .map(|entry| entry.texture_type)
    .collect())
}

pub(crate) fn parse_materials(md20: &[u8]) -> Result<Vec<M2Material>, String> {
    read_m2_array(md20, super::MD20_MATERIALS_COUNT_OFFSET, "material")
}

pub(crate) fn parse_txid(data: &[u8]) -> Vec<u32> {
    (0..data.len() / LOOKUP_ENTRY_SIZE_U32)
        .filter_map(|i| read_u32(data, i * LOOKUP_ENTRY_SIZE_U32).ok())
        .collect()
}

pub(crate) fn parse_texture_lookup(md20: &[u8]) -> Result<Vec<u16>, String> {
    read_m2_array(
        md20,
        super::MD20_TEXTURE_LOOKUP_COUNT_OFFSET,
        "texture lookup",
    )
}

pub(crate) fn parse_texture_unit_lookup(md20: &[u8]) -> Result<Vec<i16>, String> {
    read_m2_array(
        md20,
        super::MD20_TEXTURE_UNIT_LOOKUP_COUNT_OFFSET,
        "texture unit lookup",
    )
}

fn parse_i16_lookup(md20: &[u8], header_off: usize, label: &str) -> Result<Vec<i16>, String> {
    read_m2_array(md20, header_off, &format!("{label} lookup"))
}

pub(crate) fn parse_transparency_lookup(md20: &[u8]) -> Result<Vec<i16>, String> {
    parse_i16_lookup(md20, MD20_TRANSPARENCY_LOOKUP_COUNT_OFFSET, "transparency")
}

pub(crate) fn parse_uv_animation_lookup(md20: &[u8]) -> Result<Vec<i16>, String> {
    parse_i16_lookup(md20, MD20_UV_ANIMATION_LOOKUP_COUNT_OFFSET, "uv animation")
}

pub(crate) fn resolve_indices(lookup: &[u16], indices: &[u16]) -> Vec<u16> {
    indices
        .iter()
        .filter_map(|&idx| lookup.get(idx as usize).copied())
        .collect()
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

pub(crate) fn load_anim_data(path: &Path, chunks: &M2Chunks<'_>) -> SkelData {
    if let Some(skel_fdid) = chunks.skid {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let skel_path = path.with_file_name(format!("{stem}.skel"));
        super::super::asset_cache::file_at_path(skel_fdid, &skel_path);
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

pub(crate) fn load_skin_data(m2_path: &Path, sfid: &[u32]) -> Option<SkinData> {
    let stem = m2_path.file_stem()?.to_str()?;
    if let Some(&fdid) = sfid.first() {
        let canonical_skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
        if let Some(resolved_path) =
            super::super::asset_cache::file_at_path(fdid, &canonical_skin_path)
            && let Ok(data) = std::fs::read(&resolved_path)
        {
            return parse_skin_full(&data).ok();
        }
        let numeric_skin_path = m2_path.with_file_name(format!("{fdid}.skin"));
        if let Some(resolved_path) =
            super::super::asset_cache::file_at_path(fdid, &numeric_skin_path)
            && let Ok(data) = std::fs::read(&resolved_path)
        {
            return parse_skin_full(&data).ok();
        }
    }
    let skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
    let data = std::fs::read(&skin_path).ok()?;
    parse_skin_full(&data).ok()
}

pub fn ensure_primary_skin_path(m2_path: &Path) -> Option<PathBuf> {
    let data = std::fs::read(m2_path).ok()?;
    let chunks = parse_chunks(&data).ok()?;
    let stem = m2_path.file_stem()?.to_str()?;
    if let Some(&fdid) = chunks.sfid.first() {
        let canonical_skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
        if let Some(path) = super::super::asset_cache::file_at_path(fdid, &canonical_skin_path) {
            return Some(path);
        }
        let numeric_skin_path = m2_path.with_file_name(format!("{fdid}.skin"));
        return super::super::asset_cache::file_at_path(fdid, &numeric_skin_path);
    }
    let skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
    skin_path.exists().then_some(skin_path)
}
