//! The default native bridge must relay the realm's owner Currency channel.
use super::*;
use shared::protocol::{
    CurrencyChannel, CurrencyEntrySnapshot, CurrencySnapshot, CurrencyStateUpdate,
};

#[test]
fn native_default_bridge_receives_authoritative_currency_updates_over_real_udp() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(|_| {});
    let mut bridge = NetworkBridge::connect(address, 5941).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "currency connection timeout");
        server.update();
        if bridge
            .drain_events()
            .unwrap()
            .iter()
            .any(|event| matches!(event, Event::Connected))
        {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let expected = CurrencyStateUpdate {
        snapshot: Some(CurrencySnapshot {
            entries: vec![CurrencyEntrySnapshot {
                id: 1792,
                name: "Honor".into(),
                amount: 0,
            }],
        }),
        message: None,
        error: None,
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<CurrencyStateUpdate>>()
        .single_mut(world)
        .unwrap()
        .send::<CurrencyChannel>(expected.clone());
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut received = None;
    while received.is_none() && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event
                && message.is::<CurrencyStateUpdate>()
            {
                received = Some(message.downcast::<CurrencyStateUpdate>().ok().unwrap());
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(
        received,
        Some(expected),
        "default native client silently loses CurrencyStateUpdate"
    );
}
