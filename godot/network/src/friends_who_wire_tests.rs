//! The default native bridge must relay Friends and Who responses from their channels.
use super::*;
use shared::components::PresenceStatus;
use shared::protocol::{
    FriendCharacterSnapshot, FriendsChannel, FriendsSnapshot, FriendsStateUpdate, WhoChannel,
    WhoCharacterSnapshot, WhoSnapshot, WhoStateUpdate,
};

#[test]
fn native_default_bridge_receives_friends_and_who_updates_over_real_udp() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(|_| {});
    let mut bridge = NetworkBridge::connect(address, 5943).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "social connection timeout");
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

    let expected_friends = FriendsStateUpdate {
        snapshot: Some(FriendsSnapshot {
            entries: vec![FriendCharacterSnapshot {
                name: "Friendbeta".into(),
                level: 80,
                class_name: "Mage".into(),
                area: "Stormwind".into(),
                presence: PresenceStatus::Online,
                note: "Test friend".into(),
            }],
        }),
        message: Some("friends refreshed".into()),
        error: None,
    };
    let expected_who = WhoStateUpdate {
        snapshot: Some(WhoSnapshot {
            query: "Friendbeta".into(),
            entries: vec![WhoCharacterSnapshot {
                name: "Friendbeta".into(),
                level: 80,
                class_name: "Mage".into(),
                area: "Stormwind".into(),
            }],
        }),
        message: Some("who: 1 result(s)".into()),
        error: None,
    };

    let world = server.world_mut();
    world
        .query::<&mut MessageSender<FriendsStateUpdate>>()
        .single_mut(world)
        .unwrap()
        .send::<FriendsChannel>(expected_friends.clone());
    world
        .query::<&mut MessageSender<WhoStateUpdate>>()
        .single_mut(world)
        .unwrap()
        .send::<WhoChannel>(expected_who.clone());

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut received_friends = None;
    let mut received_who = None;
    while (received_friends.is_none() || received_who.is_none()) && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            let Event::Message(message) = event else {
                continue;
            };
            if message.is::<FriendsStateUpdate>() {
                received_friends = message.downcast::<FriendsStateUpdate>().ok();
            } else if message.is::<WhoStateUpdate>() {
                received_who = message.downcast::<WhoStateUpdate>().ok();
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();

    assert_eq!(received_friends, Some(expected_friends));
    assert_eq!(received_who, Some(expected_who));
}
