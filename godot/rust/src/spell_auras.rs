//! Aura kits (`SpellVisualEvent` AuraStart/AuraEnd): when a replicated `UnitAuras` entry
//! appears, its spell's AuraStart kits start with the aura's unit as the hit unit and its
//! caster (the unit itself when unknown) as the caster. Held ones (ending at AuraEnd)
//! last until the aura instance leaves the unit; its AuraEnd kits then start.

use std::collections::HashSet;

use game_engine_core::spell_visual::VisualEvent;
use game_engine_network::replica::Replica;
use shared::components::UnitAuras;

use super::{CastUnits, KitHold, Lifetime, Phase, SpellEffects, join_errors};
use crate::replicated::is_unit;
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

impl AuraRef {
    fn key(&self) -> (u64, u32) {
        (self.unit, self.instance)
    }
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
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        world.sync_animation_replacements(units)?;
        let present: Vec<AuraRef> = units
            .units()
            .flat_map(|snapshot| {
                let unit = snapshot.server_id;
                snapshot
                    .get::<UnitAuras>()
                    .into_iter()
                    .flat_map(|auras| &auras.auras)
                    .map(move |aura| AuraRef {
                        spell_id: aura.spell_id,
                        caster: aura.caster.unwrap_or(unit),
                        unit,
                        instance: aura.instance_id,
                    })
            })
            .collect();
        let viewer_auras: Vec<u32> = world
            .local_player_id()
            .and_then(|id| units.unit(id))
            .and_then(|unit| unit.get::<UnitAuras>())
            .into_iter()
            .flat_map(|auras| &auras.auras)
            .map(|aura| aura.spell_id)
            .collect();
        let held: Vec<((u64, u32), u32)> = self
            .auras
            .iter()
            .map(|(&key, aura)| (key, aura.spell_id))
            .collect();
        let Some(catalog) = self.catalog()? else {
            return Ok(());
        };
        let all_keys: HashSet<(u64, u32)> = present.iter().map(AuraRef::key).collect();
        let visible: Vec<AuraRef> = present
            .into_iter()
            .filter(|aura| catalog.aura_visual_visible(aura.spell_id, &viewer_auras))
            .collect();
        let visible_keys: HashSet<(u64, u32)> = visible.iter().map(AuraRef::key).collect();
        let gone: Vec<((u64, u32), bool)> = held
            .into_iter()
            .filter(|(key, _)| !visible_keys.contains(key))
            .map(|(key, spell_id)| {
                let play_end = !all_keys.contains(&key)
                    && catalog.aura_visual_visible(spell_id, &viewer_auras);
                (key, play_end)
            })
            .collect();
        let mut errors = Vec::new();
        for (key, play_end) in gone {
            errors.extend(self.end_aura(key, play_end, units, world).err());
        }
        for aura in visible {
            if !self.auras.contains_key(&aura.key()) {
                errors.extend(self.begin_aura(aura, units, world).err());
            }
        }
        self.aura_alpha.sync(world);
        join_errors(errors)
    }

    /// Aura `aura` appeared: its AuraStart kits start and are held for it.
    fn begin_aura(
        &mut self,
        aura: AuraRef,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let started = self.start_aura_event(aura, VisualEvent::AuraStart, units, world);
        let anims = started.as_ref().cloned().unwrap_or_default();
        self.auras.insert(
            aura.key(),
            HeldAura {
                spell_id: aura.spell_id,
                caster: aura.caster,
                anims,
            },
        );
        started.map(drop)
    }

    /// Start `aura`'s `event` kits; returns the looping clips they hold on its unit.
    fn start_aura_event(
        &mut self,
        aura: AuraRef,
        event: VisualEvent,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<Vec<u16>, String> {
        let Some(visual) = self.visual(aura.spell_id, aura.caster, units, world)? else {
            return Ok(Vec::new());
        };
        let Some(catalog) = self.catalog()? else {
            return Ok(Vec::new());
        };
        let kits = catalog.kits(visual, event);
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
        play_end: bool,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let Some(aura) = self.auras.remove(&key) else {
            return Ok(());
        };
        let (unit, instance) = key;
        self.aura_alpha.remove(unit, instance);
        for anim in aura.anims {
            world.stop_unit_action(unit, anim);
        }
        let lifetime = Lifetime::UntilAuraEnds { unit, instance };
        self.pending.retain(|pending| pending.lifetime != lifetime);
        self.sounds
            .end(SoundHold::Aura { unit, instance }, self.clock);
        self.active.retain_mut(|effect| {
            if effect.lifetime != lifetime {
                return true;
            }
            if !play_end {
                if effect.node.is_instance_valid() {
                    effect.node.clone().free();
                }
                return false;
            }
            if matches!(effect.phase, Phase::Start(_) | Phase::Hold) {
                effect.finish();
            }
            true
        });
        if !play_end || units.unit(unit).is_none_or(|unit| !is_unit(unit)) {
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
