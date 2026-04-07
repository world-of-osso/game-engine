//! Spell visual effect data model.
//!
//! Maps spell IDs to visual kits that describe what particle/model effects
//! to spawn during cast, channel, impact, and state (aura) phases.
//! Based on the WoW SpellVisualKit DB2 table structure.

use bevy::prelude::*;

/// When during a spell's lifecycle an effect plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VisualPhase {
    /// Effect on the caster during cast bar fill.
    Cast,
    /// Effect on the caster during channeling.
    Channel,
    /// Effect on the target at impact.
    Impact,
    /// Persistent aura effect on the target.
    State,
    /// Projectile/missile visual in flight.
    Missile,
}

/// A single visual effect reference within a kit.
#[derive(Clone, Debug, PartialEq)]
pub struct VisualEffect {
    /// Phase this effect belongs to.
    pub phase: VisualPhase,
    /// Attachment point on the model (e.g. "hand_right", "chest").
    pub attach_point: String,
    /// M2 model FDID to spawn for this effect (0 = particle-only).
    pub model_fdid: u32,
    /// Particle texture FDID (0 = model-only).
    pub particle_fdid: u32,
    /// Color tint override.
    pub color: [f32; 4],
    /// Scale multiplier.
    pub scale: f32,
}

impl Default for VisualEffect {
    fn default() -> Self {
        Self {
            phase: VisualPhase::Cast,
            attach_point: String::new(),
            model_fdid: 0,
            particle_fdid: 0,
            color: [1.0, 1.0, 1.0, 1.0],
            scale: 1.0,
        }
    }
}

/// A visual kit containing all effects for a spell.
#[derive(Clone, Debug, PartialEq)]
pub struct SpellVisualKit {
    pub kit_id: u32,
    pub effects: Vec<VisualEffect>,
}

impl SpellVisualKit {
    /// Get effects for a specific phase.
    pub fn effects_for_phase(&self, phase: VisualPhase) -> Vec<&VisualEffect> {
        self.effects.iter().filter(|e| e.phase == phase).collect()
    }

    /// Whether this kit has any effects for the given phase.
    pub fn has_phase(&self, phase: VisualPhase) -> bool {
        self.effects.iter().any(|e| e.phase == phase)
    }
}

/// Maps spell IDs to visual kits.
#[derive(Resource, Clone, Debug, Default)]
pub struct SpellVisualRegistry {
    entries: Vec<(u32, SpellVisualKit)>,
}

impl SpellVisualRegistry {
    /// Register a visual kit for a spell ID.
    pub fn insert(&mut self, spell_id: u32, kit: SpellVisualKit) {
        if let Some(entry) = self.entries.iter_mut().find(|(id, _)| *id == spell_id) {
            entry.1 = kit;
        } else {
            self.entries.push((spell_id, kit));
        }
    }

    /// Look up the visual kit for a spell.
    pub fn get(&self, spell_id: u32) -> Option<&SpellVisualKit> {
        self.entries
            .iter()
            .find(|(id, _)| *id == spell_id)
            .map(|(_, kit)| kit)
    }

    /// Number of registered spells.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A pending visual effect instance to be spawned.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingVisualEffect {
    pub spell_id: u32,
    pub phase: VisualPhase,
    pub caster_entity: Entity,
    pub target_entity: Option<Entity>,
}

/// Queue of visual effects waiting to be spawned.
#[derive(Resource, Default)]
pub struct VisualEffectQueue {
    pub pending: Vec<PendingVisualEffect>,
}

impl VisualEffectQueue {
    pub fn enqueue(&mut self, effect: PendingVisualEffect) {
        self.pending.push(effect);
    }

    pub fn drain(&mut self) -> Vec<PendingVisualEffect> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fireball_kit() -> SpellVisualKit {
        SpellVisualKit {
            kit_id: 1,
            effects: vec![
                VisualEffect {
                    phase: VisualPhase::Cast,
                    attach_point: "hand_right".into(),
                    model_fdid: 100,
                    particle_fdid: 200,
                    scale: 1.5,
                    ..Default::default()
                },
                VisualEffect {
                    phase: VisualPhase::Missile,
                    attach_point: String::new(),
                    model_fdid: 101,
                    particle_fdid: 0,
                    scale: 1.0,
                    ..Default::default()
                },
                VisualEffect {
                    phase: VisualPhase::Impact,
                    attach_point: "chest".into(),
                    model_fdid: 0,
                    particle_fdid: 300,
                    scale: 2.0,
                    ..Default::default()
                },
            ],
        }
    }

