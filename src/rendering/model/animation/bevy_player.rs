//! Bind M2 joints to Bevy playback; WoW sequence policy remains in `M2AnimPlayer`.

use std::collections::HashSet;

use bevy::animation::{AnimatedBy, AnimationTargetId, graph::AnimationNodeIndex};
use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;

use super::{M2AnimData, M2AnimPlayer, animation_active_state, bevy_curves};
use crate::game_state::GameState;

#[derive(Component)]
pub(crate) struct M2BevyAnimation {
    joints: Vec<Entity>,
    current_nodes: Vec<AnimationNodeIndex>,
    outgoing_nodes: Vec<AnimationNodeIndex>,
}

/// Two nodes per clip retain independent seek times even when a sequence crossfades to itself.
pub(crate) fn bind_m2_animation_players(
    mut commands: Commands,
    models: Query<
        (Entity, &M2AnimData, Option<&M2BevyAnimation>),
        (Changed<M2AnimData>, Allow<Disabled>),
    >,
    targets: Query<&AnimatedBy, Allow<Disabled>>,
    joints: Query<(), (With<super::BonePivot>, Allow<Disabled>)>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (owner, data, previous) in &models {
        if let Some(previous) = previous {
            detach_obsolete_targets(
                &mut commands,
                owner,
                &previous.joints,
                &data.joint_entities,
                &targets,
            );
        }
        let (graph, binding) = build_animation_graph(data, &mut clips, &mut graphs);
        for (index, &joint) in data.joint_entities.iter().enumerate() {
            if joints.contains(joint) {
                commands
                    .entity(joint)
                    .insert((bevy_curves::bone_target_id(index), AnimatedBy(owner)));
            }
        }
        commands
            .entity(owner)
            .insert((AnimationPlayer::default(), graph, binding));
    }
}

fn build_animation_graph(
    data: &M2AnimData,
    clips: &mut Assets<AnimationClip>,
    graphs: &mut Assets<AnimationGraph>,
) -> (AnimationGraphHandle, M2BevyAnimation) {
    let mut graph = AnimationGraph::new();
    let mut current_nodes = Vec::new();
    let mut outgoing_nodes = Vec::new();
    for sequence in 0..data.sequences.len().max(1) {
        let clip = clips.add(bevy_curves::build_clip(data, sequence));
        current_nodes.push(graph.add_clip(clip.clone(), 1.0, graph.root));
        outgoing_nodes.push(graph.add_clip(clip, 1.0, graph.root));
    }
    (
        AnimationGraphHandle(graphs.add(graph)),
        M2BevyAnimation {
            joints: data.joint_entities.clone(),
            current_nodes,
            outgoing_nodes,
        },
    )
}

fn detach_obsolete_targets(
    commands: &mut Commands,
    owner: Entity,
    previous: &[Entity],
    current: &[Entity],
    targets: &Query<&AnimatedBy, Allow<Disabled>>,
) {
    let retained: HashSet<_> = current.iter().copied().collect();
    for &joint in previous {
        if !retained.contains(&joint) && targets.get(joint).is_ok_and(|binding| binding.0 == owner)
        {
            commands
                .entity(joint)
                .remove::<(AnimationTargetId, AnimatedBy)>();
        }
    }
}

/// Removing model data also retires playback and bindings, without despawning retained joints.
pub(crate) fn remove_m2_animation_player(
    event: On<Remove, M2AnimData>,
    mut commands: Commands,
    models: Query<&M2BevyAnimation, Allow<Disabled>>,
    targets: Query<&AnimatedBy, Allow<Disabled>>,
) {
    let Ok(model) = models.get(event.entity) else {
        return;
    };
    detach_obsolete_targets(&mut commands, event.entity, &model.joints, &[], &targets);
    let owner = event.entity;
    commands.queue(move |world: &mut World| {
        if let Ok(mut entity) = world.get_entity_mut(owner) {
            entity.remove::<(AnimationPlayer, AnimationGraphHandle, M2BevyAnimation)>();
        }
    });
}

