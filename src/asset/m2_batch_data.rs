//! Renderer-independent authored M2 batch resolution, shared by Bevy and Godot.
use crate::asset::m2_format::fixed16_to_f32;
use crate::asset::m2_format::m2_anim;
use crate::asset::m2_format::parser::{M2TextureUnit, TextureTables};
use crate::asset::m2_texture;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlayScale {
    None,
    Uniform2x,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TextureOverlay {
    pub fdid: u32,
    pub x: u32,
    pub y: u32,
    pub scale: OverlayScale,
}

#[derive(Clone)]
pub struct ResolvedBatch {
    pub source_unit_index: usize,
    pub submesh_index: usize,
    pub mesh_part_id: u16,
    pub texture_fdid: Option<u32>,
    pub texture_2_fdid: Option<u32>,
    pub extra_texture_fdids: Vec<u32>,
    pub texture_type: Option<u32>,
    pub overlays: Vec<TextureOverlay>,
    pub render_flags: u16,
    pub blend_mode: u16,
    pub transparency: f32,
    pub transparency_track_index: Option<usize>,
    pub color_opacity_track_index: Option<usize>,
    pub transparency_anim: Option<m2_anim::AnimTrack<i16>>,
    pub color_opacity_anim: Option<m2_anim::AnimTrack<i16>>,
    pub texture_anim: Option<m2_anim::AnimTrack<[f32; 3]>>,
    pub texture_anim_2: Option<m2_anim::AnimTrack<[f32; 3]>>,
    pub use_uv_2_1: bool,
    pub use_uv_2_2: bool,
    pub use_env_map_2: bool,
    pub shader_id: u16,
    pub texture_count: u16,
    pub uses_texture_combiner_combos: bool,
    pub priority_plane: i8,
    pub material_layer: u16,
}

pub struct BatchInputs<'a> {
    pub units: &'a [M2TextureUnit],
    pub mesh_part_ids: &'a [u16],
    pub materials: &'a [(u16, u16)],
    pub tex: TextureTables<'a>,
    pub color_tracks: &'a [m2_anim::ColorAnimTracks],
    pub transparencies: &'a [m2_anim::AnimTrack<i16>],
    pub transparency_lookup: &'a [i16],
    pub texture_animations: &'a [m2_anim::TextureAnimTracks],
    pub uv_animation_lookup: &'a [i16],
    pub texture_unit_lookup: &'a [i16],
    pub uses_texture_combiner_combos: bool,
    pub is_hd: bool,
    pub keep_zero_opacity_batches: bool,
}

fn texture_looks_like_environment_map(
    fdid: Option<u32>,
    path_for_fdid: &impl Fn(u32) -> Option<String>,
) -> bool {
    let Some(path) = fdid.and_then(path_for_fdid) else {
        return false;
    };
    let lower = path.to_ascii_lowercase();
    lower.contains("armorreflect") || lower.contains("_reflect") || lower.contains("envmap")
}

pub(crate) fn resolve_uv_flags(
    unit: &M2TextureUnit,
    lookups: &[i16],
    second: Option<u32>,
    path_for_fdid: &impl Fn(u32) -> Option<String>,
) -> (bool, bool, bool) {
    let first = lookups.get(unit.texture_coord_index as usize).copied() == Some(2);
    if unit.texture_count <= 1 {
        return (first, false, false);
    }
    if lookups.is_empty() {
        let second_uv =
            unit.shader_id & (0x8000 | 0x80 | 0x08) == 0 && unit.shader_id & 0x4000 != 0;
        return (
            first,
            second_uv,
            texture_looks_like_environment_map(second, path_for_fdid),
        );
    }
    let second_lookup = lookups
        .get(unit.texture_coord_index.saturating_add(1) as usize)
        .copied();
    (
        first,
        second_lookup == Some(2),
        second_lookup == Some(0) || second_lookup == Some(-1),
    )
}

fn resolve_texture_anims(
    inputs: &BatchInputs<'_>,
    unit: &M2TextureUnit,
) -> (
    Option<m2_anim::AnimTrack<[f32; 3]>>,
    Option<m2_anim::AnimTrack<[f32; 3]>>,
) {
    let resolve = |id: u16| {
        inputs
            .uv_animation_lookup
            .get(id as usize)
            .copied()
            .and_then(|idx| usize::try_from(idx).ok())
            .and_then(|idx| inputs.texture_animations.get(idx))
            .map(|tracks| tracks.translation.clone())
    };
    (
        resolve(unit.texture_animation_id),
        if unit.texture_count > 1 {
            resolve(unit.texture_animation_id.saturating_add(1))
        } else {
            None
        },
    )
}

