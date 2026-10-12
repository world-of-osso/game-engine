use game_engine_core::assisted_combat::{AssistedFacts, evaluate_rule, is_spell_usable};
use game_engine_core::cooldown_assist::{CooldownAssistCatalog, Recommendation};
use game_engine_core::cooldown_conditions::{ConditionActor, CooldownConditions};
use std::collections::BTreeMap;
use std::path::Path;

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
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
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
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
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
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
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
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
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
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
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
fn cooldown_assist_missing_spec_has_no_invented_rotation() {
    let data = catalog();
    assert_eq!(
        data.recommend(999999, |_| true, |_| Ok(true), |_| Ok(true)),
        Ok(Recommendation::Unavailable)
    );
}