/// Paused Bevy clips sample controller times exactly once; Bevy does not advance a second clock.
/// This must also run in inactive states, where it stops previously selected clips.
pub(crate) fn sync_m2_animation_players(
    state: Option<Res<State<GameState>>>,
    mut players: Query<(&M2AnimPlayer, &M2BevyAnimation, &mut AnimationPlayer)>,
) {
    let active = animation_active_state(state);
    for (controller, binding, mut player) in &mut players {
        player.stop_all();
        if !active {
            continue;
        }
        let Some(&current) = binding.current_nodes.get(controller.current_seq_idx) else {
            panic!(
                "M2 animation sequence {} has no Bevy clip",
                controller.current_seq_idx
            );
        };
        let current_weight = apply_outgoing_transition(controller, binding, &mut player);
        player
            .play(current)
            .pause()
            .set_weight(current_weight)
            .seek_to(controller.time_ms / 1000.0);
    }
}

fn apply_outgoing_transition(
    controller: &M2AnimPlayer,
    binding: &M2BevyAnimation,
    player: &mut AnimationPlayer,
) -> f32 {
    let Some(transition) = &controller.transition else {
        return 1.0;
    };
    let blend = (transition.blend_elapsed_ms / transition.blend_duration_ms).clamp(0.0, 1.0);
    let Some(&outgoing) = binding.outgoing_nodes.get(transition.from_seq_idx) else {
        panic!(
            "M2 outgoing animation sequence {} has no Bevy clip",
            transition.from_seq_idx
        );
    };
    player
        .play(outgoing)
        .pause()
        .set_weight(1.0 - blend)
        .seek_to(transition.from_time_ms / 1000.0);
    blend
}

#[cfg(test)]
mod tests {
    use super::super::{AnimTransition, BonePivot};
    use super::*;
    use crate::asset::m2_anim::{AnimTrack, BoneAnimTracks, M2AnimSequence};
    use bevy::asset::AssetPlugin;
    use bevy::state::app::StatesPlugin;

