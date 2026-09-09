use bevy::anti_alias::contrast_adaptive_sharpening::ContrastAdaptiveSharpening;
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::MainPassResolutionOverride;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::query::QueryData;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter};
use bevy::post_process::dof::DepthOfField;
use bevy::prelude::*;
use bevy::render::camera::{MipBias, TemporalJitter};

use super::WowCamera;
use crate::client_options::{AntiAliasMode, GraphicsOptions};
use crate::game::inworld_scene_stage::{InWorldSceneStage, configured_inworld_scene_stage};

const MIN_RENDER_SCALE: f32 = 0.5;
const MAX_RENDER_SCALE: f32 = 1.0;
const DEFAULT_CAS_SHARPENING: f32 = 0.6;

#[derive(Resource)]
pub(crate) struct MsaaDisabled;

pub(super) fn sync_ui_camera_msaa(
    world_cameras: Query<(&Camera, &Msaa), With<Camera3d>>,
    mut ui_cameras: Query<&mut Msaa, (With<ui_toolkit::render::UiCamera>, Without<Camera3d>)>,
) {
    let mut active_cameras = world_cameras.iter().filter(|(camera, _)| camera.is_active);
    let Some((_, world_msaa)) = active_cameras.next() else {
        return;
    };
    if active_cameras.next().is_some() {
        bevy::log::error_once!("Cannot synchronize UI MSAA with multiple active 3D cameras");
        return;
    }
    // Composited cameras must share sampling so Bevy reuses their main target.
    for mut ui_msaa in &mut ui_cameras {
        if *ui_msaa != *world_msaa {
            info!(
                "UI camera MSAA synchronized: {:?} -> {:?}",
                *ui_msaa, world_msaa
            );
            *ui_msaa = *world_msaa;
        }
    }
}

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

#[derive(QueryData)]
#[query_data(mutable)]
pub(super) struct CameraPostProcessQuery {
    entity: Entity,
    camera: &'static Camera,
    bloom: Option<&'static mut Bloom>,
    resolution_override: Option<&'static mut MainPassResolutionOverride>,
    cas: Option<&'static mut ContrastAdaptiveSharpening>,
    dof: Option<&'static mut DepthOfField>,
    msaa: Option<&'static Msaa>,
    taa: Option<&'static TemporalAntiAliasing>,
    has_ssao: Has<ScreenSpaceAmbientOcclusion>,
    has_depth_prepass: Has<DepthPrepass>,
    has_normal_prepass: Has<NormalPrepass>,
    is_wow_camera: Has<WowCamera>,
}

pub(super) fn sync_camera_graphics_post_process(
    graphics: Res<GraphicsOptions>,
    scene_stage: Option<Res<InWorldSceneStage>>,
    msaa_disabled: Option<Res<MsaaDisabled>>,
    mut commands: Commands,
    mut cameras: Query<CameraPostProcessQuery, With<Camera3d>>,
) {
    graphics
        .validate()
        .unwrap_or_else(|error| panic!("{error}"));
    let desired_bloom = additive_particle_glow_bloom(&graphics);
    let wow_camera_render_bundle_enabled =
        configured_inworld_scene_stage(scene_stage).includes(InWorldSceneStage::Lighting);
    for mut camera in &mut cameras {
        sync_common_camera_post_process(&graphics, &desired_bloom, &mut commands, &mut camera);
        sync_camera_render_bundle(
            &graphics,
            wow_camera_render_bundle_enabled,
            msaa_disabled.is_some(),
            &mut commands,
            &camera,
        );
    }
}

fn sync_common_camera_post_process(
    graphics: &GraphicsOptions,
    desired_bloom: &Option<Bloom>,
    commands: &mut Commands,
    camera: &mut CameraPostProcessQueryItem<'_, '_>,
) {
    sync_bloom(
        commands,
        camera.entity,
        desired_bloom.clone(),
        camera.bloom.take(),
    );
    sync_camera_resolution(graphics, commands, camera);
    sync_sharpening(
        commands,
        camera.entity,
        graphics.render_scale < 0.999,
        camera.cas.take(),
    );
    sync_depth_of_field(
        commands,
        camera.entity,
        graphics.depth_of_field,
        camera.dof.take(),
    );
}

fn sync_camera_resolution(
    graphics: &GraphicsOptions,
    commands: &mut Commands,
    camera: &mut CameraPostProcessQueryItem<'_, '_>,
) {
    let desired_resolution = camera
        .camera
        .physical_target_size()
        .and_then(|size| scaled_main_pass_resolution(size, graphics.render_scale));
    sync_resolution(
        commands,
        camera.entity,
        desired_resolution,
        camera.resolution_override.take(),
    );
}

