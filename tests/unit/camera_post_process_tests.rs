use super::*;
use crate::client_options::{AntiAliasMode, GraphicsOptions};
use crate::game::inworld_scene_stage::InWorldSceneStage;
use bevy::anti_alias::contrast_adaptive_sharpening::ContrastAdaptiveSharpening;
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::audio::SpatialListener;
use bevy::camera::ClearColorConfig;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::ShadowFilteringMethod;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::post_process::bloom::{Bloom, BloomCompositeMode};
use bevy::post_process::dof::DepthOfField;
use bevy::render::camera::{MipBias, TemporalJitter};
use ui_toolkit::render::{UiCamera, setup_ui_camera};

#[test]
fn no_msaa_keeps_composited_ui_sampling_in_sync_without_changing_its_render_bundle() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.register_required_components::<Camera, Msaa>();
    app.insert_resource(GraphicsOptions::default());
    app.insert_resource(InWorldSceneStage::NoNpcsUi);
    app.add_systems(Startup, setup_ui_camera);
    app.add_systems(
        Update,
        (
            camera_post_process::sync_camera_graphics_post_process,
            camera_post_process::sync_ui_camera_msaa,
        )
            .chain(),
    );
    let world_entity = spawn_wow_camera(&mut app.world_mut().commands());
    app.update();
    let ui_entity = {
        let mut query = app.world_mut().query_filtered::<Entity, With<UiCamera>>();
        query.iter(app.world()).next().expect("expected UI camera")
    };

    assert_eq!(app.world().get::<Msaa>(world_entity), Some(&Msaa::Sample4));
    assert_eq!(app.world().get::<Msaa>(ui_entity), Some(&Msaa::Sample4));
    let ui_camera = app.world().entity(ui_entity);
    assert_eq!(
        ui_camera.get::<Camera>().map(|camera| camera.order),
        Some(1)
    );
    assert!(matches!(
        ui_camera.get::<Camera>().map(|camera| &camera.clear_color),
        Some(ClearColorConfig::None)
    ));
    assert!(!ui_camera.contains::<TemporalAntiAliasing>());
    assert!(!ui_camera.contains::<DepthPrepass>());
    assert!(!ui_camera.contains::<NormalPrepass>());

    crate::configure_msaa_isolation(&mut app, &["--no-msaa".to_owned()]);
    app.update();
    let world_camera = app.world().entity(world_entity);
    assert_eq!(world_camera.get::<Msaa>(), Some(&Msaa::Off));
    assert_eq!(app.world().get::<Msaa>(ui_entity), Some(&Msaa::Off));
    assert!(world_camera.contains::<DepthPrepass>());
    assert!(world_camera.contains::<NormalPrepass>());
    assert!(world_camera.contains::<Tonemapping>());
    assert!(world_camera.contains::<ShadowFilteringMethod>());
    assert!(!world_camera.contains::<TemporalAntiAliasing>());
    assert!(!world_camera.contains::<ScreenSpaceAmbientOcclusion>());
    assert_eq!(
        app.world().resource::<GraphicsOptions>().anti_alias,
        AntiAliasMode::Msaa4x
    );

    app.world_mut().remove_resource::<MsaaDisabled>();
    app.update();
    assert_eq!(app.world().get::<Msaa>(world_entity), Some(&Msaa::Sample4));
    assert_eq!(app.world().get::<Msaa>(ui_entity), Some(&Msaa::Sample4));

    app.insert_resource(MsaaDisabled);
    app.world_mut().resource_mut::<GraphicsOptions>().anti_alias = AntiAliasMode::Taa;
    app.world_mut()
        .resource_mut::<GraphicsOptions>()
        .ssao_enabled = true;
    app.update();
    let world_camera = app.world().entity(world_entity);
    assert_eq!(world_camera.get::<Msaa>(), Some(&Msaa::Off));
    assert_eq!(app.world().get::<Msaa>(ui_entity), Some(&Msaa::Off));
    assert!(world_camera.contains::<TemporalAntiAliasing>());
    assert!(world_camera.contains::<ScreenSpaceAmbientOcclusion>());
    assert!(
        !app.world()
            .entity(ui_entity)
            .contains::<TemporalAntiAliasing>()
    );
    assert!(!app.world().entity(ui_entity).contains::<DepthPrepass>());
    assert!(!app.world().entity(ui_entity).contains::<NormalPrepass>());

    app.world_mut().remove_resource::<MsaaDisabled>();
    app.world_mut().resource_mut::<GraphicsOptions>().anti_alias = AntiAliasMode::None;
    app.update();
    assert_eq!(app.world().get::<Msaa>(world_entity), Some(&Msaa::Off));
    assert_eq!(app.world().get::<Msaa>(ui_entity), Some(&Msaa::Off));

    app.world_mut().resource_mut::<GraphicsOptions>().anti_alias = AntiAliasMode::Msaa4x;
    app.world_mut()
        .resource_mut::<GraphicsOptions>()
        .ssao_enabled = false;
    app.update();
    assert_eq!(app.world().get::<Msaa>(world_entity), Some(&Msaa::Sample4));
    assert_eq!(app.world().get::<Msaa>(ui_entity), Some(&Msaa::Sample4));

    app.insert_resource(InWorldSceneStage::Character);
    app.world_mut().entity_mut(world_entity).insert(Msaa::Off);
    app.update();
    assert_eq!(app.world().get::<Msaa>(world_entity), Some(&Msaa::Off));
    assert_eq!(app.world().get::<Msaa>(ui_entity), Some(&Msaa::Off));

    app.insert_resource(InWorldSceneStage::NoNpcsUi);
    app.update();
    assert_eq!(app.world().get::<Msaa>(world_entity), Some(&Msaa::Sample4));
    assert_eq!(app.world().get::<Msaa>(ui_entity), Some(&Msaa::Sample4));
}

