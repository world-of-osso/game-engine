use bevy::asset::{load_internal_asset, uuid_handle};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Face, RenderPipelineDescriptor, ShaderType};
use bevy::shader::{Shader, ShaderRef};

use crate::asset::m2_anim::{AnimTrack, evaluate_i16_track, evaluate_vec3_track};
use crate::asset::read_bytes::fixed16_to_f32;
use crate::m2_effect_material;

const SKYBOX_M2_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("c47ea355-9536-4557-8f9c-65ddd5d2047b");
const SKYBOX_PRIORITY_PLANE_BIAS_SCALE: f32 = 10_000.0;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SkyboxM2MaterialKey {
    two_sided: bool,
}

#[derive(ShaderType, Clone)]
pub struct SkyboxM2Settings {
    pub color: Vec4,
    pub transparency: f32,
    pub alpha_test: f32,
    pub combine_mode: u32,
    pub blend_mode: u32,
    pub uv_mode_1: u32,
    pub uv_mode_2: u32,
    pub uv_mode_3: u32,
    pub uv_mode_4: u32,
    pub render_flags: u32,
    pub has_second_texture: u32,
    pub has_third_texture: u32,
    pub has_fourth_texture: u32,
    pub uv_offset_1: Vec2,
    pub uv_offset_2: Vec2,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[bind_group_data(SkyboxM2MaterialKey)]
pub struct SkyboxM2Material {
    #[uniform(0)]
    pub settings: SkyboxM2Settings,
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub second_texture: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    pub third_texture: Handle<Image>,
    #[texture(7)]
    #[sampler(8)]
    pub fourth_texture: Handle<Image>,
    pub blend_mode: u16,
    pub two_sided: bool,
    pub priority_plane: i8,
    pub material_layer: u16,
    pub default_sequence_index: u32,
    pub global_sequences: Vec<u32>,
    pub transparency_anim: Option<AnimTrack<i16>>,
    pub color_opacity_anim: Option<AnimTrack<i16>>,
    pub texture_anim_1: Option<AnimTrack<[f32; 3]>>,
    pub texture_anim_2: Option<AnimTrack<[f32; 3]>>,
}

impl From<&SkyboxM2Material> for SkyboxM2MaterialKey {
    fn from(material: &SkyboxM2Material) -> Self {
        Self {
            two_sided: material.two_sided,
        }
    }
}

impl Material for SkyboxM2Material {
    fn fragment_shader() -> ShaderRef {
        SKYBOX_M2_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        skybox_alpha_mode_for_blend(self.blend_mode)
    }

    fn depth_bias(&self) -> f32 {
        skybox_sort_bias(self.priority_plane, self.material_layer)
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        configure_skybox_pipeline(descriptor, key.bind_group_data.two_sided);
        Ok(())
    }
}

fn configure_skybox_pipeline(descriptor: &mut RenderPipelineDescriptor, two_sided: bool) {
    // Match normal M2 semantics: authored two-sided batches disable culling, while
    // single-sided batches keep default backface culling like reference clients.
    descriptor.primitive.cull_mode = if two_sided { None } else { Some(Face::Back) };
    if let Some(ds) = descriptor.depth_stencil.as_mut() {
        ds.depth_write_enabled = false;
    }
}

fn skybox_alpha_mode_for_blend(blend_mode: u16) -> AlphaMode {
    m2_effect_material::alpha_mode_for_blend(blend_mode)
}

fn skybox_sort_bias(priority_plane: i8, material_layer: u16) -> f32 {
    priority_plane as f32 * SKYBOX_PRIORITY_PLANE_BIAS_SCALE + material_layer as f32
}

pub struct SkyboxM2MaterialPlugin;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkyboxTimeOverrideMs(pub u32);

impl Plugin for SkyboxM2MaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SKYBOX_M2_SHADER_HANDLE,
            "../../../assets/shaders/m2_skybox.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(MaterialPlugin::<SkyboxM2Material>::default())
            .add_systems(Update, update_skybox_uvs);
    }
}

fn looped_track_time_ms<T>(track: &AnimTrack<T>, seq_idx: usize, elapsed_ms: u32) -> u32 {
    let Some((timestamps, _)) = track.sequences.get(seq_idx) else {
        return elapsed_ms;
    };
    let Some(last_timestamp) = timestamps.last().copied() else {
        return elapsed_ms;
    };
    if last_timestamp == 0 {
        0
    } else {
        elapsed_ms % (last_timestamp + 1)
    }
}

