//! Source-grounded Assisted Combat predicates.
//! Numeric outcomes are enabled only where strict/inclusive interpretations agree.
//! Equality, undocumented encodings and prediction remain explicit source gaps.
use crate::cooldown_assist::AssistedRule;
use crate::spell_catalog::CatalogSpell;
use std::collections::{BTreeMap, BTreeSet};

/// SpellRange1 is authored "Self Only", not a zero-yard enemy range.
const SELF_ONLY_RANGE_ID: u32 = 1;

/// TC Spell.cpp:621–650/7277–7321 checks the actual cast target, not selection.
/// Self/caster-area spells use the caster; targeted spells keep their hostile bounds.
pub fn is_assisted_spell_in_range(spell: &CatalogSpell, distance: f32) -> bool {
    if spell.range_id == SELF_ONLY_RANGE_ID {
        return true;
    }
    distance >= spell.range.min_yd[0] && distance <= spell.range.max_yd[0]
}

/// SpellName/SpellMisc/SpellEffect of the local Retail built-in, not a new spell.
pub const SINGLE_BUTTON_ASSISTANT: u32 = 1_229_376;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AssistedFacts {
    pub known: BTreeSet<u32>,
    pub ready: BTreeMap<u32, Result<bool, String>>,
    pub affordable: BTreeMap<u32, Result<bool, String>>,
    pub in_range: BTreeMap<u32, Result<bool, String>>,
    pub player_auras: BTreeSet<u32>,
    /// Current player aura stack counts; None means the reading is unavailable.
    #[serde(default)]
    pub player_aura_stacks: Option<BTreeMap<u32, u8>>,
    #[serde(default)]
    pub target_aura_stacks: Option<BTreeMap<u32, u8>>,
    #[serde(default)]
    pub player_aura_remaining_ms: Option<BTreeMap<u32, f64>>,
    #[serde(default)]
    pub target_aura_remaining_ms: Option<BTreeMap<u32, f64>>,
    #[serde(default)]
    pub active_dot_counts: Option<BTreeMap<u32, u32>>,
    #[serde(default)]
    pub active_enemies: Option<u32>,
    #[serde(default)]
    pub mana_pct: Option<f64>,
    pub target_auras: BTreeSet<u32>,
    pub target_hostile: Option<bool>,
    /// Owner-controlled pet presence; None means no authoritative reading.
    pub has_pet: Option<bool>,
    /// Same raw DB2 power units as SpellPower/UnitPowers, not display values.
    pub powers: BTreeMap<u8, f64>,
    pub charges: BTreeMap<u32, u8>,
    pub cooldowns_ms: BTreeMap<u32, f64>,
    pub player_health_pct: Option<f64>,
    pub target_health_pct: Option<f64>,
    /// ASSISTED_COMBAT_SIMC_MIRROR: current target centre distance; absent target is None.
    #[serde(default)]
    pub target_distance_yd: Option<f64>,
    /// Alive hostile replicas with the local player in their threat list, within 8yd.
    #[serde(default)]
    pub player_combat_enemies_within_8yd: Option<u32>,
}

