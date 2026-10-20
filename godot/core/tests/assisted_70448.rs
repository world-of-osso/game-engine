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
#[ignore = "RED: native Azure combat refuses rule1252/type3 before generic distance policy"]
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
#[ignore = "RED: only one of92 native type3 rules is enabled"]
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
