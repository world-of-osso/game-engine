use game_engine_core::assisted_combat::{AssistedFacts, evaluate_rule, is_spell_usable};
use game_engine_core::cooldown_assist::{CooldownAssistCatalog, Recommendation};
use game_engine_core::cooldown_conditions::{ConditionActor, CooldownConditions};
use std::collections::BTreeMap;
use std::path::Path;

#[test]
fn cooldown_assist_recorded_facts_preserve_resources_cost_errors_and_selection() {
    let mut facts = AssistedFacts::default();
    facts.known.insert(35395);
    facts.powers.insert(9, 4.0);
    facts.charges.insert(35395, 1);
    facts.cooldowns_ms.insert(20271, 2345.0);
    facts.player_health_pct = Some(80.0);
    facts.target_health_pct = Some(25.0);
    facts.target_hostile = Some(true);
    facts.player_auras.insert(137027);
    facts.target_auras.insert(197277);
    facts.ready.insert(35395, Ok(true));
    facts
        .affordable
        .insert(35395, Err("cost not received".into()));
    facts.in_range.insert(35395, Ok(true));
    let encoded = serde_json::to_value(&facts).unwrap();
    let decoded: AssistedFacts = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(&decoded).unwrap(), encoded);
    assert_eq!(decoded.powers.get(&9), Some(&4.0));
    assert_eq!(decoded.charges.get(&35395), Some(&1));
    assert_eq!(decoded.affordable[&35395], Err("cost not received".into()));
    let selection: Result<Recommendation, String> = Err("cost not received".into());
    let decoded_selection: Result<Recommendation, String> =
        serde_json::from_value(serde_json::to_value(&selection).unwrap()).unwrap();
    assert_eq!(decoded_selection, selection);
}

#[test]
fn cooldown_assist_ret_authored_next_spell_changes_with_server_readiness() {
    let data = catalog();
    let mut facts = AssistedFacts::default();
    facts.known.insert(35395);
    facts.ready.insert(35395, Ok(true));
    facts.affordable.insert(35395, Ok(true));
    facts.in_range.insert(35395, Ok(true));
    assert_eq!(
        data.recommend(
            70,
            |id| facts.known.contains(&id),
            |id| is_spell_usable(id, &facts),
            |rule| evaluate_rule(rule, &facts)
        ),
        Ok(Recommendation::Spell(35395))
    );
    facts.ready.insert(35395, Ok(false));
    assert_eq!(
        data.recommend(
            70,
            |id| facts.known.contains(&id),
            |id| is_spell_usable(id, &facts),
            |rule| evaluate_rule(rule, &facts)
        ),
        Ok(Recommendation::None)
    );
}

#[test]
fn cooldown_assist_unconditional_authored_step_still_requires_cast_readiness() {
    let data = catalog();
    let mut facts = AssistedFacts::default();
    facts.known.insert(20271);
    facts.ready.insert(20271, Ok(false));
    facts.affordable.insert(20271, Ok(true));
    facts.in_range.insert(20271, Ok(true));
    assert_eq!(
        data.recommend(
            70,
            |id| facts.known.contains(&id),
            |id| is_spell_usable(id, &facts),
            |rule| evaluate_rule(rule, &facts)
        ),
        Ok(Recommendation::None)
    );
}

#[test]
fn cooldown_assist_unknown_effective_cost_blocks_instead_of_guessing() {
    let data = catalog();
    let mut facts = AssistedFacts::default();
    facts.known.insert(35395);
    facts.ready.insert(35395, Ok(true));
    facts
        .affordable
        .insert(35395, Err("effective caster cost unavailable".into()));
    facts.in_range.insert(35395, Ok(true));
    let decision = data.recommend(
        70,
        |id| facts.known.contains(&id),
        |id| is_spell_usable(id, &facts),
        |rule| evaluate_rule(rule, &facts),
    );
    assert!(
        decision
            .unwrap_err()
            .contains("effective caster cost unavailable")
    );
}

