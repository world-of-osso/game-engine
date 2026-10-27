//! Complete native70448 rules; SimC-mirror distance policy, not Retail certification.
use game_engine_core::assisted_combat::{AssistedFacts, evaluate_rule, is_spell_usable};
use game_engine_core::cooldown_assist::{AssistedRule, CooldownAssistCatalog, Recommendation};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Deserialize)]
struct Evaluation {
    specialization: u32,
    target: u64,
    facts: AssistedFacts,
    selection: Result<Recommendation, String>,
}

#[derive(Deserialize)]
struct Capture {
    in_combat: bool,
    evaluation: Evaluation,
}

#[test]
fn native_269_rule19596_checks_dance_of_chiji_stacks() {
    let rule = native_rules()
        .into_iter()
        .find(|rule| rule.id == 19596)
        .unwrap();
    assert_eq!(
        (rule.step_spell, rule.condition_type, rule.values),
        (101546, 54, [325202, 2, 0])
    );
    assert!(evaluate_rule(&rule, &AssistedFacts::default()).is_err());
    let mut invalid = rule.clone();
    invalid.values[2] = 1;
    assert!(evaluate_rule(&invalid, &AssistedFacts::default()).is_err());
    for (stacks, expected) in [(0, false), (1, false), (2, true), (3, true)] {
        let mut value = serde_json::to_value(AssistedFacts::default()).unwrap();
        value["player_aura_stacks"] = serde_json::json!({"325202": stacks});
        let facts: AssistedFacts = serde_json::from_value(value).unwrap();
        assert_eq!(
            evaluate_rule(&rule, &facts),
            Ok(expected),
            "stacks={stacks}"
        );
    }
}

// Frozen 70448 rule IDs and operands exercise SimC-mirror predicates.
fn check_real_rule(id: u32, fields: serde_json::Value, expected: bool) {
    let rule = native_rules()
        .into_iter()
        .find(|rule| rule.id == id)
        .unwrap();
    let mut value = serde_json::to_value(AssistedFacts::default()).unwrap();
    for (key, field) in fields.as_object().unwrap() {
        value[key] = field.clone();
    }
    let facts: AssistedFacts = serde_json::from_value(value).unwrap();
    assert_eq!(evaluate_rule(&rule, &facts), Ok(expected), "rule {id}");
}

#[test]
fn native_rule_type4_simc_boundary() {
    check_real_rule(49472, serde_json::json!({"target_distance_yd":16}), true);
}

#[test]
fn native_rule_type11_simc_boundary() {
    check_real_rule(12523, serde_json::json!({"active_enemies":3}), true);
}

#[test]
fn native_rule_type12_simc_boundary() {
    check_real_rule(19632, serde_json::json!({"active_enemies":4}), true);
}

#[test]
fn native_rule_type13_simc_boundary() {
    check_real_rule(
        95076,
        serde_json::json!({"active_dot_counts":{"1943":1}}),
        true,
    );
}

#[test]
fn native_rule_type17_simc_boundary() {
    check_real_rule(
        20475,
        serde_json::json!({"player_auras":[268877],"player_aura_remaining_ms":{"268877":1000}}),
        true,
    );
}

#[test]
fn native_rule_type18_simc_boundary() {
    check_real_rule(
        34076,
        serde_json::json!({"target_aura_remaining_ms":{"335467":1000}}),
        true,
    );
}

#[test]
fn native_rule_type20_simc_boundary() {
    check_real_rule(12408, serde_json::json!({"mana_pct":5.0}), true);
}

#[test]
fn native_rule_type49_simc_boundary() {
    check_real_rule(12570, serde_json::json!({"active_enemies":2}), true);
}

#[test]
fn native_rule_type50_simc_boundary() {
    check_real_rule(19874, serde_json::json!({"active_enemies":2}), true);
}

#[test]
fn native_rule_type51_simc_boundary() {
    check_real_rule(
        64722,
        serde_json::json!({"active_dot_counts":{"55078":0}}),
        true,
    );
}

#[test]
fn native_rule_type52_simc_boundary() {
    check_real_rule(
        4544,
        serde_json::json!({"target_aura_stacks":{"1221389":5}}),
        true,
    );
}

#[test]
fn native_rule_type53_simc_boundary() {
    check_real_rule(
        81982,
        serde_json::json!({"target_aura_stacks":{"1221389":5}}),
        true,
    );
}

#[test]
fn native_rule_type55_simc_boundary() {
    check_real_rule(
        29368,
        serde_json::json!({"player_aura_stacks":{"195181":6}}),
        true,
    );
}

#[test]
fn native_rule_type25equality_simc_boundary() {
    check_real_rule(19911, serde_json::json!({"powers":{"3":75}}), true);
}

#[test]
fn native_rule_type42equality_simc_boundary() {
    check_real_rule(20057, serde_json::json!({"powers":{"12":4}}), true);
}

fn load_catalog() -> CooldownAssistCatalog {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assisted_70448");
    CooldownAssistCatalog::load(&dir).unwrap()
}

fn recommend(
    catalog: &CooldownAssistCatalog,
    evaluation: &Evaluation,
) -> Result<Recommendation, String> {
    let facts = &evaluation.facts;
    catalog.recommend(
        evaluation.specialization,
        |id| facts.known.contains(&id),
        |id| is_spell_usable(id, facts),
        |rule| evaluate_rule(rule, facts),
    )
}