fn skybox_track_seq_idx<T>(track: &AnimTrack<T>, preferred_seq_idx: usize) -> usize {
    if track.sequences.is_empty() {
        0
    } else {
        preferred_seq_idx.min(track.sequences.len() - 1)
    }
}

fn skybox_track_time_ms<T>(
    track: &AnimTrack<T>,
    seq_idx: usize,
    elapsed_ms: u32,
    global_sequences: &[u32],
) -> u32 {
    if track.global_sequence >= 0
        && let Some(duration) = global_sequences
            .get(track.global_sequence as usize)
            .copied()
        && duration > 0
    {
        return elapsed_ms % duration;
    }
    looped_track_time_ms(track, seq_idx, elapsed_ms)
}

fn evaluate_skybox_opacity_track(
    track: &AnimTrack<i16>,
    preferred_seq_idx: usize,
    global_sequences: &[u32],
    time_ms: u32,
) -> f32 {
    let seq_idx = skybox_track_seq_idx(track, preferred_seq_idx);
    evaluate_i16_track(
        track,
        seq_idx,
        skybox_track_time_ms(track, seq_idx, time_ms, global_sequences),
    )
    .map(|value| fixed16_to_f32(value).clamp(0.0, 1.0))
    .unwrap_or(1.0)
}

fn evaluate_skybox_transparency(material: &SkyboxM2Material, time_ms: u32) -> f32 {
    let preferred_seq_idx = material.default_sequence_index as usize;
    let texture_weight = material
        .transparency_anim
        .as_ref()
        .map(|track| {
            evaluate_skybox_opacity_track(
                track,
                preferred_seq_idx,
                &material.global_sequences,
                time_ms,
            )
        })
        .unwrap_or(1.0);
    let color_opacity = material
        .color_opacity_anim
        .as_ref()
        .map(|track| {
            evaluate_skybox_opacity_track(
                track,
                preferred_seq_idx,
                &material.global_sequences,
                time_ms,
            )
        })
        .unwrap_or(1.0);
    (texture_weight * color_opacity).clamp(0.0, 1.0)
}

fn evaluate_skybox_uv_offsets(material: &SkyboxM2Material, time_ms: u32) -> (Vec2, Vec2) {
    let preferred_seq_idx = material.default_sequence_index as usize;
    let uv_offset_1 = material
        .texture_anim_1
        .as_ref()
        .and_then(|track| {
            let seq_idx = skybox_track_seq_idx(track, preferred_seq_idx);
            evaluate_vec3_track(
                track,
                seq_idx,
                skybox_track_time_ms(track, seq_idx, time_ms, &material.global_sequences),
            )
        })
        .map(|offset| Vec2::new(offset[0], offset[1]))
        .unwrap_or(Vec2::ZERO);
    let uv_offset_2 = material
        .texture_anim_2
        .as_ref()
        .and_then(|track| {
            let seq_idx = skybox_track_seq_idx(track, preferred_seq_idx);
            evaluate_vec3_track(
                track,
                seq_idx,
                skybox_track_time_ms(track, seq_idx, time_ms, &material.global_sequences),
            )
        })
        .map(|offset| Vec2::new(offset[0], offset[1]))
        .unwrap_or(Vec2::ZERO);
    (uv_offset_1, uv_offset_2)
}

fn update_skybox_uvs(
    time: Res<Time>,
    time_override: Option<Res<SkyboxTimeOverrideMs>>,
    mut materials: ResMut<Assets<SkyboxM2Material>>,
) {
    let time_ms = time_override
        .as_deref()
        .map(|override_ms| override_ms.0)
        .unwrap_or_else(|| (time.elapsed_secs_f64() * 1000.0) as u32);
    for (_id, material) in materials.iter_mut() {
        let (uv_offset_1, uv_offset_2) = evaluate_skybox_uv_offsets(material, time_ms);
        material.settings.transparency = evaluate_skybox_transparency(material, time_ms);
        material.settings.uv_offset_1 = uv_offset_1;
        material.settings.uv_offset_2 = uv_offset_2;
    }
}

#[cfg(test)]
#[path = "skybox_m2_material_tests.rs"]
mod tests;