fn resolve_opacity(
    inputs: &BatchInputs<'_>,
    unit: &M2TextureUnit,
) -> (
    f32,
    Option<usize>,
    Option<usize>,
    Option<m2_anim::AnimTrack<i16>>,
    Option<m2_anim::AnimTrack<i16>>,
) {
    let idx = inputs
        .transparency_lookup
        .get(unit.transparency_index as usize)
        .copied()
        .unwrap_or(unit.transparency_index as i16);
    let transparency_track_index = usize::try_from(idx.max(0)).ok();
    let color_opacity_track_index = usize::try_from(unit.color_index).ok();
    let transparency_anim = transparency_track_index
        .and_then(|i| inputs.transparencies.get(i))
        .cloned();
    let color_opacity_anim = color_opacity_track_index
        .and_then(|i| inputs.color_tracks.get(i))
        .map(|t| t.opacity.clone());
    let sample = |track: &Option<m2_anim::AnimTrack<i16>>| {
        track
            .as_ref()
            .and_then(|t| m2_anim::evaluate_i16_track(t, 0, 0))
            .map(|v| fixed16_to_f32(v).clamp(0.0, 1.0))
            .unwrap_or(1.0)
    };
    (
        sample(&transparency_anim) * sample(&color_opacity_anim),
        transparency_track_index,
        color_opacity_track_index,
        transparency_anim,
        color_opacity_anim,
    )
}

pub fn resolve_batches(
    inputs: &BatchInputs<'_>,
    path_for_fdid: impl Fn(u32) -> Option<String>,
) -> Result<Vec<ResolvedBatch>, String> {
    let mut batches = Vec::with_capacity(inputs.units.len());
    for (source_unit_index, unit) in inputs.units.iter().enumerate() {
        let submesh_index = unit.submesh_index as usize;
        let mesh_part_id = *inputs.mesh_part_ids.get(submesh_index).ok_or_else(|| {
            format!(
                "Batch submesh_index {submesh_index} >= submesh count {}",
                inputs.mesh_part_ids.len()
            )
        })?;
        let (texture_fdid, texture_2_fdid, extra_texture_fdids, overlays) =
            m2_texture::resolve_batch_fdid_and_overlays(unit, &inputs.tex, inputs.is_hd);
        let texture_type =
            m2_texture::batch_texture_type(unit, inputs.tex.tex_lookup, inputs.tex.tex_types);
        let (
            transparency,
            transparency_track_index,
            color_opacity_track_index,
            transparency_anim,
            color_opacity_anim,
        ) = resolve_opacity(inputs, unit);
        if transparency <= 0.0 && !inputs.keep_zero_opacity_batches {
            continue;
        }
        let (texture_anim, texture_anim_2) = resolve_texture_anims(inputs, unit);
        let (use_uv_2_1, use_uv_2_2, use_env_map_2) = resolve_uv_flags(
            unit,
            inputs.texture_unit_lookup,
            texture_2_fdid,
            &path_for_fdid,
        );
        let (render_flags, blend_mode) = inputs
            .materials
            .get(unit.render_flags_index as usize)
            .copied()
            .unwrap_or_default();
        batches.push(ResolvedBatch {
            source_unit_index,
            submesh_index,
            mesh_part_id,
            texture_fdid,
            texture_2_fdid,
            extra_texture_fdids,
            texture_type,
            overlays,
            render_flags,
            blend_mode,
            transparency,
            transparency_track_index,
            color_opacity_track_index,
            transparency_anim,
            color_opacity_anim,
            texture_anim,
            texture_anim_2,
            use_uv_2_1,
            use_uv_2_2,
            use_env_map_2,
            shader_id: unit.shader_id,
            texture_count: unit.texture_count,
            uses_texture_combiner_combos: inputs.uses_texture_combiner_combos,
            priority_plane: unit.priority_plane,
            material_layer: unit.material_layer,
        });
    }
    batches.sort_by_key(|batch| (batch.priority_plane, batch.material_layer));
    Ok(batches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::m2_format::{parse_chunks, parse_texture_unit_lookup};

    fn unit(texture_count: u16) -> M2TextureUnit {
        M2TextureUnit {
            flags: 0,
            priority_plane: 0,
            shader_id: 0,
            submesh_index: 0,
            color_index: -1,
            render_flags_index: 0,
            material_layer: 0,
            texture_count,
            texture_id: 0,
            texture_coord_index: 0,
            transparency_index: 0,
            texture_animation_id: 0,
        }
    }

    #[test]
    fn authored_uv_lookup_and_empty_lookup_shader_route_textures() {
        let mut batch = unit(2);
        batch.shader_id = 0x4014;
        assert_eq!(
            resolve_uv_flags(&batch, &[], Some(4661390), &|_| None),
            (false, true, false)
        );
        assert_eq!(
            resolve_uv_flags(&batch, &[1, 2], None, &|_| None),
            (false, true, false)
        );
        assert_eq!(
            resolve_uv_flags(&batch, &[2, 0], None, &|_| None),
            (true, false, true)
        );
        assert_eq!(
            resolve_uv_flags(&batch, &[1, -1], None, &|_| None),
            (false, false, true)
        );
        assert_eq!(
            resolve_uv_flags(&unit(1), &[2, 0], None, &|_| None),
            (true, false, false)
        );
        assert_eq!(
            resolve_uv_flags(&batch, &[], Some(99), &|_| Some(
                "item_armorReflect.blp".into()
            )),
            (false, true, true)
        );
    }

    #[test]
    fn real_skybox_has_empty_texture_unit_lookup() {
        let data = std::fs::read("data/models/skyboxes/11xp_cloudsky01.m2").unwrap();
        let chunks = parse_chunks(&data).unwrap();
        assert!(parse_texture_unit_lookup(chunks.md20).unwrap().is_empty());
    }
}