    fn fixture_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            StatesPlugin,
            bevy::animation::AnimationPlugin,
        ));
        app.insert_state(GameState::M2Debug);
        app.add_systems(
            Update,
            (bind_m2_animation_players, sync_m2_animation_players).chain(),
        );
        app.add_observer(remove_m2_animation_player);
        app
    }

    fn empty_track<T>() -> AnimTrack<T> {
        AnimTrack {
            interpolation_type: 1,
            global_sequence: -1,
            sequences: vec![],
        }
    }

    fn data(joint: Entity) -> M2AnimData {
        M2AnimData {
            bones: vec![],
            spherical_billboards: vec![false],
            sequences: vec![M2AnimSequence {
                id: 0,
                variation_id: 0,
                duration: 1000,
                movespeed: 0.0,
                flags: 0,
                blend_time: 150,
                next_animation: -1,
            }],
            bone_tracks: vec![BoneAnimTracks {
                translation: AnimTrack {
                    interpolation_type: 1,
                    global_sequence: -1,
                    sequences: vec![(vec![0, 1000], vec![[0.0; 3], [10.0, 0.0, 0.0]])],
                },
                rotation: empty_track(),
                scale: empty_track(),
            }],
            joint_entities: vec![joint],
        }
    }

    fn spawn_model(app: &mut App) -> (Entity, Entity) {
        let joint = app
            .world_mut()
            .spawn((Transform::default(), BonePivot(Vec3::ZERO)))
            .id();
        let owner = app
            .world_mut()
            .spawn((
                data(joint),
                M2AnimPlayer {
                    current_seq_idx: 0,
                    time_ms: 250.0,
                    looping: true,
                    transition: None,
                },
            ))
            .id();
        (owner, joint)
    }

    fn settle(app: &mut App) {
        // Asset events make the newly built graph available to Bevy playback.
        for _ in 0..3 {
            app.update();
        }
    }

    fn assert_x(app: &App, joint: Entity, expected: f32) {
        let actual = app.world().get::<Transform>(joint).unwrap().translation.x;
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn bevy_player_samples_controller_time_without_double_advance() {
        let mut app = fixture_app();
        let (owner, joint) = spawn_model(&mut app);
        settle(&mut app);
        assert_x(&app, joint, 2.5);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(10));
        app.update();
        assert_x(&app, joint, 2.5);
        let binding = app.world().get::<M2BevyAnimation>(owner).unwrap();
        let active = app
            .world()
            .get::<AnimationPlayer>(owner)
            .unwrap()
            .animation(binding.current_nodes[0])
            .unwrap();
        assert!(active.is_paused());
    }

    #[test]
    fn same_sequence_crossfade_retains_two_independent_sample_times() {
        let mut app = fixture_app();
        let (owner, joint) = spawn_model(&mut app);
        {
            let mut controller = app.world_mut().get_mut::<M2AnimPlayer>(owner).unwrap();
            controller.time_ms = 800.0;
            controller.transition = Some(AnimTransition {
                from_seq_idx: 0,
                from_time_ms: 200.0,
                blend_duration_ms: 150.0,
                blend_elapsed_ms: 75.0,
            });
        }
        settle(&mut app);
        assert_x(&app, joint, 5.0);
    }

    #[test]
    fn replacing_model_detaches_old_joint_without_despawning_it() {
        let mut app = fixture_app();
        let (owner, old) = spawn_model(&mut app);
        settle(&mut app);
        let new = app
            .world_mut()
            .spawn((Transform::default(), BonePivot(Vec3::ZERO)))
            .id();
        app.world_mut().entity_mut(owner).insert(data(new));
        settle(&mut app);
        assert!(app.world().get_entity(old).is_ok());
        assert!(app.world().get::<AnimatedBy>(old).is_none());
        assert!(app.world().get::<AnimationTargetId>(old).is_none());
        assert_eq!(app.world().get::<AnimatedBy>(new).unwrap().0, owner);
        assert_x(&app, new, 2.5);
    }

    #[test]
    fn retiring_model_preserves_joints_reassigned_to_another_owner() {
        let mut app = fixture_app();
        let (owner, joint) = spawn_model(&mut app);
        settle(&mut app);
        let other = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(joint).insert(AnimatedBy(other));
        app.world_mut().entity_mut(owner).remove::<M2AnimData>();
        app.update();
        assert_eq!(app.world().get::<AnimatedBy>(joint).unwrap().0, other);
        assert!(app.world().get::<AnimationTargetId>(joint).is_some());
        assert!(app.world().get::<AnimationPlayer>(owner).is_none());
    }

    #[test]
    fn disabled_and_inactive_owners_do_not_overwrite_joint_pose() {
        let mut app = fixture_app();
        let (owner, joint) = spawn_model(&mut app);
        settle(&mut app);
        app.world_mut().entity_mut(owner).insert(Disabled);
        app.world_mut()
            .get_mut::<Transform>(joint)
            .unwrap()
            .translation
            .x = 99.0;
        app.update();
        assert_x(&app, joint, 99.0);
        app.world_mut().entity_mut(owner).remove::<Disabled>();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Login);
        app.update();
        assert_x(&app, joint, 99.0);
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::M2Debug);
        app.update();
        assert_x(&app, joint, 2.5);
    }

    #[test]
    fn empty_sequence_header_keeps_existing_sequence_zero_tracks() {
        let mut app = fixture_app();
        let (owner, joint) = spawn_model(&mut app);
        app.world_mut()
            .get_mut::<M2AnimData>(owner)
            .unwrap()
            .sequences
            .clear();
        settle(&mut app);
        assert_x(&app, joint, 2.5);
    }
}
