use super::*;
fn draft() -> SpecializationDraft {
    SpecializationDraft {
        snapshot: Some(ProfessionSpecializationsSnapshot {
            skill_line: 2830,
            knowledge_currency_id: 2025,
            knowledge_available: 2,
            currencies: vec![ProfessionSpecCurrencySnapshot {
                trait_currency_id: 9,
                currency_id: 2025,
                available: 2,
            }],
            trees: vec![ProfessionSpecTreeSnapshot {
                tree_id: 350,
                name: "Leatherworking Discipline".into(),
                root_node_id: 100,
                unlocked: true,
                nodes: vec![ProfessionSpecNodeSnapshot {
                    node_id: 100,
                    entry_id: 200,
                    name: "Leatherworking Discipline".into(),
                    description: "Improve your Leatherworking skill.".into(),
                    icon: 134366,
                    spell_id: 0,
                    rank: 1,
                    granted_rank: 0,
                    max_rank: 3,
                    pos_x: 500,
                    pos_y: 500,
                    kind: 0,
                    available: true,
                    unavailable_reason: None,
                    costs: vec![ProfessionSpecCost {
                        trait_currency_id: 9,
                        amount: 1,
                    }],
                    path: None,
                    perk: None,
                }],
                edges: vec![],
            }],
        }),
        selected_tree: Some(350),
        selected_node: Some(100),
        ..Default::default()
    }
}
#[test]
fn specialization_query_uses_learned_specialization_tier_not_selected_classic_recipe() {
    use crate::professions::{ProfessionBook, Recipe};
    use crate::professions_catalog::RecipeCatalog;
    use shared::profession::ProfessionSkillLine;
    let catalog = RecipeCatalog::load(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
    )
    .unwrap();
    let book = ProfessionBook {
        selected: Some(2153),
        recipes: vec![Recipe {
            spell_id: 2153,
            skill_line: 2532,
            profession: 165,
            category: "Leather Armor".into(),
            category_order: 0,
            name: "Handstitched Leather Pants".into(),
            min_rank: 30,
            trivial_low: 45,
            trivial_high: 75,
            output: Some((2303, 1)),
            reagents: vec![(2318, 4), (2320, 1)],
        }],
        snapshot: ProfessionSnapshot {
            recipe_sources: vec![],
            spells: vec![2153],
            lines: vec![
                ProfessionSkillLine {
                    skill_line: 165,
                    step: 1,
                    rank: 1,
                    max_rank: 75,
                },
                ProfessionSkillLine {
                    skill_line: 2532,
                    step: 1,
                    rank: 300,
                    max_rank: 300,
                },
                ProfessionSkillLine {
                    skill_line: 2830,
                    step: 1,
                    rank: 25,
                    max_rank: 100,
                },
            ],
        },
        ..Default::default()
    };
    assert_eq!(catalog.specialization_skill_line(&book), Some(2830));
    let mut classic_only = book;
    classic_only
        .snapshot
        .lines
        .retain(|line| line.skill_line != 2830);
    assert_eq!(catalog.specialization_skill_line(&classic_only), None);
}

#[test]
fn specialization_perk_tooltip_explains_source_path_and_pending_milestone() {
    crate::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut draft = draft();
    let tree = &mut draft.snapshot.as_mut().unwrap().trees[0];
    let mut perk = tree.nodes[0].clone();
    perk.node_id = 28534;
    perk.entry_id = 36325;
    perk.name = "Leather armor bonus".into();
    perk.description = "Gain 5 Skill when crafting leather armor.".into();
    perk.rank = 0;
    perk.max_rank = 1;
    perk.perk = Some(ProfessionSpecPerk {
        path_node_id: 100,
        required_points: 2,
        unlocks_subpath: false,
        is_major_bonus: false,
    });
    tree.nodes.push(perk);
    let tooltip = draft.tooltip(28534).unwrap();
    let lines: Vec<_> = tooltip
        .content
        .lines
        .iter()
        .map(|line| line.left_text.as_str())
        .collect();
    assert!(lines.contains(&"Requires 2 points in Leatherworking Discipline"));
    assert!(!lines.iter().any(|line| line.starts_with("Rank ")));
    assert!(draft.stage_rank());
    let tooltip = draft.tooltip(28534).unwrap();
    assert!(
        tooltip
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Pending")
    );
}

