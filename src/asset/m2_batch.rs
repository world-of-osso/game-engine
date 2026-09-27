//! Bevy geometry adapter for renderer-independent authored M2 batch decisions.
use super::{
    M2Material, M2RenderBatch, M2Vertex, SkinData, TextureTables, build_batch_mesh, build_mesh,
    resolve_indices,
};
use crate::asset::m2_anim;
use crate::asset::m2_batch_data::{self, BatchInputs, ResolvedBatch};
use crate::asset::m2_texture;

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

pub(super) fn build_batched_model(
    ctx: &BatchBuildContext<'_>,
) -> Result<Vec<M2RenderBatch>, String> {
    let mesh_part_ids: Vec<_> = ctx
        .skin
        .submeshes
        .iter()
        .map(|sub| sub.mesh_part_id)
        .collect();
    let materials: Vec<_> = ctx
        .materials
        .iter()
        .map(|mat| (mat.flags, mat.blend_mode))
        .collect();
    let data = m2_batch_data::resolve_batches(
        &BatchInputs {
            units: &ctx.skin.batches,
            mesh_part_ids: &mesh_part_ids,
            materials: &materials,
            tex: TextureTables {
                tex_lookup: ctx.tex.tex_lookup,
                tex_types: ctx.tex.tex_types,
                txid: ctx.tex.txid,
                skin_fdids: ctx.tex.skin_fdids,
            },
            color_tracks: ctx.color_tracks,
            transparencies: ctx.transparencies,
            transparency_lookup: ctx.transparency_lookup,
            texture_animations: ctx.texture_animations,
            uv_animation_lookup: ctx.uv_animation_lookup,
            texture_unit_lookup: ctx.texture_unit_lookup,
            uses_texture_combiner_combos: ctx.uses_texture_combiner_combos,
            is_hd: ctx.is_hd,
            keep_zero_opacity_batches: ctx.keep_zero_opacity_batches,
        },
        |fdid| game_engine::listfile::lookup_fdid(fdid).map(str::to_owned),
    )?;
    Ok(data
        .into_iter()
        .map(|data| {
            let mesh = build_batch_mesh(
                ctx.vertices,
                &ctx.skin.lookup,
                &ctx.skin.indices,
                &ctx.skin.submeshes[data.submesh_index],
                ctx.has_bones,
            );
            M2RenderBatch { mesh, data }
        })
        .collect())
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
        data: ResolvedBatch {
            source_unit_index: 0,
            submesh_index: 0,
            mesh_part_id: 0,
            texture_fdid: fdid,
            texture_2_fdid: None,
            extra_texture_fdids: Vec::new(),
            texture_type: None,
            overlays: Vec::new(),
            render_flags: 0,
            blend_mode: 0,
            transparency: 1.0,
            transparency_track_index: None,
            color_opacity_track_index: None,
            transparency_anim: None,
            color_opacity_anim: None,
            texture_anim: None,
            texture_anim_2: None,
            use_uv_2_1: false,
            use_uv_2_2: false,
            use_env_map_2: false,
            shader_id: 0,
            texture_count: 1,
            uses_texture_combiner_combos: false,
            priority_plane: 0,
            material_layer: 0,
        },
    }])
}
