use bevy::prelude::*;
use shared::components::{Npc, Player as NetPlayer};

use crate::game_state::GameState;

pub struct NameplatePlugin;

impl Plugin for NameplatePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_player_nameplate);
        app.add_observer(spawn_npc_nameplate);
        app.add_systems(
            Update,
            (billboard_nameplates, fade_nameplates_by_distance)
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

/// Marker component on the text entity displaying a nameplate.
#[derive(Component)]
struct Nameplate;

const PLAYER_NAMEPLATE_Y: f32 = 3.0;
const NPC_NAMEPLATE_Y: f32 = 2.5;
const PLAYER_FONT_SIZE: f32 = 24.0;
const NPC_FONT_SIZE: f32 = 20.0;
const NPC_NAME_COLOR: Color = Color::srgb(1.0, 0.82, 0.0);
/// Text scale to keep world-space text reasonably sized.
const TEXT_SCALE: f32 = 0.02;

/// Full opacity within this distance (in world units).
const FADE_NEAR: f32 = 20.0;
/// Fully transparent beyond this distance.
const FADE_FAR: f32 = 40.0;

/// Observer: spawn a nameplate child when a NetPlayer is added.
fn spawn_player_nameplate(
    trigger: On<Add, NetPlayer>,
    mut commands: Commands,
    query: Query<&NetPlayer>,
) {
    let entity = trigger.entity;
    let Ok(player) = query.get(entity) else {
        return;
    };
    let nameplate = spawn_nameplate_entity(
        &mut commands,
        &player.name,
        Color::WHITE,
        PLAYER_FONT_SIZE,
        PLAYER_NAMEPLATE_Y,
    );
    commands.entity(entity).add_child(nameplate);
}

/// Observer: spawn a nameplate child when an Npc is added.
fn spawn_npc_nameplate(trigger: On<Add, Npc>, mut commands: Commands, query: Query<&Npc>) {
    let entity = trigger.entity;
    let Ok(npc) = query.get(entity) else { return };
    let label = format!("Creature {}", npc.template_id);
    let nameplate = spawn_nameplate_entity(
        &mut commands,
        &label,
        NPC_NAME_COLOR,
        NPC_FONT_SIZE,
        NPC_NAMEPLATE_Y,
    );
    commands.entity(entity).add_child(nameplate);
}

/// Create a Text2d nameplate entity positioned above the parent.
fn spawn_nameplate_entity(
    commands: &mut Commands,
    text: &str,
    color: Color,
    font_size: f32,
    y_offset: f32,
) -> Entity {
    commands
        .spawn((
            Nameplate,
            Text2d::new(text),
            TextFont {
                font_size,
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(0.0, y_offset, 0.0).with_scale(Vec3::splat(TEXT_SCALE)),
        ))
        .id()
}

/// Rotate nameplates to always face the camera (billboard effect).
fn billboard_nameplates(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut plate_query: Query<&mut Transform, With<Nameplate>>,
) {
    let Ok(camera_global) = camera_query.single() else {
        return;
    };
    let camera_pos = camera_global.translation();
    for mut transform in plate_query.iter_mut() {
        let dir = camera_pos - transform.translation;
        if dir.length_squared() > 0.001 {
            let look = Transform::from_translation(transform.translation)
                .looking_to(Dir3::new(dir).unwrap_or(Dir3::Z), Dir3::Y);
            transform.rotation = look.rotation;
        }
    }
}

/// Compute nameplate alpha based on distance to camera.
/// Full opacity within `FADE_NEAR`, linear fade to 0 at `FADE_FAR`.
pub fn nameplate_alpha(distance: f32) -> f32 {
    if distance <= FADE_NEAR {
        1.0
    } else if distance >= FADE_FAR {
        0.0
    } else {
        1.0 - (distance - FADE_NEAR) / (FADE_FAR - FADE_NEAR)
    }
}

/// Fade nameplate alpha based on distance to camera.
fn fade_nameplates_by_distance(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut plate_query: Query<(&GlobalTransform, &mut TextColor), With<Nameplate>>,
) {
    let Ok(camera_global) = camera_query.single() else {
        return;
    };
    let camera_pos = camera_global.translation();
    for (global_tf, mut text_color) in plate_query.iter_mut() {
        let dist = camera_pos.distance(global_tf.translation());
        let alpha = nameplate_alpha(dist);
        text_color.0 = text_color.0.with_alpha(alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_nameplate_color() {
        // Player nameplates should be white.
        let color = Color::WHITE;
        let srgba = color.to_srgba();
        assert!((srgba.red - 1.0).abs() < 1e-4);
        assert!((srgba.green - 1.0).abs() < 1e-4);
        assert!((srgba.blue - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_npc_nameplate_color() {
        // NPC nameplates should be WoW yellow.
        let srgba = NPC_NAME_COLOR.to_srgba();
        assert!((srgba.red - 1.0).abs() < 1e-4);
        assert!((srgba.green - 0.82).abs() < 1e-4);
        assert!((srgba.blue - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_fade_at_distance() {
        // Full opacity at 10yd (within FADE_NEAR).
        assert!((nameplate_alpha(10.0) - 1.0).abs() < 1e-4);
        // Half opacity at 30yd (midpoint of 20..40 range).
        assert!((nameplate_alpha(30.0) - 0.5).abs() < 1e-4);
        // Zero opacity at 40yd and beyond.
        assert!((nameplate_alpha(40.0)).abs() < 1e-4);
        assert!((nameplate_alpha(50.0)).abs() < 1e-4);
    }
}
