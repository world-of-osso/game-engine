use super::*;
use crate::client_options::{AntiAliasMode, GraphicsOptions};
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::audio::SpatialListener;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::ShadowFilteringMethod;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::post_process::bloom::BloomCompositeMode;
use bevy::render::camera::{MipBias, TemporalJitter};

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
    app.update();

    let camera = app.world().entity(camera_entity);
    assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Off));
    assert!(camera.get::<TemporalAntiAliasing>().is_some());
    assert!(camera.get::<ScreenSpaceAmbientOcclusion>().is_some());
}

fn camera_post_process_diagnostic_app(disable_post_process: bool) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut graphics = GraphicsOptions::default();
    graphics.anti_alias = AntiAliasMode::Taa;
    app.insert_resource(graphics);
    app.add_systems(
        Update,
        camera_post_process::sync_camera_graphics_post_process,
    );
    if disable_post_process {
        app.insert_resource(
            crate::rendering::camera_post_process_diagnostic::DisableWowCameraPostProcess,
        );
    }
    crate::rendering::camera_post_process_diagnostic::register_wow_camera_post_process_diagnostic(
        &mut app,
    );
    let camera = spawn_wow_camera(&mut app.world_mut().commands());
    app.world_mut().flush();
    app.update();
    (app, camera)
}

#[test]
fn selected_camera_diagnostic_removes_only_post_process_bundle() {
    let (app, camera) = camera_post_process_diagnostic_app(true);
    let entity = app.world().entity(camera);

    assert!(entity.contains::<Camera3d>());
    assert!(entity.contains::<WowCamera>());
    assert!(entity.contains::<Transform>());
    assert!(entity.contains::<Msaa>());
    assert!(entity.contains::<Tonemapping>());
    assert!(entity.contains::<ShadowFilteringMethod>());
    assert!(entity.contains::<SpatialListener>());
    assert!(!entity.contains::<TemporalAntiAliasing>());
    assert!(!entity.contains::<ScreenSpaceAmbientOcclusion>());
    assert!(!entity.contains::<DepthPrepass>());
    assert!(!entity.contains::<NormalPrepass>());
    assert!(!entity.contains::<TemporalJitter>());
    assert!(!entity.contains::<MipBias>());
    assert!(!entity.contains::<MotionVectorPrepass>());
}

#[test]
fn unselected_camera_diagnostic_preserves_post_process_bundle() {
    let (app, camera) = camera_post_process_diagnostic_app(false);
    let entity = app.world().entity(camera);

    assert!(entity.contains::<TemporalAntiAliasing>());
    assert!(entity.contains::<ScreenSpaceAmbientOcclusion>());
    assert!(entity.contains::<DepthPrepass>());
    assert!(entity.contains::<NormalPrepass>());
    assert!(entity.contains::<TemporalJitter>());
    assert!(entity.contains::<MipBias>());
    assert!(entity.contains::<MotionVectorPrepass>());
}
