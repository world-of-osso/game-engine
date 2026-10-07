//! Owner profession snapshots use the same native Account dispatch as world entry.
use super::*;
use shared::profession::ProfessionSkillLine;
use shared::protocol::ProfessionSnapshot;

#[test]
fn professions_account_dispatches_loading_snapshot_and_skill_refresh() {
    let mut account = Account::new(PathBuf::new());
    account.session.screen = SessionScreen::Loading;
    let mut snapshot = ProfessionSnapshot {
        lines: vec![ProfessionSkillLine {
            skill_line: 2540,
            step: 1,
            rank: 1,
            max_rank: 300,
        }],
        spells: vec![3908, 264616, 3275],
    };
    for rank in [1, 2] {
        snapshot.lines[0].rank = rank;
        let mut events = Vec::new();
        account
            .dispatch_message(ProtocolMessage::for_tests(snapshot.clone()), &mut events)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AccountEvent::Professions(received) if received == &snapshot));
    }
}
