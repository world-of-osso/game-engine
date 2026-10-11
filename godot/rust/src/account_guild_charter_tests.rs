use super::*;
#[test]
fn guild_charter_native_account_delivers_authoritative_registrar() {
    let mut account = Account::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"));
    let mut events = Vec::new();
    let reply =
        shared::protocol::GuildCharterReply::Registrar(shared::protocol::GuildRegistrarSnapshot {
            npc: 810,
            npc_name: "Aldwin".into(),
            charter_cost: 1000,
            charter_guid: None,
        });
    account
        .dispatch_message(ProtocolMessage::for_tests(reply.clone()), &mut events)
        .unwrap();
    assert_eq!(
        events.len(),
        1,
        "native account discarded charter authority"
    );
    assert!(matches!(&events[0], AccountEvent::GuildCharter(actual) if *actual == reply));
}