#[test]
fn cooldown_assist_player_conditions_follow_real_race_mask_not_skin() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let ids = [34465, 47453].into_iter().collect();
    let conditions = CooldownConditions::load(&dir, &ids).unwrap();
    let mut actor = ConditionActor {
        race_bit: 2,
        class: 1,
        spec_index: 0,
        known: &[],
        auras: &BTreeMap::new(),
    };
    assert_eq!(conditions.evaluate(34465, &actor), Ok(true));
    assert_eq!(conditions.evaluate(47453, &actor), Ok(false));
    actor.race_bit = 4;
    assert_eq!(conditions.evaluate(34465, &actor), Ok(false));
    assert_eq!(conditions.evaluate(47453, &actor), Ok(true));
}

#[test]
fn cooldown_assist_modifier_trees_follow_authored_spell_and_aura_gates() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let conditions =
        CooldownConditions::load(&dir, &[160516, 161780].into_iter().collect()).unwrap();
    let empty = BTreeMap::new();
    let mut auras = BTreeMap::new();
    let mut actor = ConditionActor {
        race_bit: 0,
        class: 1,
        spec_index: 2,
        known: &[],
        auras: &empty,
    };
    assert_eq!(conditions.evaluate(160516, &actor), Ok(false));
    actor.known = &[228920];
    assert_eq!(conditions.evaluate(160516, &actor), Ok(true));
    actor.known = &[];
    auras.insert(1235113, 1);
    actor.auras = &auras;
    assert_eq!(conditions.evaluate(160516, &actor), Ok(true));
    assert_eq!(conditions.evaluate(161780, &actor), Ok(true));
    for spells in [&[386763][..], &[157997][..]] {
        actor.known = spells;
        assert_eq!(conditions.evaluate(161780, &actor), Ok(false));
    }
    actor.known = &[386763, 157997, 431044];
    assert_eq!(conditions.evaluate(161780, &actor), Ok(true));
}

#[test]
fn cooldown_assist_full_referenced_condition_load_handles_signed_mask_words() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let data = catalog();
    let conditions = CooldownConditions::load(&dir, &data.player_condition_ids()).unwrap();
    let auras = BTreeMap::new();
    let mut actor = ConditionActor {
        race_bit: 31,
        class: 1,
        spec_index: 0,
        known: &[],
        auras: &auras,
    };
    assert_eq!(conditions.evaluate(66693, &actor), Ok(true));
    actor.race_bit = 63;
    assert_eq!(conditions.evaluate(66693, &actor), Ok(false));
}

#[test]
fn cooldown_assist_race_only_condition_accepts_inactive_content_tuning_flag() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let conditions = CooldownConditions::load(&dir, &[104496].into_iter().collect()).unwrap();
    let auras = BTreeMap::new();
    let mut actor = ConditionActor {
        race_bit: 15,
        class: 2,
        spec_index: 2,
        known: &[],
        auras: &auras,
    };
    assert_eq!(conditions.evaluate(104496, &actor), Ok(true));
    actor.race_bit = 16;
    assert_eq!(conditions.evaluate(104496, &actor), Ok(true));
    actor.race_bit = 0;
    assert_eq!(conditions.evaluate(104496, &actor), Ok(false));
}

fn catalog() -> CooldownAssistCatalog {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    CooldownAssistCatalog::load(&dir).expect("authentic exported Retail tables")
}

#[test]
fn cooldown_assist_categories_preserve_authored_order_and_links() {
    let data = catalog();
    assert_eq!(data.charge_category(108853), Some(1500));
    let entries = data.cooldowns(71, |_| Ok(true)).unwrap();
    let strike = entries.iter().find(|entry| entry.id == 27).unwrap();
    assert_eq!(
        (strike.spell_id, strike.category, strike.order),
        (12294, 0, 0)
    );
    assert!(
        entries
            .windows(2)
            .all(|pair| (pair[0].category, pair[0].order, pair[0].id)
                <= (pair[1].category, pair[1].order, pair[1].id))
    );
    let linked = data.entry(280).unwrap();
    assert!(
        linked
            .linked_spells
            .iter()
            .any(|link| link.spell_id == 16979)
    );
}

#[test]
fn cooldown_assist_rotation_uses_authored_order_not_known_spell_order() {
    let data = catalog();
    let decision = data.recommend(
        70,
        |spell| spell == 343527 || spell == 255937,
        |_| Ok(true),
        |_| Ok(true),
    );
    assert_eq!(decision, Ok(Recommendation::Spell(343527)));
}

