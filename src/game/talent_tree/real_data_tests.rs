//! Tests against the pinned 12.1.0.69933 CSVs; fail when `data/db2` lacks them.

use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

use super::rules::{TraitContext, granted_entries, node_visible};
use super::*;
#[path = "../../../tests/unit/required_asset.rs"]
mod required_asset;
use required_asset::require_asset;

const PALADIN_TREE: u32 = 790;
const RETRIBUTION: u32 = 70;
const PROTECTION: u32 = 66;

/// Cold load (CSV build + cache write) and warm load (cache read), once per process.
fn loaded() -> &'static TalentTreeData {
    static LOADED: OnceLock<TalentTreeData> = OnceLock::new();
    LOADED.get_or_init(|| {
        require_asset(talent_source_dir(Path::new("data")).join("TraitNode.csv"));
        let mut paths = TalentTreePaths::for_data_dir(Path::new("data"));
        paths.cache_path =
            std::env::temp_dir().join(format!("talent_trees_test_{}.bin", std::process::id()));
        let started = Instant::now();
        let cold = load_talent_trees(&paths).expect("cold talent tree load");
        let cold_time = started.elapsed();
        let started = Instant::now();
        let warm = load_talent_trees(&paths).expect("warm talent tree load");
        eprintln!(
            "talent trees: cold {cold_time:?}, warm {:?}, cache {} bytes",
            started.elapsed(),
            std::fs::metadata(&paths.cache_path).map_or(0, |meta| meta.len())
        );
        std::fs::remove_file(&paths.cache_path).expect("remove test cache");
        assert_eq!(cold, warm, "cache round trip");
        warm
    })
}

#[test]
fn loads_all_ten_class_trees_with_paladin_shape() {
    let data = loaded();
    assert_eq!(data.trees().len(), 10);
    let paladin = data.tree(PALADIN_TREE).expect("paladin tree");
    assert_eq!(paladin.class_id, 2);
    assert_eq!(paladin.nodes.len(), 233);
    let currencies: Vec<u32> = paladin.currencies.iter().map(|c| c.id).collect();
    assert_eq!(currencies, [2801, 2800, 2986, 2987, 2988]);
    let sub_trees: Vec<u32> = paladin.sub_trees.iter().map(|s| s.id).collect();
    assert_eq!(sub_trees, [48, 49, 50]);
    let specs: Vec<u32> = paladin.specs.iter().map(|s| s.id).collect();
    assert!(specs.contains(&RETRIBUTION) && specs.contains(&PROTECTION));
    let apex = paladin.node(110417).expect("apex node");
    assert_eq!(apex.kind, NodeKind::Tiered);
}

#[test]
fn retribution_level_80_grants_hammer_of_wrath() {
    let data = loaded();
    let tree = data.tree(PALADIN_TREE).unwrap();
    let ctx = TraitContext {
        spec_id: RETRIBUTION,
        level: 80,
    };
    let granted_spells: Vec<u32> = granted_entries(tree, ctx)
        .iter()
        .filter_map(|entry| tree.node(entry.node_id)?.entry(entry.entry_id))
        .map(|entry| entry.spell_id)
        .collect();
    assert!(granted_spells.contains(&1241288), "{granted_spells:?}");
}

#[test]
fn spec_visibility_hides_other_spec_nodes() {
    let data = loaded();
    let tree = data.tree(PALADIN_TREE).unwrap();
    let ret = TraitContext {
        spec_id: RETRIBUTION,
        level: 80,
    };
    let prot = TraitContext {
        spec_id: PROTECTION,
        level: 80,
    };
    let visible = |ctx| {
        tree.nodes
            .iter()
            .filter(|node| node_visible(tree, ctx, node))
            .map(|node| node.id)
            .collect::<Vec<_>>()
    };
    let (ret_nodes, prot_nodes) = (visible(ret), visible(prot));
    assert_ne!(ret_nodes, prot_nodes);
    // Hero selection 99837 (Templar / Herald of the Sun) is Retribution's.
    assert!(ret_nodes.contains(&99837));
    assert!(!prot_nodes.contains(&99837));
}

#[test]
fn retribution_background_is_the_authored_retail_atlas_member() {
    let data = loaded();
    let tree = data.tree(PALADIN_TREE).unwrap();
    let background = data.spec_background(tree, RETRIBUTION).expect("background");
    // UiTextureAtlasMember 15969 in UiTextureAtlas 2048 (2048x1024).
    assert_eq!(background.fdid, 4631340);
    assert_eq!((background.width, background.height), (1612.0, 774.0));
    let node = data.art("talents-node-square-yellow").expect("node art");
    assert_eq!(node.fdid, 4556093);
}

#[test]
fn entry_spells_carry_the_passive_attribute() {
    let data = loaded();
    let tree = data.tree(PALADIN_TREE).unwrap();
    let entries = || tree.nodes.iter().flat_map(|node| &node.entries);
    // Greater Judgment 231663 is passive; Lay on Hands 633 is not.
    let spell = |id| entries().find(|entry| entry.spell_id == id).unwrap();
    assert!(spell(231663).passive);
    assert!(!spell(633).passive);
}

fn ret_config(tree: &TalentTree, level: u8) -> (TraitContext, Vec<rules::ConfigEntry>) {
    let ctx = TraitContext {
        spec_id: RETRIBUTION,
        level,
    };
    (ctx, granted_entries(tree, ctx))
}

fn buy(config: &mut Vec<rules::ConfigEntry>, node_id: u32, entry_id: u32) {
    config.push(rules::ConfigEntry {
        node_id,
        entry_id,
        ranks: 1,
        granted: 0,
    });
}

fn level_80_budget() -> std::collections::HashMap<u32, i32> {
    [(2801, 31), (2800, 30), (2986, 10), (2987, 10), (2988, 10)]
        .into_iter()
        .collect()
}

#[test]
fn mirrored_rules_reject_like_the_server() {
    let data = loaded();
    let tree = data.tree(PALADIN_TREE).unwrap();
    let owned = level_80_budget();
    let (ctx, granted) = ret_config(tree, 80);
    assert_eq!(rules::validate(tree, ctx, &granted, &owned), Ok(()));

    // Avenging Wrath 81544 without Expurgation 92689 (server test wording).
    let mut config = granted.clone();
    buy(&mut config, 81544, 102519);
    assert_eq!(
        rules::validate(tree, ctx, &config, &owned),
        Err("node 81544 requires one of nodes [92689] fully ranked".into())
    );

    // Templar hero selection 99837 needs level 71.
    let (ctx, mut config) = ret_config(tree, 70);
    buy(&mut config, 99837, 123357);
    assert_eq!(
        rules::validate(tree, ctx, &config, &owned),
        Err("node 99837 is not available: condition 27046 requires level 71".into())
    );
}
