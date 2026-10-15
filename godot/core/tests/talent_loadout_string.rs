use game_engine_core::talent_data::{
    load_trait_rules,
    loadout_string::{LoadoutError, decode, encode},
    rules::{Purchase, TraitContext, build_config},
};
use std::path::Path;

fn mage() -> game_engine_core::talent_data::rule_data::TraitTree {
    load_trait_rules(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
        658,
    )
    .unwrap()
}
const CONTEXT: TraitContext = TraitContext {
    spec_id: 62,
    level: 80,
};

#[test]
fn talent_string_real_mage_allocation_round_trips_all_class_nodes() {
    let tree = mage();
    let mut purchases = Vec::new();
    // Build a concrete legal allocation through the same authoritative rule port.
    for _ in 0..5 {
        for node in &tree.nodes {
            if purchases
                .iter()
                .any(|purchase: &Purchase| purchase.node_id == node.id)
            {
                continue;
            }
            for entry in &node.entries {
                let mut candidate = purchases.clone();
                candidate.push(Purchase {
                    node_id: node.id,
                    entry_id: entry.id,
                    ranks: 1,
                });
                if build_config(&tree, CONTEXT, &candidate).is_ok() {
                    purchases = candidate;
                    break;
                }
            }
        }
    }
    purchases.sort_by_key(|purchase| (purchase.node_id, purchase.entry_id));
    assert!(purchases.len() > 10, "Actual allocation: {purchases:?}");
    let expected = build_config(&tree, CONTEXT, &purchases).unwrap();
    let text = encode(&tree, CONTEXT.spec_id, &expected).unwrap();
    assert!(text.starts_with("C"));
    assert!(!text.contains('='));
    assert_eq!(decode(&tree, CONTEXT, &text).unwrap(), expected);
}

#[test]
fn talent_string_rejects_bad_alphabet_short_version_and_wrong_spec() {
    let tree = mage();
    let entries = build_config(&tree, CONTEXT, &[]).unwrap();
    let text = encode(&tree, CONTEXT.spec_id, &entries).unwrap();
    assert_eq!(
        decode(&tree, CONTEXT, "not a code!"),
        Err(LoadoutError::BadString)
    );
    assert_eq!(
        decode(&tree, CONTEXT, &text[..24]),
        Err(LoadoutError::BadString)
    );
    let old = format!("B{}", &text[1..]);
    assert_eq!(decode(&tree, CONTEXT, &old), Err(LoadoutError::Version));
    assert_eq!(
        decode(
            &tree,
            TraitContext {
                spec_id: 63,
                ..CONTEXT
            },
            &text
        ),
        Err(LoadoutError::WrongSpec)
    );
    assert_eq!(
        decode(&tree, CONTEXT, &text[..text.len() - 2]),
        Err(LoadoutError::BadString)
    );
}

#[test]
fn talent_string_matches_actual_blizzard_lua_zero_and_nonzero_hashes() {
    let tree = mage();
    let expected = build_config(
        &tree,
        CONTEXT,
        &[Purchase {
            node_id: 62122,
            entry_id: 80181,
            ranks: 1,
        }],
    )
    .unwrap();
    let zero = include_str!("fixtures/talent_mage_zero.txt").trim();
    let nonzero = include_str!("fixtures/talent_mage_nonzero.txt").trim();
    assert_eq!(encode(&tree, 62, &expected).unwrap(), zero);
    assert_eq!(decode(&tree, CONTEXT, zero).unwrap(), expected);
    assert_eq!(decode(&tree, CONTEXT, nonzero).unwrap(), expected);
}

#[test]
fn talent_string_rejects_lua_encoded_invalid_choices_ranks_and_allocations() {
    let tree = mage();
    for text in [
        include_str!("fixtures/talent_mage_overrank.txt"),
        include_str!("fixtures/talent_mage_choice.txt"),
        include_str!("fixtures/talent_mage_prerequisite.txt"),
        include_str!("fixtures/talent_mage_otherspec.txt"),
    ] {
        assert_eq!(
            decode(&tree, CONTEXT, text.trim()),
            Err(LoadoutError::InvalidNodes),
            "{text}"
        );
    }
}

#[test]
fn talent_string_fully_spent_real_mage_can_be_exported_for_live_proof() {
    let tree = mage();
    let mut purchases: Vec<Purchase> = Vec::new();
    for _ in 0..20 {
        let before = purchases.clone();
        for node in &tree.nodes {
            for entry in &node.entries {
                let mut candidate = purchases.clone();
                if let Some(purchase) = candidate
                    .iter_mut()
                    .find(|purchase| purchase.node_id == node.id && purchase.entry_id == entry.id)
                {
                    purchase.ranks += 1;
                } else {
                    candidate.push(Purchase {
                        node_id: node.id,
                        entry_id: entry.id,
                        ranks: 1,
                    });
                }
                if build_config(&tree, CONTEXT, &candidate).is_ok() {
                    purchases = candidate;
                }
            }
        }
        if purchases == before {
            break;
        }
    }
    purchases.sort_by_key(|purchase| (purchase.node_id, purchase.entry_id));
    let config = build_config(&tree, CONTEXT, &purchases).unwrap();
    let unspent = game_engine_core::talent_data::rules::unspent(&tree, CONTEXT, &config);
    // Selected Spellslinger currency2987 is spent; inactive hero currencies do not
    // represent points the current spec/subtree can spend (TalentView.hero_points).
    assert_eq!(
        unspent,
        vec![(2801, 0), (2800, 0), (2986, 10), (2987, 0), (2988, 10)]
    );
    let code = encode(&tree, 62, &config).unwrap();
    assert_eq!(code, include_str!("fixtures/talent_mage_full.txt").trim());
    println!("FULL_MAGE_RETAIL_CODE={code}");
    assert_eq!(decode(&tree, CONTEXT, &code).unwrap(), config);
}
