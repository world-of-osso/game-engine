use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use shared::components::{Npc, Player as NetPlayer};

use crate::asset::asset_cache;
use crate::client_options::{DEFAULT_NAMEPLATE_DISTANCE, HudOptions, HudVisibilityToggles};
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_spawn;
use game_engine::nameplate_data::QuestIndicator;

pub struct NameplatePlugin;

impl Plugin for NameplatePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_player_nameplate);
        app.add_observer(spawn_npc_nameplate);
        app.add_observer(despawn_quest_indicator_model);
        app.add_systems(
            Update,
            (
                sync_nameplate_visibility,
                billboard_nameplates,
                fade_nameplates_by_distance,
                sync_quest_indicators,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

/// Marker component on the text entity displaying a nameplate.
#[derive(Component)]
pub(crate) struct Nameplate;

/// Quest giver indicator state on an NPC entity.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct NpcQuestIndicator(pub QuestIndicator);

/// Marker on the entity subtree spawned for a quest indicator M2.
#[derive(Component)]
struct QuestIndicatorModel;

const PLAYER_NAMEPLATE_Y: f32 = 3.0;
const NPC_NAMEPLATE_Y: f32 = 2.5;
const PLAYER_FONT_SIZE: f32 = 24.0;
const NPC_FONT_SIZE: f32 = 20.0;
const NPC_NAME_COLOR: Color = Color::srgb(1.0, 0.82, 0.0);
/// Text scale to keep world-space text reasonably sized.
const TEXT_SCALE: f32 = 0.02;
/// Y offset for quest indicator M2 above the NPC origin.
const QUEST_INDICATOR_Y: f32 = 3.5;

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
            Visibility::default(),
        ))
        .id()
}

