use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use std::path::Path;
use std::path::PathBuf;

#[cfg(test)]
pub use super::m2_texture::{first_hardcoded_texture, resolve_batch_texture};

#[path = "m2_batch.rs"]
mod m2_batch;
#[path = "m2_loader.rs"]
pub(crate) mod m2_loader;

pub use super::m2_format::ensure_primary_skin_path;
#[cfg(test)]
pub(crate) use super::m2_format::parse_skin_full;
pub(crate) use super::m2_format::{
    M2Chunks, M2Material, M2Submesh, M2TextureUnit, M2Vertex, SkinData, TextureTables,
    load_anim_data, load_skin_data, parse_chunks, parse_materials, parse_texture_lookup,
    parse_texture_types, parse_texture_unit_lookup, parse_transparency_lookup, parse_txid,
    parse_uv_animation_lookup, parse_vertices, read_u32, resolve_indices,
};
pub use m2_loader::{load_m2, load_m2_uncached};

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
    pub lights: Vec<super::m2_light::M2Light>,
    /// Model-local bounding box min (from MD20 header).
    pub bounding_box_min: [f32; 3],
    /// Model-local bounding box max (from MD20 header).
    pub bounding_box_max: [f32; 3],
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct ModelCacheKey {
    pub path: PathBuf,
    pub skin_fdids: [u32; 3],
}

#[path = "m2_cache_stats.rs"]
mod cache_stats;
pub use cache_stats::{ModelCacheStats, model_cache_stats};

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

pub(crate) fn build_batch_mesh(
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

pub(crate) fn build_mesh(vertices: &[M2Vertex], indices: Vec<u16>) -> Mesh {
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

/// Default geoset visibility for initial model display.
pub fn default_geoset_visible(mesh_part_id: u16) -> bool {
    let group = mesh_part_id / 100;
    let variant = mesh_part_id % 100;
    match group {
        0 => matches!(mesh_part_id, 0 | 1 | 5 | 16 | 17 | 27..=33),
        1..=3 => variant == 2,
        7 => matches!(variant, 1 | 2),
        15 => false,
        17 => false,
        32 => variant >= 1,
        _ => variant == 1,
    }
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

pub(crate) fn build_render_batches(
    md20: &[u8],
    path: &Path,
    chunks: &M2Chunks<'_>,
    txid: &[u32],
    has_bones: bool,
    skin_fdids: &[u32; 3],
) -> Result<Vec<M2RenderBatch>, String> {
    let parsed = parse_batch_inputs(md20)?;
    let skin = load_skin_data_checked(path, &chunks.sfid)?;
    let tex = TextureTables {
        tex_lookup: &parsed.tex_lookup,
        tex_types: &parsed.tex_types,
        txid,
        skin_fdids,
    };
    if let Some(ref skin) = skin
        && !skin.submeshes.is_empty()
        && !skin.batches.is_empty()
    {
        m2_batch::build_batched_model(&m2_batch::BatchBuildContext {
            vertices: &parsed.vertices,
            skin,
            materials: &parsed.materials,
            tex: &tex,
            color_tracks: &parsed.color_tracks,
            transparencies: &parsed.transparencies,
            transparency_lookup: &parsed.transparency_lookup,
            texture_animations: &parsed.texture_animations,
            uv_animation_lookup: &parsed.uv_animation_lookup,
            texture_unit_lookup: &parsed.texture_unit_lookup,
            has_bones,
            is_hd: chunks.skid.is_some(),
        })
    } else {
        m2_batch::build_fallback_batch(&parsed.vertices, skin, &parsed.tex_types, txid)
    }
}

fn load_skin_data_checked(path: &Path, sfid: &[u32]) -> Result<Option<SkinData>, String> {
    let skin = load_skin_data(path, sfid);
    if !sfid.is_empty() && skin.is_none() {
        return Err(format!(
            "Missing external skin for {} (SFID {:?})",
            path.display(),
            sfid
        ));
    }
    Ok(skin)
}

struct ParsedBatchInputs {
    vertices: Vec<M2Vertex>,
    tex_types: Vec<u32>,
    tex_lookup: Vec<u16>,
    texture_unit_lookup: Vec<i16>,
    materials: Vec<M2Material>,
    color_tracks: Vec<super::m2_anim::ColorAnimTracks>,
    transparencies: Vec<super::m2_anim::AnimTrack<i16>>,
    transparency_lookup: Vec<i16>,
    texture_animations: Vec<super::m2_anim::TextureAnimTracks>,
    uv_animation_lookup: Vec<i16>,
}

fn parse_batch_inputs(md20: &[u8]) -> Result<ParsedBatchInputs, String> {
    Ok(ParsedBatchInputs {
        vertices: parse_vertices(md20)?,
        tex_types: parse_texture_types(md20)?,
        tex_lookup: parse_texture_lookup(md20)?,
        texture_unit_lookup: parse_texture_unit_lookup(md20)?,
        materials: parse_materials(md20)?,
        color_tracks: super::m2_anim::parse_color_tracks(md20)?,
        transparencies: super::m2_anim::parse_transparency_tracks(md20)?,
        transparency_lookup: parse_transparency_lookup(md20)?,
        texture_animations: super::m2_anim::parse_texture_animations(md20)?,
        uv_animation_lookup: parse_uv_animation_lookup(md20)?,
    })
}

#[cfg(test)]
#[path = "../../tests/unit/asset/m2_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../tests/unit/asset/m2_debug_tests.rs"]
mod debug_tests;

#[cfg(test)]
#[path = "../../tests/unit/asset/m2_jaw_debug_tests.rs"]
mod jaw_debug_tests;

#[cfg(test)]
#[path = "../../tests/unit/asset/m2_runtime_head_tests.rs"]
mod runtime_head_tests;

#[cfg(test)]
#[path = "../../tests/unit/asset/m2_runtime_model_tests.rs"]
mod runtime_model_tests;
