//! Dead units (NPCs and players at 0 health) play `Death` (animation 1) once and hold
//! its last frame: the corpse lies still until it decays or the player is revived. A
//! dead unit's animated models get [`DeathPose`], which keeps movement and emote
//! animation off them.

use bevy::prelude::*;
use shared::components::{Health as NetHealth, Npc, Player};

use super::{M2AnimData, M2AnimPlayer};

/// WoW animation `Death`.
pub(crate) const ANIM_DEATH: u16 = 1;

/// The model shows its unit's death.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeathPose {
    started: bool,
}

/// The unit's models already have [`DeathPose`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DeathPoseApplied;

fn animated_descendants(
    root: Entity,
    children: &Query<&Children>,
    players: &Query<(), With<M2AnimPlayer>>,
    out: &mut Vec<Entity>,
) {
    for child in children.get(root).into_iter().flatten() {
        if players.contains(*child) {
            out.push(*child);
        }
        animated_descendants(*child, children, players, out);
    }
}

fn animated_models(
    unit: Entity,
    children: &Query<&Children>,
    players: &Query<(), With<M2AnimPlayer>>,
) -> Vec<Entity> {
    let mut models = Vec::new();
    if players.contains(unit) {
        models.push(unit);
    }
    animated_descendants(unit, children, players, &mut models);
    models
}

/// Mark the animated models of NPCs and players at 0 health.
pub(crate) fn mark_dead_unit_models(
    units: Query<(Entity, &NetHealth), (Or<(With<Npc>, With<Player>)>, Without<DeathPoseApplied>)>,
    children: Query<&Children>,
    players: Query<(), With<M2AnimPlayer>>,
    mut commands: Commands,
) {
    for (unit, health) in &units {
        if health.current > 0.0 {
            continue;
        }
        let models = animated_models(unit, &children, &players);
        if models.is_empty() {
            continue;
        }
        for model in models {
            commands.entity(model).insert(DeathPose::default());
        }
        commands.entity(unit).insert(DeathPoseApplied);
    }
}

/// A revived unit's models leave the death pose, so stand and movement play again.
pub(crate) fn clear_revived_unit_models(
    units: Query<(Entity, &NetHealth), With<DeathPoseApplied>>,
    children: Query<&Children>,
    players: Query<(), With<M2AnimPlayer>>,
    mut commands: Commands,
) {
    for (unit, health) in &units {
        if health.current <= 0.0 {
            continue;
        }
        for model in animated_models(unit, &children, &players) {
            commands.entity(model).remove::<DeathPose>();
        }
        commands.entity(unit).remove::<DeathPoseApplied>();
    }
}

/// Start `Death` once, not looping, so time stops on its last frame.
pub(crate) fn play_death_animation(
    mut models: Query<(&mut M2AnimPlayer, &M2AnimData, &mut DeathPose)>,
) {
    for (mut player, data, mut pose) in &mut models {
        if pose.started {
            continue;
        }
        pose.started = true;
        let Some(index) = data.sequences.iter().position(|s| s.id == ANIM_DEATH) else {
            continue;
        };
        let blend_ms = data.sequences[index].blend_time as f32;
        super::runtime::start_transition(&mut player, index, blend_ms);
        player.looping = false;
    }
}
