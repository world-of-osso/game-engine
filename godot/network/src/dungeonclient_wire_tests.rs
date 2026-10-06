//! Owned ephemeral UDP peer: client requests and ordered achievement reply traffic.
use super::*;
use shared::protocol::*;

#[derive(Resource, Default)]
struct Requests(Vec<QueryAchievementCatalog>);
fn install(app: &mut App) {
    app.init_resource::<Requests>();
    app.add_systems(
        Update,
        |mut receivers: Query<&mut MessageReceiver<QueryAchievementCatalog>>,
         mut requests: ResMut<Requests>| {
            for mut receiver in &mut receivers {
                requests.0.extend(receiver.receive());
            }
        },
    );
}
#[test]
fn dungeonclient_wire_requests_catalog_and_preserves_live_update_between_pages() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 6338261).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            Instant::now() < deadline,
            "dungeonclient connection timeout"
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
    let requests = vec![
        QueryAchievementCatalog::Categories { after_id: 0 },
        QueryAchievementCatalog::Categories { after_id: 14808 },
        QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 0,
        },
        QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 633,
        },
        QueryAchievementCatalog::Criteria {
            achievement_id: 633,
            after_id: 16,
        },
    ];
    for request in &requests {
        bridge.send::<_, AchievementChannel>(*request).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<Requests>().0.len() < requests.len()
        && Instant::now() < deadline
    {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(server.world().resource::<Requests>().0, requests);
    let first = AchievementCatalogPage::Categories {
        categories: vec![AchievementCategoryEntry {
            category_id: 14808,
            parent_id: -1,
            order_index: 0,
            name: "Classic".into(),
        }],
        next_id: None,
    };
    let update = AchievementStateUpdate {
        snapshot: None,
        completed: Some(shared::protocol::AchievementToastSnapshot {
            achievement_id: 633,
            name: "Stormwind Stockade".into(),
            points: 10,
        }),
        message: None,
        error: None,
    };
    let second = AchievementCatalogPage::Category {
        category_id: 14808,
        achievements: vec![],
        next_id: None,
    };
    {
        let world = server.world_mut();
        world
            .query::<&mut MessageSender<AchievementCatalogPage>>()
            .single_mut(world)
            .unwrap()
            .send::<AchievementChannel>(first.clone());
        world
            .query::<&mut MessageSender<AchievementStateUpdate>>()
            .single_mut(world)
            .unwrap()
            .send::<AchievementChannel>(update.clone());
        world
            .query::<&mut MessageSender<AchievementCatalogPage>>()
            .single_mut(world)
            .unwrap()
            .send::<AchievementChannel>(second.clone());
        world
            .query::<&mut MessageSender<DungeonProgress>>()
            .single_mut(world)
            .unwrap()
            .send::<InstanceChannel>(DungeonProgress {
                map_id: 34,
                instance_id: 17,
                difficulty_id: 1,
                encounters: vec![DungeonEncounterProgress {
                    encounter_id: 1,
                    name: "Hogger".into(),
                    defeated: true,
                    optional: None,
                    flags: 0,
                }],
            });
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut order = Vec::new();
    let mut pages = Vec::new();
    let mut updates = Vec::new();
    let mut dungeon = None;
    while (order.len() < 3 || dungeon.is_none()) && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            let Event::Message(message) = event else {
                continue;
            };
            if message.is::<AchievementCatalogPage>() {
                pages.push(message.downcast::<AchievementCatalogPage>().ok().unwrap());
                order.push("page");
            } else if message.is::<AchievementStateUpdate>() {
                updates.push(message.downcast::<AchievementStateUpdate>().ok().unwrap());
                order.push("live");
            } else if message.is::<DungeonProgress>() {
                dungeon = Some(message.downcast::<DungeonProgress>().ok().unwrap());
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(order, ["page", "live", "page"]);
    assert_eq!(pages, [first, second]);
    assert_eq!(updates, [update]);
    assert_eq!(dungeon.unwrap().encounters[0].name, "Hogger");
}
