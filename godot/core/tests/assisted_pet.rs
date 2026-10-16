use game_engine_core::assisted_combat::{AssistedFacts, evaluate_rule, is_spell_usable};
use game_engine_core::cooldown_assist::{AssistedRule, CooldownAssistCatalog, Recommendation};
use std::path::Path;

fn pet_facts(present: Option<bool>) -> AssistedFacts {
    let mut encoded = serde_json::to_value(AssistedFacts::default()).unwrap();
    encoded["has_pet"] = serde_json::json!(present);
    serde_json::from_value(encoded).unwrap()
}

fn pet_rule(kind: u32) -> AssistedRule {
    AssistedRule {
        id: 29672,
        step_spell: 883,
        condition_type: kind,
        values: [0, 0, 0],
        field_11_1_7_60520_002: 0,
    }
}

#[test]
fn assisted_pet_boolean_conditions_distinguish_present_absent_and_unknown() {
    for (present, kind, expected) in [
        (false, 58, true),
        (true, 58, false),
        (false, 57, false),
        (true, 57, true),
    ] {
        assert_eq!(
            evaluate_rule(&pet_rule(kind), &pet_facts(Some(present))),
            Ok(expected)
        );
    }
    assert!(evaluate_rule(&pet_rule(58), &pet_facts(None)).is_err());
    for values in [[1, 0, 0], [0, 1, 0], [0, 0, 1]] {
        let mut rule = pet_rule(58);
        rule.values = values;
        assert!(evaluate_rule(&rule, &pet_facts(Some(false))).is_err());
    }
}

#[test]
fn assisted_pet_bm_authored_call_pet_requires_absent_owner_pet() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = CooldownAssistCatalog::load(&directory).unwrap();
    for (present, expected) in [
        (false, Recommendation::Spell(883)),
        (true, Recommendation::None),
    ] {
        let mut facts = pet_facts(Some(present));
        facts.known.insert(883);
        facts.ready.insert(883, Ok(true));
        facts.affordable.insert(883, Ok(true));
        facts.in_range.insert(883, Ok(true));
        let recommendation = catalog.recommend(
            253,
            |id| facts.known.contains(&id),
            |id| is_spell_usable(id, &facts),
            |rule| evaluate_rule(rule, &facts),
        );
        assert_eq!(recommendation, Ok(expected));
    }
}
