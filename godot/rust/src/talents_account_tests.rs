use super::*;
use shared::protocol::{TraitCommitResult, TraitConfigSnapshot, TraitEntrySelection};

#[test]
fn talents_account_dispatches_loading_snapshot_and_failure_to_error_text() {
    let mut account = Account::new(PathBuf::new());
    account.session.screen = SessionScreen::Loading;
    let snapshot = TraitConfigSnapshot {
        spec_id: 62,
        tree_id: 658,
        entries: vec![TraitEntrySelection {
            node_id: 62121,
            entry_id: 80180,
            rank: 1,
        }],
        unspent: vec![(2801, 31), (2800, 30)],
    };
    let mut events = Vec::new();
    account
        .dispatch_message(ProtocolMessage::for_tests(snapshot.clone()), &mut events)
        .unwrap();
    assert!(events.is_empty());
    assert_eq!(account.talents.snapshot, Some(snapshot.clone()));
    assert_eq!(account.talents.rank(62121, 80180), 1);
    account
        .dispatch_message(
            ProtocolMessage::for_tests(TraitCommitResult {
                ok: false,
                reason: Some("requires eight points".into()),
            }),
            &mut events,
        )
        .unwrap();
    assert_eq!(
        account.talents.error_text.as_deref(),
        Some("requires eight points")
    );
    assert!(
        matches!(events.as_slice(),[AccountEvent::TalentError(reason)] if reason=="requires eight points")
    );
    events.clear();
    account
        .dispatch_message(ProtocolMessage::for_tests(snapshot), &mut events)
        .unwrap();
    assert!(account.talents.error_text.is_none());
    assert!(events.is_empty());
}

#[test]
fn talents_account_spec_change_clears_previous_configuration() {
    let mut account = Account::new(PathBuf::new());
    let mut events = Vec::new();
    account
        .dispatch_message(
            ProtocolMessage::for_tests(TraitConfigSnapshot {
                spec_id: 62,
                tree_id: 658,
                entries: Vec::new(),
                unspent: vec![(2801, 31), (2800, 30)],
            }),
            &mut events,
        )
        .unwrap();
    assert!(account.talents.snapshot.is_some());
    account
        .dispatch_message(
            ProtocolMessage::for_tests(SpecializationChanged { spec_id: 64 }),
            &mut events,
        )
        .unwrap();
    assert_eq!(account.spells.spec(), Some(64));
    assert!(account.talents.snapshot.is_none());
    assert!(!account.talents.dirty());
}
