//! Journeys renown query and catalog-sized snapshot across the real UDP bridge.
use super::*;
use shared::protocol::{
    InstanceChannel, MajorFactionSnapshot, RenownRequest, RenownReward, RenownSnapshot,
    RenownUnlock,
};

/// The pinned catalog has 29 major factions and 872 rewards; this keeps that scale.
fn catalog_sized_snapshot() -> RenownSnapshot {
    let reward = |level: u32| RenownReward {
        id: 621 + level,
        level,
        ui_order: 0,
        flags: 8,
        icon: 3012071,
        name: "Among the Maruuk Centaur".into(),
        description: "You have joined forces with the Maruuk Centaurs on the Ohn'ahran Plains."
            .into(),
        toast: "Maruuk Centaur Renown Unlocked".into(),
        item_id: 0,
        spell_id: 0,
        mount_id: 0,
        transmog_id: 0,
        transmog_set_id: 0,
        transmog_illusion_id: 0,
        title_id: 0,
        quest_id: 0,
        player_condition_id: 0,
    };
    let factions = (12..41)
        .map(|id| MajorFactionSnapshot {
            id,
            faction_id: 2503,
            name: "Maruuk Centaur".into(),
            description: String::new(),
            expansion: 9,
            max_level: 25,
            level_threshold: 2500,
            level: 2,
            standing: 1000,
            unlock: RenownUnlock::Unsupported {
                condition_id: 104408,
                description:
                    "Complete the quest Maruukai with the centaur caravan in the Ohn'ahran Plains."
                        .into(),
            },
            rewards: (1..31).map(reward).collect(),
        })
        .collect();
    RenownSnapshot {
        factions,
        source_error: None,
        watched_faction: Some(2503),
        watch_error: None,
    }
}

#[test]
fn renown_bridge_round_trips_query_and_catalog_sized_snapshot() {
    #[derive(Resource, Default)]
    struct Requests(Vec<RenownRequest>);
    fn capture(
        mut receivers: Query<&mut MessageReceiver<RenownRequest>>,
        mut requests: ResMut<Requests>,
    ) {
        for mut receiver in &mut receivers {
            requests.0.extend(receiver.receive());
        }
    }
    fn install(app: &mut App) {
        app.init_resource::<Requests>().add_systems(Update, capture);
    }
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 8316).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "Renown connection timed out");
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
    let snapshot = catalog_sized_snapshot();
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<RenownSnapshot>>()
        .single_mut(world)
        .unwrap()
        .send::<InstanceChannel>(snapshot.clone());
    bridge
        .send::<_, InstanceChannel>(RenownRequest::Query)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut received = None;
    while (received.is_none() || server.world().resource::<Requests>().0.is_empty())
        && Instant::now() < deadline
    {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event
                && message.is::<RenownSnapshot>()
            {
                received = message.downcast::<RenownSnapshot>().ok();
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(received, Some(snapshot));
    assert_eq!(
        server.world().resource::<Requests>().0,
        vec![RenownRequest::Query]
    );
}
