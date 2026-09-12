use bevy::prelude::*;

use super::CharSelectScene;
use crate::sky::SkySun;

pub(super) fn spawn(commands: &mut Commands) -> Entity {
    commands.insert_resource(GlobalAmbientLight {
        brightness: 0.0,
        ..default()
    });
    commands
        .spawn((
            Name::new("EnvironmentSun"),
            CharSelectScene,
            SkySun,
            DirectionalLight {
                illuminance: light_consts::lux::OVERCAST_DAY,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
        ))
        .id()
}