#[test]
fn spawn_wow_camera_uses_particle_glow_tonemapping() {
    let mut world = World::new();
    let entity = spawn_wow_camera(&mut world.commands());
    world.flush();

    let tonemapping = world
        .entity(entity)
        .get::<Tonemapping>()
        .copied()
        .expect("expected tonemapping on wow camera");
    assert_eq!(tonemapping, Tonemapping::TonyMcMapface);
}

#[test]
fn graphics_options_build_additive_particle_bloom() {
    let bloom = camera_post_process::additive_particle_glow_bloom(&GraphicsOptions {
        particle_density: 100,
        particle_effects_enabled: true,
        ssao_enabled: true,
        render_scale: 1.0,
        ui_scale: 1.0,
        vsync_enabled: true,
        frame_rate_limit_enabled: false,
        frame_rate_limit: 144,
        colorblind_mode: false,
        bloom_enabled: true,
        bloom_intensity: 0.12,
        depth_of_field: false,
        anti_alias: AntiAliasMode::Taa,
    })
    .expect("expected bloom");

    assert_eq!(bloom.composite_mode, BloomCompositeMode::Additive);
    assert!((bloom.intensity - 0.12).abs() < f32::EPSILON);
    assert!((bloom.prefilter.threshold - 0.65).abs() < f32::EPSILON);
    assert!((bloom.prefilter.threshold_softness - 0.1).abs() < f32::EPSILON);
}

#[test]
fn disabled_graphics_bloom_returns_none() {
    assert!(
        camera_post_process::additive_particle_glow_bloom(&GraphicsOptions {
            particle_density: 100,
            particle_effects_enabled: true,
            ssao_enabled: true,
            render_scale: 1.0,
            ui_scale: 1.0,
            vsync_enabled: true,
            frame_rate_limit_enabled: false,
            frame_rate_limit: 144,
            colorblind_mode: false,
            bloom_enabled: false,
            bloom_intensity: 0.12,
            depth_of_field: false,
            anti_alias: AntiAliasMode::Taa,
        })
        .is_none()
    );
}

#[test]
fn native_render_scale_disables_main_pass_override() {
    assert_eq!(
        camera_post_process::scaled_main_pass_resolution(UVec2::new(1920, 1080), 1.0),
        None
    );
}

#[test]
fn reduced_render_scale_produces_smaller_main_pass_resolution() {
    assert_eq!(
        camera_post_process::scaled_main_pass_resolution(UVec2::new(1920, 1080), 0.75),
        Some(UVec2::new(1440, 810))
    );
    assert_eq!(
        camera_post_process::scaled_main_pass_resolution(UVec2::new(1920, 1080), 0.67),
        Some(UVec2::new(1286, 723))
    );
}

