use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Face, RenderPipelineDescriptor, ShaderType};
use bevy::shader::ShaderRef;

use crate::asset::m2_anim::{AnimTrack, evaluate_vec3_track};

#[derive(ShaderType, Clone)]
pub struct M2EffectSettings {
    pub transparency: f32,
    pub alpha_test: f32,
    pub shader_id: u32,
    pub blend_mode: u32,
    pub uv_mode_1: u32,
    pub uv_mode_2: u32,
    pub render_flags: u32,
    pub uv_offset_1: Vec2,
    pub uv_offset_2: Vec2,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct M2EffectMaterial {
    #[uniform(0)]
    pub settings: M2EffectSettings,
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub second_texture: Handle<Image>,
    pub blend_mode: u16,
    pub two_sided: bool,
    pub texture_anim_1: Option<AnimTrack<[f32; 3]>>,
    pub texture_anim_2: Option<AnimTrack<[f32; 3]>>,
}

impl Material for M2EffectMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/m2_effect.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        match self.blend_mode {
            1 => AlphaMode::Mask(224.0 / 255.0),
            2 | 3 | 7 => AlphaMode::Blend,
            4..=6 => AlphaMode::Add,
            _ => AlphaMode::Opaque,
        }
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Back);
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = false;
        }
        Ok(())
    }
}

pub struct M2EffectMaterialPlugin;

impl Plugin for M2EffectMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<M2EffectMaterial>::default())
            .add_systems(Update, update_m2_effect_uvs);
    }
}

fn update_m2_effect_uvs(time: Res<Time>, mut materials: ResMut<Assets<M2EffectMaterial>>) {
    let time_ms = (time.elapsed_secs_f64() * 1000.0) as u32;
    for (_id, material) in materials.iter_mut() {
        material.settings.uv_offset_1 = material
            .texture_anim_1
            .as_ref()
            .and_then(|track| evaluate_vec3_track(track, 0, time_ms))
            .map(|offset| Vec2::new(offset[0], offset[1]))
            .unwrap_or(Vec2::ZERO);
        material.settings.uv_offset_2 = material
            .texture_anim_2
            .as_ref()
            .and_then(|track| evaluate_vec3_track(track, 0, time_ms))
            .map(|offset| Vec2::new(offset[0], offset[1]))
            .unwrap_or(Vec2::ZERO);
    }
}

pub fn repeat_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    })
}
