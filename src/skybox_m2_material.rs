use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Face, RenderPipelineDescriptor, ShaderType};
use bevy::shader::ShaderRef;

use crate::m2_effect_material::alpha_mode_for_blend;

#[derive(ShaderType, Clone)]
pub struct SkyboxM2Settings {
    pub color: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct SkyboxM2Material {
    #[uniform(0)]
    pub settings: SkyboxM2Settings,
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Handle<Image>,
    pub blend_mode: u16,
}

impl Material for SkyboxM2Material {
    fn fragment_shader() -> ShaderRef {
        "shaders/m2_skybox.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        alpha_mode_for_blend(self.blend_mode)
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Front);
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = false;
        }
        Ok(())
    }
}

pub struct SkyboxM2MaterialPlugin;

impl Plugin for SkyboxM2MaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<SkyboxM2Material>::default());
    }
}
