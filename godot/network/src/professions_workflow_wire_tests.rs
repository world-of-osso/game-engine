//! New profession operations cross the real ephemeral-UDP NetworkBridge boundary.
use super::*;
use shared::protocol::*;

#[derive(Resource, Default)]
struct Requests {
    specializations: Vec<ProfessionSpecializationRequest>,
    orders: Vec<CraftingOrderRequest>,
}
fn capture(
    mut specializations: Query<&mut MessageReceiver<ProfessionSpecializationRequest>>,
    mut orders: Query<&mut MessageReceiver<CraftingOrderRequest>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut specializations {
        requests.specializations.extend(receiver.receive());
    }
    for mut receiver in &mut orders {
        requests.orders.extend(receiver.receive());
    }
}
fn install(app: &mut App) {
    app.init_resource::<Requests>();
    app.add_systems(Update, capture);
}
fn connect(server: &mut App, address: std::net::SocketAddr) -> NetworkBridge {
    let mut bridge = NetworkBridge::connect(address, 8321).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            Instant::now() < deadline,
            "crafting workflow connection timed out"
        );
        server.update();
        if bridge
            .drain_events()
            .unwrap()
            .iter()
            .any(|event| matches!(event, Event::Connected))
        {
            return bridge;
        }
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn professions_workflow_bridge_delivers_catalog_and_errors_and_sends_absolute_commit() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = connect(&mut server, address);
    let specialization = ProfessionSpecializationStateUpdate {
        snapshot: Some(ProfessionSpecializationsSnapshot {
            skill_line: 2830,
            knowledge_currency_id: 2025,
            knowledge_available: 1,
            currencies: vec![ProfessionSpecCurrencySnapshot {
                trait_currency_id: 9,
                currency_id: 2025,
                available: 1,
            }],
            trees: vec![],
        }),
        message: None,
        error: None,
    };
    let orders = CraftingOrderStateUpdate {
        snapshot: Some(CraftingOrdersSnapshot {
            skill_line: 0,
            kind: CraftingOrderKind::Public,
            mine: true,
            recipes: vec![],
            orders: vec![],
        }),
        message: Some("Search complete".into()),
        error: None,
    };
    let refusal = CraftingOrderStateUpdate {
        snapshot: None,
        message: None,
        error: Some("Insufficient reagents".into()),
    };
    {
        let world = server.world_mut();
        world
            .query::<&mut MessageSender<ProfessionSpecializationStateUpdate>>()
            .single_mut(world)
            .unwrap()
            .send::<ProfessionChannel>(specialization.clone());
        let mut sender = world
            .query::<&mut MessageSender<CraftingOrderStateUpdate>>()
            .single_mut(world)
            .unwrap();
        sender.send::<ProfessionChannel>(orders.clone());
        sender.send::<ProfessionChannel>(refusal.clone());
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut received_specialization = None;
    let mut received_orders = Vec::new();
    while Instant::now() < deadline
        && (received_specialization.is_none() || received_orders.len() < 2)
    {
        server.update();
        for event in bridge.drain_events().unwrap() {
            let Event::Message(message) = event else {
                continue;
            };
            if message.is::<ProfessionSpecializationStateUpdate>() {
                received_specialization = message.downcast().ok();
            } else if message.is::<CraftingOrderStateUpdate>() {
                received_orders.push(
                    message
                        .downcast::<CraftingOrderStateUpdate>()
                        .unwrap_or_else(|_| {
                            panic!("Typed crafting order update could not be decoded")
                        }),
                );
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    let commit = ProfessionSpecializationRequest::Commit {
        skill_line: 2830,
        tree_id: 350,
        purchases: vec![ProfessionTraitPurchase {
            node_id: 28546,
            entry_id: 36327,
            ranks: 1,
        }],
    };
    let query = CraftingOrderRequest::Query {
        skill_line: 0,
        kind: CraftingOrderKind::Public,
        mine: true,
    };
    bridge.send::<_, ProfessionChannel>(commit.clone()).unwrap();
    bridge.send::<_, ProfessionChannel>(query.clone()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        server.update();
        let requests = server.world().resource::<Requests>();
        if !requests.specializations.is_empty() && !requests.orders.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(received_specialization, Some(specialization));
    assert_eq!(received_orders, vec![orders, refusal]);
    let requests = server.world().resource::<Requests>();
    assert_eq!(requests.specializations, vec![commit]);
    assert_eq!(requests.orders, vec![query]);
}
