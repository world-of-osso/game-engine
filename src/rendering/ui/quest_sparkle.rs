use bevy::prelude::*;

use crate::game_state::GameState;
use game_engine::quest_data::QuestLogState;
use game_engine::quest_tracking::{QuestTrackedItem, should_sparkle};

pub struct QuestSparklePlugin;

impl Plugin for QuestSparklePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            sync_quest_sparkles.run_if(in_state(GameState::InWorld)),
        );
    }
}

/// Marker on entities with an active quest sparkle effect.
#[derive(Component)]
pub struct QuestSparkle {
    effect_entity: Entity,
}

/// Golden sparkle color matching WoW's quest item highlight.
const SPARKLE_BASE_COLOR: Color = Color::srgba(1.0, 0.85, 0.0, 0.6);
/// Radius of the sparkle glow sphere.
const SPARKLE_RADIUS: f32 = 0.15;
/// Y offset for sparkle above the object origin.
const SPARKLE_Y: f32 = 0.5;
/// Emissive intensity multiplier for HDR glow.
const SPARKLE_EMISSIVE_STRENGTH: f32 = 3.0;

/// Add or remove sparkle effects on quest-tracked entities based on quest log state.
fn sync_quest_sparkles(
    mut commands: Commands,
    quest_log: Option<Res<QuestLogState>>,
    tracked: Query<(Entity, &QuestTrackedItem, Option<&QuestSparkle>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let default_log = QuestLogState::default();
    let log = quest_log.as_deref().unwrap_or(&default_log);
    for (entity, item, sparkle) in &tracked {
        let needs_sparkle = should_sparkle(item, log);
        match (needs_sparkle, sparkle) {
            (true, None) => add_sparkle(&mut commands, &mut meshes, &mut materials, entity),
            (false, Some(s)) => remove_sparkle(&mut commands, entity, s),
            _ => {}
        }
    }
}

fn add_sparkle(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    entity: Entity,
) {
    let effect = spawn_sparkle_effect(commands, meshes, materials);
    commands.entity(entity).add_child(effect);
    commands.entity(entity).insert(QuestSparkle {
        effect_entity: effect,
    });
}

fn remove_sparkle(commands: &mut Commands, entity: Entity, sparkle: &QuestSparkle) {
    commands.entity(sparkle.effect_entity).despawn();
    commands.entity(entity).remove::<QuestSparkle>();
}

fn spawn_sparkle_effect(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    let mesh = meshes.add(Sphere::new(SPARKLE_RADIUS));
    let material = materials.add(StandardMaterial {
        base_color: SPARKLE_BASE_COLOR,
        emissive: LinearRgba::rgb(
            SPARKLE_EMISSIVE_STRENGTH,
            SPARKLE_EMISSIVE_STRENGTH * 0.85,
            0.0,
        ),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands
        .spawn((
            Name::new("QuestSparkle"),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(0.0, SPARKLE_Y, 0.0),
            Visibility::default(),
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparkle_color_is_golden() {
        let srgba = SPARKLE_BASE_COLOR.to_srgba();
        assert!(srgba.red > 0.9);
        assert!(srgba.green > 0.8);
        assert!(srgba.blue < 0.1);
    }

    #[test]
    fn sparkle_is_translucent() {
        let srgba = SPARKLE_BASE_COLOR.to_srgba();
        assert!(srgba.alpha < 1.0);
        assert!(srgba.alpha > 0.0);
    }

    #[test]
    fn sparkle_emissive_exceeds_unit_for_bloom() {
        assert!(SPARKLE_EMISSIVE_STRENGTH > 1.0);
    }
}
