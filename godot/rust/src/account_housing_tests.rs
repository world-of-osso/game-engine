use super::*;
use shared::protocol::{HousingReply, HousingState, StoredDecor};

#[test]
fn housing_wire_dispatch_keeps_owned_storage_and_refusal_without_inventing_grants() {
    let mut account = Account::new(PathBuf::from("/unused-housing-data"));
    let reply = HousingReply {
        request_id: 4,
        state: HousingState {
            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
            stored: vec![StoredDecor {
                decor_id: 1693,
                quantity: 1,
            }],
            ..Default::default()
        },
        error: Some("No verified interior grant source is available.".into()),
    };
    let mut events = vec![];
    account
        .dispatch_message(ProtocolMessage::for_tests(reply.clone()), &mut events)
        .unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], AccountEvent::Housing(received) if received == &reply));
}
