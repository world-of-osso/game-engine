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
        unspent: vec![(1820, 31), (1819, 30)],
    });
    editor
}
#[test]
fn talents_snapshot_projects_ranks_and_points() {
    let view = view();
    let editor = editor(&view);
    assert_eq!(editor.rank(62121, 80180), 1);
    assert_eq!(editor.unspent(&view), vec![(1820, 31), (1819, 30)]);
    assert!(!editor.dirty());
}
#[test]
fn talents_click_refund_and_undo_stage_without_mutating_committed() {
    let view = view();
    let mut editor = editor(&view);
    assert!(editor.purchase(&view, 80, 62084, 80140));
    assert_eq!(editor.rank(62084, 80140), 1);
    assert!(editor.dirty());
    assert!(editor.refund(&view, 80, 62084));
    assert!(!editor.dirty());
    assert!(editor.purchase(&view, 80, 62084, 80140));
    editor.undo();
    assert_eq!(editor.rank(62084, 80140), 0);
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
    snapshot.unspent = vec![(1820, 0), (1819, 0)];
    editor.receive_snapshot(snapshot);
    assert!(!editor.purchase(&view, 80, 62084, 80140));
}
#[test]
fn talents_apply_serializes_full_entry_list() {
    let view = view();
    let mut editor = editor(&view);
    assert!(editor.purchase(&view, 80, 62084, 80140));
    assert_eq!(
        editor.apply(),
        Some(CommitTraitConfig {
            spec_id: 62,
            entries: vec![
                TraitEntrySelection {
                    node_id: 62084,
                    entry_id: 80140,
                    rank: 1
                },
                TraitEntrySelection {
                    node_id: 62121,
                    entry_id: 80180,
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
    assert!(editor.purchase(&view, 80, 62084, 80140));
    let snapshot = editor.snapshot.clone().unwrap();
    editor.receive_snapshot(snapshot);
    assert!(!editor.dirty());
    assert_eq!(editor.rank(62084, 80140), 0);
    assert!(editor.error_text.is_none());
    assert!(editor.choice_node.is_none());
}
