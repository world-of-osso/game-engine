use super::{
    SKYBOX_M2_SHADER_HANDLE, SkyboxM2Material, configure_skybox_pipeline,
    skybox_alpha_mode_for_blend,
};
use crate::asset;
use crate::m2_spawn::skybox_m2_material;
use bevy::asset::RenderAssetUsages;
use bevy::color::ColorToComponents;
use bevy::mesh::Mesh;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::{AlphaMode, Assets, Color, Handle, Image, Material};
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
        extra_texture_fdids: Vec::new(),
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
        uses_texture_combiner_combos: false,
        mesh_part_id: 0,
    }
}

fn single_pixel_srgb_image(images: &mut Assets<Image>) -> Handle<Image> {
    images.add(Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

fn advanced_skybox_batch() -> asset::m2::M2RenderBatch {
    let mut batch = test_batch();
    batch.texture_2_fdid = Some(2);
    batch.use_uv_2_1 = true;
    batch.use_uv_2_2 = true;
    batch.uses_texture_combiner_combos = true;
    batch.shader_id = 0x4014;
    batch.texture_count = 2;
    batch.render_flags = 0x01;
    batch
}

fn assert_advanced_skybox_material(
    material: &SkyboxM2Material,
    base: &Handle<Image>,
    second: &Handle<Image>,
) {
    assert_eq!(material.base_texture, *base);
    assert_eq!(material.second_texture, *second);
    assert_eq!(material.third_texture, *base);
    assert_eq!(material.fourth_texture, *base);
    assert_eq!(material.settings.combine_mode, 0x4014);
    assert_eq!(material.settings.blend_mode, 1);
    assert_eq!(material.settings.uv_mode_1, 1);
    assert_eq!(material.settings.uv_mode_2, 1);
    assert_eq!(material.settings.uv_mode_3, 0);
    assert_eq!(material.settings.uv_mode_4, 0);
    assert_eq!(material.settings.render_flags, 0x01);
    assert_eq!(material.settings.has_second_texture, 1);
    assert_eq!(material.settings.has_third_texture, 0);
    assert_eq!(material.settings.has_fourth_texture, 0);
}

#[test]
fn skybox_material_uses_linearized_color_and_blend_mode() {
    let batch = test_batch();
    let material = skybox_m2_material(
        None,
        None,
        None,
        None,
        Some(Color::srgba(0.2, 0.4, 0.6, 0.5)),
        &batch,
    );
    let expected = Color::srgba(0.2, 0.4, 0.6, 0.5).to_linear().to_f32_array();

    assert_eq!(material.blend_mode, 1);
    assert_eq!(material.settings.color.to_array(), expected);
}

#[test]
fn skybox_material_ignores_batch_transparency_cutout_but_keeps_blend_mode() {
    let mut batch = test_batch();
    batch.blend_mode = 2;
    batch.transparency = 0.0;
    let material = skybox_m2_material(None, None, None, None, None, &batch);

    assert_eq!(material.settings.color.w, 1.0);
    assert_eq!(material.settings.transparency, 1.0);
    assert_eq!(material.settings.alpha_test, 0.0);
    assert!(matches!(
        <SkyboxM2Material as Material>::alpha_mode(&material),
        AlphaMode::Opaque
    ));
}

#[test]
fn skybox_material_preserves_effect_combine_state_for_advanced_batches() {
    let mut images = Assets::<Image>::default();
    let base = single_pixel_srgb_image(&mut images);
    let second = single_pixel_srgb_image(&mut images);
    let batch = advanced_skybox_batch();

    let material = skybox_m2_material(
        Some(base.clone()),
        Some(second.clone()),
        None,
        None,
        Some(Color::WHITE),
        &batch,
    );

    assert_advanced_skybox_material(&material, &base, &second);
}

#[test]
fn skybox_material_marks_missing_second_texture_for_single_texture_batches() {
    let mut batch = test_batch();
    batch.shader_id = 0x0010;
    batch.texture_count = 1;

    let material = skybox_m2_material(None, None, None, None, Some(Color::WHITE), &batch);

    assert_eq!(material.settings.combine_mode, 0x2);
    assert_eq!(material.settings.has_second_texture, 0);
}

#[test]
fn skybox_material_reuses_primary_texture_for_missing_optional_stages() {
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
    let mut batch = test_batch();
    batch.shader_id = 0x8012;
    batch.texture_count = 3;

    let material = skybox_m2_material(Some(base.clone()), None, None, None, None, &batch);

    assert_eq!(material.base_texture, base);
    assert_eq!(material.second_texture, material.base_texture);
    assert_eq!(material.third_texture, material.base_texture);
    assert_eq!(material.fourth_texture, material.base_texture);
    assert_eq!(material.settings.has_second_texture, 0);
    assert_eq!(material.settings.has_third_texture, 0);
    assert_eq!(material.settings.has_fourth_texture, 0);
}

#[test]
fn skybox_material_uses_internal_shader_handle() {
    let shader = <SkyboxM2Material as Material>::fragment_shader();

    assert!(matches!(
        shader,
        bevy::shader::ShaderRef::Handle(handle) if handle == SKYBOX_M2_SHADER_HANDLE
    ));
}

#[test]
fn configure_skybox_pipeline_disables_culling_and_enables_depth_writes() {
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
    assert!(depth.depth_write_enabled);
    assert_eq!(depth.depth_compare, CompareFunction::LessEqual);
}

#[test]
fn skybox_material_keeps_prepass_enabled_for_authored_blend_modes() {
    let mut batch = test_batch();
    batch.blend_mode = 2;
    let material = skybox_m2_material(None, None, None, None, None, &batch);

    assert!(<SkyboxM2Material as Material>::enable_prepass());
    assert!(!<SkyboxM2Material as Material>::enable_shadows());
    assert!(matches!(
        <SkyboxM2Material as Material>::alpha_mode(&material),
        AlphaMode::Opaque
    ));
}

#[test]
fn skybox_alpha_mode_mapping_matches_authored_skybox_batches() {
    assert!(matches!(skybox_alpha_mode_for_blend(0), AlphaMode::Opaque));
    assert!(matches!(skybox_alpha_mode_for_blend(2), AlphaMode::Opaque));
    assert!(matches!(skybox_alpha_mode_for_blend(4), AlphaMode::Opaque));
    assert!(matches!(
        skybox_alpha_mode_for_blend(u16::MAX),
        AlphaMode::Opaque
    ));
}
