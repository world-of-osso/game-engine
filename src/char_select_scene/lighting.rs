use bevy::prelude::*;
use game_engine::customization_data::ModelPresentation;

use super::{CharSelectScene, camera_params};

fn light_transform(eye: Vec3, focus: Vec3) -> Transform {
    let forward = (focus - eye).normalize_or_zero();
    let flatter_forward = (forward + Vec3::Y * 0.05).normalize_or_zero();
    Transform::from_translation(eye).looking_to(flatter_forward, Vec3::Y)
}

fn spawn_front_fill_light(commands: &mut Commands, eye: Vec3, focus: Vec3) {
    let cam_dir = (focus - eye).normalize_or_zero();
    let fill_pos = focus + cam_dir * -2.5 + Vec3::Y * -0.5;
    commands.spawn((
        Name::new("FrontFillLight"),
        CharSelectScene,
        PointLight {
            intensity: 160_000.0,
            range: 30.0,
            radius: 2.0,
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
        brightness: 105.0,
        ..default()
    });
    let (eye, focus, _) = camera_params(scene, placement, presentation);
    let directional = commands
        .spawn((
            Name::new("DirectionalLight"),
            CharSelectScene,
            DirectionalLight {
                illuminance: 1200.0,
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
