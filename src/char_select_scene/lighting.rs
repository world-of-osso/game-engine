use bevy::prelude::*;
use game_engine::customization_data::ModelPresentation;

use super::camera_params;

pub(crate) const CHAR_SELECT_AMBIENT_BRIGHTNESS: f32 = 150.0;
const CHAR_SELECT_AMBIENT_COLOR: Color = Color::srgb(0.92, 0.80, 0.60);
const CAMPFIRE_LIGHT_OFFSET: Vec3 = Vec3::new(-2.8, 0.9, -3.1);
const CAMPFIRE_LIGHT_COLOR: Color = Color::srgb(1.0, 0.58, 0.28);
const CAMPFIRE_LIGHT_INTENSITY: f32 = 220_000.0;
const CAMPFIRE_LIGHT_RANGE: f32 = 18.0;
const CAMPFIRE_LIGHT_RADIUS: f32 = 0.55;

pub fn spawn(
    commands: &mut Commands,
    scene: Option<&crate::warband_scene::WarbandSceneEntry>,
    placement: Option<&crate::warband_scene::WarbandScenePlacement>,
    presentation: ModelPresentation,
) -> Entity {
    commands.insert_resource(GlobalAmbientLight {
        color: CHAR_SELECT_AMBIENT_COLOR,
        brightness: CHAR_SELECT_AMBIENT_BRIGHTNESS,
        ..default()
    });
    let (_, focus, _) = camera_params(scene, placement, presentation);
    let fire_pos = placement
        .map(|placement| placement.bevy_position() + CAMPFIRE_LIGHT_OFFSET)
        .unwrap_or(focus + CAMPFIRE_LIGHT_OFFSET);
    commands
        .spawn((
            Name::new("CampfireLight"),
            PointLight {
                color: CAMPFIRE_LIGHT_COLOR,
                intensity: CAMPFIRE_LIGHT_INTENSITY,
                range: CAMPFIRE_LIGHT_RANGE,
                radius: CAMPFIRE_LIGHT_RADIUS,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(fire_pos),
        ))
        .id()
}