#[test]
fn specialization_stages_absolute_ranks_and_spends_only_pending_knowledge() {
    let mut draft = draft();
    assert!(draft.stage_rank());
    assert_eq!(draft.remaining_knowledge(), 1);
    assert!(draft.stage_rank());
    assert_eq!(draft.remaining_knowledge(), 0);
    assert!(!draft.stage_rank());
    assert_eq!(draft.snapshot.as_ref().unwrap().trees[0].nodes[0].rank, 1);
    assert_eq!(
        draft.commit_request(),
        Some(ProfessionSpecializationRequest::Commit {
            skill_line: 2830,
            tree_id: 350,
            purchases: vec![ProfessionTraitPurchase {
                node_id: 100,
                entry_id: 200,
                ranks: 3
            }],
        })
    );
    assert!(draft.pending);
    assert!(draft.commit_request().is_none());
}
#[test]
fn specialization_undo_restores_balance_and_never_requests_a_refund() {
    let mut draft = draft();
    assert!(draft.stage_rank());
    draft.undo();
    assert_eq!(draft.remaining_knowledge(), 2);
    assert!(draft.commit_request().is_none());
    assert_eq!(draft.snapshot.as_ref().unwrap().trees[0].nodes[0].rank, 1);
}
#[test]
fn specialization_unlocked_root_spends_available_knowledge_entry_not_maxed_unlock_entry() {
    let mut draft = draft();
    let snapshot = draft.snapshot.as_mut().unwrap();
    let tree = &mut snapshot.trees[0];
    tree.root_node_id = 28546;
    let unlock = &mut tree.nodes[0];
    unlock.node_id = 28546;
    unlock.entry_id = 36338;
    unlock.rank = 1;
    unlock.max_rank = 1;
    unlock.available = false;
    unlock.kind = 1;
    unlock.costs = vec![ProfessionSpecCost {
        trait_currency_id: 19,
        amount: 1,
    }];
    unlock.unavailable_reason = Some("Maximum rank purchased".into());
    let mut knowledge = unlock.clone();
    knowledge.entry_id = 36327;
    knowledge.rank = 0;
    knowledge.max_rank = 30;
    knowledge.costs = vec![ProfessionSpecCost {
        trait_currency_id: 9,
        amount: 1,
    }];
    knowledge.available = true;
    knowledge.unavailable_reason = None;
    tree.nodes.push(knowledge);
    draft.selected_node = Some(28546);
    assert_eq!(draft.display_ranks(28546), Some((0, 30)));
    assert!(draft.stage_rank());
    assert_eq!(draft.display_ranks(28546), Some((1, 30)));
    assert_eq!(
        draft.commit_request(),
        Some(ProfessionSpecializationRequest::Commit {
            skill_line: 2830,
            tree_id: 350,
            purchases: vec![
                ProfessionTraitPurchase {
                    node_id: 28546,
                    entry_id: 36338,
                    ranks: 1
                },
                ProfessionTraitPurchase {
                    node_id: 28546,
                    entry_id: 36327,
                    ranks: 1
                },
            ],
        })
    );
    assert_eq!(draft.snapshot.as_ref().unwrap().trees[0].nodes[0].rank, 1);
}

#[test]
fn specialization_combines_costs_for_the_same_trait_currency() {
    let mut draft = draft();
    let snapshot = draft.snapshot.as_mut().unwrap();
    snapshot.knowledge_available = 1;
    snapshot.currencies[0].available = 1;
    snapshot.trees[0].nodes[0].costs.push(ProfessionSpecCost {
        trait_currency_id: 9,
        amount: 1,
    });
    assert!(!draft.stage_rank());
    assert!(draft.purchases.is_empty());
}

#[test]
fn specialization_tab_unlock_requires_confirmation_and_available_currency() {
    let mut draft = draft();
    let snapshot = draft.snapshot.as_mut().unwrap();
    snapshot.trees[0].unlocked = false;
    snapshot.trees[0].nodes[0].rank = 0;
    snapshot.knowledge_available = 0;
    snapshot.currencies[0].available = 0;
    assert!(!draft.can_unlock());
    draft.snapshot.as_mut().unwrap().currencies[0].available = 1;
    draft.snapshot.as_mut().unwrap().knowledge_available = 1;
    assert!(draft.can_unlock());
    assert!(draft.confirm_unlock().is_none());
    draft.confirming_unlock = true;
    assert_eq!(
        draft.confirm_unlock(),
        Some(ProfessionSpecializationRequest::Commit {
            skill_line: 2830,
            tree_id: 350,
            purchases: vec![ProfessionTraitPurchase {
                node_id: 100,
                entry_id: 200,
                ranks: 1
            }],
        })
    );
    assert!(draft.pending);
    assert!(!draft.snapshot.as_ref().unwrap().trees[0].unlocked);
}

#[test]
fn specialization_unavailable_path_cannot_stage() {
    let mut draft = draft();
    draft.snapshot.as_mut().unwrap().trees[0].nodes[0].available = false;
    assert!(!draft.stage_rank());
    assert!(draft.purchases.is_empty());
}
