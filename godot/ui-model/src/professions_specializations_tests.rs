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