fn sync_camera_render_bundle(
    graphics: &GraphicsOptions,
    wow_camera_render_bundle_enabled: bool,
    msaa_disabled: bool,
    commands: &mut Commands,
    camera: &CameraPostProcessQueryItem<'_, '_>,
) {
    if camera.is_wow_camera && !wow_camera_render_bundle_enabled {
        remove_wow_camera_render_bundle(commands, camera.entity);
        return;
    }
    if camera.is_wow_camera {
        restore_wow_camera_prepasses(
            commands,
            camera.entity,
            camera.has_depth_prepass,
            camera.has_normal_prepass,
        );
    }
    sync_camera_anti_aliasing_and_ssao(graphics, msaa_disabled, commands, camera);
}

fn sync_camera_anti_aliasing_and_ssao(
    graphics: &GraphicsOptions,
    msaa_disabled: bool,
    commands: &mut Commands,
    camera: &CameraPostProcessQueryItem<'_, '_>,
) {
    let anti_alias = if msaa_disabled && graphics.anti_alias == AntiAliasMode::Msaa4x {
        AntiAliasMode::None
    } else {
        graphics.anti_alias
    };
    sync_anti_alias(commands, camera.entity, anti_alias, camera.msaa, camera.taa);
    sync_ssao_compatibility(
        commands,
        camera.entity,
        graphics.ssao_enabled,
        camera.has_ssao,
        camera.is_wow_camera,
    );
}

fn remove_wow_camera_render_bundle(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).remove::<(
        TemporalAntiAliasing,
        ScreenSpaceAmbientOcclusion,
        DepthPrepass,
        NormalPrepass,
        MotionVectorPrepass,
        TemporalJitter,
        MipBias,
    )>();
}

fn restore_wow_camera_prepasses(
    commands: &mut Commands,
    entity: Entity,
    has_depth_prepass: bool,
    has_normal_prepass: bool,
) {
    match (has_depth_prepass, has_normal_prepass) {
        (false, false) => {
            commands
                .entity(entity)
                .insert((DepthPrepass, NormalPrepass));
        }
        (false, true) => {
            commands.entity(entity).insert(DepthPrepass);
        }
        (true, false) => {
            commands.entity(entity).insert(NormalPrepass);
        }
        (true, true) => {}
    }
}

fn sync_depth_of_field(
    commands: &mut Commands,
    entity: Entity,
    enabled: bool,
    dof: Option<Mut<DepthOfField>>,
) {
    match (enabled, dof) {
        (true, None) => {
            commands.entity(entity).insert(DepthOfField {
                focal_distance: 15.0,
                aperture_f_stops: 1.0 / 8.0,
                max_circle_of_confusion_diameter: 64.0,
                ..Default::default()
            });
        }
        (false, Some(_)) => {
            commands.entity(entity).remove::<DepthOfField>();
        }
        _ => {}
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

fn sync_ssao_compatibility(
    commands: &mut Commands,
    entity: Entity,
    enabled: bool,
    has_ssao: bool,
    is_wow_camera: bool,
) {
    match (enabled, has_ssao) {
        (false, true) => {
            commands
                .entity(entity)
                .remove::<ScreenSpaceAmbientOcclusion>();
        }
        (true, false) if is_wow_camera => {
            commands
                .entity(entity)
                .insert(ScreenSpaceAmbientOcclusion::default());
        }
        _ => {}
    }
}

fn sync_anti_alias(
    commands: &mut Commands,
    entity: Entity,
    mode: AntiAliasMode,
    msaa: Option<&Msaa>,
    taa: Option<&TemporalAntiAliasing>,
) {
    let current_msaa = msaa.copied().unwrap_or(Msaa::Off);
    let has_taa = taa.is_some();
    match mode {
        AntiAliasMode::None => {
            if current_msaa != Msaa::Off {
                commands.entity(entity).insert(Msaa::Off);
            }
            if has_taa {
                commands.entity(entity).remove::<TemporalAntiAliasing>();
            }
        }
        AntiAliasMode::Msaa4x => {
            if current_msaa != Msaa::Sample4 {
                commands.entity(entity).insert(Msaa::Sample4);
            }
            if has_taa {
                commands.entity(entity).remove::<TemporalAntiAliasing>();
            }
        }
        AntiAliasMode::Taa => {
            if current_msaa != Msaa::Off {
                commands.entity(entity).insert(Msaa::Off);
            }
            if !has_taa {
                commands
                    .entity(entity)
                    .insert(TemporalAntiAliasing::default());
            }
        }
    }
}
