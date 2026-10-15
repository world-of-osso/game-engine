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
    for node in &tree.nodes {
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
    assert!(purchases.len() > 10);
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
