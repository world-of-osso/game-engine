use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::prelude::*;
use bevy::render::camera::{MipBias, TemporalJitter};

use crate::camera::WowCamera;

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct DisableWowCameraPostProcess;

pub(crate) fn register_wow_camera_post_process_diagnostic(app: &mut App) {
    app.add_systems(
        PostUpdate,
        remove_disabled_wow_camera_post_process
            .run_if(resource_exists::<DisableWowCameraPostProcess>),
    );
}

fn remove_disabled_wow_camera_post_process(
    mut commands: Commands,
    cameras: Query<Entity, (With<Camera3d>, With<WowCamera>)>,
) {
    for camera in &cameras {
        commands.entity(camera).remove::<(
            TemporalAntiAliasing,
            ScreenSpaceAmbientOcclusion,
            DepthPrepass,
            NormalPrepass,
            TemporalJitter,
            MipBias,
            MotionVectorPrepass,
        )>();
    }
}
