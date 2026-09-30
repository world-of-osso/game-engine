//! Aura kits (`SpellVisualEvent` AuraStart/AuraEnd): when a replicated `UnitAuras` entry
//! appears, its spell's AuraStart kits start with the aura's unit as the hit unit and its
//! caster (the unit itself when unknown) as the caster. Held ones (ending at AuraEnd)
//! last until the aura instance leaves the unit; its AuraEnd kits then start.

use std::collections::{HashMap, HashSet};

use game_engine_core::spell_visual::VisualEvent;
use game_engine_network::UnitSnapshot;

use super::{CastUnits, KitHold, Lifetime, Phase, SpellEffects, join_errors};
use crate::spell_sounds::SoundHold;
use crate::world::WorldUnits;

/// One aura instance on one unit.
#[derive(Clone, Copy)]
struct AuraRef {
    spell_id: u32,
    caster: u64,
    unit: u64,
    instance: u32,
}

/// An aura whose kits play on its unit.
pub(super) struct HeldAura {
    spell_id: u32,
    caster: u64,
    /// Looping unit clips its kits started on the aura's unit.
    anims: Vec<u16>,
}

impl SpellEffects {
    /// Start the kits of auras that appeared and end those of auras that left.
    pub(super) fn sync_auras(
        &mut self,
        units: &HashMap<u64, UnitSnapshot>,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let present: HashSet<(u64, u32)> = units
            .iter()
            .flat_map(|(&id, unit)| {
                unit.auras
                    .iter()
                    .flat_map(|auras| &auras.auras)
                    .map(move |aura| (id, aura.instance_id))
            })
            .collect();
        let gone: Vec<(u64, u32)> = self
            .auras
            .keys()
            .filter(|key| !present.contains(key))
            .copied()
            .collect();
        let mut errors = Vec::new();
        for key in gone {
            errors.extend(self.end_aura(key, units, world).err());
        }
        for (&unit, snapshot) in units {
            for aura in snapshot.auras.iter().flat_map(|auras| &auras.auras) {
                if self.auras.contains_key(&(unit, aura.instance_id)) {
                    continue;
                }
                let caster = aura.caster.unwrap_or(unit);
                let key = AuraRef {
                    spell_id: aura.spell_id,
                    caster,
                    unit,
                    instance: aura.instance_id,
                };
                let anims = self
                    .start_aura_event(key, VisualEvent::AuraStart, units, world)
                    .unwrap_or_else(|error| {
                        errors.push(error);
                        Vec::new()
                    });
                self.auras.insert(
                    (unit, aura.instance_id),
                    HeldAura {
                        spell_id: aura.spell_id,
                        caster,
                        anims,
                    },
                );
            }
        }
        join_errors(errors)
    }

    /// Start `aura`'s `event` kits; returns the looping clips they hold on its unit.
    fn start_aura_event(
        &mut self,
        aura: AuraRef,
        event: VisualEvent,
        units: &HashMap<u64, UnitSnapshot>,
        world: &mut WorldUnits,
    ) -> Result<Vec<u16>, String> {
        let Some(visual) = self.visual(aura.spell_id, aura.caster, units, world)? else {
            return Ok(Vec::new());
        };
        let kits = self.catalog()?.kits(visual, event);
        let hits = [aura.unit];
        let cast_units = CastUnits {
            caster: aura.caster,
            target: Some(aura.unit),
            hits: &hits,
        };
        let hold = KitHold::Aura {
            unit: aura.unit,
            instance: aura.instance,
        };
        let looping = self.start_kits(aura.spell_id, event, cast_units, kits, hold, world)?;
        Ok(looping
            .into_iter()
            .filter(|&(unit, _)| unit == aura.unit)
            .map(|(_, clip)| clip)
            .collect())
    }

    /// Aura `key` left its unit: its held kits end, then its AuraEnd kits start.
    fn end_aura(
        &mut self,
        key: (u64, u32),
        units: &HashMap<u64, UnitSnapshot>,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let Some(aura) = self.auras.remove(&key) else {
            return Ok(());
        };
        let (unit, instance) = key;
        for anim in aura.anims {
            world.stop_unit_action(unit, anim);
        }
        let lifetime = Lifetime::UntilAuraEnds { unit, instance };
        self.pending.retain(|pending| pending.lifetime != lifetime);
        self.sounds
            .end(SoundHold::Aura { unit, instance }, self.clock);
        for effect in &mut self.active {
            if effect.lifetime == lifetime && matches!(effect.phase, Phase::Start(_) | Phase::Hold)
            {
                effect.finish();
            }
        }
        if !units.contains_key(&unit) {
            return Ok(());
        }
        let key = AuraRef {
            spell_id: aura.spell_id,
            caster: aura.caster,
            unit,
            instance,
        };
        self.start_aura_event(key, VisualEvent::AuraEnd, units, world)
            .map(drop)
    }
}
