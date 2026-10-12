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

fn replay_skin(skin: &str) {
    let directory = std::env::var("COOLDOWN_ASSIST_PROOF_DIR")
        .expect("set COOLDOWN_ASSIST_PROOF_DIR to the owned both-skin evidence directory");
    let path = Path::new(&directory).join(skin);
    let read = |name: &str| -> Value {
        serde_json::from_slice(&std::fs::read(path.join(name)).unwrap()).unwrap()
    };
    let proof = read("recommendation-update-proof.json");
    let mut evaluation: Evaluation = serde_json::from_value(proof["evaluation"].clone()).unwrap();
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
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
    assert_eq!(evaluation.facts.charges.get(&cast), Some(&1));
    println!(
        "{skin}: exact first eligible {after}, Holy Power {:?}, charges {:?}, cooldowns {:?}, target {}",
        evaluation.facts.powers.get(&9),
        evaluation.facts.charges,
        evaluation.facts.cooldowns_ms,
        evaluation.target
    );
    // A recorded spender must stop qualifying when its observed resource is removed.
    // This control rejects accepting a spell merely because it belongs to the rotation.
    if after == 85256 {
        assert!(evaluation.facts.powers[&9] > 3.0);
        evaluation.facts.powers.insert(9, 0.0);
        assert_ne!(
            evaluate(&catalog, &evaluation),
            Ok(Recommendation::Spell(after))
        );
    }
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
