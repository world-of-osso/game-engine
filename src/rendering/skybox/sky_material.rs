use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Face, ShaderType};
use bevy::shader::ShaderRef;

/// Per-frame sky color uniforms written by the sky system from LightData.
#[derive(ShaderType, Debug, Clone)]
pub struct SkyUniforms {
    pub sky_top: Vec4,
    pub sky_middle: Vec4,
    pub sky_band1: Vec4,
    pub sky_band2: Vec4,
    pub sky_smog: Vec4,
    pub sun_color: Vec4,
    pub sun_halo_color: Vec4,
    pub cloud_emissive_color: Vec4,
    pub cloud_layer1_ambient_color: Vec4,
    pub cloud_layer2_ambient_color: Vec4,
    pub sun_direction: Vec4,
    pub cloud_params: Vec4,
}

impl Default for SkyUniforms {
    fn default() -> Self {
        Self {
            sky_top: Vec4::ONE,
            sky_middle: Vec4::ONE,
            sky_band1: Vec4::ONE,
            sky_band2: Vec4::ONE,
            sky_smog: Vec4::ONE,
            sun_color: Vec4::ONE,
            sun_halo_color: Vec4::ONE,
            cloud_emissive_color: Vec4::ZERO,
            cloud_layer1_ambient_color: Vec4::ZERO,
            cloud_layer2_ambient_color: Vec4::ZERO,
            sun_direction: Vec4::new(0.0, 1.0, 0.0, 0.0),
            cloud_params: Vec4::new(0.0, 0.0, 0.0, 0.0),
        }
    }
}

/// Material for the procedural sky dome sphere.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyMaterial {
    #[uniform(0)]
    pub uniforms: SkyUniforms,
    #[texture(1)]
    #[sampler(2)]
    pub cloud_texture: Handle<Image>,
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sky.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Render inside of sphere: mesh winding is inward-facing, cull outer surface.
        descriptor.primitive.cull_mode = Some(Face::Back);
        // Sky must render behind everything — disable depth write.
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = false;
        }
        Ok(())
    }
}
