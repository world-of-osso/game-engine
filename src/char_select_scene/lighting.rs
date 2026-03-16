use bevy::prelude::*;
use game_engine::customization_data::ModelPresentation;

use super::{CharSelectScene, camera_params};

fn light_transform(eye: Vec3, focus: Vec3) -> Transform {
    let cam_dir = (focus - eye).normalize_or_zero();
    let right = cam_dir.cross(Vec3::Y).normalize_or_zero();
    // Keep the light related to the authored camera, but rotate it off-axis so the
    // campsite backdrop gets readable side-lighting instead of a head-on flashlight wash.
    let sun_dir = (cam_dir * 0.2 - right * 0.85 - Vec3::Y * 0.5).normalize_or_zero();
    Transform::from_translation(focus - sun_dir * 12.0).looking_to(sun_dir, Vec3::Y)
}

fn spawn_front_fill_light(commands: &mut Commands, eye: Vec3, focus: Vec3) {
    let cam_dir = (focus - eye).normalize_or_zero();
    let right = cam_dir.cross(Vec3::Y).normalize_or_zero();
    let fill_pos = focus + cam_dir * -2.5 + Vec3::Y * -0.5 - right * 1.2;
    commands.spawn((
        Name::new("FrontFillLight"),
        CharSelectScene,
        PointLight {
            intensity: 70_000.0,
            range: 12.0,
            radius: 1.5,
            shadows_enabled: false,
            color: Color::srgb(1.0, 0.94, 0.86),
            ..default()
        },
        Transform::from_translation(fill_pos),
    ));
}

pub fn spawn(
    commands: &mut Commands,
    scene: Option<&crate::warband_scene::WarbandSceneEntry>,
    placement: Option<&crate::warband_scene::WarbandScenePlacement>,
    presentation: ModelPresentation,
) -> Entity {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(1.0, 0.95, 0.85),
        brightness: 70.0,
        ..default()
    });
    let (eye, focus, _) = camera_params(scene, placement, presentation);
    let directional = commands
        .spawn((
            Name::new("DirectionalLight"),
            CharSelectScene,
            DirectionalLight {
                illuminance: 2200.0,
                shadows_enabled: true,
                color: Color::srgb(1.0, 0.92, 0.8),
                ..default()
            },
            light_transform(eye, focus),
        ))
        .id();
    spawn_front_fill_light(commands, eye, focus);
    directional
}
