use super::*;
use shared::protocol::{PartyKeystoneMember, PartyKeystoneSnapshot};

#[test]
fn mythic_mentor_shared_teleport_category_replaces_clears_and_expires() {
    let mut account = Account::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"));
    let snapshot = PartyKeystoneSnapshot {
        members: vec![PartyKeystoneMember {
            character_id: 30,
            name: "Mentorone".into(),
            class: 2,
            is_leader: true,
            key: Some(shared::protocol::PartyKeystone {
                challenge_id: 399,
                level: 8,
                dungeon_name: "Ruby Life Pools".into(),
                time_limit_ms: 1_680_000,
            }),
        }],
    };
    account
        .dispatch_message(
            ProtocolMessage::for_tests(snapshot.clone()),
            &mut Vec::new(),
        )
        .unwrap();
    // Real Path of the Scarlet Mitre shares DB2 category1407 with current panel teleports.
    let cooldown = shared::protocol::SpellCooldownUpdate {
        spell_id: 131229,
        category: 1407,
        duration_ms: 28_800_000,
        remaining_ms: 28_800_000,
        is_gcd: false,
    };
    account
        .dispatch_message(
            ProtocolMessage::for_tests(cooldown.clone()),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(
        crate::mythic_mentor::mentor_cooldowns(&account.party_keystones, &account.spells)
            .get(&393256)
            .map(String::as_str),
        Some("480m")
    );
    let clear = shared::protocol::SpellCooldownUpdate {
        spell_id: 131229,
        category: 1407,
        duration_ms: 28_800_000,
        remaining_ms: 0,
        is_gcd: false,
    };
    account
        .dispatch_message(ProtocolMessage::for_tests(clear), &mut Vec::new())
        .unwrap();
    assert!(
        crate::mythic_mentor::mentor_cooldowns(&account.party_keystones, &account.spells)
            .is_empty()
    );
    account
        .dispatch_message(ProtocolMessage::for_tests(cooldown), &mut Vec::new())
        .unwrap();
    account.spells.tick(28_800.0);
    assert!(
        crate::mythic_mentor::mentor_cooldowns(&account.party_keystones, &account.spells)
            .is_empty()
    );
}

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
    account.stop().unwrap();
    assert!(
        account.party_keystones.members.is_empty(),
        "Logout's transport stop must discard old character rows"
    );
    account.prepare_connection("127.0.0.1:5811").unwrap();
    assert!(
        account.party_keystones.members.is_empty(),
        "A new transport must not expose the previous character's keys"
    );
}
