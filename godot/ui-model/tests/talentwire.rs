use game_engine_ui_model::{
    spellbook_frame_component::PlayerSpellsTab,
    spellbook_preview::load_preview_state,
    talents::{TalentEditor, TalentView},
};
use shared::protocol::{
    CommitTraitConfig, TraitCommitResult, TraitConfigSnapshot, TraitEntrySelection,
};

fn view() -> TalentView {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    load_preview_state(&data, PlayerSpellsTab::Talents)
        .unwrap()
        .talents
        .unwrap()
}
fn editor(view: &TalentView) -> TalentEditor {
    let mut editor = TalentEditor::default();
    editor.receive_snapshot(TraitConfigSnapshot {
        spec_id: 62,
        tree_id: view.graph.tree_id,
        entries: vec![TraitEntrySelection {
            node_id: 62121,
            entry_id: 80180,
            rank: 1,
        }],
        unspent: vec![(2801, 31), (2800, 30)],
    });
    editor
}
#[test]
fn talents_snapshot_projects_ranks_and_points() {
    let view = view();
    let mut editor = editor(&view);
    assert_eq!(editor.rank(62121, 80180), 1);
    assert_eq!(editor.unspent(&view), vec![(2801, 31), (2800, 30)]);
    assert!(!editor.dirty());
    let mut snapshot = editor.snapshot.clone().unwrap();
    snapshot.entries.push(TraitEntrySelection {
        node_id: 62122,
        entry_id: 80181,
        rank: 1,
    });
    snapshot.unspent = vec![(2801, 30), (2800, 30)];
    editor.receive_snapshot(snapshot);
    assert_eq!(editor.rank(62122, 80181), 1);
    assert_eq!(editor.unspent(&view), vec![(2801, 30), (2800, 30)]);
    assert!(!editor.dirty());
}
#[test]
fn talents_click_refund_and_undo_stage_without_mutating_committed() {
    let view = view();
    let mut editor = editor(&view);
    assert!(editor.purchase(&view, 80, 62122, 80181));
    assert_eq!(editor.rank(62122, 80181), 1);
    assert_eq!(editor.unspent(&view), vec![(2801, 30), (2800, 30)]);
    assert!(!editor.purchase(&view, 80, 62122, 80181));
    assert!(editor.dirty());
    assert!(editor.refund(&view, 80, 62122));
    assert_eq!(editor.unspent(&view), vec![(2801, 31), (2800, 30)]);
    assert!(!editor.dirty());
    assert!(editor.purchase(&view, 80, 62122, 80181));
    editor.undo();
    assert_eq!(editor.rank(62122, 80181), 0);
    assert!(!editor.dirty());
    editor.action(&view, 80, "talent:node:62122").unwrap();
    assert!(editor.dirty());
    editor.action(&view, 80, "talent:reset").unwrap();
    assert!(!editor.dirty());
}
#[test]
fn talents_impossible_clicks_do_nothing() {
    let view = view();
    let mut editor = editor(&view);
    assert!(!editor.purchase(&view, 80, 62087, 80143)); // Ice Nova needs the upper path.
    assert!(!editor.refund(&view, 80, 62121)); // granted.
    assert!(!editor.purchase(&view, 80, 999999, 1));
    assert!(!editor.dirty());
    let mut snapshot = editor.snapshot.clone().unwrap();
    snapshot.unspent = vec![(2801, 0), (2800, 0)];
    editor.receive_snapshot(snapshot);
    assert!(!editor.purchase(&view, 80, 62122, 80181));
}
#[test]
fn talents_apply_serializes_full_entry_list() {
    let view = view();
    let mut editor = editor(&view);
    assert!(editor.purchase(&view, 80, 62122, 80181));
    assert_eq!(
        editor.action(&view, 80, "talent:apply").unwrap(),
        Some(CommitTraitConfig {
            spec_id: 62,
            entries: vec![
                TraitEntrySelection {
                    node_id: 62121,
                    entry_id: 80180,
                    rank: 1
                },
                TraitEntrySelection {
                    node_id: 62122,
                    entry_id: 80181,
                    rank: 1
                }
            ]
        })
    );
}
#[test]
fn talents_failure_reason_and_new_snapshot_clear_pending() {
    let view = view();
    let mut editor = editor(&view);
    editor.receive_result(TraitCommitResult {
        ok: false,
        reason: Some("requires eight points".into()),
    });
    assert_eq!(editor.error_text.as_deref(), Some("requires eight points"));
    assert!(editor.purchase(&view, 80, 62122, 80181));
    let hero = view.graph.hero_selection.as_ref().unwrap();
    assert!(editor.click(&view, 80, hero.id, false));
    editor.receive_result(TraitCommitResult {
        ok: false,
        reason: Some("requires eight points".into()),
    });
    let snapshot = editor.snapshot.clone().unwrap();
    editor.receive_snapshot(snapshot);
    assert!(!editor.dirty());
    assert_eq!(editor.rank(62122, 80181), 0);
    assert!(editor.error_text.is_none());
    assert!(editor.choice_node.is_none());
}

