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
        alpha_mode_for_blend(self.blend_mode)
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Back);
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

pub struct M2EffectMaterialPlugin;

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct M2EffectUvUpdatesEnabled(pub(crate) bool);

impl Default for M2EffectUvUpdatesEnabled {
    fn default() -> Self {
        Self(true)
    }
}

impl Plugin for M2EffectMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<M2EffectMaterial>::default())
            .init_resource::<M2EffectUvUpdatesEnabled>();
        register_m2_effect_uv_update_system(app);
    }
}

pub(crate) fn register_m2_effect_uv_update_system(app: &mut App) {
    app.add_systems(
        Update,
        update_m2_effect_uvs.run_if(m2_effect_uv_updates_enabled),
    );
}

fn m2_effect_uv_updates_enabled(enabled: Res<M2EffectUvUpdatesEnabled>) -> bool {
    enabled.0
}

fn update_m2_effect_uvs(time: Res<Time>, mut materials: ResMut<Assets<M2EffectMaterial>>) {
    let time_ms = (time.elapsed_secs_f64() * 1000.0) as u32;
    let changed: Vec<_> = materials
        .iter()
        .filter_map(|(id, material)| {
            let offsets = sample_m2_effect_uv_offsets(material, time_ms);
            let current = (material.settings.uv_offset_1, material.settings.uv_offset_2);
            (offsets != current).then_some((id, offsets))
        })
        .collect();
    for (id, (offset_1, offset_2)) in changed {
        let mut material = materials
            .get_mut(id)
            .expect("material sampled during this UV update");
        material.settings.uv_offset_1 = offset_1;
        material.settings.uv_offset_2 = offset_2;
    }
}

pub(crate) fn update_m2_effect_material_uv(material: &mut M2EffectMaterial, time_ms: u32) {
    let (offset_1, offset_2) = sample_m2_effect_uv_offsets(material, time_ms);
    material.settings.uv_offset_1 = offset_1;
    material.settings.uv_offset_2 = offset_2;
}

fn sample_m2_effect_uv_offsets(material: &M2EffectMaterial, time_ms: u32) -> (Vec2, Vec2) {
    let offset_1 = material
        .texture_anim_1
        .as_ref()
        .and_then(|track| evaluate_vec3_track(track, 0, time_ms))
        .map(|offset| Vec2::new(offset[0], offset[1]))
        .unwrap_or(Vec2::ZERO);
    let offset_2 = material
        .texture_anim_2
        .as_ref()
        .and_then(|track| evaluate_vec3_track(track, 0, time_ms))
        .map(|offset| Vec2::new(offset[0], offset[1]))
        .unwrap_or(Vec2::ZERO);
    (offset_1, offset_2)
}

pub fn repeat_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    })
}

pub(crate) fn alpha_mode_for_blend(blend_mode: u16) -> AlphaMode {
    match blend_mode {
        0 => AlphaMode::Opaque,
        1 => AlphaMode::AlphaToCoverage,
        2 | 3 | 7 => AlphaMode::Blend,
        4..=6 => AlphaMode::Add,
        _ => AlphaMode::Add,
    }
}

pub(crate) fn alpha_test_threshold_for_blend(blend_mode: u16, transparency: f32) -> f32 {
    match blend_mode {
        1 => 224.0 / 255.0 * transparency,
        2..=7 => 1.0 / 255.0 * transparency,
        _ => 0.0,
    }
}

#[cfg(test)]
#[path = "m2_effect_fog_gpu_tests.rs"]
mod fog_gpu_tests;

#[cfg(test)]
mod tests {
    use super::{
        M2EffectMaterial, M2EffectSettings, M2EffectUvUpdatesEnabled, alpha_mode_for_blend,
        alpha_test_threshold_for_blend, register_m2_effect_uv_update_system,
    };
    use crate::asset::m2_anim::AnimTrack;
    use bevy::app::TaskPoolPlugin;
    use bevy::asset::{AssetApp, AssetEvent, AssetId, AssetPlugin};
    use bevy::ecs::message::Messages;
    use bevy::prelude::{AlphaMode, App, Assets, Handle, Time, Vec2};
    use std::time::Duration;

