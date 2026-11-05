//! Held ground visuals for replicated spell area triggers.
//!
//! A trigger from `SPELL_EFFECT_CREATE_AREATRIGGER` is its own replicated entity, and the
//! spell's cast kit is a one-shot, so the persistent ground look (Death and Decay,
//! Consecration, Flamestrike) has to come from the trigger, not from the cast. This module
//! is the client half: it collects the live triggers and decides what to show for each.
//!
//! Decompilation has not yet said whether retail holds the one-shot destination kit or
//! re-triggers it for the trigger's lifetime, so nothing is drawn until that lands:
//! [`SHOW_AREA_TRIGGER_VISUALS`] stays false and the collection below is inert.

use game_engine_network::replica::Replica;
use shared::{AreaTriggerVisual, components::Position};

/// Whether the client draws anything for a replicated trigger. Flip only when the retail
/// behaviour is known; the two candidate modes are hold-one-frame and re-trigger.
pub(crate) const SHOW_AREA_TRIGGER_VISUALS: bool = true;

/// One replicated trigger that should carry a held ground visual.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LiveAreaTrigger {
    /// Server entity id, stable for the trigger's whole life.
    pub(crate) id: u64,
    /// Spell that created the trigger; the client resolves its own SpellVisual.
    pub(crate) spell_id: u32,
    pub(crate) center: [f32; 3],
    pub(crate) radius: f32,
    /// Lifetime the server still reports, in seconds.
    pub(crate) duration: f32,
    /// Server update counter; a change means the kits are re-issued.
    pub(crate) pulse: u32,
}

/// Every live trigger whose visual can be placed. Malformed replication (non-finite
/// position or radius, negative lifetime) is skipped instead of drawn somewhere wrong.
pub(crate) fn live_area_triggers(replica: &Replica) -> Vec<LiveAreaTrigger> {
    let mut triggers: Vec<LiveAreaTrigger> = replica
        .units()
        .filter_map(|unit| {
            let visual = unit.get::<AreaTriggerVisual>()?;
            let position = unit.get::<Position>()?;
            let center = [position.x, position.y, position.z];
            let placeable = center.iter().all(|value| value.is_finite())
                && visual.radius.is_finite()
                && visual.radius > 0.0
                && visual.duration.is_finite()
                && visual.duration > 0.0;
            placeable.then_some(LiveAreaTrigger {
                id: unit.server_id,
                spell_id: visual.spell_id,
                center,
                radius: visual.radius,
                duration: visual.duration,
                pulse: visual.pulse,
            })
        })
        .collect();
    triggers.sort_by_key(|trigger| trigger.id);
    triggers
}

/// Which live triggers must re-issue their kits this frame.
#[derive(Debug, Default)]
pub(crate) struct AreaTriggerVisuals {
    seen: std::collections::HashMap<u64, u32>,
}

impl AreaTriggerVisuals {
    /// Update the pulse bookkeeping and return the triggers that are new or whose pulse
    /// moved since the last frame: those replay the spell's one-shot kits.
    pub(crate) fn replays(&mut self, live: &[LiveAreaTrigger]) -> Vec<LiveAreaTrigger> {
        let mut replays = Vec::new();
        self.seen
            .retain(|id, _| live.iter().any(|trigger| trigger.id == *id));
        for trigger in live {
            match self.seen.insert(trigger.id, trigger.pulse) {
                Some(pulse) if pulse == trigger.pulse => {}
                _ => replays.push(*trigger),
            }
        }
        replays
    }
}

#[cfg(test)]
#[path = "area_trigger_visuals_tests.rs"]
mod tests;
