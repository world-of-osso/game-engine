//! Real UDP bridge proof: no client-authoritative Tender or collection grant fields.
use super::*;
use shared::protocol::{InstanceChannel, TradingPostRequest, TradingPostSnapshot};
#[test]
fn trading_post_udp_bridge_round_trips_authoritative_balance_and_requests() {
    #[derive(Resource, Default)]
    struct Requests(Vec<TradingPostRequest>);
    fn capture(
        mut receivers: Query<&mut MessageReceiver<TradingPostRequest>>,
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
        assert!(
            Instant::now() < deadline,
            "Trading Post connection timed out"
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
    let snapshot = TradingPostSnapshot {
        npc_net_id: Some(185468),
        tender: 100,
        points: 200,
        now_unix: 1790812800,
        month_end_unix: 1793491200,
        frozen_item_id: Some(4),
        categories: vec![],
        items: vec![],
        thresholds: vec![],
        source_gaps: vec!["Refund duration unsourced".into()],
        error: None,
        catalog_complete: true,
        reply_kind: shared::protocol::TradingPostReplyKind::Query,
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<TradingPostSnapshot>>()
        .single_mut(world)
        .unwrap()
        .send::<InstanceChannel>(snapshot.clone());
    let requests = [
        TradingPostRequest::Query,
        TradingPostRequest::Freeze {
            vendor_item_id: Some(4),
        },
        TradingPostRequest::Purchase { vendor_item_id: 4 },
        TradingPostRequest::Refund { vendor_item_id: 4 },
        TradingPostRequest::Balance,
    ];
    for request in &requests {
        bridge.send::<_, InstanceChannel>(request.clone()).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut received = None;
    while (received.is_none() || server.world().resource::<Requests>().0.len() < requests.len())
        && Instant::now() < deadline
    {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event
                && message.is::<TradingPostSnapshot>()
            {
                received = message.downcast::<TradingPostSnapshot>().ok();
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(received, Some(snapshot));
    assert_eq!(server.world().resource::<Requests>().0, requests);
}