    const SENTINEL_UV_OFFSET_1: Vec2 = Vec2::new(9.0, 10.0);
    const SENTINEL_UV_OFFSET_2: Vec2 = Vec2::new(11.0, 12.0);

    #[test]
    fn invalid_blend_modes_fallback_to_additive() {
        assert!(matches!(alpha_mode_for_blend(u16::MAX), AlphaMode::Add));
        assert!(matches!(alpha_mode_for_blend(8), AlphaMode::Add));
    }

    #[test]
    fn blend_mode_seven_uses_alpha_blend() {
        assert!(matches!(alpha_mode_for_blend(7), AlphaMode::Blend));
    }

    #[test]
    fn alpha_test_thresholds_match_authored_m2_contract() {
        assert_eq!(alpha_test_threshold_for_blend(0, 0.5), 0.0);
        assert_eq!(alpha_test_threshold_for_blend(1, 0.5), 224.0 / 255.0 * 0.5);
        assert_eq!(alpha_test_threshold_for_blend(2, 0.25), 1.0 / 255.0 * 0.25);
        assert_eq!(alpha_test_threshold_for_blend(7, 0.75), 1.0 / 255.0 * 0.75);
    }

    #[test]
    fn disabled_m2_effect_uv_updates_leave_material_offsets_unchanged() {
        let (mut app, material_handle) = build_uv_update_test_app(false);
        app.update();
        take_modified_materials(&mut app);
        advance_test_time(&mut app);
        assert!(take_modified_materials(&mut app).is_empty());

        let material = app
            .world()
            .resource::<Assets<M2EffectMaterial>>()
            .get(&material_handle)
            .expect("test material");

        assert_eq!(material.settings.uv_offset_1, SENTINEL_UV_OFFSET_1);
        assert_eq!(material.settings.uv_offset_2, SENTINEL_UV_OFFSET_2);
    }

    #[test]
    fn enabled_m2_effect_uv_updates_evaluate_animation_tracks() {
        let (mut app, material_handle) = build_uv_update_test_app(true);
        app.update();
        take_modified_materials(&mut app);
        advance_test_time(&mut app);
        assert_eq!(
            take_modified_materials(&mut app),
            vec![material_handle.id()]
        );

        let material = app
            .world()
            .resource::<Assets<M2EffectMaterial>>()
            .get(&material_handle)
            .expect("test material");

        assert_eq!(material.settings.uv_offset_1, Vec2::new(0.5, 0.75));
        assert_eq!(material.settings.uv_offset_2, Vec2::ZERO);
    }

    #[test]
    fn static_and_constant_uvs_do_not_repeat_modified_events() {
        let (mut app, static_handle) = build_uv_update_test_app(true);
        let constant_handle = {
            let mut materials = app.world_mut().resource_mut::<Assets<M2EffectMaterial>>();
            materials.get_mut(&static_handle).unwrap().texture_anim_1 = None;
            let mut constant = test_material();
            constant.texture_anim_1 = Some(constant_anim_track());
            constant.texture_anim_2 = Some(constant_anim_track());
            materials.add(constant)
        };
        app.update();
        take_modified_materials(&mut app);

        for _ in 0..3 {
            advance_test_time(&mut app);
            assert!(
                take_modified_materials(&mut app).is_empty(),
                "unchanged UVs must not emit Modified events",
            );
        }
        let materials = app.world().resource::<Assets<M2EffectMaterial>>();
        let static_material = materials.get(&static_handle).unwrap();
        assert_eq!(static_material.settings.uv_offset_1, Vec2::ZERO);
        assert_eq!(static_material.settings.uv_offset_2, Vec2::ZERO);
        let constant = materials.get(&constant_handle).unwrap();
        assert_eq!(constant.settings.uv_offset_1, Vec2::new(0.25, -0.5));
        assert_eq!(constant.settings.uv_offset_2, Vec2::new(0.25, -0.5));
    }

