//! Authored M2 model and primary skin, with coordinates left in WoW model space.
use crate::asset::m2_batch_data::{self, BatchInputs, ResolvedBatch};
use crate::asset::m2_format::{
    self as format, m2_anim, m2_attach, m2_event, m2_light, m2_particle,
};
use format::parser::TextureTables;

pub use format::m2_collision::M2CollisionMesh;
pub use format::m2_variation::VariationFamily;
pub use m2_event::M2Event as Event;
pub use m2_particle::M2ParticleEmitter as ParticleEmitter;

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
    /// Shared: every animated instance of the model reads the same tracks.
    pub bone_tracks: std::sync::Arc<Vec<BoneAnimTracks>>,
    pub global_sequences: Vec<u32>,
    pub texture_types: Vec<u32>,
    /// M2Texture flags (0x1 wrap U, 0x2 wrap V), indexed like `texture_types`.
    pub texture_flags: Vec<u32>,
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
    /// MD20 particle emitters with their TXID texture FDIDs resolved.
    pub particle_emitters: Vec<ParticleEmitter>,
    /// MD20 animation events, their timestamps indexed like `sequences`.
    pub events: Vec<Event>,
    /// MD20 global flags.
    pub flags: u32,
    pub lights: Vec<m2_light::M2Light>,
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

/// FDIDs of the external `.anim` files the model's and its skeleton's `AFID` chunks name, so
/// a caller can fetch them before parsing.
pub fn external_anim_fdids(model: &[u8], skeleton: Option<&[u8]>) -> Result<Vec<u32>, String> {
    let chunks = format::parse_chunks(model)?;
    let mut fdids: Vec<u32> = chunks.afid.iter().map(|&(_, _, fdid)| fdid).collect();
    if let Some(skeleton) = skeleton {
        fdids.extend(
            format::parser::skeleton_afid(skeleton)?
                .iter()
                .map(|&(_, _, fdid)| fdid),
        );
    }
    fdids.retain(|&fdid| fdid != 0);
    fdids.sort_unstable();
    fdids.dedup();
    Ok(fdids)
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
/// Sequences kept in external `.anim` files get no keyframes.
/// A track whose sampled value never changes: every key of every sequence holds the
/// same value. A track with keys in some sequences but none in another is not
/// constant, since the empty sequence samples the default.
pub fn track_is_constant<T: PartialEq>(track: &AnimTrack<T>) -> bool {
    let mut values = track.sequences.iter().flat_map(|(_, values)| values);
    let Some(first) = values.next() else {
        return true;
    };
    values.all(|value| value == first)
        && track.sequences.iter().all(|(_, values)| !values.is_empty())
}

/// No bone track of `model` changes its pose over time.
pub fn bones_are_static(model: &Model) -> bool {
    model.bone_tracks.iter().all(|bone| {
        track_is_constant(&bone.translation)
            && track_is_constant(&bone.rotation)
            && track_is_constant(&bone.scale)
    })
}

pub fn parse_model(model: &[u8], skin: &[u8]) -> Result<Model, String> {
    parse_model_with_skeleton(model, skin, None, |_| None)
}

/// `load_anim` returns the bytes of an external sequence's `.anim` file by FDID (the model's or
/// skeleton's `AFID`); a sequence whose file it can't provide has no keyframes.
pub fn parse_model_with_skeleton(
    model: &[u8],
    skin: &[u8],
    skeleton: Option<&[u8]>,
    mut load_anim: impl FnMut(u32) -> Option<Vec<u8>>,
) -> Result<Model, String> {
    let chunks = format::parse_chunks(model)?;
    if chunks.skid.is_some() && skeleton.is_none() {
        return Err("M2 SKID requires external skeleton bytes".into());
    }
    let skin = format::parser::parse_skin_full(skin)?;
    // A skeleton's `.anim` files hold both its bone (AFSB) and the model's (AFM2) tracks.
    let mut anim_files = std::collections::HashMap::new();
    let mut load_anim = |fdid: u32| -> Option<Vec<u8>> {
        anim_files
            .entry(fdid)
            .or_insert_with(|| load_anim(fdid))
            .clone()
    };
    let (bones, sequences, skeleton_tracks, global_sequences, afid) =
        if let Some(skeleton) = skeleton {
            let parsed = format::parser::parse_skel_data_with_anims(skeleton, &mut load_anim)?;
            if parsed.bones.is_empty() {
                return Err("SKB1 skeleton chunk missing or empty".into());
            }
            (
                parsed.bones,
                parsed.sequences,
                Some(parsed.bone_tracks),
                parsed.global_sequences,
                format::parser::skeleton_afid(skeleton)?,
            )
        } else {
            let sequences = m2_anim::parse_sequences(chunks.md20)?;
            (
                m2_anim::parse_bones(chunks.md20)?,
                sequences,
                None,
                m2_anim::parse_global_sequences(chunks.md20)?,
                chunks.afid.clone(),
            )
        };
    // Model tracks (bones of an unskeletoned model, events) of external sequences are
    // in their `.anim` files' AFM2 chunks.
    let model_anim_files =
        format::parser::load_anim_track_chunks(&sequences, &afid, b"AFM2", &mut load_anim);
    let model_sources = m2_anim::sequence_data_sources(&sequences, &model_anim_files);
    let bone_tracks = match skeleton_tracks {
        Some(tracks) => tracks,
        None => m2_anim::parse_md20_bone_animations(chunks.md20, &model_sources)?,
    };
    let events = m2_event::parse_events(chunks.md20, &model_sources)?;
    m2_anim::validate_bone_hierarchy(&bones)?;
    let vertices = format::parse_vertices(chunks.md20)?;
    let materials = format::parse_materials(chunks.md20)?;
    let skeleton_ska1 = skeleton.map(find_skeleton_ska1).transpose()?.flatten();
    let (attachments, attachment_lookup) =
        parse_attachments(chunks.md20, skeleton_ska1.or(chunks.ska1))?;
    let (bounding_box_min, bounding_box_max) = format::parse_bounding_box(chunks.md20);
    let collision = format::m2_collision::parse_collision_mesh(chunks.md20)?;
    let texture_fdids = chunks.txid.map(format::parse_txid).unwrap_or_default();
    let flags = format::parse_model_flags(chunks.md20)?;
    let mut particle_emitters = m2_particle::parse_particle_emitters(chunks.md20);
    m2_particle::resolve_texture_fdids(&mut particle_emitters, &texture_fdids);
    if let Some(exp2) = chunks.exp2 {
        m2_particle::apply_exp2_z_sources(&mut particle_emitters, exp2);
    }
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
        bone_tracks: std::sync::Arc::new(bone_tracks),
        global_sequences,
        texture_types: format::parse_texture_types(chunks.md20)?,
        texture_flags: format::parser::parse_texture_flags(chunks.md20)?,
        texture_fdids,
        texture_lookup: format::parse_texture_lookup(chunks.md20)?,
        texture_unit_lookup: format::parse_texture_unit_lookup(chunks.md20)?,
        transparency_lookup: format::parse_transparency_lookup(chunks.md20)?,
        uv_animation_lookup: format::parse_uv_animation_lookup(chunks.md20)?,
        uses_texture_combiner_combos: flags & 0x8 != 0,
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
        particle_emitters,
        events,
        flags,
        lights: m2_light::parse_lights(chunks.md20),
    })
}

