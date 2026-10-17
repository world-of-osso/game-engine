//! Replay one-decision native observations, not facts sampled after the decision.
use game_engine_core::assisted_combat::{AssistedFacts, evaluate_rule, is_spell_usable};
use game_engine_core::cooldown_assist::{CooldownAssistCatalog, Recommendation};
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
}

#[test]
fn captured_player_specs_reach_their_own_fail_closed_boundary() {
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
        assert_eq!(evaluate(&catalog, &evaluation), evaluation.selection);
        println!("spec {}: {}", capture.spec, capture.expected_error);
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