/// Condition numbers/names from SimulationCraft data_enums.hh; spell0 means this step.
/// SimC player_t::parse_assisted_combat_rule still marks comparator inclusivity TODO.
/// Away from equality both possible comparators have the same result; at equality
/// refuse the step. Never substitute SimC's guessed boundaries for Retail semantics.
pub fn evaluate_rule(rule: &AssistedRule, facts: &AssistedFacts) -> Result<bool, String> {
    let spell = if rule.values[0] == 0 {
        rule.step_spell
    } else {
        rule.values[0]
    };
    if let Some((power, greater)) = power_comparison(rule.condition_type) {
        require_zero_tail(rule)?;
        let value = facts
            .powers
            .get(&power)
            .copied()
            .ok_or_else(|| format!("rule {}: raw power {power} not received", rule.id))?;
        // SimC76f8118 player.cpp3948–3951/3988–3991: inclusive energy/chi thresholds.
        if matches!(rule.condition_type, 25 | 42) {
            return Ok(if greater {
                value >= f64::from(rule.values[0])
            } else {
                value <= f64::from(rule.values[0])
            });
        }
        return compare_number(rule, value, f64::from(rule.values[0]), greater);
    }
    match rule.condition_type {
        0 => Ok(facts.known.contains(&spell)),
        1 => read_boolean(&facts.ready, spell, "cooldown readiness").map(|ready| !ready),
        2 => read_boolean(&facts.ready, spell, "cooldown readiness"),
        // SimC76f8118 player.cpp3839–3841: target.distance<=v1.
        // Project SimC-mirror policy uses current target centre distance for all
        // authored type3 rows; this is not verified Retail comparator geometry.
        3 => {
            require_zero_tail(rule)?;
            match facts.target_distance_yd {
                None => Ok(false),
                Some(distance) if distance.is_finite() && distance >= 0.0 => {
                    Ok(distance <= f64::from(rule.values[0]))
                }
                Some(_) => Err(format!("rule {}: invalid target distance", rule.id)),
            }
        }
        4 => {
            require_zero_tail(rule)?;
            match facts.target_distance_yd {
                None => Ok(false),
                Some(distance) if distance.is_finite() && distance >= 0.0 => {
                    Ok(distance > f64::from(rule.values[0]))
                }
                Some(_) => Err(format!("rule {}: invalid target distance", rule.id)),
            }
        }
        11 | 12 | 49 | 50 if rule.id != 1268 => {
            if rule.values[2] != 0 {
                return Err(format!(
                    "rule {}: enemy count operands are unverified",
                    rule.id
                ));
            }
            // SimC76f8118 player.cpp3901–3909/4031–4038 ignores radius and uses active_enemies.
            let count = facts
                .active_enemies
                .ok_or_else(|| format!("rule {}: active enemies not received", rule.id))?;
            Ok(if matches!(rule.condition_type, 11 | 12) {
                count > rule.values[0]
            } else {
                count <= rule.values[0]
            })
        }
        13 | 51 => {
            if rule.values[2] == 0 {
                return Err(format!(
                    "rule {}: active dot operands are unverified",
                    rule.id
                ));
            }
            // SimC76f8118 player.cpp3910–3917/4039–4045 uses active_dot.<name>.
            let counts = facts
                .active_dot_counts
                .as_ref()
                .ok_or_else(|| format!("rule {}: active dot counts not received", rule.id))?;
            let count = counts
                .get(&rule.values[2])
                .copied()
                .ok_or_else(|| format!("rule {}: dot identity unverified", rule.id))?;
            Ok(if rule.condition_type == 13 {
                count >= rule.values[0]
            } else {
                count <= rule.values[0]
            })
        }
        12 if rule.id == 1268 && rule.step_spell == 190411 => {
            require_zero_tail(rule)?;
            facts
                .player_combat_enemies_within_8yd
                .map(|count| count > rule.values[0])
                .ok_or_else(|| format!("rule {}: player combat nearby count not received", rule.id))
        }
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
        17 | 18 => {
            if rule.values[0] == 0 || rule.values[2] != 0 {
                return Err(format!(
                    "rule {}: aura duration operands are unverified",
                    rule.id
                ));
            }
            // SimC76f8118 player.cpp3932–3941: player aura must be up; target absence has zero remains.
            let durations = if rule.condition_type == 17 {
                &facts.player_aura_remaining_ms
            } else {
                &facts.target_aura_remaining_ms
            };
            let durations = durations
                .as_ref()
                .ok_or_else(|| format!("rule {}: aura durations not received", rule.id))?;
            let remaining = durations.get(&rule.values[0]).copied().unwrap_or(0.0);
            Ok(
                (rule.condition_type != 17 || facts.player_auras.contains(&rule.values[0]))
                    && remaining <= f64::from(rule.values[1]),
            )
        }
        20 => {
            require_zero_tail(rule)?;
            let pct = facts
                .mana_pct
                .ok_or_else(|| format!("rule {}: mana percentage not received", rule.id))?;
            if !pct.is_finite() {
                return Err(format!("rule {}: invalid mana percentage", rule.id));
            }
            // SimC76f8118 player.cpp3945–3947: raw threshold is tenths of a percent.
            Ok(pct <= f64::from(rule.values[0]) / 10.0)
        }
        15 => Ok(!facts.target_auras.contains(&rule.values[0])),
        16 => Ok(!facts.player_auras.contains(&rule.values[0])),
        // SimC76f8118 player.cpp4046–4061: target/player stack inclusive thresholds.
        52 | 53 | 54 | 55 => {
            if rule.values[0] == 0 || rule.values[2] != 0 {
                return Err(format!(
                    "rule {}: aura stack operands are unverified",
                    rule.id
                ));
            }
            let stacks = if matches!(rule.condition_type, 52 | 53) {
                &facts.target_aura_stacks
            } else {
                &facts.player_aura_stacks
            };
            let stacks = stacks
                .as_ref()
                .ok_or_else(|| format!("rule {}: aura stacks not received", rule.id))?;
            let value = u32::from(stacks.get(&rule.values[0]).copied().unwrap_or(0));
            Ok(if matches!(rule.condition_type, 52 | 54) {
                value >= rule.values[1]
            } else {
                value <= rule.values[1]
            })
        }
        56 => read_boolean(&facts.in_range, spell, "spell range"),
        57 | 58 => evaluate_pet(rule, facts),
        7 | 8 | 68 | 69 => evaluate_health(rule, facts),
        65 | 66 => evaluate_cooldown(rule, facts),
        63 | 64 => evaluate_charges(rule, facts),
        // SimC's step option permits cast overlap; it adds no Boolean predicate.
        // Ordinary native usability and the caller's input gates still apply.
        67 if rule.values == [0, 0, 0] => Ok(true),
        67 => Err(format!(
            "rule {}: cast-overlap modifier operands are unverified",
            rule.id
        )),
        _ => Err(format!(
            "AssistedCombatRule {}: condition {} has unverified numeric/predictive semantics",
            rule.id, rule.condition_type
        )),
    }
}

