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

/// A resolved DB2 SpellVisualKit row: animation ID + attached M2 effects.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellVisualKitEntry {
    pub kit_id: u32,
    /// WoW animation ID to play (0 = use default for cast type).
    pub anim_id: u16,
    /// Start animation M2 model FDID (cast wind-up effect).
    pub start_anim_fdid: u32,
    /// Cast effect M2 model FDID (sustained during cast bar).
    pub cast_effect_fdid: u32,
    /// Impact effect M2 model FDID.
    pub impact_effect_fdid: u32,
    /// State (aura) effect M2 model FDID.
    pub state_effect_fdid: u32,
    /// Channel effect M2 model FDID.
    pub channel_effect_fdid: u32,
    /// Missile model FDID.
    pub missile_fdid: u32,
}

impl SpellVisualKitEntry {
    /// Convert this DB2 entry into a `SpellVisualKit` with resolved effects.
    pub fn to_visual_kit(&self) -> SpellVisualKit {
        let mut effects = Vec::new();
        if self.start_anim_fdid != 0 {
            effects.push(VisualEffect {
                phase: VisualPhase::Cast,
                model_fdid: self.start_anim_fdid,
                ..Default::default()
            });
        }
        if self.cast_effect_fdid != 0 {
            effects.push(VisualEffect {
                phase: VisualPhase::Cast,
                model_fdid: self.cast_effect_fdid,
                ..Default::default()
            });
        }
        if self.impact_effect_fdid != 0 {
            effects.push(VisualEffect {
                phase: VisualPhase::Impact,
                model_fdid: self.impact_effect_fdid,
                ..Default::default()
            });
        }
        if self.state_effect_fdid != 0 {
            effects.push(VisualEffect {
                phase: VisualPhase::State,
                model_fdid: self.state_effect_fdid,
                ..Default::default()
            });
        }
        if self.channel_effect_fdid != 0 {
            effects.push(VisualEffect {
                phase: VisualPhase::Channel,
                model_fdid: self.channel_effect_fdid,
                ..Default::default()
            });
        }
        if self.missile_fdid != 0 {
            effects.push(VisualEffect {
                phase: VisualPhase::Missile,
                model_fdid: self.missile_fdid,
                ..Default::default()
            });
        }
        SpellVisualKit {
            kit_id: self.kit_id,
            effects,
        }
    }

    /// Whether this entry has a custom animation (non-zero anim_id).
    pub fn has_custom_anim(&self) -> bool {
        self.anim_id != 0
    }

    /// Count of non-zero effect FDIDs.
    pub fn effect_count(&self) -> usize {
        [
            self.start_anim_fdid,
            self.cast_effect_fdid,
            self.impact_effect_fdid,
            self.state_effect_fdid,
            self.channel_effect_fdid,
            self.missile_fdid,
        ]
        .iter()
        .filter(|&&fdid| fdid != 0)
        .count()
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

/// An active visual effect attached to an entity with a lifetime.
#[derive(Component, Clone, Debug)]
pub struct ActiveVisualEffect {
    pub spell_id: u32,
    pub phase: VisualPhase,
    /// Remaining lifetime in seconds (0 = permanent until cancelled).
    pub remaining: f32,
    /// Total duration for progress calculation (0 = permanent).
    pub duration: f32,
    /// Whether this effect loops (auras) or plays once (cast/impact).
    pub looping: bool,
}

impl ActiveVisualEffect {
    /// Create a one-shot effect (cast/impact).
    pub fn one_shot(spell_id: u32, phase: VisualPhase, duration: f32) -> Self {
        Self {
            spell_id,
            phase,
            remaining: duration,
            duration,
            looping: false,
        }
    }

    /// Create a persistent aura effect (stays until cancelled).
    pub fn aura(spell_id: u32) -> Self {
        Self {
            spell_id,
            phase: VisualPhase::State,
            remaining: 0.0,
            duration: 0.0,
            looping: true,
        }
    }

    /// Whether this effect has expired (non-looping only).
    pub fn is_expired(&self) -> bool {
        !self.looping && self.duration > 0.0 && self.remaining <= 0.0
    }

    /// Playback progress (0.0–1.0) for one-shot effects.
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        (1.0 - self.remaining / self.duration).clamp(0.0, 1.0)
    }

    /// Advance the effect timer.
    pub fn tick(&mut self, dt: f32) {
        if !self.looping && self.remaining > 0.0 {
            self.remaining = (self.remaining - dt).max(0.0);
        }
    }
}

/// Manages all active visual effects on a single entity.
#[derive(Component, Default)]
pub struct VisualEffectStack {
    pub effects: Vec<ActiveVisualEffect>,
}

impl VisualEffectStack {
    pub fn add(&mut self, effect: ActiveVisualEffect) {
        self.effects.push(effect);
    }

    /// Cancel all effects for a given spell.
    pub fn cancel_spell(&mut self, spell_id: u32) {
        self.effects.retain(|e| e.spell_id != spell_id);
    }

    /// Cancel all effects of a given phase.
    pub fn cancel_phase(&mut self, phase: VisualPhase) {
        self.effects.retain(|e| e.phase != phase);
    }

    /// Tick all effects and remove expired ones.
    pub fn tick(&mut self, dt: f32) {
        for effect in &mut self.effects {
            effect.tick(dt);
        }
        self.effects.retain(|e| !e.is_expired());
    }

    /// Number of active effects.
    pub fn active_count(&self) -> usize {
        self.effects.len()
    }