    // --- SpellVisualKit ---

    #[test]
    fn effects_for_phase_filters() {
        let kit = fireball_kit();
        assert_eq!(kit.effects_for_phase(VisualPhase::Cast).len(), 1);
        assert_eq!(kit.effects_for_phase(VisualPhase::Missile).len(), 1);
        assert_eq!(kit.effects_for_phase(VisualPhase::Impact).len(), 1);
        assert_eq!(kit.effects_for_phase(VisualPhase::Channel).len(), 0);
        assert_eq!(kit.effects_for_phase(VisualPhase::State).len(), 0);
    }

    #[test]
    fn has_phase_checks() {
        let kit = fireball_kit();
        assert!(kit.has_phase(VisualPhase::Cast));
        assert!(kit.has_phase(VisualPhase::Impact));
        assert!(!kit.has_phase(VisualPhase::Channel));
        assert!(!kit.has_phase(VisualPhase::State));
    }

    #[test]
    fn cast_effect_has_correct_attach_point() {
        let kit = fireball_kit();
        let cast = kit.effects_for_phase(VisualPhase::Cast);
        assert_eq!(cast[0].attach_point, "hand_right");
        assert_eq!(cast[0].model_fdid, 100);
        assert_eq!(cast[0].scale, 1.5);
    }

    // --- SpellVisualRegistry ---

    #[test]
    fn registry_insert_and_get() {
        let mut reg = SpellVisualRegistry::default();
        assert!(reg.is_empty());
        reg.insert(133, fireball_kit());
        assert_eq!(reg.len(), 1);
        let kit = reg.get(133).expect("should find fireball");
        assert_eq!(kit.kit_id, 1);
    }

    #[test]
    fn registry_get_missing_returns_none() {
        let reg = SpellVisualRegistry::default();
        assert!(reg.get(999).is_none());
    }

    #[test]
    fn registry_insert_overwrites() {
        let mut reg = SpellVisualRegistry::default();
        reg.insert(133, fireball_kit());
        let updated = SpellVisualKit {
            kit_id: 2,
            effects: vec![],
        };
        reg.insert(133, updated);
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get(133).unwrap().kit_id, 2);
    }

    #[test]
    fn registry_multiple_spells() {
        let mut reg = SpellVisualRegistry::default();
        reg.insert(133, fireball_kit());
        reg.insert(
            116,
            SpellVisualKit {
                kit_id: 3,
                effects: vec![VisualEffect {
                    phase: VisualPhase::Channel,
                    ..Default::default()
                }],
            },
        );
        assert_eq!(reg.len(), 2);
        assert!(reg.get(133).unwrap().has_phase(VisualPhase::Cast));
        assert!(reg.get(116).unwrap().has_phase(VisualPhase::Channel));
    }

    // --- VisualEffectQueue ---

    #[test]
    fn queue_enqueue_and_drain() {
        let mut queue = VisualEffectQueue::default();
        assert!(queue.pending.is_empty());
        queue.enqueue(PendingVisualEffect {
            spell_id: 133,
            phase: VisualPhase::Cast,
            caster_entity: Entity::from_raw_u32(1).unwrap(),
            target_entity: None,
        });
        queue.enqueue(PendingVisualEffect {
            spell_id: 133,
            phase: VisualPhase::Impact,
            caster_entity: Entity::from_raw_u32(1).unwrap(),
            target_entity: Some(Entity::from_raw_u32(2).unwrap()),
        });
        assert_eq!(queue.pending.len(), 2);
        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert!(queue.pending.is_empty());
    }

    #[test]
    fn queue_drain_empty() {
        let mut queue = VisualEffectQueue::default();
        let drained = queue.drain();
        assert!(drained.is_empty());
    }

    #[test]
    fn visual_effect_default() {
        let e = VisualEffect::default();
        assert_eq!(e.phase, VisualPhase::Cast);
        assert_eq!(e.model_fdid, 0);
        assert_eq!(e.scale, 1.0);
        assert_eq!(e.color, [1.0, 1.0, 1.0, 1.0]);
    }
}
