//! Full-catalog fragmentation followed by a freeze delta on the live ordered channel.
use super::*;
use shared::protocol::{InstanceChannel, TradingPostItem, TradingPostSnapshot};

#[test]
fn trading_post_catalog_then_freeze_delta_survive_slow_server_frames() {
    let (mut server, address) = crate::wire_tests::start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8391).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "connect deadline");
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
    let catalog = TradingPostSnapshot {
        npc_net_id: Some(185468),
        tender: 10000,
        points: 0,
        now_unix: 1790812800,
        month_end_unix: 1793491200,
        frozen_item_id: None,
        categories: vec![],
        items: (1..=1321)
            .map(|id| TradingPostItem {
                id,
                category_id: 1,
                item_id: 212369,
                name: format!("Battlefield Lover's Bow {id}"),
                description: "Sourced catalog description. ".repeat(10),
                icon_fdid: 5751509,
                quality: 3,
                usable: Some(true),
                price: 250,
                purchased: false,
                collected: false,
                refund_seconds: 0,
                ui_group_name: String::new(),
                ui_group_priority: 0,
                model_scene_id: 0,
                creature_display_id: 0,
                preview_item_ids: vec![212369],
                grant_unavailable: None,
            })
            .collect(),
        thresholds: vec![],
        source_gaps: vec![],
        error: None,
        catalog_complete: true,
    };
    let delta = TradingPostSnapshot {
        frozen_item_id: Some(575),
        items: vec![],
        catalog_complete: false,
        ..catalog.clone()
    };
    let mut received = Vec::new();
    for expected in [&catalog, &delta] {
        let world = server.world_mut();
        world
            .query::<&mut MessageSender<TradingPostSnapshot>>()
            .single_mut(world)
            .unwrap()
            .send::<InstanceChannel>(expected.clone());
        let deadline = Instant::now() + Duration::from_secs(30);
        while received.len() < if expected.catalog_complete { 1 } else { 2 }
            && Instant::now() < deadline
        {
            server.update();
            thread::sleep(Duration::from_millis(500));
            for event in bridge.drain_events().unwrap() {
                if let Event::Message(message) = event
                    && message.is::<TradingPostSnapshot>()
                {
                    received.push(message.downcast::<TradingPostSnapshot>().ok().unwrap());
                }
            }
        }
    }
    bridge.stop().unwrap();
    assert_eq!(received, vec![catalog, delta]);
}