#[test]
fn talents_choice_flyout_selects_one_entry_and_switches() {
    let view = view();
    let mut editor = editor(&view);
    let node = view.graph.hero_selection.as_ref().unwrap();
    assert!(editor.click(&view, 80, node.id, false));
    assert_eq!(editor.choice_node, Some(node.id));
    assert_eq!(editor.node_rank(node.id), 0);
    assert!(editor.purchase(&view, 80, node.id, node.entries[0].id));
    assert_eq!(editor.rank(node.id, node.entries[0].id), 1);
    assert!(editor.choice_node.is_none());
    assert!(editor.click(&view, 80, node.id, false));
    assert!(editor.purchase(&view, 80, node.id, node.entries[1].id));
    assert_eq!(editor.rank(node.id, node.entries[0].id), 0);
    assert_eq!(editor.rank(node.id, node.entries[1].id), 1);
    editor.undo();
    assert_eq!(editor.node_rank(node.id), 0);
}
#[test]
fn talents_gate_and_hero_level_reject_unavailable_purchases() {
    let view = view();
    let mut editor = editor(&view);
    let hero = view.graph.hero_selection.as_ref().unwrap();
    assert!(!editor.click(&view, 70, hero.id, false));
    assert!(!editor.purchase(&view, 80, 110420, 137028));
    assert!(!editor.purchase(
        &view,
        80,
        view.graph.heroes[0].nodes[0].id,
        view.graph.heroes[0].nodes[0].entries[0].id
    ));
    assert!(!editor.dirty());
}

#[test]
fn talents_tiered_refund_uses_db2_entry_order_not_sorted_wire_ids() {
    let mut view = view();
    let node = view
        .rules
        .as_mut()
        .unwrap()
        .nodes
        .iter_mut()
        .find(|node| node.id == 110420)
        .unwrap();
    // Isolate the tier boundary from the unrelated currency gate. These are the
    // real Arcane apex entries in DB2 order: 137028 (1),137027 (2),137026 (1).
    node.conds.clear();
    node.groups.clear();
    node.parents.clear();
    node.costs.clear();
    for entry in &mut node.entries {
        entry.conds.clear();
        entry.costs.clear();
    }
    let mut editor = editor(&view);
    let mut snapshot = editor.snapshot.clone().unwrap();
    snapshot.entries.extend([
        TraitEntrySelection {
            node_id: 110420,
            entry_id: 137028,
            rank: 1,
        },
        TraitEntrySelection {
            node_id: 110420,
            entry_id: 137027,
            rank: 1,
        },
    ]);
    editor.receive_snapshot(snapshot);
    assert!(editor.refund(&view, 80, 110420));
    assert_eq!(
        editor.rank(110420, 137028),
        1,
        "first tier must remain learned"
    );
    assert_eq!(
        editor.rank(110420, 137027),
        0,
        "refund the last active tier"
    );
    assert!(
        !editor.purchase(&view, 80, 110420, 137026),
        "later tier requires earlier tiers maxed"
    );
}
