//! Source-grounded qualitative Assisted Combat predicates.
//! Numeric comparison/predictive conditions remain explicit gaps, not guessed rules.
use crate::cooldown_assist::AssistedRule;
use std::collections::{BTreeMap, BTreeSet};

/// SpellName/SpellMisc/SpellEffect of the local Retail built-in, not a new spell.
pub const SINGLE_BUTTON_ASSISTANT: u32 = 1_229_376;

#[derive(Clone, Debug, Default)]
pub struct AssistedFacts {
    pub known: BTreeSet<u32>,
    pub ready: BTreeMap<u32, Result<bool, String>>,
    pub affordable: BTreeMap<u32, Result<bool, String>>,
    pub in_range: BTreeMap<u32, Result<bool, String>>,
    pub player_auras: BTreeSet<u32>,
    pub target_auras: BTreeSet<u32>,
    pub target_hostile: Option<bool>,
}

/// Condition numbers/names from SimulationCraft data_enums.hh; spell0 means this step.
/// Only qualitative operations whose semantics are supported by that primary source
/// are enabled. Unsupported numeric/charge/predictive semantics are never assumed true.
pub fn evaluate_rule(rule: &AssistedRule, facts: &AssistedFacts) -> Result<bool, String> {
    let spell = if rule.values[0] == 0 {
        rule.step_spell
    } else {
        rule.values[0]
    };
    match rule.condition_type {
        0 => Ok(facts.known.contains(&spell)),
        1 => read_boolean(&facts.ready, spell, "cooldown readiness").map(|ready| !ready),
        2 => read_boolean(&facts.ready, spell, "cooldown readiness"),
        5 => facts
            .target_hostile
            .ok_or_else(|| "target hostility unavailable".into()),
        6 => facts
            .target_hostile
            .map(|hostile| !hostile)
            .ok_or_else(|| "target hostility unavailable".into()),
        9 => Ok(rule
            .values
            .iter()
            .filter(|id| **id != 0)
            .any(|id| facts.player_auras.contains(id))),
        10 => Ok(facts.target_auras.contains(&rule.values[0])),
        14 => read_boolean(&facts.affordable, spell, "effective caster cost"),
        15 => Ok(!facts.target_auras.contains(&rule.values[0])),
        16 => Ok(!facts.player_auras.contains(&rule.values[0])),
        56 => read_boolean(&facts.in_range, spell, "spell range"),
        _ => Err(format!(
            "AssistedCombatRule {}: condition {} has unverified numeric/predictive semantics",
            rule.id, rule.condition_type
        )),
    }
}

pub fn is_spell_usable(spell: u32, facts: &AssistedFacts) -> Result<bool, String> {
    let readings = [
        read_boolean(&facts.ready, spell, "cooldown readiness"),
        read_boolean(&facts.affordable, spell, "effective caster cost"),
        read_boolean(&facts.in_range, spell, "spell range"),
    ];
    if readings.iter().any(|reading| *reading == Ok(false)) {
        return Ok(false);
    }
    for reading in readings {
        reading?;
    }
    Ok(true)
}

fn read_boolean(
    values: &BTreeMap<u32, Result<bool, String>>,
    spell: u32,
    fact: &str,
) -> Result<bool, String> {
    values
        .get(&spell)
        .cloned()
        .ok_or_else(|| format!("spell {spell}: {fact} not received"))?
}