#[test]
fn sync_camera_graphics_post_process_keeps_ssao_compatible_with_anti_aliasing() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GraphicsOptions::default());
    app.add_systems(
        Update,
        camera_post_process::sync_camera_graphics_post_process,
    );

    let camera_entity = spawn_wow_camera(&mut app.world_mut().commands());
    app.update();

    let camera = app.world().entity(camera_entity);
    assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Sample4));
    assert!(camera.get::<ScreenSpaceAmbientOcclusion>().is_none());

    app.world_mut().resource_mut::<GraphicsOptions>().anti_alias = AntiAliasMode::Taa;
    app.world_mut()
        .resource_mut::<GraphicsOptions>()
        .ssao_enabled = true;
    app.update();

    let camera = app.world().entity(camera_entity);
    assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Off));
    assert!(camera.get::<TemporalAntiAliasing>().is_some());
    assert!(camera.get::<ScreenSpaceAmbientOcclusion>().is_some());
}

fn graphics_effects_test_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GraphicsOptions {
        anti_alias: AntiAliasMode::None,
        ..default()
    });
    app.add_systems(
        Update,
        camera_post_process::sync_camera_graphics_post_process,
    );
    let camera = spawn_wow_camera(&mut app.world_mut().commands());
    (app, camera)
}

#[test]
fn graphics_config_camera_effects_are_independent() {
    let (mut app, entity) = graphics_effects_test_app();
    for (blur, glow, contact) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (true, true, true),
        (false, false, false),
    ] {
        let mut graphics = app.world_mut().resource_mut::<GraphicsOptions>();
        graphics.depth_of_field = blur;
        graphics.bloom_enabled = glow;
        graphics.ssao_enabled = contact;
        app.update();
        let camera = app.world().entity(entity);
        assert_eq!(camera.contains::<DepthOfField>(), blur);
        assert_eq!(camera.contains::<Bloom>(), glow);
        assert_eq!(camera.contains::<ScreenSpaceAmbientOcclusion>(), contact);
        assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Off));
        assert!(!camera.contains::<TemporalAntiAliasing>());
        assert!(camera.contains::<Camera3d>());
    }
}

#[test]
fn graphics_config_aa_does_not_enable_contact_shading() {
    let (mut app, entity) = graphics_effects_test_app();
    for mode in [
        AntiAliasMode::None,
        AntiAliasMode::Taa,
        AntiAliasMode::Msaa4x,
        AntiAliasMode::None,
    ] {
        app.world_mut().resource_mut::<GraphicsOptions>().anti_alias = mode;
        app.update();
        let camera = app.world().entity(entity);
        let expected_msaa = if mode == AntiAliasMode::Msaa4x {
            Msaa::Sample4
        } else {
            Msaa::Off
        };
        assert_eq!(camera.get::<Msaa>(), Some(&expected_msaa));
        assert_eq!(
            camera.contains::<TemporalAntiAliasing>(),
            mode == AntiAliasMode::Taa
        );
        assert!(!camera.contains::<ScreenSpaceAmbientOcclusion>());
        assert!(!camera.contains::<Bloom>());
        assert!(!camera.contains::<DepthOfField>());
    }
}

fn camera_post_process_stage_graphics_options() -> GraphicsOptions {
    GraphicsOptions {
        particle_density: 100,
        particle_effects_enabled: true,
        ssao_enabled: true,
        render_scale: 0.75,
        ui_scale: 1.0,
        vsync_enabled: true,
        frame_rate_limit_enabled: false,
        frame_rate_limit: 144,
        colorblind_mode: false,
        bloom_enabled: true,
        bloom_intensity: 0.12,
        depth_of_field: true,
        anti_alias: AntiAliasMode::Taa,
    }
}

#[test]
fn camera_post_process_stage_empty_removes_only_render_bundle_and_syncs_common_effects() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(camera_post_process_stage_graphics_options());
    app.insert_resource(InWorldSceneStage::Empty);
    app.add_systems(
        Update,
        camera_post_process::sync_camera_graphics_post_process,
    );

    let camera_entity = spawn_wow_camera(&mut app.world_mut().commands());
    app.update();

    let camera = app.world().entity(camera_entity);
    assert!(camera.contains::<Camera3d>());
    assert!(camera.contains::<WowCamera>());
    assert!(camera.contains::<Transform>());
    assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Off));
    assert!(camera.contains::<Tonemapping>());
    assert!(camera.contains::<ShadowFilteringMethod>());
    assert!(camera.contains::<SpatialListener>());
    assert!(!camera.contains::<TemporalAntiAliasing>());
    assert!(!camera.contains::<ScreenSpaceAmbientOcclusion>());
    assert!(!camera.contains::<DepthPrepass>());
    assert!(!camera.contains::<NormalPrepass>());
    assert!(!camera.contains::<MotionVectorPrepass>());
    assert!(!camera.contains::<TemporalJitter>());
    assert!(!camera.contains::<MipBias>());

    let bloom = camera.get::<Bloom>().expect("expected synchronized bloom");
    assert_eq!(bloom.composite_mode, BloomCompositeMode::Additive);
    assert!((bloom.intensity - 0.12).abs() < f32::EPSILON);
    let cas = camera
        .get::<ContrastAdaptiveSharpening>()
        .expect("expected synchronized CAS");
    assert!(cas.enabled);
    assert!((cas.sharpening_strength - 0.6).abs() < f32::EPSILON);
    let dof = camera
        .get::<DepthOfField>()
        .expect("expected synchronized depth of field");
    assert!((dof.focal_distance - 15.0).abs() < f32::EPSILON);
}

