//! User policy, not verified Retail predicates. New spatial readings are synthetic.
use game_engine_core::assisted_combat::{AssistedFacts, evaluate_rule, is_spell_usable};
use game_engine_core::cooldown_assist::{AssistedRule, CooldownAssistCatalog, Recommendation};
use serde_json::json;
use std::path::Path;

fn rule(id: u32, kind: u32, values: [u32; 3]) -> AssistedRule {
    AssistedRule {
        id,
        step_spell: 190411,
        condition_type: kind,
        values,
        field_11_1_7_60520_002: 0,
    }
}

fn facts(distance: Option<f64>, count: Option<u32>) -> AssistedFacts {
    serde_json::from_value(json!({
        "known": [], "ready": {}, "affordable": {}, "in_range": {},
        "player_auras": [], "target_auras": [], "powers": {}, "charges": {}, "cooldowns_ms": {},
        "target_distance_yd": distance, "player_combat_enemies_within_8yd": count
    }))
    .unwrap()
}

#[test]
fn policy_target_distance_is_inclusive_and_absent_target_is_false() {
    let rule = rule(1261, 3, [10, 0, 0]);
    for (distance, expected) in [
        (Some(9.999), true),
        (Some(10.0), true),
        (Some(10.001), false),
        (None, false),
    ] {
        assert_eq!(
            evaluate_rule(&rule, &facts(distance, None)),
            Ok(expected),
            "{distance:?}"
        );
    }
    assert!(evaluate_rule(&rule, &facts(Some(-1.0), None)).is_err());
}

#[test]
fn policy_nearby_count_requires_more_than_three_and_received_count() {
    let rule = rule(1268, 12, [3, 0, 0]);
    for (count, expected) in [(0, false), (3, false), (4, true), (5, true)] {
        assert_eq!(
            evaluate_rule(&rule, &facts(None, Some(count))),
            Ok(expected)
        );
    }
    assert!(evaluate_rule(&rule, &facts(None, None)).is_err());
}

#[test]
fn policy_does_not_enable_other_rule_ids_or_nonzero_unused_operands() {
    let facts = facts(Some(1.0), Some(9));
    for (id, kind) in [(1261, 3), (1268, 12)] {
        for values in [[3, 1, 0], [3, 0, 1]] {
            assert!(evaluate_rule(&rule(id, kind, values), &facts).is_err());
        }
        assert!(evaluate_rule(&rule(id + 100000, kind, [3, 0, 0]), &facts).is_err());
    }
    assert!(evaluate_rule(&rule(1261, 12, [3, 0, 0]), &facts).is_err());
    assert!(evaluate_rule(&rule(1268, 3, [3, 0, 0]), &facts).is_err());
}

#[test]
fn captured_fury_order_replays_synthetic_spatial_readings_without_changing_later_steps() {
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assisted_simc_mirror");
    let catalog = CooldownAssistCatalog::load(&directory).unwrap();
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/assisted_simc_mirror/native-fury.json"
    ))
    .unwrap();
    let original: serde_json::Value =
        serde_json::from_str(capture["evaluation_json"].as_str().unwrap()).unwrap();
    assert!(
        original["selection"]["Err"]
            .as_str()
            .unwrap()
            .contains("1261")
    );
    // Below the qualified AoE step, authored step18147 still offers Whirlwind.
    // Rejecting that later spell would invent a rotation change.
    for (distance, count) in [(10.0, Some(4)), (10.001, Some(4)), (10.0, Some(3))] {
        let mut reading = original["facts"].clone();
        reading["target_distance_yd"] = json!(distance);
        reading["player_combat_enemies_within_8yd"] = json!(count);
        let facts: AssistedFacts = serde_json::from_value(reading).unwrap();
        assert_eq!(
            catalog.recommend(
                72,
                |id| facts.known.contains(&id),
                |id| is_spell_usable(id, &facts),
                |rule| evaluate_rule(rule, &facts)
            ),
            Ok(Recommendation::Spell(190411))
        );
    }
    let mut reading = original["facts"].clone();
    reading["target_distance_yd"] = json!(10.0);
    let facts: AssistedFacts = serde_json::from_value(reading).unwrap();
    let decision = catalog.recommend(
        72,
        |id| facts.known.contains(&id),
        |id| is_spell_usable(id, &facts),
        |rule| evaluate_rule(rule, &facts),
    );
    assert_eq!(
        decision,
        Err("rule 1268: player combat nearby count not received".into())
    );
}
