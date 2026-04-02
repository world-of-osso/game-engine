//! Batch building for M2 render geometry.
//! This is a private submodule of `m2` — `super` refers to `m2`, `super::super` to `asset`.

use bevy::prelude::Mesh;

use super::super::m2_anim;
use super::super::m2_texture;
use super::{
    M2Material, M2RenderBatch, M2TextureUnit, M2Vertex, SkinData, TextureTables, build_batch_mesh,
    build_mesh, mesh_has_meaningful_uv1, resolve_indices,
};

fn texture_looks_like_environment_map(fdid: Option<u32>) -> bool {
    let Some(path) = fdid.and_then(game_engine::listfile::lookup_fdid) else {
        return false;
    };
    let lower = path.to_ascii_lowercase();
    lower.contains("armorreflect") || lower.contains("_reflect") || lower.contains("envmap")
}

struct BatchOpacity<'a> {
    color_tracks: &'a [m2_anim::ColorAnimTracks],
    transparencies: &'a [m2_anim::AnimTrack<i16>],
    transparency_lookup: &'a [i16],
}

impl BatchOpacity<'_> {
    fn evaluate(&self, unit: &M2TextureUnit) -> f32 {
        let track_idx = self
            .transparency_lookup
            .get(unit.transparency_index as usize)
            .copied()
            .unwrap_or(unit.transparency_index as i16);
        let transparency = self
            .transparencies
            .get(track_idx.max(0) as usize)
            .and_then(|t| m2_anim::evaluate_i16_track(t, 0, 0))
            .map(|v| (v as f32 / 32767.0).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        let color_opacity = usize::try_from(unit.color_index)
            .ok()
            .and_then(|idx| self.color_tracks.get(idx))
            .and_then(|tracks| m2_anim::evaluate_i16_track(&tracks.opacity, 0, 0))
            .map(|v| (v as f32 / 32767.0).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        transparency * color_opacity
    }
}

struct BatchUvFlags<'a> {
    texture_unit_lookup: &'a [i16],
}

impl BatchUvFlags<'_> {
    fn evaluate(
        &self,
        unit: &M2TextureUnit,
        mesh: &Mesh,
        texture_2_fdid: Option<u32>,
    ) -> (bool, bool, bool) {
        let use_uv_2_1 = self
            .texture_unit_lookup
            .get(unit.texture_coord_index as usize)
            .copied()
            == Some(1);
        let (use_uv_2_2, use_env_map_2) = if unit.texture_count > 1 {
            if self.texture_unit_lookup.is_empty() {
                (
                    mesh_has_meaningful_uv1(mesh),
                    texture_looks_like_environment_map(texture_2_fdid),
                )
            } else {
                let lookup = self
                    .texture_unit_lookup
                    .get(unit.texture_coord_index.saturating_add(1) as usize)
                    .copied();
                (lookup == Some(1), lookup == Some(-1))
            }
        } else {
            (false, false)
        };
        (use_uv_2_1, use_uv_2_2, use_env_map_2)
    }
}

fn resolve_texture_anims(
    texture_animations: &[m2_anim::TextureAnimTracks],
    uv_animation_lookup: &[i16],
    unit: &M2TextureUnit,
) -> TextureAnimPair {
    let resolve = |id: u16| {
        uv_animation_lookup
            .get(id as usize)
            .copied()
            .and_then(|idx| usize::try_from(idx).ok())
            .and_then(|idx| texture_animations.get(idx))
            .map(|tracks| tracks.translation.clone())
    };
    let anim_1 = resolve(unit.texture_animation_id);
    let anim_2 = if unit.texture_count > 1 {
        resolve(unit.texture_animation_id.saturating_add(1))
    } else {
        None
    };
    (anim_1, anim_2)
}

type TextureAnimPair = (
    Option<m2_anim::AnimTrack<[f32; 3]>>,
    Option<m2_anim::AnimTrack<[f32; 3]>>,
);

#[allow(clippy::too_many_arguments)]
pub(super) fn build_one_batch(
    vertices: &[M2Vertex],
    skin: &SkinData,
    materials: &[M2Material],
    tex: &TextureTables<'_>,
    color_tracks: &[m2_anim::ColorAnimTracks],
    transparencies: &[m2_anim::AnimTrack<i16>],
    transparency_lookup: &[i16],
    texture_animations: &[m2_anim::TextureAnimTracks],
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
    let (texture_fdid, texture_2_fdid, overlays) =
        m2_texture::resolve_batch_fdid_and_overlays(unit, tex, is_hd);
    let opacity = BatchOpacity {
        color_tracks,
        transparencies,
        transparency_lookup,
    };
    let transparency = opacity.evaluate(unit);
    if transparency <= 0.0 {
        return Ok(None);
    }
    let (texture_anim, texture_anim_2) =
        resolve_texture_anims(texture_animations, uv_animation_lookup, unit);
    let uv_flags = BatchUvFlags {
        texture_unit_lookup,
    };
    let (use_uv_2_1, use_uv_2_2, use_env_map_2) = uv_flags.evaluate(unit, &mesh, texture_2_fdid);
    let mat = materials.get(unit.render_flags_index as usize);
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

#[allow(clippy::too_many_arguments)]
pub(super) fn build_batched_model(
    vertices: &[M2Vertex],
    skin: &SkinData,
    materials: &[M2Material],
    tex: &TextureTables<'_>,
    color_tracks: &[m2_anim::ColorAnimTracks],
    transparencies: &[m2_anim::AnimTrack<i16>],
    transparency_lookup: &[i16],
    texture_animations: &[m2_anim::TextureAnimTracks],
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

pub(super) fn build_fallback_batch(
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
