use super::*;
use shared::protocol::{GroupRoleSnapshot, LfgChannel, LfgStateUpdate, QueueForLfg};

#[derive(Resource, Default)]
struct FollowerRequests(Vec<QueueForLfg>);

fn install(app: &mut App) {
    app.init_resource::<FollowerRequests>().add_systems(
        Update,
        |mut receivers: Query<&mut MessageReceiver<QueueForLfg>>,
         mut requests: ResMut<FollowerRequests>| {
            for mut receiver in &mut receivers {
                requests.0.extend(receiver.receive());
            }
        },
    );
}

#[test]
fn follower_wire_sends_selected_role_and_receives_authoritative_lfg_rejection() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 5750261).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            Instant::now() < deadline,
            "follower network connection timeout"
        );
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
    let request = QueueForLfg {
        role: GroupRoleSnapshot::Healer,
        dungeon_ids: vec![2436],
    };
    bridge.send::<_, LfgChannel>(request.clone()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<FollowerRequests>().0.is_empty() && Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        server.world().resource::<FollowerRequests>().0,
        vec![request]
    );
    let refusal = LfgStateUpdate {
        snapshot: None,
        message: None,
        error: Some("follower dungeon admission refused".into()),
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<LfgStateUpdate>>()
        .single_mut(world)
        .unwrap()
        .send::<LfgChannel>(refusal.clone());
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut received = None;
    while received.is_none() && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event {
                if message.is::<LfgStateUpdate>() {
                    received = Some(message.downcast::<LfgStateUpdate>().ok().unwrap());
                }
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(received, Some(refusal));
    bridge.stop().unwrap();
}