/// One submesh's triangles as engine vertex streams: each vertex it references once, in
/// first-reference order, with position and normal in engine axes (WoW `x, z, -y`), both
/// UV sets and four bone indices and weights; `indices` into those streams are wound
/// for counter-clockwise front faces, the reverse of M2's outward winding.
#[derive(Debug, PartialEq)]
pub struct SubmeshArrays {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub uv2: Vec<[f32; 2]>,
    pub bones: Vec<i32>,
    pub weights: Vec<f32>,
    pub indices: Vec<i32>,
}

pub fn submesh_arrays(model: &Model, sub: &Submesh) -> Result<SubmeshArrays, String> {
    let start = sub.triangle_start as usize;
    let end = start + sub.triangle_count as usize;
    let triangles = model
        .indices
        .get(start..end)
        .ok_or("Submesh indices out of bounds")?;
    if triangles.is_empty() || triangles.len() % 3 != 0 {
        return Err("Submesh has no complete triangles".into());
    }
    let engine = |[x, y, z]: [f32; 3]| [x, z, -y];
    let mut arrays = SubmeshArrays {
        positions: Vec::new(),
        normals: Vec::new(),
        uv: Vec::new(),
        uv2: Vec::new(),
        bones: Vec::new(),
        weights: Vec::new(),
        indices: Vec::with_capacity(triangles.len()),
    };
    let mut remap = std::collections::HashMap::<u16, i32>::new();
    for &global in triangles {
        if let Some(&local) = remap.get(&global) {
            arrays.indices.push(local);
            continue;
        }
        let vertex = model
            .vertices
            .get(global as usize)
            .ok_or_else(|| format!("Vertex {global} out of bounds"))?;
        let local = arrays.positions.len() as i32;
        arrays.positions.push(engine(vertex.position));
        arrays.normals.push(engine(vertex.normal));
        arrays.uv.push(vertex.tex_coords);
        arrays.uv2.push(vertex.tex_coords_2);
        for (&bone, &weight) in vertex.bone_indices.iter().zip(&vertex.bone_weights) {
            if weight > 0 && bone as usize >= model.bones.len() {
                return Err(format!("Vertex {global} references absent bone {bone}"));
            }
            arrays.bones.push(i32::from(bone));
            arrays.weights.push(f32::from(weight) / 255.0);
        }
        remap.insert(global, local);
        arrays.indices.push(local);
    }
    for triangle in arrays.indices.chunks_exact_mut(3) {
        triangle.swap(1, 2);
    }
    Ok(arrays)
}
