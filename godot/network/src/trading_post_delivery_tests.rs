//! Full-catalog fragmentation followed by a freeze delta on the live ordered channel.
use super::*;
use shared::protocol::{InstanceChannel, TradingPostItem, TradingPostSnapshot};

#[test]
fn trading_post_udp_receiver_keeps_a_catalog_sized_datagram_burst() {
    use lightyear::link::LinkReceiveSystems;
    use lightyear::prelude::{Link, LinkStart, LocalAddr, PeerAddr, UdpIo};
    use std::{collections::BTreeSet, net::UdpSocket};
    #[derive(Resource, Default)]
    struct Received(BTreeSet<u32>);
    fn capture(mut links: Query<&mut Link>, mut received: ResMut<Received>) {
        for mut link in &mut links {
            while let Some(packet) = link.recv.pop() {
                received
                    .0
                    .insert(u32::from_le_bytes(packet[0..4].try_into().unwrap()));
            }
        }
    }
    let reservation = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);
    let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut client = client_app();
    client
        .init_resource::<Received>()
        .add_systems(PreUpdate, capture.after(LinkReceiveSystems::BufferToLink));
    let entity = client
        .world_mut()
        .spawn((
            LocalAddr(address),
            PeerAddr(sender.local_addr().unwrap()),
            UdpIo::default(),
        ))
        .id();
    client.finish();
    client.cleanup();
    client.world_mut().trigger(LinkStart { entity });
    client.update();
    // A real 614400-byte catalog burst arrives during a busy frame, before the next drain.
    // Production socket evidence:212992-byte receive budget,54k dropped datagrams in40s.
    for id in 0u32..512 {
        let mut packet = [0x5au8; 1200];
        packet[0..4].copy_from_slice(&id.to_le_bytes());
        assert_eq!(sender.send_to(&packet, address).unwrap(), packet.len());
    }
    client.update();
    assert_eq!(
        client.world().resource::<Received>().0,
        (0..512).collect(),
        "UDP must retain this observed legal catalog-sized burst until the next frame"
    );
}

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
        reply_kind: shared::protocol::TradingPostReplyKind::Query,
    };
    let delta = TradingPostSnapshot {
        frozen_item_id: Some(575),
        items: vec![],
        catalog_complete: false,
        reply_kind: shared::protocol::TradingPostReplyKind::Freeze,
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
