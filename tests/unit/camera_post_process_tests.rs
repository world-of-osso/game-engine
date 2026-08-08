use super::*;
use crate::client_options::{AntiAliasMode, GraphicsOptions};
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::post_process::bloom::BloomCompositeMode;

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
