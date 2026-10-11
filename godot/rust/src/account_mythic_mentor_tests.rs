use super::*;
use shared::protocol::{PartyKeystoneMember, PartyKeystoneSnapshot};

#[test]
fn mythic_mentor_account_retains_loading_snapshot_and_clears_on_connection_change() {
    let mut account = Account::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"));
    account.session.screen = SessionScreen::Loading;
    let summary = PartyKeystoneSnapshot {
        members: vec![PartyKeystoneMember {
            character_id: 30,
            name: "Mentorone".into(),
            class: 2,
            is_leader: true,
            key: None,
        }],
    };
    account
        .dispatch_message(ProtocolMessage::for_tests(summary.clone()), &mut Vec::new())
        .unwrap();
    assert_eq!(account.party_keystones, summary);
    account.session.screen = SessionScreen::InWorld;
    assert_eq!(account.party_keystones.members[0].name, "Mentorone");
    account.prepare_connection("127.0.0.1:5811").unwrap();
    assert!(
        account.party_keystones.members.is_empty(),
        "A new transport must not expose the previous character's keys"
    );
}