#[test]
fn camera_post_process_stage_lighting_restores_render_bundle_after_empty() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(camera_post_process_stage_graphics_options());
    app.insert_resource(InWorldSceneStage::Empty);
    app.add_systems(
        Update,
        camera_post_process::sync_camera_graphics_post_process,
    );

    let camera_entity = spawn_wow_camera(&mut app.world_mut().commands());
    app.update();
    let camera = app.world().entity(camera_entity);
    assert!(!camera.contains::<TemporalAntiAliasing>());
    assert!(!camera.contains::<ScreenSpaceAmbientOcclusion>());
    assert!(!camera.contains::<DepthPrepass>());
    assert!(!camera.contains::<NormalPrepass>());
    assert!(!camera.contains::<MotionVectorPrepass>());
    assert!(!camera.contains::<TemporalJitter>());
    assert!(!camera.contains::<MipBias>());

    *app.world_mut().resource_mut::<InWorldSceneStage>() = InWorldSceneStage::Lighting;
    app.update();

    let camera = app.world().entity(camera_entity);
    assert!(camera.contains::<TemporalAntiAliasing>());
    assert!(camera.contains::<ScreenSpaceAmbientOcclusion>());
    assert!(camera.contains::<DepthPrepass>());
    assert!(camera.contains::<NormalPrepass>());
    assert!(camera.contains::<MotionVectorPrepass>());
    assert!(camera.contains::<TemporalJitter>());
    assert!(camera.contains::<MipBias>());
    assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Off));
}

#[test]
fn camera_post_process_stage_lighting_restores_msaa_prepasses_after_empty() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GraphicsOptions::default());
    app.insert_resource(InWorldSceneStage::Empty);
    app.add_systems(
        Update,
        camera_post_process::sync_camera_graphics_post_process,
    );

    let camera_entity = spawn_wow_camera(&mut app.world_mut().commands());
    app.update();
    let camera = app.world().entity(camera_entity);
    assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Off));
    assert!(!camera.contains::<DepthPrepass>());
    assert!(!camera.contains::<NormalPrepass>());

    *app.world_mut().resource_mut::<InWorldSceneStage>() = InWorldSceneStage::Lighting;
    app.update();

    let camera = app.world().entity(camera_entity);
    assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Sample4));
    assert!(camera.contains::<DepthPrepass>());
    assert!(camera.contains::<NormalPrepass>());
    assert!(!camera.contains::<TemporalAntiAliasing>());
    assert!(!camera.contains::<ScreenSpaceAmbientOcclusion>());
    assert!(!camera.contains::<MotionVectorPrepass>());
    assert!(!camera.contains::<TemporalJitter>());
    assert!(!camera.contains::<MipBias>());
}

#[test]
fn camera_post_process_stage_unconfigured_preserves_graphics_option_behavior() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(camera_post_process_stage_graphics_options());
    app.add_systems(
        Update,
        camera_post_process::sync_camera_graphics_post_process,
    );

    let camera_entity = spawn_wow_camera(&mut app.world_mut().commands());
    app.update();

    let camera = app.world().entity(camera_entity);
    assert!(camera.contains::<Camera3d>());
    assert!(camera.contains::<WowCamera>());
    assert!(camera.contains::<Transform>());
    assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Off));
    assert!(camera.contains::<TemporalAntiAliasing>());
    assert!(camera.contains::<ScreenSpaceAmbientOcclusion>());
    assert!(camera.contains::<DepthPrepass>());
    assert!(camera.contains::<NormalPrepass>());
    assert!(camera.contains::<Tonemapping>());
    assert!(camera.contains::<ShadowFilteringMethod>());
    assert!(camera.contains::<SpatialListener>());
    assert!(camera.contains::<Bloom>());
    assert!(camera.contains::<ContrastAdaptiveSharpening>());
    assert!(camera.contains::<DepthOfField>());
}
