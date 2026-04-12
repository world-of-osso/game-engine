use super::{
    SKYBOX_M2_SHADER_HANDLE, SkyboxM2Material, SkyboxM2Settings, configure_skybox_pipeline,
    evaluate_skybox_uv_offsets, skybox_alpha_mode_for_blend,
};
use crate::asset;
use crate::asset::m2_anim::AnimTrack;
use crate::m2_spawn::skybox_m2_material;
use bevy::asset::RenderAssetUsages;
use bevy::color::ColorToComponents;
use bevy::mesh::Mesh;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::{AlphaMode, Assets, Color, Handle, Image, Material, Vec2, Vec4};
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
        priority_plane: 0,
        material_layer: 0,
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
        0,
        &[],
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
    let material = skybox_m2_material(None, None, None, None, None, &batch, 0, &[]);

    assert_eq!(material.settings.color.w, 1.0);
    assert_eq!(material.settings.transparency, 1.0);
    assert_eq!(material.settings.alpha_test, 0.0);
    assert!(matches!(
        <SkyboxM2Material as Material>::alpha_mode(&material),
        AlphaMode::Blend
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
        0,
        &[],
    );

    assert_advanced_skybox_material(&material, &base, &second);
}

#[test]
fn skybox_material_marks_missing_second_texture_for_single_texture_batches() {
    let mut batch = test_batch();
    batch.shader_id = 0x0010;
    batch.texture_count = 1;

    let material = skybox_m2_material(None, None, None, None, Some(Color::WHITE), &batch, 0, &[]);

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

    let material = skybox_m2_material(Some(base.clone()), None, None, None, None, &batch, 0, &[]);

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
fn configure_skybox_pipeline_uses_default_backface_culling_for_single_sided_batches() {
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

    configure_skybox_pipeline(&mut descriptor, false);

    assert_eq!(descriptor.primitive.cull_mode, Some(Face::Back));
    let depth = descriptor.depth_stencil.unwrap();
    assert!(!depth.depth_write_enabled);
    assert_eq!(depth.depth_compare, CompareFunction::LessEqual);
}

#[test]
fn configure_skybox_pipeline_preserves_two_sided_batches() {
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

    configure_skybox_pipeline(&mut descriptor, true);

    assert_eq!(descriptor.primitive.cull_mode, None);
    let depth = descriptor.depth_stencil.unwrap();
    assert!(!depth.depth_write_enabled);
    assert_eq!(depth.depth_compare, CompareFunction::LessEqual);
}

#[test]
fn skybox_material_disables_prepass_for_authored_blend_modes() {
    let mut batch = test_batch();
    batch.blend_mode = 2;
    let material = skybox_m2_material(None, None, None, None, None, &batch, 0, &[]);

    assert!(!<SkyboxM2Material as Material>::enable_prepass());
    assert!(!<SkyboxM2Material as Material>::enable_shadows());
    assert!(matches!(
        <SkyboxM2Material as Material>::alpha_mode(&material),
        AlphaMode::Blend
    ));
}

#[test]
fn skybox_alpha_mode_mapping_matches_authored_skybox_batches() {
    assert!(matches!(skybox_alpha_mode_for_blend(0), AlphaMode::Opaque));
    assert!(matches!(
        skybox_alpha_mode_for_blend(1),
        AlphaMode::AlphaToCoverage
    ));
    assert!(matches!(skybox_alpha_mode_for_blend(2), AlphaMode::Blend));
    assert!(matches!(skybox_alpha_mode_for_blend(3), AlphaMode::Blend));
    assert!(matches!(skybox_alpha_mode_for_blend(4), AlphaMode::Add));
    assert!(matches!(
        skybox_alpha_mode_for_blend(u16::MAX),
        AlphaMode::Add
    ));
}

fn vec2_close(left: Vec2, right: Vec2) -> bool {
    (left - right).length() <= 0.0001
}

fn test_anim_track(
    global_sequence: i16,
    sequences: Vec<(Vec<u32>, Vec<[f32; 3]>)>,
) -> AnimTrack<[f32; 3]> {
    AnimTrack {
        interpolation_type: 0,
        global_sequence,
        sequences,
    }
}

#[test]
fn evaluate_skybox_uv_offsets_prefers_material_default_sequence_index() {
    let material = SkyboxM2Material {
        settings: SkyboxM2Settings {
            color: Vec4::ONE,
            transparency: 1.0,
            alpha_test: 0.0,
            combine_mode: 0,
            blend_mode: 0,
            uv_mode_1: 0,
            uv_mode_2: 0,
            uv_mode_3: 0,
            uv_mode_4: 0,
            render_flags: 0,
            has_second_texture: 0,
            has_third_texture: 0,
            has_fourth_texture: 0,
            uv_offset_1: Vec2::ZERO,
            uv_offset_2: Vec2::ZERO,
        },
        base_texture: Handle::default(),
        second_texture: Handle::default(),
        third_texture: Handle::default(),
        fourth_texture: Handle::default(),
        blend_mode: 0,
        two_sided: false,
        default_sequence_index: 1,
        global_sequences: Vec::new(),
        texture_anim_1: Some(test_anim_track(
            -1,
            vec![
                (vec![0], vec![[0.0, 0.0, 0.0]]),
                (vec![0], vec![[0.5, 0.25, 0.0]]),
            ],
        )),
        texture_anim_2: None,
    };

    let (uv_offset_1, uv_offset_2) = evaluate_skybox_uv_offsets(&material, 0);

    assert!(vec2_close(uv_offset_1, Vec2::new(0.5, 0.25)));
    assert_eq!(uv_offset_2, Vec2::ZERO);
}

#[test]
fn evaluate_skybox_uv_offsets_loops_on_global_sequence_duration() {
    let material = SkyboxM2Material {
        settings: SkyboxM2Settings {
            color: Vec4::ONE,
            transparency: 1.0,
            alpha_test: 0.0,
            combine_mode: 0,
            blend_mode: 0,
            uv_mode_1: 0,
            uv_mode_2: 0,
            uv_mode_3: 0,
            uv_mode_4: 0,
            render_flags: 0,
            has_second_texture: 0,
            has_third_texture: 0,
            has_fourth_texture: 0,
            uv_offset_1: Vec2::ZERO,
            uv_offset_2: Vec2::ZERO,
        },
        base_texture: Handle::default(),
        second_texture: Handle::default(),
        third_texture: Handle::default(),
        fourth_texture: Handle::default(),
        blend_mode: 0,
        two_sided: false,
        default_sequence_index: 0,
        global_sequences: vec![51],
        texture_anim_1: Some(test_anim_track(
            0,
            vec![(vec![0, 50], vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]])],
        )),
        texture_anim_2: None,
    };

    let (start_uv, _) = evaluate_skybox_uv_offsets(&material, 0);
    let (changed_uv, _) = evaluate_skybox_uv_offsets(&material, 50);
    let (wrapped_uv, _) = evaluate_skybox_uv_offsets(&material, 101);

    assert!(!vec2_close(start_uv, changed_uv));
    assert!(vec2_close(changed_uv, wrapped_uv));
}

#[test]
fn known_animated_cloudsky_material_preserves_animation_metadata() {
    let model = crate::asset::m2::load_skybox_m2_uncached(
        std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2"),
        &[0, 0, 0],
    )
    .expect("load animated cloud skybox");
    let default_sequence_index = model
        .sequences
        .iter()
        .position(|sequence| sequence.id == 0)
        .unwrap_or(0);
    let material = model
        .batches
        .iter()
        .filter(|batch| batch.texture_anim.is_some() || batch.texture_anim_2.is_some())
        .map(|batch| {
            skybox_m2_material(
                None,
                None,
                None,
                None,
                None,
                batch,
                default_sequence_index,
                &model.global_sequences,
            )
        })
        .next()
        .expect("animated cloud skybox material");

    assert_eq!(
        material.default_sequence_index as usize,
        default_sequence_index
    );
    assert_eq!(material.global_sequences, model.global_sequences);
    assert!(
        material.texture_anim_1.is_some() || material.texture_anim_2.is_some(),
        "known animated cloud skybox should preserve authored texture animation metadata"
    );
}
