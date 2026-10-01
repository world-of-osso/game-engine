//! Scene-one authored sky: M2 geometry and texture stages stay behind world depth.
use std::path::Path;

use game_engine_core::{
    asset::m2_format::m2_anim::{evaluate_i16_track, evaluate_vec3_track},
    m2,
    m2_batch_data::ResolvedBatch,
};
use godot::{
    classes::{
        ArrayMesh, ImageTexture, MeshInstance3D, Node3D, ResourceLoader, Shader, ShaderMaterial,
    },
    prelude::*,
};

use crate::{animation::WowAnimationPlayer, assets};

#[path = "sky_time.rs"]
mod sky_time;
use sky_time::fixed_sequence_phase_ms;

const SKY_FDID: u32 = 525142;
const SKY_NAME: &str = "costalislandskybox";
const SHADER_PATH: &str = "res://shaders/sky_m2.gdshader";
const RENDER_MODE: &str =
    "render_mode unshaded, fog_disabled, cull_back, blend_mix, depth_draw_never, shadows_disabled;";

struct AnimatedBatch {
    material: Gd<ShaderMaterial>,
    transparency: Option<m2::AnimTrack<i16>>,
    color_opacity: Option<m2::AnimTrack<i16>>,
    first_uv: Option<m2::AnimTrack<[f32; 3]>>,
    second_uv: Option<m2::AnimTrack<[f32; 3]>>,
}

pub(crate) struct Sky {
    pub node: Gd<Node3D>,
    animations: Vec<AnimatedBatch>,
    global_sequences: Vec<u32>,
    default_sequence_index: usize,
}

impl Sky {
    pub fn load(data_root: &Path) -> Result<Self, String> {
        let path = data_root
            .join("models/skyboxes")
            .join(format!("{SKY_NAME}.m2"));
        Self::load_model(data_root, &path, SKY_FDID, None)
    }

    pub fn load_model(
        data_root: &Path,
        model_path: &Path,
        fdid: u32,
        time_override_ms: Option<u32>,
    ) -> Result<Self, String> {
        let context = |error| format!("Sky FDID {fdid} at {}: {error}", model_path.display());
        if !model_path.is_file() {
            return Err(context("missing cached model".to_string()));
        }
        let source = GString::from(model_path.to_string_lossy().as_ref());
        let model = assets::read_model(&source).map_err(&context)?;
        let prepared = prepare_batches(&model, data_root).map_err(&context)?;
        let mut sky = assemble_sky(model, prepared, time_override_ms).map_err(&context)?;
        sky.node.set_name(&format!("AuthoredSky{fdid}"));
        sky.node
            .set_meta(assets::M2_SOURCE_META, &source.to_variant());
        Ok(sky)
    }

    pub fn sample(&mut self, time_ms: u32) {
        for batch in &mut self.animations {
            let transparency = opacity(
                batch.transparency.as_ref(),
                self.default_sequence_index,
                &self.global_sequences,
                time_ms,
            ) * opacity(
                batch.color_opacity.as_ref(),
                self.default_sequence_index,
                &self.global_sequences,
                time_ms,
            );
            let first = uv_offset(
                batch.first_uv.as_ref(),
                self.default_sequence_index,
                &self.global_sequences,
                time_ms,
            );
            let second = uv_offset(
                batch.second_uv.as_ref(),
                self.default_sequence_index,
                &self.global_sequences,
                time_ms,
            );
            batch
                .material
                .set_shader_parameter("transparency", &transparency.to_variant());
            batch
                .material
                .set_shader_parameter("uv_offset_1", &first.to_variant());
            batch
                .material
                .set_shader_parameter("uv_offset_2", &second.to_variant());
        }
    }
}

type PreparedBatch = (ResolvedBatch, Gd<ArrayMesh>, Gd<ShaderMaterial>);

