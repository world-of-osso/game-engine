//! Dungeon encounters (docs/specs/boss-encounters.md; Retail `ENCOUNTER_START` /
//! `ENCOUNTER_END`, `INSTANCE_ENCOUNTER_ENGAGE_UNIT`). `EncounterStart`/`End` track the
//! running encounter; `EncounterEngageUnit`/`DisengageUnit` fill the `boss1..5` units
//! the boss frames show, lowest `target_frame_priority` first, then in engage order.
//! The loading screen (a map transfer, a new login) clears both.

use bevy::prelude::*;
use game_engine::network_runtime::messages::MessageReceivers;
use shared::protocol::{EncounterDisengageUnit, EncounterEnd, EncounterEngageUnit, EncounterStart};

use crate::game_state::GameState;

/// `MAX_BOSS_FRAMES`.
pub(crate) const MAX_BOSS_FRAMES: usize = 5;

#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub(crate) struct EncounterFrames {
    /// `DungeonEncounter` ID of the running encounter.
    pub(crate) encounter: Option<u32>,
    /// (server entity bits, priority), in boss-frame order.
    pub(crate) bosses: Vec<(u64, u8)>,
}

impl EncounterFrames {
    /// Server entity bits of `boss1..boss5`.
    pub(crate) fn boss_units(&self) -> impl Iterator<Item = u64> + '_ {
        self.bosses
            .iter()
            .take(MAX_BOSS_FRAMES)
            .map(|&(unit, _)| unit)
    }

    fn engage(&mut self, unit: u64, priority: u8) {
        if self.bosses.iter().any(|&(boss, _)| boss == unit) {
            return;
        }
        let at = self.bosses.partition_point(|&(_, other)| other <= priority);
        self.bosses.insert(at, (unit, priority));
    }
}

pub struct EncounterNetworkPlugin;

impl Plugin for EncounterNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::register_message_handler;

        app.init_resource::<EncounterFrames>()
            .add_systems(OnEnter(GameState::Loading), clear_encounter_frames);
        register_message_handler::<EncounterStart, _>(app, receive_encounter_start, |_| true);
        register_message_handler::<EncounterEnd, _>(app, receive_encounter_end, |_| true);
        register_message_handler::<EncounterEngageUnit, _>(app, receive_engage_unit, |_| true);
        register_message_handler::<EncounterDisengageUnit, _>(app, receive_disengage_unit, |_| {
            true
        });
    }
}

fn receive_encounter_start(
    mut receivers: MessageReceivers<EncounterStart>,
    mut frames: ResMut<EncounterFrames>,
) {
    for receiver in receivers.iter_mut() {
        for start in receiver.receive() {
            info!(
                "ENCOUNTER_START {} (difficulty {}, group size {})",
                start.encounter_id, start.difficulty_id, start.group_size
            );
            frames.encounter = Some(start.encounter_id);
        }
    }
}

fn receive_encounter_end(
    mut receivers: MessageReceivers<EncounterEnd>,
    mut frames: ResMut<EncounterFrames>,
) {
    for receiver in receivers.iter_mut() {
        for end in receiver.receive() {
            info!("ENCOUNTER_END {} success {}", end.encounter_id, end.success);
            if frames.encounter == Some(end.encounter_id) {
                frames.encounter = None;
            }
        }
    }
}

fn receive_engage_unit(
    mut receivers: MessageReceivers<EncounterEngageUnit>,
    mut frames: ResMut<EncounterFrames>,
) {
    for receiver in receivers.iter_mut() {
        for engage in receiver.receive() {
            frames.engage(engage.unit, engage.target_frame_priority);
        }
    }
}

fn receive_disengage_unit(
    mut receivers: MessageReceivers<EncounterDisengageUnit>,
    mut frames: ResMut<EncounterFrames>,
) {
    for receiver in receivers.iter_mut() {
        for disengage in receiver.receive() {
            frames.bosses.retain(|&(unit, _)| unit != disengage.unit);
        }
    }
}

fn clear_encounter_frames(mut frames: ResMut<EncounterFrames>) {
    *frames = EncounterFrames::default();
}

#[cfg(test)]
#[path = "encounter_tests.rs"]
mod tests;
