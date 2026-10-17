//! Achievement deltas and Guide clocks keep their channel order with the full snapshots
//! and earned alerts they interleave with, across the real UDP bridge.
use super::*;
use shared::protocol::*;

fn connect(server: &mut App, address: SocketAddr, client_id: u64) -> NetworkBridge {
    let mut bridge = NetworkBridge::connect(address, client_id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "netdelta connection timeout");
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

fn flush_server(server: &mut App) {
    for _ in 0..8 {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
}

/// MessageSender queues per type: flushing after each send establishes the actual channel
/// order, while the paused client worker receives every type in one frame.
fn send_in_channel_order(
    server: &mut App,
    bridge: &NetworkBridge,
    sends: Vec<Box<dyn FnOnce(&mut World)>>,
) {
    let (paused, ready) = mpsc::channel();
    let (resume, wait) = mpsc::channel();
    bridge
        .enqueue(move |_| {
            paused.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(10)).unwrap();
        })
        .unwrap();
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    for send in sends {
        send(server.world_mut());
        flush_server(server);
    }
    resume.send(()).unwrap();
}

fn send<M: network::Message, C: network::Channel>(message: M) -> Box<dyn FnOnce(&mut World)> {
    Box::new(move |world: &mut World| {
        world
            .query::<&mut MessageSender<M>>()
            .single_mut(world)
            .unwrap()
            .send::<C>(message);
    })
}

#[derive(Debug, PartialEq)]
enum Received {
    Update(AchievementStateUpdate),
    Delta(AchievementDelta),
    Guide(AdventureGuideSnapshot),
    Clock(AdventureGuideClock),
}

fn receive(server: &mut App, bridge: &mut NetworkBridge, count: usize) -> Vec<Received> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut received = Vec::new();
    while received.len() < count && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            let Event::Message(message) = event else {
                continue;
            };
            let message = match message.downcast::<AchievementStateUpdate>() {
                Ok(update) => Received::Update(update),
                Err(message) => match message.downcast::<AchievementDelta>() {
                    Ok(delta) => Received::Delta(delta),
                    Err(message) => match message.downcast::<AdventureGuideSnapshot>() {
                        Ok(guide) => Received::Guide(guide),
                        Err(message) => match message.downcast::<AdventureGuideClock>() {
                            Ok(clock) => Received::Clock(clock),
                            Err(_) => continue,
                        },
                    },
                },
            };
            received.push(message);
        }
        thread::sleep(Duration::from_millis(5));
    }
    received
}

fn toast(achievement_id: u32, name: &str) -> AchievementStateUpdate {
    AchievementStateUpdate {
        snapshot: None,
        completed: Some(AchievementToastSnapshot {
            achievement_id,
            name: name.into(),
            points: 10,
            icon_fdid: 134163,
        }),
        message: None,
        error: None,
    }
}

#[test]
fn netdelta_wire_relays_achievement_delta_in_order_between_snapshot_and_toasts() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(|_| {});
    let mut bridge = connect(&mut server, address, 7239001);
    let full = AchievementStateUpdate {
        snapshot: Some(AchievementSnapshot {
            earned_ids: vec![6, 633],
            progress: vec![AchievementProgressSnapshot {
                achievement_id: 7,
                current: 10,
                required: 20,
                completed: false,
            }],
        }),
        completed: None,
        message: None,
        error: None,
    };
    let delta = AchievementDelta {
        earned_ids: Some(vec![6, 7, 633]),
        progress_upserts: vec![AchievementProgressSnapshot {
            achievement_id: 7,
            current: 20,
            required: 20,
            completed: true,
        }],
        removed_progress_ids: vec![],
    };
    let first = toast(633, "Stormwind Stockade");
    let second = toast(7, "Level 20");
    send_in_channel_order(
        &mut server,
        &bridge,
        vec![
            send::<_, AchievementChannel>(full.clone()),
            send::<_, AchievementChannel>(first.clone()),
            send::<_, AchievementChannel>(delta.clone()),
            send::<_, AchievementChannel>(second.clone()),
        ],
    );
    let received = receive(&mut server, &mut bridge, 4);
    bridge.stop().unwrap();
    assert_eq!(
        received,
        [
            Received::Update(full),
            Received::Update(first),
            Received::Delta(delta),
            Received::Update(second),
        ]
    );
}

fn guide(now_unix: i64, points: u32) -> AdventureGuideSnapshot {
    AdventureGuideSnapshot {
        month: JournalMonth {
            year: 2026,
            month: 10,
            name: "October".into(),
            start_unix: 1790838000,
            end_unix: 1793516400,
            now_unix,
            tracking_since: 1790838000,
        },
        activities: vec![],
        tags: vec![],
        suggestions: vec![],
        suggestion_census: SuggestionCensus::default(),
        points,
        activity_scope_notice: None,
        source_error: None,
    }
}

#[test]
fn netdelta_wire_relays_guide_clock_in_instance_channel_order_with_full_guides() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(|_| {});
    let mut bridge = connect(&mut server, address, 7239002);
    let initial = guide(1791759600, 0);
    let tick = AdventureGuideClock {
        now_unix: 1791759601,
    };
    let changed = guide(1791759602, 50);
    let later = AdventureGuideClock {
        now_unix: 1791759603,
    };
    send_in_channel_order(
        &mut server,
        &bridge,
        vec![
            send::<_, InstanceChannel>(initial.clone()),
            send::<_, InstanceChannel>(tick.clone()),
            send::<_, InstanceChannel>(changed.clone()),
            send::<_, InstanceChannel>(later.clone()),
        ],
    );
    let received = receive(&mut server, &mut bridge, 4);
    bridge.stop().unwrap();
    assert_eq!(
        received,
        [
            Received::Guide(initial),
            Received::Clock(tick),
            Received::Guide(changed),
            Received::Clock(later),
        ]
    );
}
