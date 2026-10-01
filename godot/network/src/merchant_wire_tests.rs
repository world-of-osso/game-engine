//! MerchantChannel ordering through the real loopback UDP/native bridge boundary.
use super::*;
use shared::protocol::{BuybackItem, MerchantChannel, MerchantError, VendorItem};

#[test]
fn native_bridge_receives_merchant_messages_in_channel_order() {
    let (mut server, address) = crate::wire_tests::start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8241).expect("start merchant bridge");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "merchant connection timed out");
        server.update();
        if bridge
            .drain_events()
            .expect("poll merchant connection")
            .iter()
            .any(|event| matches!(event, Event::Connected))
        {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    // Hold the receiving worker so different message types reach one relay frame.
    let (held, hold_confirmed) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    bridge
        .enqueue(move |_| {
            held.send(()).expect("confirm merchant worker hold");
            resumed
                .recv_timeout(Duration::from_secs(10))
                .expect("release merchant worker hold");
        })
        .expect("hold merchant worker");
    hold_confirmed
        .recv_timeout(Duration::from_secs(10))
        .expect("merchant worker entered hold");

    let inventory = VendorInventory {
        npc: 4294966979,
        can_repair: true,
        items: vec![VendorItem {
            slot: 0,
            item_id: 2589,
            name: "Linen Cloth".into(),
            quality: 1,
            price: 20,
            stack_count: 1,
            max_stack: 200,
            num_available: Some(3),
            usable: true,
        }],
    };
    let buyback = BuybackList {
        items: vec![BuybackItem {
            slot: 0,
            item_id: 4865,
            name: "Ruined Pelt".into(),
            quality: 0,
            count: 2,
            price: 10,
        }],
    };
    let failed = MerchantFailed {
        npc: inventory.npc,
        error: MerchantError::SoldOut,
    };
    let updated_inventory = VendorInventory {
        items: vec![VendorItem {
            num_available: Some(2),
            ..inventory.items[0].clone()
        }],
        ..inventory.clone()
    };
    let empty_buyback = BuybackList::default();
    // MessageSender buffers per type. Update after each send establishes actual
    // channel send order, rather than assuming cross-type enqueue order does so.
    macro_rules! send_and_flush {
        ($ty:ty, $value:expr) => {{
            let world = server.world_mut();
            world
                .query::<&mut MessageSender<$ty>>()
                .single_mut(world)
                .expect("one connected merchant sender")
                .send::<MerchantChannel>($value.clone());
            server.update();
        }};
    }
    send_and_flush!(VendorInventory, inventory);
    send_and_flush!(BuybackList, buyback);
    send_and_flush!(MerchantFailed, failed);
    send_and_flush!(VendorInventory, updated_inventory);
    send_and_flush!(BuybackList, empty_buyback);
    let flush_deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < flush_deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    resume.send(()).expect("resume merchant worker");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut messages = Vec::new();
    while messages.len() < 5 && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().expect("poll merchant replies") {
            match event {
                Event::Message(message) => messages.push(message),
                Event::Disconnected(reason) => panic!("merchant disconnected: {reason:?}"),
                Event::ProtocolRejected(reason) => panic!("merchant protocol rejected: {reason}"),
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().expect("join merchant worker");
    assert_eq!(messages.len(), 5, "all five merchant replies delivered");
    let mut received = messages.into_iter();
    macro_rules! assert_next {
        ($ty:ty, $expected:expr) => {
            assert_eq!(
                received.next().unwrap().downcast::<$ty>().ok(),
                Some($expected)
            );
        };
    }
    assert_next!(VendorInventory, inventory);
    assert_next!(BuybackList, buyback);
    assert_next!(MerchantFailed, failed);
    assert_next!(VendorInventory, updated_inventory);
    assert_next!(BuybackList, empty_buyback);
}
