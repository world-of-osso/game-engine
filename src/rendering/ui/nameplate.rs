use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use shared::components::{Npc, Player as NetPlayer};

use crate::asset::asset_cache;
use crate::client_options::{
    DEFAULT_NAMEPLATE_DISTANCE, GraphicsOptions, HudOptions, HudVisibilityToggles,
};
use crate::game::inworld_scene_stage::{InWorldSceneStage, inworld_scene_stage_allows_ui};
use crate::game_state::GameState;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_spawn;
use game_engine::nameplate_data::{QuestIndicator, UnitReaction};

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
                sync_nameplate_colors,
                billboard_nameplates,
                fade_nameplates_by_distance,
                sync_quest_indicators,
            )
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// Marker component on the text entity displaying a nameplate.
#[derive(Component)]
pub(crate) struct Nameplate;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum NameplateKind {
    Player,
    Npc,
}

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
    scene_stage: Option<Res<InWorldSceneStage>>,
    ui_disabled: Option<Res<crate::client_options::UiDisabled>>,
) {
    if !inworld_scene_stage_allows_ui(scene_stage, ui_disabled) {
        return;
    }
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
        NameplateKind::Player,
    );
    commands.entity(entity).add_child(nameplate);
}

/// Observer: spawn a nameplate child when an Npc is added.
fn spawn_npc_nameplate(
    trigger: On<Add, Npc>,
    mut commands: Commands,
    query: Query<&Npc>,
    scene_stage: Option<Res<InWorldSceneStage>>,
    ui_disabled: Option<Res<crate::client_options::UiDisabled>>,
) {
    if !inworld_scene_stage_allows_ui(scene_stage, ui_disabled) {
        return;
    }
    let entity = trigger.entity;
    let Ok(npc) = query.get(entity) else { return };
    let label = format!("Creature {}", npc.template_id);
    let nameplate = spawn_nameplate_entity(
        &mut commands,
        &label,
        NPC_NAME_COLOR,
        NPC_FONT_SIZE,
        NPC_NAMEPLATE_Y,
        NameplateKind::Npc,
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
    kind: NameplateKind,
) -> Entity {
    commands
        .spawn((
            Nameplate,
            kind,
            Text2d::new(text),
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(0.0, y_offset, 0.0).with_scale(Vec3::splat(TEXT_SCALE)),
            Visibility::default(),
        ))
        .id()
}

fn sync_nameplate_visibility(
    ui_disabled: Option<Res<crate::client_options::UiDisabled>>,
    hud_visibility: Option<Res<HudVisibilityToggles>>,
    mut query: Query<&mut Visibility, Or<(With<Nameplate>, With<QuestIndicatorModel>)>>,
) {
    let visible =
        ui_disabled.is_none() && hud_visibility.is_none_or(|toggles| toggles.show_nameplates);
    for mut visibility in &mut query {
        visibility.set_if_neq(if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

fn sync_nameplate_colors(
    graphics_options: Option<Res<GraphicsOptions>>,
    mut query: Query<(&NameplateKind, &mut TextColor), With<Nameplate>>,
) {
    let colorblind_mode = graphics_options.is_some_and(|graphics| graphics.colorblind_mode);
    for (kind, mut color) in &mut query {
        color.set_if_neq(TextColor(nameplate_text_color(*kind, colorblind_mode)));
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

fn nameplate_text_color(kind: NameplateKind, colorblind_mode: bool) -> Color {
    let rgba = match kind {
        NameplateKind::Player => {
            if colorblind_mode {
                UnitReaction::Friendly.name_color_for_mode(true)
            } else {
                [1.0, 1.0, 1.0, 1.0]
            }
        }
        NameplateKind::Npc => UnitReaction::Neutral.name_color_for_mode(colorblind_mode),
    };
    Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3])
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
    let mut assets = quest_indicator_spawn_assets(
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inv_bp,
    );
    for (entity, npc_qi, children) in &changed {
        sync_indicator_for_entity(
            &mut commands,
            &mut assets,
            &indicator_models,
            entity,
            npc_qi,
            children,
        );
    }
}

fn quest_indicator_spawn_assets<'a>(
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    effect_materials: &'a mut Assets<M2EffectMaterial>,
    images: &'a mut Assets<Image>,
    inverse_bindposes: &'a mut Assets<SkinnedMeshInverseBindposes>,
) -> m2_spawn::SpawnAssets<'a> {
    m2_spawn::SpawnAssets {
        meshes,
        materials,
        effect_materials,
        skybox_materials: None,
        images,
        inverse_bindposes,
    }
}

fn sync_indicator_for_entity(
    commands: &mut Commands,
    assets: &mut m2_spawn::SpawnAssets<'_>,
    indicator_models: &Query<Entity, With<QuestIndicatorModel>>,
    entity: Entity,
    npc_qi: &NpcQuestIndicator,
    children: Option<&Children>,
) {
    despawn_indicator_children(commands, children, indicator_models);
    if npc_qi.0.is_visible() {
        spawn_indicator_m2(commands, assets, entity, npc_qi.0);
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

    #[derive(Resource, Default)]
    struct ColorChanges(Vec<Entity>);

    fn observe_color_changes(
        query: Query<Entity, (With<Nameplate>, Changed<TextColor>)>,
        mut changes: ResMut<ColorChanges>,
    ) {
        changes.0 = query.iter().collect();
    }

    fn color_test_app() -> App {
        let mut app = App::new();
        app.init_resource::<GraphicsOptions>();
        app.init_resource::<ColorChanges>();
        app.add_systems(Update, (sync_nameplate_colors, fade_nameplates_by_distance));
        app.add_systems(PostUpdate, observe_color_changes);
        app
    }

    fn spawn_color_plate(app: &mut App, kind: NameplateKind) -> Entity {
        app.world_mut()
            .spawn((
                Nameplate,
                kind,
                TextColor(Color::BLACK),
                GlobalTransform::IDENTITY,
            ))
            .id()
    }

    fn assert_color_frame(app: &mut App, expected: &[(Entity, Color)], changed: &[Entity]) {
        app.update();
        for &(entity, color) in expected {
            assert_eq!(
                app.world().get::<TextColor>(entity),
                Some(&TextColor(color))
            );
        }
        let observed = &app.world().resource::<ColorChanges>().0;
        assert_eq!(observed.len(), changed.len(), "unexpected color changes");
        for entity in changed {
            assert!(
                observed.contains(entity),
                "missing color change for {entity:?}"
            );
        }
    }

    fn faded_color_test_app() -> (App, Entity, Entity) {
        let mut app = color_test_app();
        app.insert_resource(HudOptions {
            nameplate_distance: 20.0,
            ..default()
        });
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), GlobalTransform::IDENTITY))
            .id();
        let plate = spawn_color_plate(&mut app, NameplateKind::Npc);
        app.world_mut()
            .entity_mut(plate)
            .insert(GlobalTransform::from_translation(Vec3::new(15.0, 0.0, 0.0)));
        (app, camera, plate)
    }

    #[test]
    fn nameplate_final_color_stays_unchanged_after_distance_fade() {
        let (mut app, _, plate) = faded_color_test_app();
        let yellow = Color::srgba(1.0, 1.0, 0.0, 0.5);
        app.update();
        app.update();
        assert!(app.world().resource::<ColorChanges>().0.is_empty());
        for _ in 0..3 {
            assert_color_frame(&mut app, &[(plate, yellow)], &[]);
        }
    }

    #[test]
    fn nameplate_final_color_tracks_inputs_and_new_plates() {
        let (mut app, camera, plate) = faded_color_test_app();
        assert_color_frame(
            &mut app,
            &[(plate, Color::srgba(1.0, 1.0, 0.0, 0.5))],
            &[plate],
        );
        app.world_mut()
            .entity_mut(camera)
            .insert(GlobalTransform::from_translation(Vec3::new(5.0, 0.0, 0.0)));
        assert_color_frame(
            &mut app,
            &[(plate, Color::srgba(1.0, 1.0, 0.0, 1.0))],
            &[plate],
        );
        app.world_mut()
            .resource_mut::<GraphicsOptions>()
            .colorblind_mode = true;
        assert_color_frame(
            &mut app,
            &[(plate, Color::srgba(1.0, 0.92, 0.35, 1.0))],
            &[plate],
        );
        app.world_mut()
            .entity_mut(plate)
            .insert(NameplateKind::Player);
        let friendly = Color::srgba(0.45, 0.9, 1.0, 1.0);
        assert_color_frame(&mut app, &[(plate, friendly)], &[plate]);
        app.world_mut()
            .resource_mut::<HudOptions>()
            .nameplate_distance = 10.0;
        assert_color_frame(&mut app, &[(plate, friendly.with_alpha(0.0))], &[plate]);
        app.world_mut()
            .entity_mut(plate)
            .insert(GlobalTransform::from_translation(Vec3::new(12.5, 0.0, 0.0)));
        let faded = friendly.with_alpha(0.5);
        assert_color_frame(&mut app, &[(plate, faded)], &[plate]);
        let added = spawn_color_plate(&mut app, NameplateKind::Player);
        app.world_mut()
            .entity_mut(added)
            .insert(GlobalTransform::from_translation(Vec3::new(12.5, 0.0, 0.0)));
        assert_color_frame(&mut app, &[(plate, faded), (added, faded)], &[added]);
        assert_color_frame(&mut app, &[(plate, faded), (added, faded)], &[]);
    }

    #[test]
    fn nameplate_final_color_uses_base_color_without_unique_camera() {
        let (mut app, camera, plate) = faded_color_test_app();
        let yellow = Color::srgba(1.0, 1.0, 0.0, 1.0);
        assert_color_frame(&mut app, &[(plate, yellow.with_alpha(0.5))], &[plate]);
        let second = app
            .world_mut()
            .spawn((Camera3d::default(), GlobalTransform::IDENTITY))
            .id();
        assert_color_frame(&mut app, &[(plate, yellow)], &[plate]);
        assert_color_frame(&mut app, &[(plate, yellow)], &[]);
        app.world_mut().despawn(second);
        assert_color_frame(&mut app, &[(plate, yellow.with_alpha(0.5))], &[plate]);
        app.world_mut().despawn(camera);
        assert_color_frame(&mut app, &[(plate, yellow)], &[plate]);
        app.world_mut()
            .resource_mut::<GraphicsOptions>()
            .colorblind_mode = true;
        assert_color_frame(
            &mut app,
            &[(plate, Color::srgba(1.0, 0.92, 0.35, 1.0))],
            &[plate],
        );
        assert_color_frame(
            &mut app,
            &[(plate, Color::srgba(1.0, 0.92, 0.35, 1.0))],
            &[],
        );
    }

    #[test]
    fn nameplate_color_sync_leaves_stable_frames_unchanged() {
        let mut app = color_test_app();
        let player = spawn_color_plate(&mut app, NameplateKind::Player);
        let npc = spawn_color_plate(&mut app, NameplateKind::Npc);
        let expected = [
            (player, Color::srgba(1.0, 1.0, 1.0, 1.0)),
            (npc, Color::srgba(1.0, 1.0, 0.0, 1.0)),
        ];
        assert_color_frame(&mut app, &expected, &[player, npc]);
        for _ in 0..3 {
            assert_color_frame(&mut app, &expected, &[]);
        }
    }

    #[test]
    fn nameplate_color_sync_applies_mode_and_kind_changes() {
        let mut app = color_test_app();
        let entity = spawn_color_plate(&mut app, NameplateKind::Player);
        assert_color_frame(
            &mut app,
            &[(entity, Color::srgba(1.0, 1.0, 1.0, 1.0))],
            &[entity],
        );
        app.world_mut()
            .resource_mut::<GraphicsOptions>()
            .colorblind_mode = true;
        let friendly = Color::srgba(0.45, 0.9, 1.0, 1.0);
        assert_color_frame(&mut app, &[(entity, friendly)], &[entity]);
        assert_color_frame(&mut app, &[(entity, friendly)], &[]);
        app.world_mut()
            .entity_mut(entity)
            .insert(NameplateKind::Npc);
        let neutral = Color::srgba(1.0, 0.92, 0.35, 1.0);
        assert_color_frame(&mut app, &[(entity, neutral)], &[entity]);
        app.world_mut()
            .resource_mut::<GraphicsOptions>()
            .colorblind_mode = false;
        let yellow = Color::srgba(1.0, 1.0, 0.0, 1.0);
        assert_color_frame(&mut app, &[(entity, yellow)], &[entity]);
        assert_color_frame(&mut app, &[(entity, yellow)], &[]);
    }

    #[test]
    fn nameplate_color_sync_initializes_new_entities_without_touching_existing() {
        let mut app = color_test_app();
        app.world_mut()
            .resource_mut::<GraphicsOptions>()
            .colorblind_mode = true;
        let player = spawn_color_plate(&mut app, NameplateKind::Player);
        let friendly = Color::srgba(0.45, 0.9, 1.0, 1.0);
        assert_color_frame(&mut app, &[(player, friendly)], &[player]);
        let npc = spawn_color_plate(&mut app, NameplateKind::Npc);
        let expected = [
            (player, friendly),
            (npc, Color::srgba(1.0, 0.92, 0.35, 1.0)),
        ];
        assert_color_frame(&mut app, &expected, &[npc]);
        assert_color_frame(&mut app, &expected, &[]);
    }

    #[derive(Resource, Default)]
    struct VisibilityChanges(Vec<Entity>);

    fn observe_visibility_changes(
        query: Query<Entity, Changed<Visibility>>,
        mut changes: ResMut<VisibilityChanges>,
    ) {
        changes.0 = query.iter().collect();
    }

    fn spawn_visibility_pair(app: &mut App, visibility: Visibility) -> [Entity; 2] {
        [
            app.world_mut().spawn((Nameplate, visibility)).id(),
            app.world_mut()
                .spawn((QuestIndicatorModel, visibility))
                .id(),
        ]
    }

    fn assert_visibility_frame(
        app: &mut App,
        entities: &[Entity],
        expected: Visibility,
        changed: &[Entity],
    ) {
        app.update();
        for &entity in entities {
            assert_eq!(app.world().get::<Visibility>(entity), Some(&expected));
        }
        let observed = &app.world().resource::<VisibilityChanges>().0;
        assert_eq!(
            observed.len(),
            changed.len(),
            "unexpected visibility changes"
        );
        for entity in changed {
            assert!(observed.contains(entity), "missing change for {entity:?}");
        }
    }

    #[test]
    fn no_ui_hides_nameplates_and_quest_indicators() {
        let mut app = App::new();
        app.insert_resource(crate::client_options::UiDisabled);
        app.add_systems(Update, sync_nameplate_visibility);
        let entities = spawn_visibility_pair(&mut app, Visibility::Visible);
        app.update();
        for entity in entities {
            assert_eq!(
                app.world().get::<Visibility>(entity),
                Some(&Visibility::Hidden)
            );
        }
    }

    #[test]
    fn nameplate_visibility_changes_only_when_needed() {
        for initial_toggle in [None, Some(true), Some(false)] {
            let mut app = App::new();
            app.init_resource::<VisibilityChanges>();
            app.add_systems(Update, sync_nameplate_visibility);
            app.add_systems(PostUpdate, observe_visibility_changes);
            if let Some(show_nameplates) = initial_toggle {
                app.insert_resource(HudVisibilityToggles {
                    show_nameplates,
                    ..default()
                });
            }
            let initial_visibility = if initial_toggle == Some(false) {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            let mut entities = spawn_visibility_pair(&mut app, Visibility::Visible).to_vec();
            let initial_entities = entities.clone();
            assert_visibility_frame(&mut app, &entities, initial_visibility, &initial_entities);
            for _ in 0..2 {
                assert_visibility_frame(&mut app, &entities, initial_visibility, &[]);
            }

            // New mismatched entities must be corrected even with unchanged HUD input.
            let added = spawn_visibility_pair(&mut app, Visibility::Visible);
            entities.extend(added);
            assert_visibility_frame(&mut app, &entities, initial_visibility, &added);
            assert_visibility_frame(&mut app, &entities, initial_visibility, &[]);

            let mut previous = initial_visibility;
            for show_nameplates in [false, true] {
                app.insert_resource(HudVisibilityToggles {
                    show_nameplates,
                    ..default()
                });
                let expected = if show_nameplates {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                let changed = if expected == previous {
                    &[][..]
                } else {
                    &entities
                };
                assert_visibility_frame(&mut app, &entities, expected, changed);
                for _ in 0..2 {
                    assert_visibility_frame(&mut app, &entities, expected, &[]);
                }
                previous = expected;
            }
        }
    }

    #[test]
    fn no_ui_nameplate_observers_do_not_spawn_children() {
        for disabled in [false, true] {
            let mut app = App::new();
            if disabled {
                app.insert_resource(crate::client_options::UiDisabled);
            }
            app.add_observer(spawn_player_nameplate);
            app.add_observer(spawn_npc_nameplate);
            let player = app
                .world_mut()
                .spawn(NetPlayer {
                    name: "Theron".into(),
                    race: 1,
                    class: 2,
                    appearance: default(),
                })
                .id();
            let npc = app.world_mut().spawn(Npc { template_id: 1642 }).id();
            app.update();
            let expected = usize::from(!disabled);
            for parent in [player, npc] {
                assert_eq!(
                    app.world()
                        .get::<Children>(parent)
                        .map_or(0, |children| children.len()),
                    expected
                );
            }
            let mut nameplates = app.world_mut().query_filtered::<Entity, With<Nameplate>>();
            assert_eq!(nameplates.iter(app.world()).count(), expected * 2);
        }
    }

    #[test]
    fn no_npcs_ui_skips_replicated_npc_nameplate_creation() {
        for (selector, expected_count) in [("ui", 1), ("no-npcs-ui", 0)] {
            let mut app = App::new();
            app.insert_resource(InWorldSceneStage::parse(selector).expect("valid selector"));
            app.add_observer(spawn_npc_nameplate);
            let npc = app.world_mut().spawn(Npc { template_id: 1642 }).id();
            app.update();

            let mut nameplates = app.world_mut().query_filtered::<Entity, With<Nameplate>>();
            assert_eq!(nameplates.iter(app.world()).count(), expected_count);
            assert!(app.world().get::<Npc>(npc).is_some());
        }
    }

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
    fn colorblind_player_nameplate_uses_friendly_palette() {
        let srgba = nameplate_text_color(NameplateKind::Player, true).to_srgba();
        let expected = UnitReaction::Friendly.name_color_for_mode(true);
        assert!((srgba.red - expected[0]).abs() < 1e-4);
        assert!((srgba.green - expected[1]).abs() < 1e-4);
        assert!((srgba.blue - expected[2]).abs() < 1e-4);
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