    #[test]
    fn removing_uv_tracks_resets_offsets_and_then_stays_clean() {
        let (mut app, handle) = build_uv_update_test_app(true);
        app.world_mut()
            .resource_mut::<Assets<M2EffectMaterial>>()
            .get_mut(&handle)
            .unwrap()
            .texture_anim_2 = Some(constant_anim_track());
        advance_test_time(&mut app);
        take_modified_materials(&mut app);
        {
            let materials = app.world().resource::<Assets<M2EffectMaterial>>();
            let material = materials.get(&handle).unwrap();
            assert_eq!(material.settings.uv_offset_1, Vec2::new(0.5, 0.75));
            assert_eq!(material.settings.uv_offset_2, Vec2::new(0.25, -0.5));
        }
        {
            let mut materials = app.world_mut().resource_mut::<Assets<M2EffectMaterial>>();
            let mut material = materials.get_mut(&handle).unwrap();
            material.texture_anim_1 = None;
            material.texture_anim_2 = None;
        }
        advance_test_time(&mut app);
        assert!(take_modified_materials(&mut app).contains(&handle.id()));
        {
            let materials = app.world().resource::<Assets<M2EffectMaterial>>();
            let material = materials.get(&handle).unwrap();
            assert_eq!(material.settings.uv_offset_1, Vec2::ZERO);
            assert_eq!(material.settings.uv_offset_2, Vec2::ZERO);
        }
        advance_test_time(&mut app);
        assert!(take_modified_materials(&mut app).is_empty());
    }

    fn take_modified_materials(app: &mut App) -> Vec<AssetId<M2EffectMaterial>> {
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<M2EffectMaterial>>>()
            .drain()
            .filter_map(|event| match event {
                AssetEvent::Modified { id } => Some(id),
                _ => None,
            })
            .collect()
    }

    fn constant_anim_track() -> AnimTrack<[f32; 3]> {
        AnimTrack {
            interpolation_type: 0,
            global_sequence: -1,
            sequences: vec![(vec![0, 1000], vec![[0.25, -0.5, 0.0]; 2])],
        }
    }

    fn build_uv_update_test_app(enabled: bool) -> (App, Handle<M2EffectMaterial>) {
        let mut app = App::new();
        app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()));
        app.insert_resource(Time::<()>::default());
        app.insert_resource(M2EffectUvUpdatesEnabled(enabled));
        app.init_asset::<M2EffectMaterial>();
        register_m2_effect_uv_update_system(&mut app);

        let material_handle = app
            .world_mut()
            .resource_mut::<Assets<M2EffectMaterial>>()
            .add(test_material());
        (app, material_handle)
    }

    fn advance_test_time(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(500));
        app.update();
    }

    fn test_material() -> M2EffectMaterial {
        M2EffectMaterial {
            settings: M2EffectSettings {
                transparency: 1.0,
                alpha_test: 0.0,
                shader_id: 0,
                blend_mode: 0,
                uv_mode_1: 0,
                uv_mode_2: 0,
                render_flags: 0,
                uv_offset_1: SENTINEL_UV_OFFSET_1,
                uv_offset_2: SENTINEL_UV_OFFSET_2,
            },
            base_texture: Handle::default(),
            second_texture: Handle::default(),
            blend_mode: 0,
            two_sided: false,
            texture_anim_1: Some(test_anim_track()),
            texture_anim_2: None,
        }
    }

    fn test_anim_track() -> AnimTrack<[f32; 3]> {
        AnimTrack {
            interpolation_type: 0,
            global_sequence: -1,
            sequences: vec![(vec![0, 1000], vec![[0.0, 0.0, 0.0], [1.0, 1.5, 0.0]])],
        }
    }
}
