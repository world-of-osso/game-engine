//! Authored M2 model and primary skin, with coordinates left in WoW model space.
use crate::asset::m2_batch_data::{self, BatchInputs, ResolvedBatch};
use crate::asset::m2_format::{self as format, m2_anim, m2_attach};
use format::parser::TextureTables;

pub use format::m2_collision::M2CollisionMesh;

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
    pub texture_unit_lookup: Vec<i16>,
    pub transparency_lookup: Vec<i16>,
    pub uv_animation_lookup: Vec<i16>,
    pub uses_texture_combiner_combos: bool,
    pub color_tracks: Vec<m2_anim::ColorAnimTracks>,
    pub transparency_tracks: Vec<m2_anim::AnimTrack<i16>>,
    pub texture_animations: Vec<m2_anim::TextureAnimTracks>,
    pub skin_fdids: Vec<u32>,
    pub skeleton_fdid: Option<u32>,
    pub attachments: Vec<m2_attach::M2Attachment>,
    pub attachment_lookup: Vec<i16>,
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub collision: Option<M2CollisionMesh>,
}

/// Resolve authored skin batches in the same draw order and opacity policy as the Bevy renderer.
/// Creature texture slots are separate from `Model::skin_fdids` (geometry skin file IDs).
/// The caller supplies the FDID path lookup used by the original empty-UV-table envmap heuristic.
pub fn resolve_render_batches(
    model: &Model,
    skin_texture_fdids: &[u32; 3],
    keep_zero_opacity_batches: bool,
    fdid_path: impl Fn(u32) -> Option<String>,
) -> Result<Vec<ResolvedBatch>, String> {
    let mesh_part_ids: Vec<_> = model.submeshes.iter().map(|sub| sub.mesh_part_id).collect();
    let materials: Vec<_> = model
        .materials
        .iter()
        .map(|mat| (mat.flags, mat.blend_mode))
        .collect();
    m2_batch_data::resolve_batches(
        &BatchInputs {
            units: &model.batches,
            mesh_part_ids: &mesh_part_ids,
            materials: &materials,
            tex: TextureTables {
                tex_lookup: &model.texture_lookup,
                tex_types: &model.texture_types,
                txid: &model.texture_fdids,
                skin_fdids: skin_texture_fdids,
            },
            color_tracks: &model.color_tracks,
            transparencies: &model.transparency_tracks,
            transparency_lookup: &model.transparency_lookup,
            texture_animations: &model.texture_animations,
            uv_animation_lookup: &model.uv_animation_lookup,
            texture_unit_lookup: &model.texture_unit_lookup,
            uses_texture_combiner_combos: model.uses_texture_combiner_combos,
            is_hd: model.skeleton_fdid.is_some(),
            keep_zero_opacity_batches,
        },
        fdid_path,
    )
}

/// The batch colour's RGB track, the pixel shader's `meshColor` (authored gamma space).
pub fn batch_color_track<'a>(
    model: &'a Model,
    batch: &ResolvedBatch,
) -> Option<&'a AnimTrack<[f32; 3]>> {
    batch
        .color_opacity_track_index
        .and_then(|index| model.color_tracks.get(index))
        .map(|tracks| &tracks.color)
}

/// `meshColor` at time zero; white without a colour.
pub fn batch_mesh_color(model: &Model, batch: &ResolvedBatch) -> [f32; 3] {
    batch_color_track(model, batch)
        .and_then(|track| m2_anim::evaluate_vec3_track(track, 0, 0))
        .unwrap_or([1.0; 3])
}

/// FileDataIDs for the primary geometry skin and optional external skeleton.
#[derive(Debug, PartialEq, Eq)]
pub struct AssetReferences {
    pub skin_fdids: Vec<u32>,
    pub skeleton_fdid: Option<u32>,
}

pub fn parse_asset_references(model: &[u8]) -> Result<AssetReferences, String> {
    let chunks = format::parse_chunks(model)?;
    Ok(AssetReferences {
        skin_fdids: chunks.sfid,
        skeleton_fdid: chunks.skid,
    })
}

fn find_skeleton_ska1(skeleton: &[u8]) -> Result<Option<&[u8]>, String> {
    let mut offset = 0;
    while offset + 8 <= skeleton.len() {
        let size = format::read_u32(skeleton, offset + 4)? as usize;
        let end = offset
            .checked_add(8)
            .and_then(|start| start.checked_add(size))
            .ok_or("Skeleton chunk length overflow")?;
        let payload = skeleton
            .get(offset + 8..end)
            .ok_or("Skeleton chunk is truncated")?;
        if &skeleton[offset..offset + 4] == b"SKA1" {
            return Ok(Some(payload));
        }
        offset = end;
    }
    Ok(None)
}

fn parse_attachments(
    md20: &[u8],
    ska1: Option<&[u8]>,
) -> Result<(Vec<m2_attach::M2Attachment>, Vec<i16>), String> {
    let attachments = ska1
        .map(m2_attach::parse_ska1_attachments)
        .transpose()?
        .filter(|items| !items.is_empty())
        .map(Ok)
        .unwrap_or_else(|| m2_attach::parse_attachments(md20))?;
    let lookup = ska1
        .map(m2_attach::parse_ska1_attachment_lookup)
        .transpose()?
        .filter(|items| !items.is_empty())
        .map(Ok)
        .unwrap_or_else(|| m2_attach::parse_attachment_lookup(md20))?;
    Ok((attachments, lookup))
}

#[cfg(test)]
mod asset_references_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn reads_geometry_skin_and_external_skeleton_fdids_without_skin_bytes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
        let plain = std::fs::read(root.join("126278.m2")).unwrap();
        let hd = std::fs::read(root.join("1011653.m2")).unwrap();
        assert_eq!(
            parse_asset_references(&plain).unwrap().skin_fdids[0],
            480325
        );
        let refs = parse_asset_references(&hd).unwrap();
        assert_eq!(refs.skin_fdids[0], 1012983);
        assert_eq!(refs.skeleton_fdid, Some(2138400));
    }

    #[test]
    fn rejects_missing_md21_chunk() {
        assert!(
            parse_asset_references(b"SFID\x04\0\0\0\x01\0\0\0")
                .unwrap_err()
                .contains("No MD21")
        );
    }
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
        // External `.anim` sequences are not loaded here: they keep no keyframes.
        let parsed = format::parser::parse_skel_data_with_anims(skeleton, |_| None)?;
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
    let skeleton_ska1 = skeleton.map(find_skeleton_ska1).transpose()?.flatten();
    let (attachments, attachment_lookup) =
        parse_attachments(chunks.md20, skeleton_ska1.or(chunks.ska1))?;
    let (bounding_box_min, bounding_box_max) = format::parse_bounding_box(chunks.md20);
    let collision = format::m2_collision::parse_collision_mesh(chunks.md20)?;
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
        texture_unit_lookup: format::parse_texture_unit_lookup(chunks.md20)?,
        transparency_lookup: format::parse_transparency_lookup(chunks.md20)?,
        uv_animation_lookup: format::parse_uv_animation_lookup(chunks.md20)?,
        uses_texture_combiner_combos: format::parse_model_flags(chunks.md20)? & 0x8 != 0,
        color_tracks: m2_anim::parse_color_tracks(chunks.md20)?,
        transparency_tracks: m2_anim::parse_transparency_tracks(chunks.md20)?,
        texture_animations: m2_anim::parse_texture_animations(chunks.md20)?,
        skin_fdids: chunks.sfid,
        skeleton_fdid: chunks.skid,
        attachments,
        attachment_lookup,
        bounding_box_min,
        bounding_box_max,
        collision,
    })
}
