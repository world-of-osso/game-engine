use bevy::prelude::*;

use crate::asset::m2_anim::{AnimTrack, evaluate_i16_track};
use game_engine::asset::read_bytes::fixed16_to_f32;

use super::{SkyboxDebugSetup, SpawnedSkyboxDebug};

pub(super) fn log_debug_skybox_spawn(setup: &SkyboxDebugSetup, spawned: &SpawnedSkyboxDebug) {
    let authored_light_params = setup
        .scene
        .as_ref()
        .and_then(|scene| scene.authored_light_params_id());
    let authored_light_skybox = setup
        .scene
        .as_ref()
        .and_then(|scene| scene.authored_light_skybox_id());
    info!(
        "skybox_debug_scene: resolved skybox {} via {} (scene={:?}, authored_light_params={:?}, authored_light_skybox={:?}, resolved_light_params={:?}, resolved_light_params_flags=0x{:X}, resolved_light_skybox={:?}, resolved_light_skybox_flags=0x{:X})",
        spawned.path.display(),
        spawned.source,
        setup
            .scene
            .as_ref()
            .map(|scene| (scene.id, scene.name.as_str())),
        authored_light_params,
        authored_light_skybox,
        spawned.light_params_id,
        spawned
            .light_params_flags
            .map(|flags| flags.bits())
            .unwrap_or(0),
        spawned.light_skybox_id,
        spawned
            .light_skybox_flags
            .map(|flags| flags.bits())
            .unwrap_or(0)
    );
    log_debug_skybox_alpha_tracks(&spawned.path);
}

fn log_debug_skybox_alpha_tracks(path: &std::path::Path) {
    let Some(model) = load_model_for_alpha_dump(path) else {
        return;
    };
    let default_sequence_index = default_sequence_index(&model);
    log_alpha_dump_header(path, &model);
    log_transparency_tracks(&model, default_sequence_index);
    log_debug_skybox_additive_batches(&model);
    log_batch_opacity_tracks(&model, default_sequence_index);
}

fn load_model_for_alpha_dump(path: &std::path::Path) -> Option<crate::asset::m2::M2Model> {
    let Ok(model) = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0]) else {
        warn!(
            "skybox_debug_scene: failed to load {} for alpha-track dump",
            path.display()
        );
        return None;
    };
    Some(model)
}

fn default_sequence_index(model: &crate::asset::m2::M2Model) -> usize {
    model
        .sequences
        .iter()
        .position(|sequence| sequence.id == 0)
        .unwrap_or(0)
}

fn log_alpha_dump_header(path: &std::path::Path, model: &crate::asset::m2::M2Model) {
    info!(
        "skybox_debug_scene: alpha dump {} transparency_tracks={} color_tracks={} batches={}",
        path.display(),
        model.transparency_tracks.len(),
        model.color_tracks.len(),
        model.batches.len()
    );
}

fn log_transparency_tracks(model: &crate::asset::m2::M2Model, default_sequence_index: usize) {
    for (track_index, track) in model.transparency_tracks.iter().enumerate() {
        info!(
            "skybox_debug_scene: transparency_track[{track_index}] {}",
            format_opacity_track_samples(track, default_sequence_index, &model.global_sequences)
        );
    }
}

fn log_batch_opacity_tracks(model: &crate::asset::m2::M2Model, default_sequence_index: usize) {
    for (batch_index, batch) in model.batches.iter().enumerate() {
        let transparency_track = format_optional_track_index(batch.transparency_track_index);
        let color_track = format_optional_track_index(batch.color_opacity_track_index);
        let transparency_samples = format_optional_opacity_samples(
            batch.transparency_anim.as_ref(),
            default_sequence_index,
            &model.global_sequences,
        );
        let color_samples = format_optional_opacity_samples(
            batch.color_opacity_anim.as_ref(),
            default_sequence_index,
            &model.global_sequences,
        );
        info!(
            "skybox_debug_scene: batch[{batch_index}] mesh_part_id={} blend={} flags=0x{:x} priority_plane={} layer={} transparency_track={} color_opacity_track={} transparency_samples={} color_samples={}",
            batch.mesh_part_id,
            batch.blend_mode,
            batch.render_flags,
            batch.priority_plane,
            batch.material_layer,
            transparency_track,
            color_track,
            transparency_samples,
            color_samples
        );
    }
}

fn format_optional_track_index(index: Option<usize>) -> String {
    index.map_or_else(|| "-".into(), |idx| idx.to_string())
}

fn format_optional_opacity_samples(
    track: Option<&AnimTrack<i16>>,
    default_sequence_index: usize,
    global_sequences: &[u32],
) -> String {
    track.map_or_else(
        || "none".into(),
        |track| format_opacity_track_samples(track, default_sequence_index, global_sequences),
    )
}

fn log_debug_skybox_additive_batches(model: &crate::asset::m2::M2Model) {
    let additive_batches = additive_batches(model);
    info!(
        "skybox_debug_scene: additive_batches={}",
        additive_batches.len()
    );
    for (batch_index, batch) in additive_batches {
        log_additive_batch_summary(model, batch_index, batch);
        log_additive_batch_geometry(model, batch_index, batch);
    }
}

