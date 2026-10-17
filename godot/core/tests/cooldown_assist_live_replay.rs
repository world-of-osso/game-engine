//! Replay one-decision native observations, not facts sampled after the decision.
use game_engine_core::assisted_combat::{AssistedFacts, evaluate_rule, is_spell_usable};
use game_engine_core::cooldown_assist::{AssistedRule, CooldownAssistCatalog, Recommendation};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Deserialize)]
struct Evaluation {
    specialization: u32,
    target: u64,
    facts: AssistedFacts,
    selection: Result<Recommendation, String>,
}

fn evaluate(
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

#[derive(Deserialize)]
struct CapturedBoundary {
    spec: u32,
    player_health: f64,
    evaluation_json: String,
    expected_error: String,
    expected_replay: Result<Recommendation, String>,
}

#[test]
fn captured_player_specs_replay_recorded_or_newly_sourced_outcomes() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bot_boundaries");
    let catalog = CooldownAssistCatalog::load(&directory).unwrap();
    for encoded in [
        include_str!("fixtures/bot_boundaries/native-105.json"),
        include_str!("fixtures/bot_boundaries/native-262.json"),
        include_str!("fixtures/bot_boundaries/native-63.json"),
    ] {
        let capture: CapturedBoundary = serde_json::from_str(encoded).unwrap();
        let evaluation: Evaluation = serde_json::from_str(&capture.evaluation_json).unwrap();
        assert!(capture.player_health > 0.0);
        assert_eq!(evaluation.specialization, capture.spec);
        assert_eq!(evaluation.facts.target_hostile, Some(true));
        assert_eq!(evaluation.selection, Err(capture.expected_error.clone()));
        assert_eq!(evaluate(&catalog, &evaluation), capture.expected_replay);
        println!("spec {}: {:?}", capture.spec, capture.expected_replay);
    }
}

#[test]
fn fire_cast_overlap_modifier_preserves_ordinary_usability_gates() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bot_boundaries");
    let catalog = CooldownAssistCatalog::load(&directory).unwrap();
    let capture: CapturedBoundary =
        serde_json::from_str(include_str!("fixtures/bot_boundaries/native-63.json")).unwrap();
    let evaluation: Evaluation = serde_json::from_str(&capture.evaluation_json).unwrap();
    assert_eq!(
        evaluate(&catalog, &evaluation),
        Ok(Recommendation::Spell(133))
    );
    for fact in ["ready", "affordable", "in_range"] {
        let mut encoded: Value = serde_json::from_str(&capture.evaluation_json).unwrap();
        encoded["facts"][fact]["133"] = serde_json::json!({"Ok": false});
        let blocked: Evaluation = serde_json::from_value(encoded).unwrap();
        assert_ne!(
            evaluate(&catalog, &blocked),
            Ok(Recommendation::Spell(133)),
            "{fact}"
        );
    }
}

#[test]
fn cast_overlap_modifier_refuses_every_nonzero_operand_position() {
    let facts = AssistedFacts::default();
    for values in [[1, 0, 0], [0, 1, 0], [0, 0, 1]] {
        let rule = AssistedRule {
            id: 33340,
            step_spell: 133,
            condition_type: 67,
            values,
            field_11_1_7_60520_002: 0,
        };
        assert!(evaluate_rule(&rule, &facts).is_err(), "{values:?}");
    }
}

#[test]
fn fury_spatial_policy_has_no_target_or_missing_count_without_new_readings() {
    for (id, condition_type, values) in [(1261, 3, [10, 0, 0]), (1268, 12, [3, 0, 0])] {
        let rule = AssistedRule {
            id,
            step_spell: 190411,
            condition_type,
            values,
            field_11_1_7_60520_002: 0,
        };
        let result = evaluate_rule(&rule, &AssistedFacts::default());
        if condition_type == 3 {
            assert_eq!(result, Ok(false));
        } else {
            assert!(result.is_err());
        }
    }
}

fn replay_skin(skin: &str) {
    let directory = std::env::var("COOLDOWN_ASSIST_PROOF_DIR")
        .expect("set COOLDOWN_ASSIST_PROOF_DIR to the owned both-skin evidence directory");
    let path = Path::new(&directory).join(skin);
    let read = |name: &str| -> Value {
        serde_json::from_slice(&std::fs::read(path.join(name)).unwrap()).unwrap()
    };
    let proof = read("recommendation-update-proof.json");
    // Godot's parsed/display JSON converts integer cells to floats. Replay the
    // untouched native JSON string, not that presentation projection.
    let encoded = proof["assisted"]["evaluation_json"].as_str().unwrap();
    let mut evaluation: Evaluation = serde_json::from_str(encoded).unwrap();
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = CooldownAssistCatalog::load(&data).unwrap();
    let after = proof["after"].as_u64().unwrap() as u32;
    assert_ne!(proof["before"].as_u64().unwrap(), u64::from(after));
    assert_eq!(evaluation.specialization, 70);
    assert_eq!(proof["target"]["target"].as_u64(), Some(evaluation.target));
    assert_eq!(evaluation.facts.target_hostile, Some(true));
    assert_eq!(evaluation.selection, Ok(Recommendation::Spell(after)));
    assert_eq!(evaluate(&catalog, &evaluation), evaluation.selection);
    let charge = read("charge-proof.json");
    assert_eq!(charge["before"]["charges"].as_u64(), Some(2));
    assert_eq!(charge["after"]["charges"].as_u64(), Some(1));
    assert!(charge["after"]["remaining"].as_f64().unwrap() > 0.0);
    let cast = charge["cast"].as_u64().unwrap() as u32;
    assert!(evaluation.facts.charges.contains_key(&cast));
    assert_eq!(evaluation.facts.ready.get(&after), Some(&Ok(true)));
    assert_eq!(evaluation.facts.affordable.get(&after), Some(&Ok(true)));
    assert_eq!(evaluation.facts.in_range.get(&after), Some(&Ok(true)));
    println!(
        "{skin}: exact first eligible {after}, Holy Power {:?}, charges {:?}, cooldowns {:?}, target {}",
        evaluation.facts.powers.get(&9),
        evaluation.facts.charges,
        evaluation.facts.cooldowns_ms,
        evaluation.target
    );
    // Remove the selected spender's resource, or an ordinary spell's readiness.
    // This control rejects accepting a spell merely because it belongs to the rotation.
    if after == 85256 {
        assert!(evaluation.facts.powers[&9] > 3.0);
        evaluation.facts.powers.insert(9, 0.0);
    } else {
        evaluation.facts.ready.insert(after, Ok(false));
    }
    assert_ne!(
        evaluate(&catalog, &evaluation),
        Ok(Recommendation::Spell(after))
    );
}

#[test]
#[ignore = "requires owned private Modern/Forever recordings; set COOLDOWN_ASSIST_PROOF_DIR"]
fn cooldown_assist_live_modern_replays_exact_first_eligible_step() {
    replay_skin("Modern");
}

#[test]
#[ignore = "requires owned private Modern/Forever recordings; set COOLDOWN_ASSIST_PROOF_DIR"]
fn cooldown_assist_live_forever_replays_exact_first_eligible_step() {
    replay_skin("Forever");
}