#[test]
fn assisted_70448_rule1252_replays_actual_in_combat_decision() {
    let capture: Capture =
        serde_json::from_str(include_str!("fixtures/assisted_70448/azure-fury.json")).unwrap();
    let evaluation = capture.evaluation;
    assert!(capture.in_combat);
    assert_eq!(evaluation.target, 4294739577);
    assert_eq!(evaluation.facts.target_distance_yd, Some(2.359868049621582));
    assert_eq!(
        evaluation.selection,
        Err(
            "AssistedCombatRule 1252: condition 3 has unverified numeric/predictive semantics"
                .into()
        )
    );
    // Authored step328 is Odyn's Fury385059, known/ready/affordable/in range.
    // The complete native facts/order, not a substituted rotation, select it.
    assert_eq!(
        recommend(&load_catalog(), &evaluation),
        Ok(Recommendation::Spell(385059))
    );
}

// These three native tables have integer-only fields; parse every record, not a
// hand-selected set of rule IDs. Column order is preserved by the pinned fixture.
fn native_integer_rows(table: &str) -> Vec<Vec<u32>> {
    table
        .lines()
        .skip(1)
        .map(|line| {
            line.split(',')
                .map(|field| field.parse().unwrap())
                .collect()
        })
        .collect()
}

fn native_rules() -> Vec<AssistedRule> {
    let spells: BTreeMap<_, _> = native_integer_rows(include_str!(
        "fixtures/assisted_70448/AssistedCombatStep.csv"
    ))
    .into_iter()
    .map(|row| (row[0], row[1]))
    .collect();
    native_integer_rows(include_str!(
        "fixtures/assisted_70448/AssistedCombatRule.csv"
    ))
    .into_iter()
    .map(|row| AssistedRule {
        id: row[0],
        step_spell: spells[&row[7]],
        condition_type: row[3],
        values: [row[4], row[5], row[6]],
        field_11_1_7_60520_002: row[2],
    })
    .collect()
}

#[test]
fn assisted_70448_all_type3_rules_share_distance_boundaries() {
    let rules = native_rules();
    let distance_rules: Vec<_> = rules
        .iter()
        .filter(|rule| rule.condition_type == 3)
        .collect();
    assert_eq!(distance_rules.len(), 92);
    for rule in distance_rules {
        let threshold = f64::from(rule.values[0]);
        for (distance, expected) in [
            (Some(0.0), true),
            (Some(threshold), true),
            (Some(threshold + 0.001), false),
            (None, false),
        ] {
            let facts = AssistedFacts {
                target_distance_yd: distance,
                ..Default::default()
            };
            assert_eq!(
                evaluate_rule(rule, &facts),
                Ok(expected),
                "rule{} distance{distance:?}",
                rule.id
            );
        }
        for distance in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let facts = AssistedFacts {
                target_distance_yd: Some(distance),
                ..Default::default()
            };
            assert!(
                evaluate_rule(rule, &facts).is_err(),
                "rule{} distance{distance}",
                rule.id
            );
        }
    }
}

#[test]
fn assisted_70448_condition_type_sweep_names_every_rule_outcome() {
    let rules = native_rules();
    assert_eq!(rules.len(), 3116);
    // Synthetic supplied readings measure predicate coverage, not live success
    // or all-spec rotation correctness. Unsupported semantics remain errors.
    let facts = sweep_facts(&rules);
    let mut report: BTreeMap<u32, BTreeMap<String, Vec<u32>>> = BTreeMap::new();
    for rule in &rules {
        let result = if rule.field_11_1_7_60520_002 != 0 {
            Err(format!(
                "unidentified field {}",
                rule.field_11_1_7_60520_002
            ))
        } else {
            evaluate_rule(rule, &facts)
        };
        let outcome = match result {
            Ok(_) => "evaluated".to_string(),
            Err(error) => format!("gap: {error}"),
        };
        report
            .entry(rule.condition_type)
            .or_default()
            .entry(outcome)
            .or_default()
            .push(rule.id);
    }
    assert_eq!(report[&3]["evaluated"].len(), 92);
    assert!(!report[&4].contains_key("evaluated"));
    for (kind, outcomes) in &report {
        let evaluated = outcomes.get("evaluated").map_or(0, Vec::len);
        let total: usize = outcomes.values().map(Vec::len).sum();
        println!(
            "type{kind}: {evaluated}/{total} evaluated; {} named gaps",
            total - evaluated
        );
    }
    println!(
        "ASSISTED_70448_SWEEP={}",
        serde_json::to_string(&report).unwrap()
    );
}

fn sweep_facts(rules: &[AssistedRule]) -> AssistedFacts {
    let mut facts = AssistedFacts {
        target_hostile: Some(true),
        has_pet: Some(false),
        player_health_pct: Some(50.5),
        target_health_pct: Some(50.5),
        target_distance_yd: Some(2.359868049621582),
        player_combat_enemies_within_8yd: Some(1),
        powers: (0..=19).map(|power| (power, 75.5)).collect(),
        ..Default::default()
    };
    for rule in rules {
        for spell in [rule.step_spell, rule.values[0]] {
            facts.ready.insert(spell, Ok(true));
            facts.affordable.insert(spell, Ok(true));
            facts.in_range.insert(spell, Ok(true));
            facts.cooldowns_ms.insert(spell, 1500.5);
        }
        facts.charges.insert(rule.step_spell, 1);
    }
    facts
}
