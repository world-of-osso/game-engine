//! Batch building for M2 render geometry.
//! This is a private submodule of `m2` — `super` refers to `m2`, `super::super` to `asset`.

use bevy::prelude::Mesh;

use super::{
    M2Material, M2RenderBatch, M2TextureUnit, M2Vertex, SkinData, TextureTables, build_batch_mesh,
    build_mesh, resolve_indices,
};
use crate::asset::m2_anim;
use crate::asset::m2_format::fixed16_to_f32;
use crate::asset::m2_texture;

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
            .map(|v| fixed16_to_f32(v).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        let color_opacity = usize::try_from(unit.color_index)
            .ok()
            .and_then(|idx| self.color_tracks.get(idx))
            .and_then(|tracks| m2_anim::evaluate_i16_track(&tracks.opacity, 0, 0))
            .map(|v| fixed16_to_f32(v).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        transparency * color_opacity
    }
}

struct BatchUvFlags<'a> {
    texture_unit_lookup: &'a [i16],
}

fn lookup_uses_uv1(lookup: Option<i16>) -> bool {
    lookup == Some(2)
}

fn lookup_uses_env_map(lookup: Option<i16>) -> bool {
    lookup == Some(0) || lookup == Some(-1)
}

impl BatchUvFlags<'_> {
    fn evaluate(&self, unit: &M2TextureUnit, texture_2_fdid: Option<u32>) -> (bool, bool, bool) {
        let use_uv_2_1 = lookup_uses_uv1(
            self.texture_unit_lookup
                .get(unit.texture_coord_index as usize)
                .copied(),
        );
        let (use_uv_2_2, use_env_map_2) = if unit.texture_count > 1 {
            if self.texture_unit_lookup.is_empty() {
                (false, texture_looks_like_environment_map(texture_2_fdid))
            } else {
                let lookup = self
                    .texture_unit_lookup
                    .get(unit.texture_coord_index.saturating_add(1) as usize)
                    .copied();
                (lookup_uses_uv1(lookup), lookup_uses_env_map(lookup))
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

pub(super) struct BatchBuildContext<'a> {
    pub(super) vertices: &'a [M2Vertex],
    pub(super) skin: &'a SkinData,
    pub(super) materials: &'a [M2Material],
    pub(super) tex: &'a TextureTables<'a>,
    pub(super) color_tracks: &'a [m2_anim::ColorAnimTracks],
    pub(super) transparencies: &'a [m2_anim::AnimTrack<i16>],
    pub(super) transparency_lookup: &'a [i16],
    pub(super) texture_animations: &'a [m2_anim::TextureAnimTracks],
    pub(super) uv_animation_lookup: &'a [i16],
    pub(super) texture_unit_lookup: &'a [i16],
    pub(super) uses_texture_combiner_combos: bool,
    pub(super) has_bones: bool,
    pub(super) is_hd: bool,
    pub(super) keep_zero_opacity_batches: bool,
}

pub(super) fn build_one_batch(
    ctx: &BatchBuildContext<'_>,
    unit: &M2TextureUnit,
) -> Result<Option<M2RenderBatch>, String> {
    let (sub, mesh) = build_batch_geometry(ctx, unit)?;
    let texture = resolve_batch_texture(unit, ctx);
    let opacity = build_batch_opacity(ctx);
    let transparency = opacity.evaluate(unit);
    if transparency <= 0.0 && !ctx.keep_zero_opacity_batches {
        return Ok(None);
    }
    let texture_anims = resolve_batch_texture_anims(ctx, unit);
    let uv_flags = resolve_batch_uv_flags(ctx, unit, texture.texture_2_fdid);
    Ok(Some(build_render_batch(
        ctx,
        BatchRenderInputs {
            unit,
            sub,
            mesh,
            texture,
            transparency,
            texture_anims,
            uv_flags,
        },
    )))
}

fn build_batch_geometry<'a>(
    ctx: &'a BatchBuildContext<'_>,
    unit: &M2TextureUnit,
) -> Result<(&'a super::M2Submesh, Mesh), String> {
    let sub_idx = unit.submesh_index as usize;
    if sub_idx >= ctx.skin.submeshes.len() {
        return Err(format!(
            "Batch submesh_index {sub_idx} >= submesh count {}",
            ctx.skin.submeshes.len()
        ));
    }
    let sub = &ctx.skin.submeshes[sub_idx];
    let mesh = build_batch_mesh(
        ctx.vertices,
        &ctx.skin.lookup,
        &ctx.skin.indices,
        sub,
        ctx.has_bones,
    );
    Ok((sub, mesh))
}

struct BatchTexture {
    texture_type: Option<u32>,
    texture_fdid: Option<u32>,
    texture_2_fdid: Option<u32>,
    extra_texture_fdids: Vec<u32>,
    overlays: Vec<super::TextureOverlay>,
}

fn resolve_batch_texture(unit: &M2TextureUnit, ctx: &BatchBuildContext<'_>) -> BatchTexture {
    let texture_type = m2_texture::batch_texture_type(unit, ctx.tex.tex_lookup, ctx.tex.tex_types);
    let (texture_fdid, texture_2_fdid, extra_texture_fdids, overlays) =
        m2_texture::resolve_batch_fdid_and_overlays(unit, ctx.tex, ctx.is_hd);
    BatchTexture {
        texture_type,
        texture_fdid,
        texture_2_fdid,
        extra_texture_fdids,
        overlays,
    }
}

fn build_batch_opacity<'a>(ctx: &'a BatchBuildContext<'_>) -> BatchOpacity<'a> {
    BatchOpacity {
        color_tracks: ctx.color_tracks,
        transparencies: ctx.transparencies,
        transparency_lookup: ctx.transparency_lookup,
    }
}

fn resolve_batch_texture_anims(
    ctx: &BatchBuildContext<'_>,
    unit: &M2TextureUnit,
) -> TextureAnimPair {
    resolve_texture_anims(ctx.texture_animations, ctx.uv_animation_lookup, unit)
}

fn resolve_batch_uv_flags(
    ctx: &BatchBuildContext<'_>,
    unit: &M2TextureUnit,
    texture_2_fdid: Option<u32>,
) -> (bool, bool, bool) {
    let uv_flags = BatchUvFlags {
        texture_unit_lookup: ctx.texture_unit_lookup,
    };
    uv_flags.evaluate(unit, texture_2_fdid)
}

struct BatchRenderInputs<'a> {
    unit: &'a M2TextureUnit,
    sub: &'a super::M2Submesh,
    mesh: Mesh,
    texture: BatchTexture,
    transparency: f32,
    texture_anims: TextureAnimPair,
    uv_flags: (bool, bool, bool),
}

