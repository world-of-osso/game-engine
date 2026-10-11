//! Group-loot snapshots and choices over the real private loopback UDP boundary.
use super::*;
use shared::loot::{LootRollChoice, LootRollEntry};
use shared::protocol::{GroupLootChoice, GroupLootPlayer, GroupLootRoll, LootChannel, LootContent};

#[derive(Resource, Default)]
struct Choices(Vec<GroupLootChoice>);

fn capture(
    mut receivers: Query<&mut MessageReceiver<GroupLootChoice>>,
    mut choices: ResMut<Choices>,
) {
    for mut receiver in &mut receivers {
        choices.0.extend(receiver.receive());
    }
}

fn install(app: &mut App) {
    app.init_resource::<Choices>().add_systems(Update, capture);
}

#[test]
fn group_loot_bridge_delivers_ordered_snapshots_and_authoritative_choice() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 8351).expect("group loot bridge");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "group loot connect timed out");
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
    let initial = GroupLootRoll {
        roll_id: 19,
        corpse: 77,
        item: LootContent::Item {
            item_id: 1206,
            name: "Moss Agate".into(),
            quality: 2,
            count: 1,
        },
        seconds_remaining: 60.0,
        players: vec![GroupLootPlayer {
            character_id: 11,
            name: "Alice".into(),
            can_need: true,
            need_reason: None,
            can_transmog: false,
            response: None,
        }],
        winner: None,
        complete: false,
        delivered: false,
    };
    let mut finished = initial.clone();
    finished.players[0].response = Some(LootRollEntry {
        player_id: 11,
        choice: LootRollChoice::Need,
        roll_value: 42,
    });
    finished.seconds_remaining = 0.0;
    finished.winner = Some(11);
    finished.complete = true;
    finished.delivered = true;
    {
        let world = server.world_mut();
        let mut query = world.query::<&mut MessageSender<GroupLootRoll>>();
        let mut sender = query.single_mut(world).unwrap();
        sender.send::<LootChannel>(initial.clone());
        sender.send::<LootChannel>(finished.clone());
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut snapshots = Vec::new();
    while snapshots.len() < 2 && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event
                && message.is::<GroupLootRoll>()
            {
                snapshots.push(
                    message
                        .downcast::<GroupLootRoll>()
                        .ok()
                        .expect("decoded group roll"),
                );
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    let choice = GroupLootChoice {
        roll_id: 19,
        choice: LootRollChoice::Need,
    };
    bridge.send::<_, LootChannel>(choice).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while server.world().resource::<Choices>().0.is_empty() && Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(snapshots, [initial, finished]);
    assert_eq!(server.world().resource::<Choices>().0, [choice]);
}
