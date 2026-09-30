//! Retail melee and unit-event sounds on replicated units, timed by the M2 events of
//! their action clips (wowdev.wiki/M2 Events) and resolved by
//! `game_engine_core::spell_visual`:
//!
//! - A melee `CombatEvent` is held as its attacker's pending swing; the attacker's
//!   attack clip then fires `$CSS`, which plays its weapon swoosh, and `$CAH`, where the
//!   swing lands: a hit or crit plays the weapon's impact on the victim's material and
//!   the victim's injury voice, a parry the impact on the parrying weapon, a miss or
//!   dodge nothing more.
//! - `$SCD` plays the unit's `SpellCastDirectedSoundID`.
//! - An NPC whose death clip starts plays its `SoundDeathID`.

use game_engine_core::spell_visual::{KitSound, MeleeHand, SwingResult, UnitSound};
use shared::protocol::{CombatEvent, CombatEventType};

use super::{SoundCue, SpellEffects, join_errors};
use crate::spell_sounds::SoundSource;
use crate::world::WorldUnits;

/// A melee swing waiting for its attacker's clip to land it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PendingSwing {
    target: u64,
    result: SwingResult,
}

/// A melee `CombatEvent` seen, for automation and logs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeleeSeen {
    pub attacker: u64,
    pub target: u64,
    pub result: SwingResult,
    /// `SpellEffects` clock (seconds) when it arrived.
    pub at: f32,
}

/// The swing result of a melee `CombatEvent` (`None`: not a melee swing). The server
/// sends a block as MeleeDamage.
fn swing_result(kind: &CombatEventType) -> Option<SwingResult> {
    match kind {
        CombatEventType::MeleeDamage => Some(SwingResult::Hit { critical: false }),
        CombatEventType::CriticalHit => Some(SwingResult::Hit { critical: true }),
        CombatEventType::Parry => Some(SwingResult::Parry),
        CombatEventType::Miss | CombatEventType::Dodge => Some(SwingResult::Avoided),
        _ => None,
    }
}

impl SpellEffects {
    /// Hold a melee swing until its attacker's attack clip lands it.
    pub fn observe_melee(&mut self, event: &CombatEvent) {
        let Some(result) = swing_result(&event.event_type) else {
            return;
        };
        let swing = PendingSwing {
            target: event.target,
            result,
        };
        self.swings.insert(event.attacker, swing);
        if self.melee_seen.len() == super::STARTED_KEEP {
            self.melee_seen.remove(0);
        }
        self.melee_seen.push(MeleeSeen {
            attacker: event.attacker,
            target: event.target,
            result,
            at: self.clock,
        });
    }

    /// Recent melee swings seen, oldest first.
    pub fn melee_seen(&self) -> &[MeleeSeen] {
        &self.melee_seen
    }

    /// The sounds of the M2 events units' action clips passed and of units that died.
    pub(super) fn play_unit_events(&mut self, world: &mut WorldUnits) -> Result<(), String> {
        let mut errors = Vec::new();
        for (unit, event) in world.take_animation_events() {
            let played = match &event {
                b"$SCD" => self.play_voice(unit, UnitSound::SpellCastDirected, world),
                b"$CSS" => self.play_swoosh(unit, world),
                b"$CAH" => self.land_swing(unit, world),
                _ => Ok(()),
            };
            errors.extend(played.err());
        }
        for unit in world.take_deaths() {
            errors.extend(self.play_voice(unit, UnitSound::Death, world).err());
        }
        join_errors(errors)
    }

    fn play_voice(
        &mut self,
        unit: u64,
        sound: UnitSound,
        world: &WorldUnits,
    ) -> Result<(), String> {
        let Some(&voice) = self.voices.get(&unit) else {
            return Ok(());
        };
        let sound = self.catalog()?.unit_sound(voice, sound).cloned();
        self.play_melee(sound, unit, SoundSource::Voice, world)
    }

    fn play_swoosh(&mut self, unit: u64, world: &WorldUnits) -> Result<(), String> {
        let critical = self.swings.get(&unit).map(|swing| swing.result)
            == Some(SwingResult::Hit { critical: true });
        let Some(hand) = self.hand(unit, world) else {
            return Ok(());
        };
        let sound = self.catalog()?.swing_sound(hand, critical).cloned();
        self.play_melee(sound, unit, SoundSource::Swing, world)
    }

    /// `unit`'s pending swing lands: its impact, and on a hit the victim's injury.
    fn land_swing(&mut self, unit: u64, world: &WorldUnits) -> Result<(), String> {
        let Some(swing) = self.swings.remove(&unit) else {
            return Ok(());
        };
        let (Some(attacker), Some(victim)) =
            (self.hand(unit, world), self.hand(swing.target, world))
        else {
            return Ok(());
        };
        let catalog = self.catalog()?;
        let impact = catalog
            .impact_sound(attacker, victim, swing.result)
            .cloned();
        let wound = match swing.result {
            SwingResult::Hit { critical } => catalog.wound_sound(victim.unit, critical).cloned(),
            SwingResult::Parry | SwingResult::Avoided => None,
        };
        let impacted = self.play_melee(impact, swing.target, SoundSource::Impact, world);
        let wounded = self.play_melee(wound, swing.target, SoundSource::Voice, world);
        impacted.and(wounded)
    }

    /// What unit `id` swings or parries with, if its voice is known.
    fn hand(&self, id: u64, world: &WorldUnits) -> Option<MeleeHand> {
        let (item_id, display_info_id) = world.unit_main_hand(id);
        Some(MeleeHand {
            item_id,
            display_info_id,
            unit: *self.voices.get(&id)?,
        })
    }

    fn play_melee(
        &mut self,
        sound: Option<KitSound>,
        unit: u64,
        source: SoundSource,
        world: &WorldUnits,
    ) -> Result<(), String> {
        let Some(sound) = sound else {
            return Ok(());
        };
        let cue = SoundCue {
            unit,
            spell_id: 0,
            kit_id: 0,
            hold: None,
            source,
        };
        self.play_on_unit(&sound, cue, world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn melee_events_select_their_swing_result() {
        let hit = SwingResult::Hit { critical: false };
        let crit = SwingResult::Hit { critical: true };
        let cases = [
            (CombatEventType::MeleeDamage, Some(hit)),
            (CombatEventType::CriticalHit, Some(crit)),
            (CombatEventType::Parry, Some(SwingResult::Parry)),
            (CombatEventType::Miss, Some(SwingResult::Avoided)),
            (CombatEventType::Dodge, Some(SwingResult::Avoided)),
            (CombatEventType::Death, None),
            (CombatEventType::SpellDamage, None),
            (CombatEventType::Interrupt, None),
        ];
        for (kind, result) in cases {
            assert_eq!(swing_result(&kind), result, "{kind:?}");
        }
    }
}
