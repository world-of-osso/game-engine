use super::*;
use crate::client_options::GraphicsOptions;
use bevy::core_pipeline::tonemapping::Tonemapping;
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
        bloom_enabled: true,
        bloom_intensity: 0.12,
        depth_of_field: false,
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
            bloom_enabled: false,
            bloom_intensity: 0.12,
            depth_of_field: false,
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