#[test]
fn cooldown_assist_unknown_rule_never_recommends_a_lower_priority_spell() {
    let data = catalog();
    let decision = data.recommend(
        70,
        |_| true,
        |_| Ok(true),
        |rule| {
            if rule.condition_type == 56 {
                Err("range unavailable".to_owned())
            } else {
                Ok(true)
            }
        },
    );
    assert!(decision.unwrap_err().contains("range unavailable"));
}

#[test]
fn cooldown_assist_numeric_rules_never_guess_unverified_equality_boundaries() {
    use game_engine_core::cooldown_assist::AssistedRule;
    let mut facts = AssistedFacts::default();
    let mut rule = AssistedRule {
        id: 100,
        step_spell: 85256,
        condition_type: 37,
        values: [3, 0, 0],
        field_11_1_7_60520_002: 0,
    };
    for (power, threshold, expected) in [(0.0, 3, false), (4.0, 3, true)] {
        facts.powers.insert(9, power);
        rule.values[0] = threshold;
        assert_eq!(evaluate_rule(&rule, &facts), Ok(expected));
    }
    facts.powers.insert(9, 3.0);
    assert!(
        evaluate_rule(&rule, &facts)
            .unwrap_err()
            .contains("equality")
    );
    rule.condition_type = 38;
    facts.powers.insert(9, 0.0);
    assert_eq!(evaluate_rule(&rule, &facts), Ok(true));
    rule.condition_type = 21;
    rule.values[0] = 300;
    facts.powers.insert(1, 500.0);
    assert_eq!(
        evaluate_rule(&rule, &facts),
        Ok(true),
        "rage uses DB2 raw tenths, not UI display units"
    );
    rule.condition_type = 63;
    rule.values = [2, 0, 0];
    facts.charges.insert(85256, 1);
    assert_eq!(evaluate_rule(&rule, &facts), Ok(false));
    facts.charges.insert(85256, 2);
    assert!(
        evaluate_rule(&rule, &facts)
            .unwrap_err()
            .contains("equality")
    );
    rule.values[1] = 108853;
    assert!(
        evaluate_rule(&rule, &facts)
            .unwrap_err()
            .contains("encoding")
    );
    rule.condition_type = 65;
    rule.values = [108853, 1500, 0];
    facts.cooldowns_ms.insert(108853, 2000.0);
    assert_eq!(evaluate_rule(&rule, &facts), Ok(true));
    facts.cooldowns_ms.insert(108853, 1500.0);
    assert!(
        evaluate_rule(&rule, &facts)
            .unwrap_err()
            .contains("equality")
    );
    rule.condition_type = 68;
    rule.values = [50, 0, 0];
    facts.player_health_pct = Some(75.0);
    assert_eq!(evaluate_rule(&rule, &facts), Ok(true));
    rule.condition_type = 67;
    rule.values = [0, 0, 0];
    assert!(
        evaluate_rule(&rule, &facts).is_err(),
        "prediction policy is not derivable from the layout/name"
    );
}

#[test]
fn cooldown_assist_all_forty_specs_preserve_exact_source_identities_and_order() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    // These tables contain only integer cells; decode the external CSV contract
    // independently of the catalog's join/ordering implementation.
    let read = |table: &str| -> Vec<Vec<i64>> {
        std::fs::read_to_string(dir.join(format!("{table}.csv")))
            .unwrap()
            .lines()
            .skip(1)
            .map(|line| {
                line.split(',')
                    .map(|cell| cell.trim_matches('"').parse().unwrap())
                    .collect()
            })
            .collect()
    };
    let sets: BTreeMap<_, _> = read("CooldownSet")
        .into_iter()
        .map(|row| (row[0], row[1]))
        .collect();
    let source = read("CooldownSetSpell");
    let data = catalog();
    let specs: std::collections::BTreeSet<_> = read("AssistedCombat")
        .into_iter()
        .map(|row| row[1])
        .collect();
    assert_eq!(specs.len(), 40);
    for spec in specs {
        let mut expected: Vec<_> = source.iter().filter(|row| matches!(sets.get(&row[1]), Some(set_spec) if *set_spec == 0 || *set_spec == spec)).map(|row| (row[0] as u32, row[2] as u32, row[3] as u32, row[5] as u32, row[4] as u32, row[6] as u32)).collect();
        expected.sort_by_key(|(id, _, category, order, _, _)| (*category, *order, *id));
        let actual: Vec<_> = data
            .cooldowns(spec as u32, |_| Ok(true))
            .unwrap()
            .iter()
            .map(|entry| {
                (
                    entry.id,
                    entry.spell_id,
                    entry.category,
                    entry.order,
                    entry.player_condition_id,
                    entry.flags,
                )
            })
            .collect();
        assert_eq!(
            actual, expected,
            "authored identities/order/conditions/flags for spec{spec}"
        );
    }
}