fn build_render_batch(ctx: &BatchBuildContext<'_>, inputs: BatchRenderInputs<'_>) -> M2RenderBatch {
    let mat = ctx.materials.get(inputs.unit.render_flags_index as usize);
    let (texture_anim, texture_anim_2) = inputs.texture_anims;
    let (use_uv_2_1, use_uv_2_2, use_env_map_2) = inputs.uv_flags;
    M2RenderBatch {
        mesh: inputs.mesh,
        texture_fdid: inputs.texture.texture_fdid,
        texture_2_fdid: inputs.texture.texture_2_fdid,
        extra_texture_fdids: inputs.texture.extra_texture_fdids,
        texture_type: inputs.texture.texture_type,
        overlays: inputs.texture.overlays,
        render_flags: mat.map(|m| m.flags).unwrap_or(0),
        blend_mode: mat.map(|m| m.blend_mode).unwrap_or(0),
        transparency: inputs.transparency,
        texture_anim,
        texture_anim_2,
        use_uv_2_1,
        use_uv_2_2,
        use_env_map_2,
        shader_id: inputs.unit.shader_id,
        texture_count: inputs.unit.texture_count,
        uses_texture_combiner_combos: ctx.uses_texture_combiner_combos,
        mesh_part_id: inputs.sub.mesh_part_id,
    }
}

pub(super) fn build_batched_model(
    ctx: &BatchBuildContext<'_>,
) -> Result<Vec<M2RenderBatch>, String> {
    let mut batches = Vec::with_capacity(ctx.skin.batches.len());
    for unit in &ctx.skin.batches {
        let batch = build_one_batch(ctx, unit)?;
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
        extra_texture_fdids: Vec::new(),
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
        uses_texture_combiner_combos: false,
        mesh_part_id: 0,
    }])
}

#[cfg(test)]
mod tests {
    use super::{BatchUvFlags, lookup_uses_env_map, lookup_uses_uv1};
    use crate::asset::m2::M2TextureUnit;
    use crate::asset::m2_format::{parse_chunks, parse_texture_unit_lookup};
    use std::collections::BTreeSet;

    fn test_unit(texture_count: u16, texture_coord_index: u16) -> M2TextureUnit {
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
            texture_coord_index,
            transparency_index: 0,
            texture_animation_id: 0,
        }
    }

    #[test]
    fn lookup_value_two_selects_second_uv_channel() {
        assert!(lookup_uses_uv1(Some(2)));
        assert!(!lookup_uses_uv1(Some(1)));
        assert!(!lookup_uses_uv1(Some(0)));
    }

    #[test]
    fn lookup_value_zero_marks_environment_map() {
        assert!(lookup_uses_env_map(Some(0)));
        assert!(lookup_uses_env_map(Some(-1)));
        assert!(!lookup_uses_env_map(Some(1)));
        assert!(!lookup_uses_env_map(Some(2)));
    }

    #[test]
    fn texture_unit_lookup_interprets_first_and_second_uv_channels_correctly() {
        let flags = BatchUvFlags {
            texture_unit_lookup: &[1, 2],
        };
        let unit = test_unit(2, 0);

        let (use_uv_2_1, use_uv_2_2, use_env_map_2) = flags.evaluate(&unit, None);

        assert!(
            !use_uv_2_1,
            "lookup value 1 should keep the base texture on UV0"
        );
        assert!(
            use_uv_2_2,
            "lookup value 2 should route the second texture to UV1"
        );
        assert!(!use_env_map_2);
    }

    #[test]
    fn authored_skybox_texture_unit_lookup_can_be_empty_in_modern_assets() {
        let mut observed = Vec::new();
        for path in [
            "data/models/skyboxes/11xp_cloudsky01.m2",
            "data/models/skyboxes/deathskybox.m2",
        ] {
            let data = std::fs::read(path).expect("read skybox m2");
            let chunks = parse_chunks(&data).expect("parse chunks");
            let lookups =
                parse_texture_unit_lookup(chunks.md20).expect("parse texture unit lookup");
            let unique: BTreeSet<_> = lookups.iter().copied().collect();
            eprintln!("{path} texture_unit_lookup unique={unique:?}");
            observed.push((path, unique));
        }
        assert!(
            observed
                .iter()
                .any(|(path, unique)| path.ends_with("11xp_cloudsky01.m2") && unique.is_empty()),
            "expected 11xp_cloudsky01.m2 to exercise the empty texture-unit-lookup fallback"
        );
    }
}
