//! Assisted Combat predicates evaluated from a captured authoritative-state projection.
use crate::cooldown_assist::AssistedRule;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct AssistedFacts {
    pub known: BTreeSet<u32>,
    pub ready: BTreeMap<u32, Result<bool, String>>,
    pub affordable: BTreeMap<u32, Result<bool, String>>,
    pub in_range: BTreeMap<u32, Result<bool, String>>,
    /// SpellID -> (applications, remaining milliseconds), including permanent auras.
    pub player_auras: BTreeMap<u32, (u32, u32)>,
    pub target_auras: BTreeMap<u32, (u32, u32)>,
    /// PowerType -> (current raw DB2 units, maximum).
    pub powers: BTreeMap<u8, (f32, f32)>,
    pub target_distance: Option<f32>,
    pub target_hostile: Option<bool>,
    pub player_health_percent: Option<f32>,
    pub target_health_percent: Option<f32>,
    pub cooldown_ms: BTreeMap<u32, u32>,
}

pub fn evaluate_rule(_rule: &AssistedRule, _facts: &AssistedFacts) -> Result<bool, String> {
    Err("Assisted Combat predicate implementation missing".into())
}
