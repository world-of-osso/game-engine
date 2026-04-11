use bevy::prelude::*;
use game_engine::customization_data::ModelPresentation;

use super::CharSelectScene;
use super::camera::camera_params;

pub(crate) const CHAR_SELECT_AMBIENT_BRIGHTNESS: f32 = 450.0;
const CHAR_SELECT_AMBIENT_COLOR: Color = Color::srgb(0.92, 0.80, 0.60);
pub(crate) const CHAR_SELECT_FILL_LIGHT_ILLUMINANCE: f32 = 35_000.0;
const CHAR_SELECT_FILL_LIGHT_COLOR: Color = Color::srgb(0.82, 0.84, 0.92);
const CHAR_SELECT_FILL_LIGHT_EULER: Vec3 = Vec3::new(-0.95, 0.72, 0.0);
const CAMPFIRE_LIGHT_COLOR: Color = Color::srgb(1.0, 0.58, 0.28);
const CAMPFIRE_LIGHT_ILLUMINANCE: f32 = 12_000.0;
const CAMPFIRE_LIGHT_EULER: Vec3 = Vec3::new(-1.2, -0.4, 0.0);

pub(super) struct CharSelectLightingEntities {
    pub(super) primary_light: Entity,
    pub(super) fill_light: Entity,
}

pub(super) fn spawn(
    commands: &mut Commands,
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
    placement: Option<&crate::scenes::char_select::warband::WarbandScenePlacement>,
    presentation: ModelPresentation,
) -> CharSelectLightingEntities {
    insert_char_select_ambient_light(commands);
    let fill_light = spawn_fill_light(commands);
    let focus = resolve_light_focus(scene, placement, presentation);
    let primary_light = spawn_primary_light(commands, placement, focus);
    CharSelectLightingEntities {
        primary_light,
        fill_light,
    }
}

fn insert_char_select_ambient_light(commands: &mut Commands) {
    commands.insert_resource(GlobalAmbientLight {
        color: CHAR_SELECT_AMBIENT_COLOR,
        brightness: CHAR_SELECT_AMBIENT_BRIGHTNESS,
        ..default()
    });
}

fn spawn_fill_light(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Name::new("TerrainFillLight"),
            CharSelectScene,
            DirectionalLight {
                color: CHAR_SELECT_FILL_LIGHT_COLOR,
                illuminance: CHAR_SELECT_FILL_LIGHT_ILLUMINANCE,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(
                EulerRot::XYZ,
                CHAR_SELECT_FILL_LIGHT_EULER.x,
                CHAR_SELECT_FILL_LIGHT_EULER.y,
                CHAR_SELECT_FILL_LIGHT_EULER.z,
            )),
        ))
        .id()
}

fn resolve_light_focus(
    scene: Option<&crate::scenes::char_select::warband::WarbandSceneEntry>,
    placement: Option<&crate::scenes::char_select::warband::WarbandScenePlacement>,
    presentation: ModelPresentation,
) -> Vec3 {
    let (_, focus, _) = camera_params(scene, placement, presentation);
    focus
}

fn spawn_primary_light(
    commands: &mut Commands,
    _placement: Option<&crate::scenes::char_select::warband::WarbandScenePlacement>,
    _focus: Vec3,
) -> Entity {
    commands
        .spawn((
            Name::new("CampfireLight"),
            CharSelectScene,
            DirectionalLight {
                color: CAMPFIRE_LIGHT_COLOR,
                illuminance: CAMPFIRE_LIGHT_ILLUMINANCE,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(
                EulerRot::XYZ,
                CAMPFIRE_LIGHT_EULER.x,
                CAMPFIRE_LIGHT_EULER.y,
                CAMPFIRE_LIGHT_EULER.z,
            )),
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::App;

    #[test]
    fn spawn_uses_directional_light_for_char_select_campfire() {
        let mut app = App::new();
        let lights = spawn(
            &mut app.world_mut().commands(),
            None,
            None,
            ModelPresentation::default(),
        );
        app.update();

        assert!(
            app.world()
                .get::<DirectionalLight>(lights.primary_light)
                .is_some(),
            "campfire should use directional light for uniform warm fill"
        );
        assert!(
            app.world()
                .get::<CharSelectScene>(lights.primary_light)
                .is_some()
        );
    }

    #[test]
    fn spawn_adds_directional_fill_light_for_terrain_visibility() {
        let mut app = App::new();
        let lights = spawn(
            &mut app.world_mut().commands(),
            None,
            None,
            ModelPresentation::default(),
        );
        app.update();

        let Some(light) = app.world().get::<DirectionalLight>(lights.fill_light) else {
            panic!("char-select should include a broad directional fill light");
        };
        assert!(
            app.world()
                .get::<CharSelectScene>(lights.fill_light)
                .is_some(),
            "fill light should tear down with the char-select scene"
        );
        assert_eq!(light.illuminance, CHAR_SELECT_FILL_LIGHT_ILLUMINANCE);
    }
}
