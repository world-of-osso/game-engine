use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType};
use bevy::shader::ShaderRef;

use crate::asset::m2_anim::{AnimTrack, evaluate_vec3_track};

#[derive(ShaderType, Clone)]
pub struct SkyboxM2Settings {
    pub color: Vec4,
    pub transparency: f32,
    pub alpha_test: f32,
    pub shader_id: u32,
    pub blend_mode: u32,
    pub uv_mode_1: u32,
    pub uv_mode_2: u32,
    pub render_flags: u32,
    pub has_second_texture: u32,
    pub uv_offset_1: Vec2,
    pub uv_offset_2: Vec2,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct SkyboxM2Material {
    #[uniform(0)]
    pub settings: SkyboxM2Settings,
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub second_texture: Handle<Image>,
    pub blend_mode: u16,
    pub texture_anim_1: Option<AnimTrack<[f32; 3]>>,
    pub texture_anim_2: Option<AnimTrack<[f32; 3]>>,
}

impl Material for SkyboxM2Material {
    fn fragment_shader() -> ShaderRef {
        "shaders/m2_skybox.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
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
        app.add_plugins(MaterialPlugin::<SkyboxM2Material>::default())
            .add_systems(Update, update_skybox_uvs);
    }
}

fn update_skybox_uvs(time: Res<Time>, mut materials: ResMut<Assets<SkyboxM2Material>>) {
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

#[cfg(test)]
mod tests {
    use super::{SkyboxM2Material, configure_skybox_pipeline};
    use crate::asset;
    use crate::m2_spawn::skybox_m2_material;
    use bevy::asset::RenderAssetUsages;
    use bevy::color::ColorToComponents;
    use bevy::mesh::Mesh;
    use bevy::mesh::PrimitiveTopology;
    use bevy::prelude::{AlphaMode, Assets, Color, Image, Material};
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
        let material =
            skybox_m2_material(None, None, Some(Color::srgba(0.2, 0.4, 0.6, 0.5)), &batch);
        let expected = Color::srgba(0.2, 0.4, 0.6, 0.5).to_linear().to_f32_array();

        assert_eq!(material.blend_mode, 1);
        assert_eq!(material.settings.color.to_array(), expected);
    }

    #[test]
    fn skybox_material_ignores_batch_transparency_and_stays_opaque() {
        let mut batch = test_batch();
        batch.transparency = 0.0;
        let material = skybox_m2_material(None, None, None, &batch);

        assert_eq!(material.settings.color.w, 1.0);
        assert_eq!(material.settings.transparency, 1.0);
        assert_eq!(material.settings.alpha_test, 0.0);
    }

    #[test]
    fn skybox_material_preserves_effect_combine_state_for_advanced_batches() {
        let mut images = Assets::<Image>::default();
        let base = images.add(Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        let second = images.add(Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        let mut batch = test_batch();
        batch.texture_2_fdid = Some(2);
        batch.use_uv_2_1 = true;
        batch.use_uv_2_2 = true;
        batch.shader_id = 0x4014;
        batch.texture_count = 2;
        batch.render_flags = 0x01;

        let material = skybox_m2_material(
            Some(base.clone()),
            Some(second.clone()),
            Some(Color::WHITE),
            &batch,
        );

        assert_eq!(material.base_texture, base);
        assert_eq!(material.second_texture, second);
        assert_eq!(material.settings.shader_id, 0x4014);
        assert_eq!(material.settings.blend_mode, 1);
        assert_eq!(material.settings.uv_mode_1, 1);
        assert_eq!(material.settings.uv_mode_2, 1);
        assert_eq!(material.settings.render_flags, 0x01);
        assert_eq!(material.settings.has_second_texture, 1);
    }

    #[test]
    fn skybox_material_marks_missing_second_texture_for_single_texture_batches() {
        let mut batch = test_batch();
        batch.shader_id = 0x0010;
        batch.texture_count = 1;

        let material = skybox_m2_material(None, None, Some(Color::WHITE), &batch);

        assert_eq!(material.settings.shader_id, 0x0010);
        assert_eq!(material.settings.has_second_texture, 0);
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
    fn skybox_material_stays_opaque_once_alpha_cutout_is_disabled() {
        let material = skybox_m2_material(None, None, None, &test_batch());

        assert!(!<SkyboxM2Material as Material>::enable_prepass());
        assert!(!<SkyboxM2Material as Material>::enable_shadows());
        assert!(matches!(
            <SkyboxM2Material as Material>::alpha_mode(&material),
            AlphaMode::Opaque
        ));
    }
}
