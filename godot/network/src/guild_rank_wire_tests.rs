//! GuildChannel proof through real loopback UDP. Own ephemeral fixture, no dev server.
use super::*;
use shared::protocol::*;

#[derive(Resource, Default)]
struct Requests(Vec<GuildRankRequest>);
fn capture_requests(
    mut receivers: Query<&mut MessageReceiver<GuildRankRequest>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut receivers {
        requests.0.extend(receiver.receive());
    }
}
fn install(app: &mut App) {
    app.init_resource::<Requests>();
    app.add_systems(Update, capture_requests);
}
fn connect(server: &mut App, bridge: &mut NetworkBridge) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "guild connection timeout");
        server.update();
        if bridge
            .drain_events()
            .unwrap()
            .iter()
            .any(|event| matches!(event, Event::Connected))
        {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn guild_ranks_wire_sends_exact_requests_and_receives_ordered_authority() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 8261).unwrap();
    connect(&mut server, &mut bridge);
    let requests = vec![
        GuildRankRequest::Query,
        GuildRankRequest::Add {
            name: "Raider".into(),
        },
        GuildRankRequest::Rename {
            rank: 2,
            name: "Officer".into(),
        },
        GuildRankRequest::Move { rank: 2, up: true },
        GuildRankRequest::SetPermissions {
            rank: 2,
            rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD,
            gold_per_day: 70_000,
        },
        GuildRankRequest::SetTab {
            rank: 2,
            tab: 0,
            view: true,
            deposit: false,
            withdrawals_per_day: 5,
        },
        GuildRankRequest::Promote {
            character_name: "Cara".into(),
        },
        GuildRankRequest::Demote {
            character_name: "Cara".into(),
        },
        GuildRankRequest::Remove { rank: 3 },
    ];
    for request in &requests {
        bridge.send::<_, GuildChannel>(request.clone()).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<Requests>().0.len() < requests.len()
        && Instant::now() < deadline
    {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(server.world().resource::<Requests>().0, requests);
    let state = GuildRanksState {
        own_rank: 0,
        ranks: vec![GuildRankSettings {
            name: "Officer".into(),
            rights: GUILD_RIGHT_CHAT_LISTEN,
            gold_per_day: 70_000,
            tabs: vec![GuildRankTab {
                view: true,
                deposit: false,
                withdrawals_per_day: 5,
            }],
        }],
        members: vec![GuildRankMember {
            character_name: "Cara".into(),
            rank: 0,
        }],
        tab_names: vec!["Materials".into()],
        error: None,
    };
    let mut refused = state.clone();
    refused.error = Some(GuildRankError::Permissions);
    for state in [&state, &refused] {
        let world = server.world_mut();
        world
            .query::<&mut MessageSender<GuildRanksState>>()
            .single_mut(world)
            .unwrap()
            .send::<GuildChannel>(state.clone());
        server.update();
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut received = Vec::new();
    while received.len() < 2 && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event {
                if message.is::<GuildRanksState>() {
                    received.push(message.downcast::<GuildRanksState>().ok().unwrap());
                }
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(received, vec![state, refused]);
}
