//! Authored M2 model and primary skin, with coordinates left in WoW model space.
use crate::asset::m2_format::{self as format, m2_anim};

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub tex_coords_2: [f32; 2],
    pub bone_weights: [u8; 4],
    pub bone_indices: [u8; 4],
}

#[derive(Debug)]
pub struct Submesh {
    pub mesh_part_id: u16,
    pub vertex_start: u16,
    pub vertex_count: u16,
    pub triangle_start: u32,
    pub triangle_count: u16,
}

#[derive(Debug)]
pub struct Material {
    pub flags: u16,
    pub blend_mode: u16,
}

pub use format::parser::M2TextureUnit as TextureUnit;
pub use m2_anim::{AnimTrack, BoneAnimTracks, M2AnimSequence as Sequence, M2Bone as Bone};

pub struct Model {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub submeshes: Vec<Submesh>,
    pub batches: Vec<TextureUnit>,
    pub materials: Vec<Material>,
    pub bones: Vec<Bone>,
    pub sequences: Vec<Sequence>,
    pub bone_tracks: Vec<BoneAnimTracks>,
    pub global_sequences: Vec<u32>,
    pub texture_types: Vec<u32>,
    pub texture_fdids: Vec<u32>,
    pub texture_lookup: Vec<u16>,
}

/// Parse the primary skin. Models with external SKID skeletons require the separate skeleton bytes.
pub fn parse_model(model: &[u8], skin: &[u8]) -> Result<Model, String> {
    parse_model_with_skeleton(model, skin, None)
}

pub fn parse_model_with_skeleton(
    model: &[u8],
    skin: &[u8],
    skeleton: Option<&[u8]>,
) -> Result<Model, String> {
    let chunks = format::parse_chunks(model)?;
    if chunks.skid.is_some() && skeleton.is_none() {
        return Err("M2 SKID requires external skeleton bytes".into());
    }
    let skin = format::parser::parse_skin_full(skin)?;
    let (bones, sequences, bone_tracks, global_sequences) = if let Some(skeleton) = skeleton {
        let parsed = format::parser::parse_skel_data(skeleton)?;
        if parsed.bones.is_empty() {
            return Err("SKB1 skeleton chunk missing or empty".into());
        }
        (
            parsed.bones,
            parsed.sequences,
            parsed.bone_tracks,
            parsed.global_sequences,
        )
    } else {
        (
            m2_anim::parse_bones(chunks.md20)?,
            m2_anim::parse_sequences(chunks.md20)?,
            m2_anim::parse_bone_animations(chunks.md20)?,
            m2_anim::parse_global_sequences(chunks.md20)?,
        )
    };
    m2_anim::validate_bone_hierarchy(&bones)?;
    let vertices = format::parse_vertices(chunks.md20)?;
    let materials = format::parse_materials(chunks.md20)?;
    Ok(Model {
        vertices: vertices
            .into_iter()
            .map(|v| Vertex {
                position: v.position,
                normal: v.normal,
                tex_coords: v.tex_coords,
                tex_coords_2: v.tex_coords_2,
                bone_weights: v.bone_weights,
                bone_indices: v.bone_indices,
            })
            .collect(),
        indices: format::resolve_indices(&skin.lookup, &skin.indices),
        submeshes: skin
            .submeshes
            .into_iter()
            .map(|s| Submesh {
                mesh_part_id: s.mesh_part_id,
                vertex_start: s.vertex_start,
                vertex_count: s.vertex_count,
                triangle_start: s.triangle_start,
                triangle_count: s.triangle_count,
            })
            .collect(),
        batches: skin.batches,
        materials: materials
            .into_iter()
            .map(|m| Material {
                flags: m.flags,
                blend_mode: m.blend_mode,
            })
            .collect(),
        bones,
        sequences,
        bone_tracks,
        global_sequences,
        texture_types: format::parse_texture_types(chunks.md20)?,
        texture_fdids: chunks.txid.map(format::parse_txid).unwrap_or_default(),
        texture_lookup: format::parse_texture_lookup(chunks.md20)?,
    })
}