    /// Whether any effect of the given phase is active.
    pub fn has_phase(&self, phase: VisualPhase) -> bool {
        self.effects.iter().any(|e| e.phase == phase)
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

    // --- ActiveVisualEffect playback ---

    #[test]
    fn one_shot_expires_after_duration() {
        let mut effect = ActiveVisualEffect::one_shot(133, VisualPhase::Impact, 0.5);
        assert!(!effect.is_expired());
        effect.tick(0.6);
        assert!(effect.is_expired());
    }

    #[test]
    fn one_shot_progress_advances() {
        let mut effect = ActiveVisualEffect::one_shot(133, VisualPhase::Cast, 2.0);
        assert!((effect.progress() - 0.0).abs() < 0.01);
        effect.tick(1.0);
        assert!((effect.progress() - 0.5).abs() < 0.01);
        effect.tick(1.0);
        assert!((effect.progress() - 1.0).abs() < 0.01);
    }

    #[test]
    fn aura_never_expires() {
        let mut effect = ActiveVisualEffect::aura(1000);
        assert_eq!(effect.phase, VisualPhase::State);
        assert!(effect.looping);
        effect.tick(100.0);
        assert!(!effect.is_expired());
    }

    #[test]
    fn aura_progress_is_zero() {
        let effect = ActiveVisualEffect::aura(1000);
        assert_eq!(effect.progress(), 0.0);
    }

    // --- VisualEffectStack ---

    #[test]
    fn stack_add_and_tick() {
        let mut stack = VisualEffectStack::default();
        stack.add(ActiveVisualEffect::one_shot(1, VisualPhase::Cast, 0.5));
        stack.add(ActiveVisualEffect::aura(2));
        assert_eq!(stack.active_count(), 2);
        stack.tick(1.0);
        // Cast expired, aura remains
        assert_eq!(stack.active_count(), 1);
        assert!(stack.has_phase(VisualPhase::State));
        assert!(!stack.has_phase(VisualPhase::Cast));
    }

    #[test]
    fn stack_cancel_spell() {
        let mut stack = VisualEffectStack::default();
        stack.add(ActiveVisualEffect::aura(100));
        stack.add(ActiveVisualEffect::one_shot(200, VisualPhase::Impact, 1.0));
        stack.cancel_spell(100);
        assert_eq!(stack.active_count(), 1);
        assert_eq!(stack.effects[0].spell_id, 200);
    }

    #[test]
    fn stack_cancel_phase() {
        let mut stack = VisualEffectStack::default();
        stack.add(ActiveVisualEffect::one_shot(1, VisualPhase::Cast, 1.0));
        stack.add(ActiveVisualEffect::one_shot(2, VisualPhase::Impact, 1.0));
        stack.add(ActiveVisualEffect::one_shot(3, VisualPhase::Cast, 1.0));
        stack.cancel_phase(VisualPhase::Cast);
        assert_eq!(stack.active_count(), 1);
        assert_eq!(stack.effects[0].phase, VisualPhase::Impact);
    }

    #[test]
    fn stack_has_phase() {
        let mut stack = VisualEffectStack::default();
        assert!(!stack.has_phase(VisualPhase::Cast));
        stack.add(ActiveVisualEffect::one_shot(1, VisualPhase::Cast, 1.0));
        assert!(stack.has_phase(VisualPhase::Cast));
        assert!(!stack.has_phase(VisualPhase::Impact));
    }

    // --- SpellVisualKitEntry (DB2 lookup) ---

    #[test]
    fn kit_entry_to_visual_kit_maps_phases() {
        let entry = SpellVisualKitEntry {
            kit_id: 42,
            anim_id: 51,
            start_anim_fdid: 100,
            cast_effect_fdid: 0,
            impact_effect_fdid: 200,
            state_effect_fdid: 0,
            channel_effect_fdid: 0,
            missile_fdid: 300,
        };
        let kit = entry.to_visual_kit();
        assert_eq!(kit.kit_id, 42);
        assert!(kit.has_phase(VisualPhase::Cast));
        assert!(kit.has_phase(VisualPhase::Impact));
        assert!(kit.has_phase(VisualPhase::Missile));
        assert!(!kit.has_phase(VisualPhase::Channel));
        assert!(!kit.has_phase(VisualPhase::State));
    }

    #[test]
    fn kit_entry_empty_produces_no_effects() {
        let entry = SpellVisualKitEntry::default();
        let kit = entry.to_visual_kit();
        assert!(kit.effects.is_empty());
    }

    #[test]
    fn kit_entry_effect_count() {
        let entry = SpellVisualKitEntry {
            cast_effect_fdid: 1,
            impact_effect_fdid: 2,
            missile_fdid: 3,
            ..Default::default()
        };
        assert_eq!(entry.effect_count(), 3);
    }

    #[test]
    fn kit_entry_has_custom_anim() {
        let with = SpellVisualKitEntry {
            anim_id: 51,
            ..Default::default()
        };
        assert!(with.has_custom_anim());
        assert!(!SpellVisualKitEntry::default().has_custom_anim());
    }

    #[test]
    fn kit_entry_all_phases_populated() {
        let entry = SpellVisualKitEntry {
            kit_id: 1,
            anim_id: 0,
            start_anim_fdid: 10,
            cast_effect_fdid: 20,
            impact_effect_fdid: 30,
            state_effect_fdid: 40,
            channel_effect_fdid: 50,
            missile_fdid: 60,
        };
        assert_eq!(entry.effect_count(), 6);
        let kit = entry.to_visual_kit();
        // start_anim + cast_effect both map to Cast phase
        assert_eq!(kit.effects_for_phase(VisualPhase::Cast).len(), 2);
        assert_eq!(kit.effects_for_phase(VisualPhase::Impact).len(), 1);
        assert_eq!(kit.effects_for_phase(VisualPhase::State).len(), 1);
        assert_eq!(kit.effects_for_phase(VisualPhase::Channel).len(), 1);
        assert_eq!(kit.effects_for_phase(VisualPhase::Missile).len(), 1);
    }
}
