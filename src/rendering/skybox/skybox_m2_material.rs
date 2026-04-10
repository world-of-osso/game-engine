use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType};
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
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        configure_skybox_pipeline(descriptor);
        Ok(())
    }
}

fn configure_skybox_pipeline(descriptor: &mut RenderPipelineDescriptor) {
    // Skyboxes render from inside the dome and should never punch holes into later draws.
    descriptor.primitive.cull_mode = None;
    if let Some(ds) = descriptor.depth_stencil.as_mut() {
        ds.depth_write_enabled = false;
    }
}

pub struct SkyboxM2MaterialPlugin;

impl Plugin for SkyboxM2MaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<SkyboxM2Material>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::{SkyboxM2Material, configure_skybox_pipeline};
    use crate::asset;
    use crate::m2_spawn::skybox_m2_material;
    use bevy::color::ColorToComponents;
    use bevy::mesh::Mesh;
    use bevy::mesh::PrimitiveTopology;
    use bevy::prelude::{AlphaMode, Color, Material};
    use bevy::render::render_resource::{
        ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Face,
        FragmentState, MultisampleState, PrimitiveState, TextureFormat, VertexState,
    };

    fn test_batch() -> asset::m2::M2RenderBatch {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        asset::m2::M2RenderBatch {
            mesh,
            texture_fdid: None,
            texture_2_fdid: None,
            texture_type: None,
            overlays: Vec::new(),
            render_flags: 0,
            blend_mode: 1,
            transparency: 0.5,
            texture_anim: None,
            texture_anim_2: None,
            use_uv_2_1: false,
            use_uv_2_2: false,
            use_env_map_2: false,
            shader_id: 0,
            texture_count: 0,
            mesh_part_id: 0,
        }
    }

    #[test]
    fn skybox_material_uses_linearized_color_and_blend_mode() {
        let batch = test_batch();
        let material = skybox_m2_material(None, Some(Color::srgba(0.2, 0.4, 0.6, 0.5)), &batch);
        let expected = Color::srgba(0.2, 0.4, 0.6, 0.5).to_linear().to_f32_array();

        assert_eq!(material.blend_mode, 1);
        assert_eq!(material.settings.color.to_array(), expected);
    }

    #[test]
    fn skybox_material_ignores_batch_transparency_and_stays_opaque() {
        let mut batch = test_batch();
        batch.transparency = 0.0;
        let material = skybox_m2_material(None, None, &batch);

        assert_eq!(material.settings.color.w, 1.0);
    }

    #[test]
    fn configure_skybox_pipeline_disables_culling_and_preserves_depth_compare() {
        let mut descriptor = bevy::render::render_resource::RenderPipelineDescriptor {
            vertex: VertexState::default(),
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: Default::default(),
                shader_defs: Vec::new(),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: Vec::new(),
            push_constant_ranges: Vec::new(),
            label: None,
            zero_initialize_workgroup_memory: false,
        };

        configure_skybox_pipeline(&mut descriptor);

        assert_eq!(descriptor.primitive.cull_mode, None);
        let depth = descriptor.depth_stencil.unwrap();
        assert!(!depth.depth_write_enabled);
        assert_eq!(depth.depth_compare, CompareFunction::LessEqual);
    }

    #[test]
    fn skybox_material_disables_prepass_and_shadows() {
        let material = skybox_m2_material(None, None, &test_batch());

        assert!(!<SkyboxM2Material as Material>::enable_prepass());
        assert!(!<SkyboxM2Material as Material>::enable_shadows());
        assert!(matches!(
            <SkyboxM2Material as Material>::alpha_mode(&material),
            AlphaMode::AlphaToCoverage
        ));
    }
}
