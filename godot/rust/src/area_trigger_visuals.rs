//! Held ground visuals for replicated spell area triggers.
//!
//! A trigger from `SPELL_EFFECT_CREATE_AREATRIGGER` is its own replicated entity, and the
//! spell's cast kit is a one-shot, so the persistent ground look (Death and Decay,
//! Consecration, Flamestrike) has to come from the trigger, not from the cast. This module
//! is the client half: it collects the live triggers and decides what to show for each.
//!
//! The client re-issues the trigger's one-shot destination kit on each server pulse, so the
//! lifetime stays authoritative on the server instead of being guessed from a timestamp.

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
    /// Server update counter. The display kit is held for the trigger's lifetime, so a new
    /// pulse does not replay the burst; it is kept for triggers that move or change shape.
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
    /// Return the triggers seen for the first time: those start the effect. The ground art
    /// is a display kit held until the anchor retires, so a server pulse is not a re-cast
    /// and must not play the burst again.
    pub(crate) fn replays(&mut self, live: &[LiveAreaTrigger]) -> Vec<LiveAreaTrigger> {
        let mut replays = Vec::new();
        for trigger in live {
            if self.seen.insert(trigger.id, trigger.pulse).is_none() {
                replays.push(*trigger);
            }
        }
        replays
    }

    /// Triggers the replica reported before and no longer does: only these may be freed.
    /// Anchors the preview fixture created directly were never in `seen`, so a replica with
    /// no triggers never tears them down.
    pub(crate) fn retired(&mut self, live: &[LiveAreaTrigger]) -> Vec<u64> {
        let mut retired = Vec::new();
        self.seen.retain(|id, _| {
            let present = live.iter().any(|trigger| trigger.id == *id);
            if !present {
                retired.push(*id);
            }
            present
        });
        retired
    }
}

#[cfg(test)]
#[path = "area_trigger_visuals_tests.rs"]
mod tests;