fn prepare_batches(model: &m2::Model, data_root: &Path) -> Result<Vec<PreparedBatch>, String> {
    let batches = m2::resolve_render_batches(model, &[0; 3], true, |_| None)?;
    if batches.is_empty() {
        return Err("Sky has no render batches".into());
    }
    let shader = load_shader_source()?;
    let mut ordered = batches;
    ordered.sort_by_key(|batch| {
        (
            batch.priority_plane,
            batch.material_layer,
            batch.source_unit_index,
        )
    });
    if ordered.len() > 256 {
        return Err(format!(
            "Sky needs {} render priorities (maximum 256)",
            ordered.len()
        ));
    }
    ordered
        .into_iter()
        .enumerate()
        .map(|(order, batch)| {
            let submesh = model.submeshes.get(batch.submesh_index).ok_or_else(|| {
                format!(
                    "Sky batch {} references absent submesh",
                    batch.source_unit_index
                )
            })?;
            let mesh = assets::build_batch_mesh(model, submesh)?;
            let material = build_material(&batch, &shader, data_root, order as i32 - 128)?;
            Ok((batch, mesh, material))
        })
        .collect()
}

fn assemble_sky(
    model: m2::Model,
    prepared: Vec<PreparedBatch>,
    time_override_ms: Option<u32>,
) -> Result<Sky, String> {
    let (skeleton, skin) = assets::build_skeleton(&model.bones);
    let player = match load_player(&model, skeleton.clone(), time_override_ms) {
        Ok(player) => player,
        Err(error) => {
            skeleton.free();
            return Err(error);
        }
    };
    let mut node = Node3D::new_alloc();
    node.add_child(&skeleton);
    if let Some(mut player) = player {
        player.set_name("M2Animation");
        node.add_child(&player);
    }
    let animations = prepared
        .into_iter()
        .filter_map(|batch| attach_batch(&mut node, batch, skin.as_ref()))
        .collect();
    let mut sky = Sky {
        node,
        animations,
        default_sequence_index: default_sequence_index(&model),
        global_sequences: model.global_sequences,
    };
    sky.sample(time_override_ms.unwrap_or(0));
    Ok(sky)
}

fn default_sequence_index(model: &m2::Model) -> usize {
    model
        .sequences
        .iter()
        .position(|sequence| sequence.id == 0)
        .unwrap_or(0)
}

fn load_player(
    model: &m2::Model,
    skeleton: Gd<godot::classes::Skeleton3D>,
    time_override_ms: Option<u32>,
) -> Result<Option<Gd<WowAnimationPlayer>>, String> {
    if model.sequences.is_empty() {
        return Ok(None);
    }
    let mut player = WowAnimationPlayer::from_model(model, skeleton)?;
    if let Some(time_ms) = time_override_ms {
        let duration = model.sequences[default_sequence_index(model)].duration;
        let phase = fixed_sequence_phase_ms(time_ms, duration);
        let advanced = player.bind_mut().advance_time_ms(phase);
        if !advanced {
            player.free();
            return Err("Cannot sample fixed sky bone animation phase".into());
        }
        player.call("set_paused", &[true.to_variant()]);
    }
    Ok(Some(player))
}

fn attach_batch(
    node: &mut Gd<Node3D>,
    prepared: PreparedBatch,
    skin: Option<&Gd<godot::classes::Skin>>,
) -> Option<AnimatedBatch> {
    let (batch, mesh, material) = prepared;
    let mut instance = MeshInstance3D::new_alloc();
    instance.set_name(&format!("SkyBatch{}", batch.source_unit_index));
    instance.set_mesh(&mesh);
    instance.set_surface_override_material(0, &material);
    instance
        .set_cast_shadows_setting(godot::classes::geometry_instance_3d::ShadowCastingSetting::OFF);
    if let Some(skin) = skin {
        instance.set_skin(skin);
        instance.set_skeleton_path("../Skeleton3D");
    }
    node.add_child(&instance);
    animated_batch(batch, material)
}

