use game_engine_core::talent_data::{TalentPage, load_talent_page};
use std::path::Path;
fn arcane() -> TalentPage {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
    load_talent_page(&dir, 8, 62).expect("local Mage Arcane rows")
}
#[test]
fn arcane_class_skillline_selects_tree_and_spec_filters_nodes() {
    let page = arcane();
    assert_eq!(page.tree_id, 658); // SkillLine 904; SkillLineXTraitTree 40.
    assert_eq!((page.class.nodes.len(), page.spec.nodes.len()), (43, 38));
    assert_eq!(page.starter_loadout_id, Some(991)); // NOT applied as purchased ranks.
    assert!(
        !page
            .class
            .nodes
            .iter()
            .any(|n| n.id == 62117 || n.id == 62119)
    );
}
#[test]
fn arcane_three_nodes_match_db2_positions_spells_and_edges() {
    let page = arcane();
    for (tree, id, position, spell, edge_ids) in [
        (
            &page.class,
            62121,
            [3900.0, 1500.0],
            235450,
            vec![127166, 130124],
        ),
        (
            &page.class,
            62084,
            [2100.0, 3900.0],
            30449,
            vec![130503, 130504],
        ),
        (
            &page.spec,
            102439,
            [11100.0, 3300.0],
            1241462,
            vec![126342, 126497],
        ),
    ] {
        let node = tree.nodes.iter().find(|n| n.id == id).expect("cited node");
        assert_eq!(node.position, position);
        assert_eq!(node.entries[0].spell_id, spell);
        assert_eq!(
            tree.edges
                .iter()
                .filter(|e| e.from == id)
                .map(|e| e.id)
                .collect::<Vec<_>>(),
            edge_ids
        );
    }
}
#[test]
fn arcane_choice_preserves_both_ordered_entries() {
    let page = arcane();
    let node = page
        .class
        .nodes
        .iter()
        .find(|n| n.id == 62087)
        .expect("choice node");
    assert_eq!((node.node_type, node.flags), (2, 1));
    assert_eq!(
        node.entries
            .iter()
            .map(|e| (e.id, e.spell_id))
            .collect::<Vec<_>>(),
        vec![(80143, 386763), (134199, 157997)]
    );
}
#[test]
fn arcane_only_default_granted_class_root_is_learned() {
    let page = arcane();
    assert_eq!(
        page.class
            .nodes
            .iter()
            .filter(|n| n.granted_ranks > 0)
            .map(|n| (n.id, n.granted_ranks))
            .collect::<Vec<_>>(),
        vec![(62121, 1)]
    );
    assert!(page.spec.nodes.iter().all(|n| n.granted_ranks == 0));
}
#[test]
fn arcane_hero_visibility_uses_sufficient_spec_conditions() {
    let page = arcane();
    assert_eq!(
        page.heroes
            .iter()
            .map(|t| (t.id, t.name.as_str(), t.nodes.len()))
            .collect::<Vec<_>>(),
        vec![(39, "Sunfury", 14), (40, "Spellslinger", 14)]
    );
    let selection = page.hero_selection.expect("Arcane subtree choice");
    assert_eq!(selection.id, 99830);
    assert_eq!(
        selection
            .entries
            .iter()
            .map(|e| e.subtree_id)
            .collect::<Vec<_>>(),
        vec![40, 39]
    );
    assert!(
        page.heroes
            .iter()
            .flat_map(|t| &t.nodes)
            .all(|n| n.granted_ranks == 0)
    ); // no chosen hero subtree
}