fn additive_batches(
    model: &crate::asset::m2::M2Model,
) -> Vec<(usize, &crate::asset::m2::M2RenderBatch)> {
    model
        .batches
        .iter()
        .enumerate()
        .filter(|(_, batch)| batch.blend_mode == 4)
        .collect()
}

fn log_additive_batch_summary(
    model: &crate::asset::m2::M2Model,
    batch_index: usize,
    batch: &crate::asset::m2::M2RenderBatch,
) {
    let duplicate_count = duplicate_additive_priority_layer_count(model, batch);
    info!(
        "skybox_debug_scene: additive_batch[{batch_index}] mesh_part_id={} flags=0x{:x} priority_plane={} layer={} shader_id=0x{:x} texture_count={} use_uv_2_1={} use_uv_2_2={} use_env_map_2={} texture_anim={} texture_anim_2={} texture_fdid={:?} texture_2_fdid={:?} extra_textures={:?} duplicates_same_priority_layer={duplicate_count}",
        batch.mesh_part_id,
        batch.render_flags,
        batch.priority_plane,
        batch.material_layer,
        batch.shader_id,
        batch.texture_count,
        batch.use_uv_2_1,
        batch.use_uv_2_2,
        batch.use_env_map_2,
        batch.texture_anim.is_some(),
        batch.texture_anim_2.is_some(),
        batch.texture_fdid,
        batch.texture_2_fdid,
        batch.extra_texture_fdids
    );
}

fn duplicate_additive_priority_layer_count(
    model: &crate::asset::m2::M2Model,
    batch: &crate::asset::m2::M2RenderBatch,
) -> usize {
    model
        .batches
        .iter()
        .filter(|other| {
            other.blend_mode == batch.blend_mode
                && other.priority_plane == batch.priority_plane
                && other.material_layer == batch.material_layer
        })
        .count()
}

fn log_additive_batch_geometry(
    model: &crate::asset::m2::M2Model,
    batch_index: usize,
    batch: &crate::asset::m2::M2RenderBatch,
) {
    let triangle_count = match batch.mesh.indices() {
        Some(bevy::mesh::Indices::U16(indices)) => indices.len() / 3,
        Some(bevy::mesh::Indices::U32(indices)) => indices.len() / 3,
        None => 0,
    };
    let vertex_count = batch.mesh.count_vertices();
    let joint_summary = additive_batch_joint_summary(model, batch);
    info!(
        "skybox_debug_scene: additive_batch[{batch_index}] geometry verts={} tris={} joints=[{}]",
        vertex_count, triangle_count, joint_summary
    );
}

fn additive_batch_joint_summary(
    model: &crate::asset::m2::M2Model,
    batch: &crate::asset::m2::M2RenderBatch,
) -> String {
    let mut joint_indices = std::collections::BTreeSet::new();
    if let Some(bevy::mesh::VertexAttributeValues::Uint16x4(joints)) =
        batch.mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
    {
        for joint_set in joints {
            for joint in joint_set {
                joint_indices.insert(*joint as usize);
            }
        }
    }
    if joint_indices.is_empty() {
        return "none".to_string();
    }
    joint_indices
        .iter()
        .map(|joint_index| {
            let billboard = model
                .bones
                .get(*joint_index)
                .map(|bone| bone.flags & crate::animation::M2_BONE_SPHERICAL_BILLBOARD != 0)
                .unwrap_or(false);
            format!("{joint_index}:billboard={billboard}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn format_opacity_track_samples(
    track: &AnimTrack<i16>,
    default_sequence_index: usize,
    global_sequences: &[u32],
) -> String {
    let seq_idx = track_sequence_index(track, default_sequence_index);
    let Some((timestamps, _)) = track.sequences.get(seq_idx) else {
        return format!("global_seq={} seq={} empty", track.global_sequence, seq_idx);
    };
    let duration = track_duration(track, timestamps, global_sequences);
    let samples = format_opacity_samples(track, seq_idx, duration);
    format!(
        "global_seq={} seq={} keyframes={} duration={} [{}]",
        track.global_sequence,
        seq_idx,
        timestamps.len(),
        duration,
        samples
    )
}

fn track_sequence_index(track: &AnimTrack<i16>, default_sequence_index: usize) -> usize {
    if track.sequences.is_empty() {
        0
    } else {
        default_sequence_index.min(track.sequences.len() - 1)
    }
}

fn track_duration(track: &AnimTrack<i16>, timestamps: &[u32], global_sequences: &[u32]) -> u32 {
    if track.global_sequence >= 0 {
        return global_sequences
            .get(track.global_sequence as usize)
            .copied()
            .unwrap_or_else(|| timestamps.last().copied().unwrap_or(0).saturating_add(1));
    }
    timestamps.last().copied().unwrap_or(0).saturating_add(1)
}

fn format_opacity_samples(track: &AnimTrack<i16>, seq_idx: usize, duration: u32) -> String {
    [
        0_u32,
        duration / 4,
        duration / 2,
        duration.saturating_sub(1),
    ]
    .into_iter()
    .map(|time_ms| {
        let value = evaluate_i16_track(track, seq_idx, time_ms)
            .map(|raw| format!("{:.3}", fixed16_to_f32(raw).clamp(0.0, 1.0)))
            .unwrap_or_else(|| "none".into());
        format!("{time_ms}ms={value}")
    })
    .collect::<Vec<_>>()
    .join(",")
}