fn animated_batch(batch: ResolvedBatch, material: Gd<ShaderMaterial>) -> Option<AnimatedBatch> {
    let has_tracks = [
        batch.transparency_anim.is_some(),
        batch.color_opacity_anim.is_some(),
        batch.texture_anim.is_some(),
        batch.texture_anim_2.is_some(),
    ]
    .into_iter()
    .any(|present| present);
    has_tracks.then_some(AnimatedBatch {
        material,
        transparency: batch.transparency_anim,
        color_opacity: batch.color_opacity_anim,
        first_uv: batch.texture_anim,
        second_uv: batch.texture_anim_2,
    })
}

fn track_sample_time<T>(
    track: &m2::AnimTrack<T>,
    preferred_index: usize,
    sequences: &[u32],
    elapsed_ms: u32,
) -> (usize, u32) {
    let index = preferred_index.min(track.sequences.len().saturating_sub(1));
    if let Ok(global_index) = usize::try_from(track.global_sequence) {
        if let Some(&duration) = sequences.get(global_index) {
            if duration > 0 {
                return (index, elapsed_ms % duration);
            }
        }
    }
    let end = track
        .sequences
        .get(index)
        .and_then(|(times, _)| times.last())
        .copied();
    (
        index,
        end.map_or(elapsed_ms, |last| {
            if last == 0 {
                0
            } else {
                elapsed_ms % last.saturating_add(1)
            }
        }),
    )
}