fn sync_nameplate_visibility(
    hud_visibility: Option<Res<HudVisibilityToggles>>,
    mut query: Query<&mut Visibility, Or<(With<Nameplate>, With<QuestIndicatorModel>)>>,
) {
    let visible = hud_visibility.is_none_or(|toggles| toggles.show_nameplates);
    for mut visibility in &mut query {
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Rotate nameplates to always face the camera (billboard effect).
fn billboard_nameplates(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut plate_query: Query<&mut Transform, Or<(With<Nameplate>, With<QuestIndicatorModel>)>>,
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

fn nameplate_fade_near(fade_far: f32) -> f32 {
    (fade_far * 0.5).max(1.0)
}

/// Compute nameplate alpha based on distance to camera.
/// Full opacity within half the configured max distance, linear fade to 0
/// at the configured max distance.
pub fn nameplate_alpha(distance: f32, fade_far: f32) -> f32 {
    let fade_far = fade_far.max(1.0);
    let fade_near = nameplate_fade_near(fade_far);
    if distance <= fade_near {
        1.0
    } else if distance >= fade_far {
        0.0
    } else {
        1.0 - (distance - fade_near) / (fade_far - fade_near)
    }
}

/// Fade nameplate alpha based on distance to camera.
fn fade_nameplates_by_distance(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    hud_options: Option<Res<HudOptions>>,
    mut plate_query: Query<(&GlobalTransform, &mut TextColor), With<Nameplate>>,
) {
    let Ok(camera_global) = camera_query.single() else {
        return;
    };
    let camera_pos = camera_global.translation();
    let fade_far = hud_options
        .as_deref()
        .map_or(DEFAULT_NAMEPLATE_DISTANCE, |hud| hud.nameplate_distance);
    for (global_tf, mut text_color) in plate_query.iter_mut() {
        let dist = camera_pos.distance(global_tf.translation());
        let alpha = nameplate_alpha(dist, fade_far);
        text_color.0 = text_color.0.with_alpha(alpha);
    }
}

/// Spawn or update quest indicator M2 models when `NpcQuestIndicator` changes.
fn sync_quest_indicators(
    mut commands: Commands,
    changed: Query<(Entity, &NpcQuestIndicator, Option<&Children>), Changed<NpcQuestIndicator>>,
    indicator_models: Query<Entity, With<QuestIndicatorModel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let mut assets = m2_spawn::SpawnAssets {
        meshes: &mut meshes,
        materials: &mut materials,
        effect_materials: &mut effect_materials,
        skybox_materials: None,
        images: &mut images,
        inverse_bindposes: &mut inv_bp,
    };
    for (entity, npc_qi, children) in &changed {
        despawn_indicator_children(&mut commands, children, &indicator_models);
        if npc_qi.0.is_visible() {
            spawn_indicator_m2(&mut commands, &mut assets, entity, npc_qi.0);
        }
    }
}

fn despawn_indicator_children(
    commands: &mut Commands,
    children: Option<&Children>,
    indicator_models: &Query<Entity, With<QuestIndicatorModel>>,
) {
    let Some(children) = children else { return };
    for child in children.iter() {
        if indicator_models.get(child).is_ok() {
            commands.entity(child).despawn();
        }
    }
}

fn spawn_indicator_m2(
    commands: &mut Commands,
    assets: &mut m2_spawn::SpawnAssets<'_>,
    parent: Entity,
    indicator: QuestIndicator,
) {
    let fdid = indicator.model_fdid();
    let Some(m2_path) = asset_cache::model(fdid) else {
        warn!("Quest indicator M2 FDID {fdid} not cached");
        return;
    };
    let indicator_root = commands
        .spawn((
            QuestIndicatorModel,
            Name::new("QuestIndicator"),
            Transform::from_xyz(0.0, QUEST_INDICATOR_Y, 0.0),
            Visibility::default(),
        ))
        .id();
    commands.entity(parent).add_child(indicator_root);
    m2_spawn::spawn_m2_on_entity(commands, assets, &m2_path, indicator_root, &[0, 0, 0]);
}

/// Clean up quest indicator M2 when the component is removed.
fn despawn_quest_indicator_model(
    trigger: On<Remove, NpcQuestIndicator>,
    mut commands: Commands,
    children: Query<&Children>,
    indicator_models: Query<Entity, With<QuestIndicatorModel>>,
) {
    let entity = trigger.entity;
    let Ok(kids) = children.get(entity) else {
        return;
    };
    for child in kids.iter() {
        if indicator_models.get(child).is_ok() {
            commands.entity(child).despawn();
        }
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
        assert!((nameplate_alpha(10.0, DEFAULT_NAMEPLATE_DISTANCE) - 1.0).abs() < 1e-4);
        assert!((nameplate_alpha(30.0, DEFAULT_NAMEPLATE_DISTANCE) - 0.5).abs() < 1e-4);
        assert!((nameplate_alpha(40.0, DEFAULT_NAMEPLATE_DISTANCE)).abs() < 1e-4);
        assert!((nameplate_alpha(50.0, DEFAULT_NAMEPLATE_DISTANCE)).abs() < 1e-4);
    }

    #[test]
    fn farther_nameplate_distance_pushes_fade_out() {
        assert!((nameplate_alpha(30.0, 60.0) - 1.0).abs() < 1e-4);
        assert!((nameplate_alpha(45.0, 60.0) - 0.5).abs() < 1e-4);
        assert!((nameplate_alpha(60.0, 60.0)).abs() < 1e-4);
    }

    #[test]
    fn npc_quest_indicator_wraps_enum() {
        let qi = NpcQuestIndicator(QuestIndicator::Available);
        assert!(qi.0.is_visible());
        assert_eq!(qi.0.glyph(), "!");

        let none = NpcQuestIndicator(QuestIndicator::None);
        assert!(!none.0.is_visible());
    }

    #[test]
    fn quest_indicator_y_above_nameplate() {
        assert!(QUEST_INDICATOR_Y > NPC_NAMEPLATE_Y);
    }
}
