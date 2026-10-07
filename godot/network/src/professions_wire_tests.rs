//! Profession book refresh and crafting through the real native UDP boundary.
use super::*;
use shared::profession::ProfessionSkillLine;
use shared::protocol::{CraftRecipe, ProfessionChannel, ProfessionSnapshot};

#[test]
fn professions_bridge_receives_snapshot_and_sends_craft() {
    #[derive(Resource, Default)]
    struct Requests(Vec<CraftRecipe>);
    fn capture(
        mut receivers: Query<&mut MessageReceiver<CraftRecipe>>,
        mut requests: ResMut<Requests>,
    ) {
        for mut receiver in &mut receivers {
            requests.0.extend(receiver.receive());
        }
    }
    fn install(app: &mut App) {
        app.init_resource::<Requests>();
        app.add_systems(Update, capture);
    }
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 8314).expect("profession bridge");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "profession connection timed out");
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
    let snapshot = ProfessionSnapshot {
        lines: vec![ProfessionSkillLine {
            skill_line: 2540,
            step: 1,
            rank: 1,
            max_rank: 300,
        }],
        spells: vec![3908, 264616, 3275],
    };
    {
        let world = server.world_mut();
        world
            .query::<&mut MessageSender<ProfessionSnapshot>>()
            .single_mut(world)
            .unwrap()
            .send::<ProfessionChannel>(snapshot.clone());
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut received = None;
    while received.is_none() && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event
                && message.is::<ProfessionSnapshot>()
            {
                received = message.downcast::<ProfessionSnapshot>().ok();
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge
        .send::<_, ProfessionChannel>(CraftRecipe {
            spell_id: 3275,
            casts: 2,
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while server.world().resource::<Requests>().0.is_empty() && Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(received, Some(snapshot));
    let requests = &server.world().resource::<Requests>().0;
    assert_eq!(
        requests,
        &[CraftRecipe {
            spell_id: 3275,
            casts: 2
        }]
    );
}
