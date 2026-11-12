//! Calendar snapshot transport proof over the default native client bridge.
use super::*;
use shared::protocol::{
    CalendarChannel, CalendarEventSnapshot, CalendarSignupSnapshot, CalendarSignupStatusSnapshot,
    CalendarSnapshot, CalendarStateUpdate,
};

#[test]
fn native_default_bridge_receives_server_calendar_snapshot_over_real_udp() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(|_| {});
    let mut bridge = NetworkBridge::connect(address, 6941).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "calendar connection timeout");
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

    let expected = CalendarStateUpdate {
        snapshot: Some(CalendarSnapshot {
            events: vec![CalendarEventSnapshot {
                event_id: 42,
                title: "Server calendar event".into(),
                organizer_name: "FixturePlayer".into(),
                starts_at_unix_secs: 1_799_000_100,
                max_signups: 5,
                is_raid: true,
                signups: vec![CalendarSignupSnapshot {
                    character_name: "FixturePlayer".into(),
                    status: CalendarSignupStatusSnapshot::Confirmed,
                }],
            }],
        }),
        message: Some("calendar loaded".into()),
        error: None,
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<CalendarStateUpdate>>()
        .single_mut(world)
        .unwrap()
        .send::<CalendarChannel>(expected.clone());

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut received = None;
    while received.is_none() && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event
                && message.is::<CalendarStateUpdate>()
            {
                received = Some(message.downcast::<CalendarStateUpdate>().ok().unwrap());
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(
        received,
        Some(expected),
        "default native client silently loses CalendarStateUpdate"
    );
}
