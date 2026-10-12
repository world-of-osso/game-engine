//! Guide query/tracking and unavailable-source snapshot across the real UDP bridge.
use super::*;
use shared::protocol::{
    AdventureGuideRequest, AdventureGuideSnapshot, InstanceChannel, JournalMonth, SuggestionCensus,
};

#[test]
fn adventure_guide_bridge_round_trips_query_tracking_and_source_error() {
    #[derive(Resource, Default)]
    struct Requests(Vec<AdventureGuideRequest>);
    fn capture(
        mut receivers: Query<&mut MessageReceiver<AdventureGuideRequest>>,
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
    let mut bridge = NetworkBridge::connect(address, 8315).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "Guide connection timed out");
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
    let snapshot = AdventureGuideSnapshot {
        month: JournalMonth {
            year: 2026,
            month: 10,
            name: "October".into(),
            start_unix: 1790838000,
            end_unix: 1793516400,
            now_unix: 1791759600,
            tracking_since: 1791759600,
        },
        activities: vec![],
        tags: vec![],
        suggestions: vec![],
        suggestion_census: SuggestionCensus::default(),
        points: 0,
        activity_scope_notice: None,
        source_error: Some("Pinned Adventure Guide catalog not installed".into()),
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<AdventureGuideSnapshot>>()
        .single_mut(world)
        .unwrap()
        .send::<InstanceChannel>(snapshot.clone());
    let query = AdventureGuideRequest::Query;
    let tracking = AdventureGuideRequest::TrackActivity {
        activity_id: 3,
        tracked: true,
    };
    bridge.send::<_, InstanceChannel>(query.clone()).unwrap();
    bridge.send::<_, InstanceChannel>(tracking.clone()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut received = None;
    while (received.is_none() || server.world().resource::<Requests>().0.len() < 2)
        && Instant::now() < deadline
    {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event
                && message.is::<AdventureGuideSnapshot>()
            {
                received = message.downcast::<AdventureGuideSnapshot>().ok();
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(received, Some(snapshot));
    assert_eq!(
        server.world().resource::<Requests>().0,
        vec![query, tracking]
    );
}