fn evaluate_pet(rule: &AssistedRule, facts: &AssistedFacts) -> Result<bool, String> {
    if rule.values != [0, 0, 0] {
        return Err(format!("rule {}: pet operands are unverified", rule.id));
    }
    let present = facts
        .has_pet
        .ok_or_else(|| format!("rule {}: owner pet presence not received", rule.id))?;
    Ok(present == (rule.condition_type == 57))
}

fn evaluate_health(rule: &AssistedRule, facts: &AssistedFacts) -> Result<bool, String> {
    require_zero_tail(rule)?;
    let reading = if rule.condition_type < 68 {
        facts.target_health_pct
    } else {
        facts.player_health_pct
    };
    let value =
        reading.ok_or_else(|| format!("rule {}: health percentage not received", rule.id))?;
    compare_number(
        rule,
        value,
        f64::from(rule.values[0]),
        matches!(rule.condition_type, 7 | 68),
    )
}

fn evaluate_cooldown(rule: &AssistedRule, facts: &AssistedFacts) -> Result<bool, String> {
    if rule.values[0] == 0 || rule.values[2] != 0 {
        return Err(format!(
            "rule {}: cooldown operands are unverified",
            rule.id
        ));
    }
    let value = facts
        .cooldowns_ms
        .get(&rule.values[0])
        .copied()
        .ok_or_else(|| format!("rule {}: cooldown clock not received", rule.id))?;
    compare_number(
        rule,
        value,
        f64::from(rule.values[1]),
        rule.condition_type == 65,
    )
}

fn evaluate_charges(rule: &AssistedRule, facts: &AssistedFacts) -> Result<bool, String> {
    if rule.values[1] != 0 {
        return Err(format!(
            "rule {}: charge encoding with value2 is unverified",
            rule.id
        ));
    }
    require_zero_tail(rule)?;
    let value = facts
        .charges
        .get(&rule.step_spell)
        .copied()
        .ok_or_else(|| format!("rule {}: charges not received", rule.id))?;
    compare_number(
        rule,
        f64::from(value),
        f64::from(rule.values[0]),
        rule.condition_type == 63,
    )
}

fn power_comparison(kind: u32) -> Option<(u8, bool)> {
    let power = match kind {
        21 | 22 => 1,
        23 | 24 => 2,
        25 | 26 => 3,
        27 | 28 => 4,
        29 | 30 => 5,
        31 | 32 => 6,
        33 | 34 => 7,
        35 | 36 => 8,
        37 | 38 => 9,
        39 | 40 => 11,
        41 | 42 => 12,
        43 | 44 => 13,
        45 | 46 => 19,
        47 | 48 => 16,
        59 | 60 => 17,
        61 | 62 => 18,
        _ => return None,
    };
    // Raw DB2 thresholds and replicated raw power cancel the display divisor.
    Some((power, kind % 2 == 1))
}

fn require_zero_tail(rule: &AssistedRule) -> Result<(), String> {
    if rule.values[1..] != [0, 0] {
        return Err(format!(
            "rule {}: unused numeric operands are unverified",
            rule.id
        ));
    }
    Ok(())
}

fn compare_number(
    rule: &AssistedRule,
    value: f64,
    threshold: f64,
    greater: bool,
) -> Result<bool, String> {
    if !value.is_finite() {
        return Err(format!("rule {}: invalid numeric reading", rule.id));
    }
    if value == threshold {
        return Err(format!(
            "rule {}: equality comparator inclusivity is unverified",
            rule.id
        ));
    }
    Ok(if greater {
        value > threshold
    } else {
        value < threshold
    })
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
