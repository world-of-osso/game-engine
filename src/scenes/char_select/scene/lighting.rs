use bevy::prelude::*;
use game_engine::customization_data::ModelPresentation;

use super::CharSelectScene;
use super::camera::camera_params;

pub(crate) const CHAR_SELECT_AMBIENT_BRIGHTNESS: f32 = 150.0;
const CHAR_SELECT_AMBIENT_COLOR: Color = Color::srgb(0.92, 0.80, 0.60);
const CAMPFIRE_LIGHT_OFFSET: Vec3 = Vec3::new(-2.8, 0.9, -3.1);
const CAMPFIRE_LIGHT_COLOR: Color = Color::srgb(1.0, 0.58, 0.28);
const CAMPFIRE_LIGHT_INTENSITY: f32 = 220_000.0;
const CAMPFIRE_LIGHT_RANGE: f32 = 18.0;
const CAMPFIRE_LIGHT_RADIUS: f32 = 0.55;
const CAMPFIRE_LIGHT_INNER_ANGLE: f32 = std::f32::consts::FRAC_PI_6;
const CAMPFIRE_LIGHT_OUTER_ANGLE: f32 = std::f32::consts::FRAC_PI_3;

pub fn spawn(
    commands: &mut Commands,
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
    placement: Option<&crate::scenes::char_select::warband::WarbandScenePlacement>,
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
            CharSelectScene,
            SpotLight {
                color: CAMPFIRE_LIGHT_COLOR,
                intensity: CAMPFIRE_LIGHT_INTENSITY,
                range: CAMPFIRE_LIGHT_RANGE,
                radius: CAMPFIRE_LIGHT_RADIUS,
                inner_angle: CAMPFIRE_LIGHT_INNER_ANGLE,
                outer_angle: CAMPFIRE_LIGHT_OUTER_ANGLE,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(fire_pos).looking_at(focus, Vec3::Y),
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::App;

    #[test]
    fn spawn_uses_spot_light_for_char_select_primary_light() {
        let mut app = App::new();
        let entity = spawn(
            &mut app.world_mut().commands(),
            None,
            None,
            ModelPresentation::default(),
        );
        app.update();

        assert!(
            app.world().get::<SpotLight>(entity).is_some(),
            "char-select should avoid point lights because Bevy 0.18 blacks out 3D when PointLight + SkinnedMesh + Text are present"
        );
        assert!(app.world().get::<PointLight>(entity).is_none());
        assert!(app.world().get::<CharSelectScene>(entity).is_some());
    }
}