/// Exact authored candidate counts, including spec-agnostic sets, before actor gates.
/// Frost/Devourer have no authored tracked bars; do not invent entries for them.
#[test]
fn cooldown_assist_all_forty_specs_preserve_authored_category_coverage() {
    let data = catalog();
    let expected = [
        (62, [21, 53, 69, 2]),
        (63, [19, 53, 70, 2]),
        (64, [22, 55, 70, 0]),
        (65, [28, 46, 51, 1]),
        (66, [22, 51, 73, 4]),
        (70, [20, 50, 62, 7]),
        (71, [28, 54, 75, 1]),
        (72, [24, 53, 74, 2]),
        (73, [25, 52, 72, 3]),
        (102, [26, 54, 80, 3]),
        (103, [26, 58, 81, 2]),
        (104, [28, 56, 67, 3]),
        (105, [37, 53, 62, 4]),
        (250, [22, 51, 71, 7]),
        (251, [23, 48, 72, 2]),
        (252, [24, 50, 78, 4]),
        (253, [17, 55, 63, 5]),
        (254, [17, 55, 69, 5]),
        (255, [17, 61, 67, 4]),
        (256, [24, 46, 62, 2]),
        (257, [29, 43, 63, 2]),
        (258, [26, 49, 80, 2]),
        (259, [24, 54, 85, 1]),
        (260, [26, 54, 77, 2]),
        (261, [22, 53, 64, 1]),
        (262, [25, 60, 82, 3]),
        (263, [20, 63, 93, 3]),
        (264, [26, 58, 78, 7]),
        (265, [24, 48, 61, 1]),
        (266, [26, 48, 62, 5]),
        (267, [28, 48, 72, 1]),
        (268, [24, 51, 81, 2]),
        (269, [24, 49, 52, 5]),
        (270, [28, 48, 49, 5]),
        (577, [21, 48, 61, 3]),
        (581, [24, 48, 62, 3]),
        (1467, [24, 51, 69, 1]),
        (1468, [29, 47, 54, 2]),
        (1473, [22, 53, 69, 2]),
        (1480, [17, 47, 64, 0]),
    ];
    for (spec, counts) in expected {
        let mut actual = [0; 4];
        for entry in data.cooldowns(spec, |_| Ok(true)).unwrap() {
            if entry.category < 4 {
                actual[entry.category as usize] += 1;
            }
        }
        assert_eq!(actual, counts, "spec{spec}");
        assert!(
            !data.rotation_spells(spec).is_empty(),
            "spec{spec}: authored rotation absent"
        );
    }
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let conditions = CooldownConditions::load(&dir, &data.player_condition_ids()).unwrap();
    let actor = ConditionActor {
        race_bit: 0,
        class: 1,
        spec_index: 2,
        known: &[],
        auras: &BTreeMap::new(),
    };
    for id in data.player_condition_ids() {
        conditions
            .evaluate(id, &actor)
            .unwrap_or_else(|error| panic!("condition{id}: {error}"));
    }
}

#[test]
fn cooldown_assist_missing_spec_has_no_invented_rotation() {
    let data = catalog();
    assert_eq!(
        data.recommend(999999, |_| true, |_| Ok(true), |_| Ok(true)),
        Ok(Recommendation::Unavailable)
    );
}