fn opacity(
    track: Option<&m2::AnimTrack<i16>>,
    preferred_index: usize,
    sequences: &[u32],
    time_ms: u32,
) -> f32 {
    track
        .and_then(|track| {
            let (index, time) = track_sample_time(track, preferred_index, sequences, time_ms);
            evaluate_i16_track(track, index, time)
        })
        .map(|value| (value as f32 / 32767.0).clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

fn uv_offset(
    track: Option<&m2::AnimTrack<[f32; 3]>>,
    preferred_index: usize,
    sequences: &[u32],
    time_ms: u32,
) -> Vector2 {
    track
        .and_then(|track| {
            let (index, time) = track_sample_time(track, preferred_index, sequences, time_ms);
            evaluate_vec3_track(track, index, time)
        })
        .map(|value| Vector2::new(value[0], value[1]))
        .unwrap_or(Vector2::ZERO)
}

fn load_shader_source() -> Result<String, String> {
    Ok(ResourceLoader::singleton()
        .load(SHADER_PATH)
        .ok_or_else(|| format!("Cannot load sky shader {SHADER_PATH}"))?
        .try_cast::<Shader>()
        .map_err(|_| format!("Sky shader {SHADER_PATH} is not Shader"))?
        .get_code()
        .to_string())
}

fn build_material(
    batch: &ResolvedBatch,
    source: &str,
    data_root: &Path,
    priority: i32,
) -> Result<Gd<ShaderMaterial>, String> {
    let cull = if batch.render_flags & 4 != 0 {
        "cull_disabled"
    } else {
        "cull_back"
    };
    let blend = if (4..=6).contains(&batch.blend_mode) {
        "blend_add"
    } else {
        "blend_mix"
    };
    let variant = format!(
        "render_mode unshaded, fog_disabled, {cull}, {blend}, depth_draw_never, shadows_disabled;"
    );
    if source.matches(RENDER_MODE).count() != 1 {
        return Err("Sky shader render mode signature changed".into());
    }
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&crate::assets::material::shared_shader(
        &source.replace(RENDER_MODE, &variant),
    ));
    material.set_render_priority(priority);
    let texture_dir = data_root.join("textures");
    let mut missing = PackedInt32Array::new();
    let first = load_stage(batch.texture_fdid, &texture_dir, &mut missing)?;
    let second = load_stage(batch.texture_2_fdid, &texture_dir, &mut missing)?;
    let third = load_stage(
        batch.extra_texture_fdids.first().copied(),
        &texture_dir,
        &mut missing,
    )?;
    let fourth = load_stage(
        batch.extra_texture_fdids.get(1).copied(),
        &texture_dir,
        &mut missing,
    )?;
    for (name, stage) in [
        ("base_texture", first.as_ref()),
        ("second_texture", second.as_ref().or(first.as_ref())),
        ("third_texture", third.as_ref().or(first.as_ref())),
        ("fourth_texture", fourth.as_ref().or(first.as_ref())),
    ] {
        if let Some(texture) = stage {
            material.set_shader_parameter(name, &texture.to_variant());
        }
    }
    material.set_shader_parameter("has_second_texture", &second.is_some().to_variant());
    material.set_shader_parameter("has_third_texture", &third.is_some().to_variant());
    material.set_shader_parameter("has_fourth_texture", &fourth.is_some().to_variant());
    material.set_shader_parameter("combine_mode", &(combine_mode(batch) as i32).to_variant());
    let (uv1, uv2, uv3, uv4) = match batch.shader_id {
        0x8012 => (0, 0, 0, 0),
        0x8016 => (0, 0, 0, 1),
        _ => (batch.use_uv_2_1 as i32, batch.use_uv_2_2 as i32, 0, 0),
    };
    for (name, mode) in [
        ("uv_mode_1", uv1),
        ("uv_mode_2", uv2),
        ("uv_mode_3", uv3),
        ("uv_mode_4", uv4),
    ] {
        material.set_shader_parameter(name, &mode.to_variant());
    }
    // Soft cloud edges: original sky path never applies generic M2 alpha cutouts.
    material.set_shader_parameter("alpha_test", &0.0f32.to_variant());
    Ok(material)
}

fn load_stage(
    fdid: Option<u32>,
    dir: &Path,
    missing: &mut PackedInt32Array,
) -> Result<Option<Gd<ImageTexture>>, String> {
    let Some(fdid) = fdid else {
        return Ok(None);
    };
    assets::material::shared_texture(fdid, dir, missing)?
        .ok_or_else(|| format!("Sky missing texture {fdid} at {}", dir.display()))
        .map(Some)
}

#[cfg(test)]
mod fixed_sky_track_tests {
    use super::*;

    fn translated_bone_at(time_ms: u32, duration_ms: u32) -> [f32; 3] {
        let track = m2::AnimTrack {
            interpolation_type: 1,
            global_sequence: -1,
            sequences: vec![(vec![0, 2000], vec![[0.0; 3], [20.0, 0.0, 0.0]])],
        };
        let phase = fixed_sequence_phase_ms(time_ms, duration_ms) as u32;
        evaluate_vec3_track(&track, 0, phase).unwrap()
    }

    #[test]
    fn fixed_positive_duration_samples_wrapped_concrete_bone_track() {
        assert_eq!(translated_bone_at(2500, 1000), [5.0, 0.0, 0.0]);
    }

    #[test]
    fn fixed_wide_time_samples_original_f32_phase_concrete_bone_track() {
        let position = translated_bone_at(u32::MAX, 1000);
        assert!((position[0] - 2.96).abs() < 0.00001);
        assert_eq!([position[1], position[2]], [0.0; 2]);
    }

    #[test]
    fn fixed_zero_duration_samples_requested_concrete_bone_track() {
        let position = translated_bone_at(1234, 0);
        assert!((position[0] - 12.34).abs() < 0.00001);
        assert_eq!([position[1], position[2]], [0.0; 2]);
    }
}

fn combine_mode(batch: &ResolvedBatch) -> u16 {
    if batch.uses_texture_combiner_combos || batch.shader_id & 0x8000 != 0 {
        return batch.shader_id;
    }
    let mode = [0, 1, 1, 1, 1, 5, 5]
        .get(batch.blend_mode as usize)
        .copied()
        .unwrap_or(1);
    if batch.texture_count > 1 {
        [0x7, 0xE, 0x15, 0x1C, 0x23, 0x23][mode]
    } else {
        [1, 2, 3, 4, 5, 6][mode]
    }
}
