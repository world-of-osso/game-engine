//! Native renown acknowledgement and existing reputation-error feedback boundary.
use super::*;

#[test]
fn renown_watch_refusal_updates_authoritative_cache_and_reports_world_error() {
    let mut account = Account::new(PathBuf::new());
    account.session.screen = SessionScreen::InWorld;
    let reply = shared::protocol::RenownSnapshot {
        factions: vec![],
        source_error: None,
        watched_faction: Some(72),
        watch_error: Some("Faction unlock condition is not evaluated by this realm.".into()),
    };
    let mut events = Vec::new();
    account
        .dispatch_message(ProtocolMessage::for_tests(reply.clone()), &mut events)
        .unwrap();
    assert_eq!(account.renown.as_deref(), Some(&reply));
    assert!(matches!(&events[..], [AccountEvent::Reputation(update)]
        if update.snapshot.is_none() && update.error == reply.watch_error));
}
