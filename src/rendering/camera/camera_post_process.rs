use bevy::anti_alias::contrast_adaptive_sharpening::ContrastAdaptiveSharpening;
use bevy::camera::MainPassResolutionOverride;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter};
use bevy::prelude::*;

use crate::client_options::GraphicsOptions;

const MIN_RENDER_SCALE: f32 = 0.5;
const MAX_RENDER_SCALE: f32 = 1.0;
const DEFAULT_CAS_SHARPENING: f32 = 0.6;

pub(crate) fn additive_particle_glow_tonemapping() -> Tonemapping {
    Tonemapping::TonyMcMapface
}

pub(super) fn additive_particle_glow_bloom(graphics: &GraphicsOptions) -> Option<Bloom> {
    graphics.bloom_enabled.then_some(Bloom {
        intensity: graphics.bloom_intensity.clamp(0.0, 1.0),
        low_frequency_boost: 0.7,
        low_frequency_boost_curvature: 0.95,
        high_pass_frequency: 1.0,
        prefilter: BloomPrefilter {
            threshold: 0.65,
            threshold_softness: 0.1,
        },
        composite_mode: BloomCompositeMode::Additive,
        max_mip_dimension: Bloom::OLD_SCHOOL.max_mip_dimension,
        scale: Vec2::ONE,
    })
}

pub(super) fn scaled_main_pass_resolution(target_size: UVec2, render_scale: f32) -> Option<UVec2> {
    let render_scale = render_scale.clamp(MIN_RENDER_SCALE, MAX_RENDER_SCALE);
    if render_scale >= 0.999 {
        return None;
    }
    let scaled = (target_size.as_vec2() * render_scale).floor().as_uvec2();
    let scaled = UVec2::new(
        scaled.x.clamp(1, target_size.x.saturating_sub(1).max(1)),
        scaled.y.clamp(1, target_size.y.saturating_sub(1).max(1)),
    );
    if scaled == target_size {
        None
    } else {
        Some(scaled)
    }
}

pub(super) fn sync_camera_graphics_post_process(
    graphics: Res<GraphicsOptions>,
    mut commands: Commands,
    mut cameras: Query<
        (
            Entity,
            &Camera,
            Option<&mut Bloom>,
            Option<&mut MainPassResolutionOverride>,
            Option<&mut ContrastAdaptiveSharpening>,
        ),
        With<Camera3d>,
    >,
) {
    let desired_bloom = additive_particle_glow_bloom(&graphics);
    for (entity, camera, bloom, resolution_override, cas) in &mut cameras {
        sync_bloom(&mut commands, entity, desired_bloom.clone(), bloom);
        let desired_resolution = camera
            .physical_target_size()
            .and_then(|size| scaled_main_pass_resolution(size, graphics.render_scale));
        sync_resolution(
            &mut commands,
            entity,
            desired_resolution,
            resolution_override,
        );
        sync_sharpening(&mut commands, entity, graphics.render_scale < 0.999, cas);
    }
}

fn sync_bloom(
    commands: &mut Commands,
    entity: Entity,
    desired: Option<Bloom>,
    bloom: Option<Mut<Bloom>>,
) {
    match (desired, bloom) {
        (Some(target), Some(mut existing)) => *existing = target,
        (Some(target), None) => {
            commands.entity(entity).insert(target);
        }
        (None, Some(_)) => {
            commands.entity(entity).remove::<Bloom>();
        }
        (None, None) => {}
    }
}

fn sync_resolution(
    commands: &mut Commands,
    entity: Entity,
    desired_resolution: Option<UVec2>,
    resolution_override: Option<Mut<MainPassResolutionOverride>>,
) {
    match (desired_resolution, resolution_override) {
        (Some(target), Some(mut existing)) => existing.0 = target,
        (Some(target), None) => {
            commands
                .entity(entity)
                .insert(MainPassResolutionOverride(target));
        }
        (None, Some(_)) => {
            commands
                .entity(entity)
                .remove::<MainPassResolutionOverride>();
        }
        (None, None) => {}
    }
}

fn sync_sharpening(
    commands: &mut Commands,
    entity: Entity,
    cas_enabled: bool,
    cas: Option<Mut<ContrastAdaptiveSharpening>>,
) {
    match (cas_enabled, cas) {
        (true, Some(mut existing)) => {
            existing.enabled = true;
            existing.sharpening_strength = DEFAULT_CAS_SHARPENING;
            existing.denoise = false;
        }
        (true, None) => {
            commands.entity(entity).insert(ContrastAdaptiveSharpening {
                enabled: true,
                sharpening_strength: DEFAULT_CAS_SHARPENING,
                denoise: false,
            });
        }
        (false, Some(_)) => {
            commands
                .entity(entity)
                .remove::<ContrastAdaptiveSharpening>();
        }
        (false, None) => {}
    }
}
