use game_engine_core::cooldown_assist::{CooldownAssistCatalog, Recommendation};
use std::path::Path;

fn catalog() -> CooldownAssistCatalog {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
    CooldownAssistCatalog::load(&dir).expect("authentic exported Retail tables")
}

#[test]
fn cooldown_assist_categories_preserve_authored_order_and_links() {
    let data = catalog();
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
    assert!(linked.linked_spells.contains(&16979));
}

#[test]
fn cooldown_assist_rotation_uses_authored_order_not_known_spell_order() {
    let data = catalog();
    let decision = data.recommend(70, |spell| spell == 343527 || spell == 255937, |_| Ok(true));
    assert_eq!(decision, Ok(Recommendation::Spell(343527)));
}

#[test]
fn cooldown_assist_unknown_rule_never_recommends_a_lower_priority_spell() {
    let data = catalog();
    let decision = data.recommend(
        70,
        |_| true,
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
        data.recommend(999999, |_| true, |_| Ok(true)),
        Ok(Recommendation::Unavailable)
    );
}
