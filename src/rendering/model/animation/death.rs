//! Dead NPCs play `Death` (animation 1) once and hold its last frame: the corpse
//! lies still until it decays. A dead NPC's animated models get [`DeathPose`], which
//! keeps movement and emote animation off them.

use bevy::prelude::*;
use shared::components::{Health as NetHealth, Npc};

use super::{M2AnimData, M2AnimPlayer};

/// WoW animation `Death`.
pub(crate) const ANIM_DEATH: u16 = 1;

/// The model shows its NPC's death.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeathPose {
    started: bool,
}

/// The NPC's models already have [`DeathPose`].
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

/// Mark the animated models of NPCs at 0 health.
pub(crate) fn mark_dead_npc_models(
    npcs: Query<(Entity, &NetHealth), (With<Npc>, Without<DeathPoseApplied>)>,
    children: Query<&Children>,
    players: Query<(), With<M2AnimPlayer>>,
    mut commands: Commands,
) {
    for (npc, health) in &npcs {
        if health.current > 0.0 {
            continue;
        }
        let mut models = Vec::new();
        animated_descendants(npc, &children, &players, &mut models);
        if models.is_empty() {
            continue;
        }
        for model in models {
            commands.entity(model).insert(DeathPose::default());
        }
        commands.entity(npc).insert(DeathPoseApplied);
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
