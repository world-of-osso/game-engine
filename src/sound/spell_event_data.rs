//! Outcome sound selection from the original reliable CombatEvent stream.

use shared::protocol::{CombatEvent, CombatEventType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeSound {
    Impact,
    Heal,
    Miss,
    Interrupt,
}

/// Return the legacy category and server entity bits. The adapter must require a present unit.
pub fn spell_outcome(event: &CombatEvent) -> Option<(OutcomeSound, u64)> {
    if event.spell_id == 0 {
        return None;
    }
    let (kind, emitter) = match event.event_type {
        CombatEventType::SpellDamage
        | CombatEventType::PeriodicDamage
        | CombatEventType::CriticalHit => (OutcomeSound::Impact, event.target),
        CombatEventType::SpellHeal | CombatEventType::PeriodicHeal => {
            (OutcomeSound::Heal, event.target)
        }
        CombatEventType::Miss => (OutcomeSound::Miss, event.target),
        CombatEventType::Interrupt => (OutcomeSound::Interrupt, event.attacker),
        CombatEventType::MeleeDamage
        | CombatEventType::Absorb
        | CombatEventType::Dodge
        | CombatEventType::Parry
        | CombatEventType::Block
        | CombatEventType::Death
        | CombatEventType::Respawn => return None,
    };
    Some((kind, emitter))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(event_type: CombatEventType, spell_id: u32) -> CombatEvent {
        CombatEvent {
            attacker: 101,
            target: 202,
            amount: 40.0,
            spell_id,
            event_type,
        }
    }

    #[test]
    fn outcomes_select_original_emitter_and_ignore_non_spell_events() {
        let cases = [
            (CombatEventType::SpellDamage, OutcomeSound::Impact, 202),
            (CombatEventType::PeriodicDamage, OutcomeSound::Impact, 202),
            (CombatEventType::CriticalHit, OutcomeSound::Impact, 202),
            (CombatEventType::SpellHeal, OutcomeSound::Heal, 202),
            (CombatEventType::PeriodicHeal, OutcomeSound::Heal, 202),
            (CombatEventType::Miss, OutcomeSound::Miss, 202),
            (CombatEventType::Interrupt, OutcomeSound::Interrupt, 101),
        ];
        for (kind, sound, emitter) in cases {
            assert_eq!(spell_outcome(&event(kind, 133)), Some((sound, emitter)));
        }
        for kind in [
            CombatEventType::MeleeDamage,
            CombatEventType::Absorb,
            CombatEventType::Dodge,
            CombatEventType::Parry,
            CombatEventType::Block,
            CombatEventType::Death,
            CombatEventType::Respawn,
        ] {
            assert_eq!(spell_outcome(&event(kind, 133)), None);
        }
        assert_eq!(spell_outcome(&event(CombatEventType::SpellDamage, 0)), None);
    }
}
