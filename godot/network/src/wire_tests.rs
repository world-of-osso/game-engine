//! Real loopback UDP proof; owns its server and never contacts the development game server.

use super::*;
use lightyear::prelude::{LinkOf, NetworkTarget, Replicate, ReplicationSender, server};
use shared::{
    components::{
        CombatStatus, CreatureMotion, MovementControl, SheathState, StandState, UnitPose,
    },
    protocol::{
        CombatChannel, CombatEvent, CombatEventType, InputChannel, PlayerInput, RestChannel,
        RestSnapshot, RestStateUpdate,
    },
};
use std::net::UdpSocket;

#[derive(Resource, Default)]
struct ReceivedInputs(Vec<PlayerInput>);

fn receive_inputs(
    mut receivers: Query<&mut MessageReceiver<PlayerInput>>,
    mut received: ResMut<ReceivedInputs>,
) {
    for mut receiver in &mut receivers {
        received.0.extend(receiver.receive());
    }
}

fn create_fixture_server(register_extra: fn(&mut App)) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(server::ServerPlugins {
        tick_duration: SIMULATION_INTERVAL,
    });
    app.add_plugins(shared::ProtocolPlugin);
    register_extra(&mut app);
    app.init_resource::<ReceivedInputs>();
    app.add_systems(Update, receive_inputs);
    app.add_observer(|link: On<Add, LinkOf>, mut commands: Commands| {
        commands.entity(link.entity).insert(ReplicationSender);
    });
    app.finish();
    app.cleanup();
    app
}

fn start_fixture_server() -> (App, SocketAddr) {
    start_fixture_server_with(|_| {})
}

fn start_fixture_server_with(register_extra: fn(&mut App)) -> (App, SocketAddr) {
    // ServerUdpIo binds its own socket and does not expose the assigned port for port zero.
    let reservation = UdpSocket::bind("127.0.0.1:0").expect("reserve fixture UDP port");
    let address = reservation.local_addr().expect("read fixture UDP address");
    drop(reservation);
    let mut app = create_fixture_server(register_extra);
    let entity = app
        .world_mut()
        .spawn((
            network::LocalAddr(address),
            server::ServerUdpIo::default(),
            server::NetcodeServer::new(server::NetcodeConfig::default()),
        ))
        .id();
    app.world_mut().trigger(server::Start { entity });
    (app, address)
}

fn await_bridge_event(
    server: &mut App,
    bridge: &mut NetworkBridge,
    description: &str,
    mut matches: impl FnMut(&Event) -> bool,
) -> Event {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().expect("poll fixture bridge") {
            assert!(
                !matches!(event, Event::Disconnected(_)),
                "fixture disconnected waiting for {description}"
            );
            if matches(&event) {
                return event;
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {description}");
}

/// The next `count` protocol messages, polled together (one drain can hold several).
fn await_messages(
    server: &mut App,
    bridge: &mut NetworkBridge,
    count: usize,
) -> Vec<ProtocolMessage> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut messages = Vec::new();
    while Instant::now() < deadline && messages.len() < count {
        server.update();
        for event in bridge.drain_events().expect("poll fixture bridge") {
            match event {
                Event::Message(message) => messages.push(message),
                Event::Disconnected(_) => panic!("fixture disconnected waiting for messages"),
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        messages.len(),
        count,
        "timed out waiting for {count} messages"
    );
    messages
}

fn await_input(server: &mut App, bridge: &mut NetworkBridge) -> PlayerInput {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.update();
        for event in bridge
            .drain_events()
            .expect("poll bridge while server decodes input")
        {
            assert!(
                !matches!(event, Event::Disconnected(_)),
                "fixture disconnected before decoding input"
            );
        }
        if let Some(input) = server.world_mut().resource_mut::<ReceivedInputs>().0.pop() {
            return input;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("server did not decode PlayerInput from native bridge UDP");
}

fn await_control(
    server: &mut App,
    bridge: &mut NetworkBridge,
    server_id: u64,
    expected: MovementControl,
) -> UnitSnapshot {
    let event = await_bridge_event(
        server,
        bridge,
        "replicated movement control",
        |event| matches!(event, Event::UnitUpdated(unit) if unit.server_id == server_id && unit.movement_control == Some(expected)),
    );
    let Event::UnitUpdated(unit) = event else {
        unreachable!()
    };
    unit
}

fn await_combat(
    server: &mut App,
    bridge: &mut NetworkBridge,
    server_id: u64,
    expected: Option<CombatStatus>,
) -> UnitSnapshot {
    let event = await_bridge_event(
        server,
        bridge,
        "replicated combat transition",
        |event| matches!(event, Event::UnitUpdated(unit) if unit.server_id == server_id && unit.combat_status == expected),
    );
    let Event::UnitUpdated(unit) = event else {
        unreachable!()
    };
    unit
}

fn send_rest(server: &mut App, update: RestStateUpdate) {
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<RestStateUpdate>>()
        .single_mut(world)
        .expect("connected fixture rest sender")
        .send::<RestChannel>(update);
}

fn await_rest(server: &mut App, bridge: &mut NetworkBridge, expected: RestStateUpdate) {
    let event = await_bridge_event(
        server,
        bridge,
        "rest state update",
        |event| matches!(event, Event::Message(message) if message.is::<RestStateUpdate>()),
    );
    let Event::Message(message) = event else {
        unreachable!()
    };
    assert_eq!(message.downcast::<RestStateUpdate>().ok(), Some(expected));
}

#[test]
fn native_bridge_reports_protocol_rejection_instead_of_connecting() {
    use lightyear::prelude::AppComponentExt;
    // The server replicates one component more than the client, like a component added to
    // `shared` after the client was built.
    let (mut server, address) = start_fixture_server_with(|app| {
        app.component::<shared::components::VerticalVelocity>()
            .replicate();
    });
    let mut bridge = NetworkBridge::connect(address, 8200).expect("start fixture bridge");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut events = Vec::new();
    while Instant::now() < deadline && !matches!(events.last(), Some(Event::Disconnected(_))) {
        server.update();
        events.extend(bridge.drain_events().expect("poll fixture bridge"));
        thread::sleep(Duration::from_millis(5));
    }
    let described: Vec<String> = events
        .iter()
        .map(|event| match event {
            Event::Connected => "connected".into(),
            Event::ProtocolRejected(reason) => format!("rejected: {reason}"),
            Event::Disconnected(_) => "disconnected".into(),
            Event::Message(_) | Event::UnitUpdated(_) | Event::UnitRemoved(_) => "data".into(),
        })
        .collect();
    assert_eq!(
        described,
        [
            "rejected: Client and server protocols differ (component registry). Rebuild both from the same shared-protocol revision.",
            "disconnected",
        ]
    );
}

#[test]
fn native_bridge_delivers_all_original_combat_events_once_over_udp() {
    let (mut server, address) = start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8194).expect("start fixture bridge");
    await_bridge_event(&mut server, &mut bridge, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });

    let kinds = [
        CombatEventType::SpellDamage,
        CombatEventType::PeriodicDamage,
        CombatEventType::CriticalHit,
        CombatEventType::SpellHeal,
        CombatEventType::PeriodicHeal,
        CombatEventType::Miss,
        CombatEventType::Interrupt,
        CombatEventType::MeleeDamage,
    ];
    let expected: Vec<_> = (0..65)
        .map(|index| CombatEvent {
            attacker: 101,
            target: 202,
            amount: index as f32,
            spell_id: if index == 63 { 0 } else { 133 },
            event_type: kinds[index % kinds.len()].clone(),
        })
        .collect();
    for event in &expected {
        send_combat(&mut server, event.clone());
    }
    let received = await_messages(&mut server, &mut bridge, expected.len());
    for (message, event) in received.into_iter().zip(&expected) {
        let actual = message
            .downcast::<CombatEvent>()
            .ok()
            .expect("original CombatEvent");
        assert_eq!(actual.event_type, event.event_type);
        assert_eq!(actual.amount, event.amount);
        assert_eq!(actual.spell_id, event.spell_id);
    }
    for _ in 0..3 {
        server.update();
        assert!(bridge.drain_events().expect("second frame").is_empty());
    }
    bridge.stop().expect("join fixture worker");
}

fn send_combat(server: &mut App, event: CombatEvent) {
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<CombatEvent>>()
        .single_mut(world)
        .expect("connected fixture combat sender")
        .send::<CombatChannel>(event);
}

#[test]
fn native_bridge_tracks_combat_and_rest_transitions_over_udp() {
    let (mut server, address) = start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8193).expect("start fixture bridge");
    await_bridge_event(&mut server, &mut bridge, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });

    let entity = server
        .world_mut()
        .spawn((
            Player {
                name: "Resting fighter".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            CombatStatus(false),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let id = entity.to_bits();
    assert_eq!(
        await_combat(&mut server, &mut bridge, id, Some(CombatStatus(false))).combat_status,
        Some(CombatStatus(false))
    );

    server
        .world_mut()
        .entity_mut(entity)
        .insert(CombatStatus(true));
    assert_eq!(
        await_combat(&mut server, &mut bridge, id, Some(CombatStatus(true))).combat_status,
        Some(CombatStatus(true))
    );

    server
        .world_mut()
        .entity_mut(entity)
        .insert(CombatStatus(false));
    assert_eq!(
        await_combat(&mut server, &mut bridge, id, Some(CombatStatus(false))).combat_status,
        Some(CombatStatus(false))
    );

    server
        .world_mut()
        .entity_mut(entity)
        .remove::<CombatStatus>();
    assert_eq!(
        await_combat(&mut server, &mut bridge, id, None).combat_status,
        None
    );

    let present = RestStateUpdate {
        snapshot: Some(RestSnapshot {
            in_rest_area: true,
            rest_area_kind: None,
            rested_xp: 42,
            rested_xp_max: 100,
        }),
        message: None,
        error: None,
    };
    send_rest(&mut server, present.clone());
    await_rest(&mut server, &mut bridge, present);
    let cleared = RestStateUpdate {
        snapshot: None,
        message: None,
        error: None,
    };
    send_rest(&mut server, cleared.clone());
    await_rest(&mut server, &mut bridge, cleared);
    bridge.stop().expect("join fixture worker");
}

#[test]
fn native_bridge_decodes_udp_input_and_receives_control_epochs() {
    let (mut server, address) = start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8192).expect("start fixture bridge");
    await_bridge_event(&mut server, &mut bridge, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });

    let input = PlayerInput {
        direction: [0.6, 0.0, 0.8],
        facing_yaw: 1.25,
        jumping: false,
        running: true,
        swimming: true,
        position: [-8949.5, 112.88, 0.25],
        epoch: 7,
    };
    bridge
        .send::<PlayerInput, InputChannel>(input.clone())
        .expect("queue player input");
    let received = await_input(&mut server, &mut bridge);
    assert_eq!(received.direction, input.direction);
    assert_eq!(received.facing_yaw, input.facing_yaw);
    assert_eq!(received.jumping, input.jumping);
    assert_eq!(received.running, input.running);
    assert_eq!(received.swimming, input.swimming);
    assert_eq!(received.position, input.position);
    assert_eq!(received.epoch, input.epoch);

    let first_control = MovementControl {
        epoch: 7,
        controlled: true,
    };
    let entity = server
        .world_mut()
        .spawn((
            Player {
                name: "UDP fixture".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            first_control,
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let first = await_control(&mut server, &mut bridge, entity.to_bits(), first_control);
    assert_eq!(first.player.as_ref().unwrap().name, "UDP fixture");
    assert_eq!(first.position.unwrap().x, 1.0);

    let next_control = MovementControl {
        epoch: 8,
        controlled: false,
    };
    server.world_mut().entity_mut(entity).insert((
        next_control,
        Position {
            x: 20.0,
            y: 30.0,
            z: 40.0,
        },
    ));
    let next = await_control(&mut server, &mut bridge, entity.to_bits(), next_control);
    assert_eq!(next.position.unwrap().x, 20.0);
    assert_eq!(first.movement_control, Some(first_control));
    assert_eq!(first.position.unwrap().x, 1.0);

    server.world_mut().despawn(entity);
    await_bridge_event(
        &mut server,
        &mut bridge,
        "replicated unit removal",
        |event| matches!(event, Event::UnitRemoved(id) if *id == entity.to_bits()),
    );
    bridge.stop().expect("join fixture worker");
}

/// A wandering creature's replicated `CreatureMotion` reaches the host with its unit,
/// including a stop that changes no other component.
#[test]
fn native_bridge_receives_creature_motion_changes() {
    let (mut server, address) = start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8193).expect("start fixture bridge");
    await_bridge_event(&mut server, &mut bridge, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });
    let entity = server
        .world_mut()
        .spawn((
            Npc {
                template_id: 116,
                name: "Defias Bandit".into(),
            },
            Position {
                x: -9050.0,
                y: 5.0,
                z: 60.0,
            },
            CreatureMotion::Walk,
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let server_id = entity.to_bits();
    let mut await_motion = |server: &mut App, motion: CreatureMotion| {
        await_bridge_event(server, &mut bridge, "replicated creature motion", |event| {
            matches!(event, Event::UnitUpdated(unit)
                if unit.server_id == server_id && unit.creature_motion == Some(motion))
        })
    };
    let Event::UnitUpdated(walking) = await_motion(&mut server, CreatureMotion::Walk) else {
        unreachable!()
    };
    assert_eq!(walking.npc.as_ref().unwrap().name, "Defias Bandit");
    server
        .world_mut()
        .entity_mut(entity)
        .insert(CreatureMotion::Still);
    let Event::UnitUpdated(stopped) = await_motion(&mut server, CreatureMotion::Still) else {
        unreachable!()
    };
    assert_eq!(stopped.position.unwrap().x, -9050.0);
    bridge.stop().expect("join fixture worker");
}

/// Another player's replicated `PlayerMotion` (Retail `MovementFlags`) reaches the host
/// with its unit, including the stop that clears it without moving the player.
#[test]
fn native_bridge_receives_remote_player_motion_changes() {
    use shared::components::{Player, PlayerMotion};
    let (mut server, address) = start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8195).expect("start fixture bridge");
    await_bridge_event(&mut server, &mut bridge, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });
    let strafing_jump = PlayerMotion(PlayerMotion::STRAFE_LEFT | PlayerMotion::FALLING);
    let entity = server
        .world_mut()
        .spawn((
            Player {
                name: "Fbfps".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Position {
                x: -8913.0,
                y: 82.0,
                z: -140.0,
            },
            strafing_jump,
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let server_id = entity.to_bits();
    let mut await_motion = |server: &mut App, motion: PlayerMotion| {
        await_bridge_event(server, &mut bridge, "replicated player motion", |event| {
            matches!(event, Event::UnitUpdated(unit)
                if unit.server_id == server_id && unit.player_motion == Some(motion))
        })
    };
    let Event::UnitUpdated(moving) = await_motion(&mut server, strafing_jump) else {
        unreachable!()
    };
    assert_eq!(moving.player.as_ref().unwrap().name, "Fbfps");
    server
        .world_mut()
        .entity_mut(entity)
        .insert(PlayerMotion::default());
    let Event::UnitUpdated(stopped) = await_motion(&mut server, PlayerMotion::default()) else {
        unreachable!()
    };
    assert_eq!(stopped.position.unwrap().x, -8913.0);
    bridge.stop().expect("join fixture worker");
}

/// The nameplate rule inputs reach the host with the unit: FactionTemplate, UnitFlags
/// and the combat flag, including a combat drop that changes nothing else.
#[test]
fn native_bridge_receives_faction_flags_and_combat_status() {
    use shared::components::{CombatStatus, UnitFactionTemplate, UnitFlags};
    let (mut server, address) = start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8194).expect("start fixture bridge");
    await_bridge_event(&mut server, &mut bridge, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });
    let entity = server
        .world_mut()
        .spawn((
            Npc {
                template_id: 38,
                name: "Defias Thug".into(),
            },
            Position {
                x: -8900.0,
                y: 80.0,
                z: -120.0,
            },
            UnitFactionTemplate(7),
            UnitFlags(UnitFlags::NOT_SELECTABLE),
            CombatStatus(true),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let server_id = entity.to_bits();
    let Event::UnitUpdated(fighting) = await_bridge_event(
        &mut server,
        &mut bridge,
        "unit in combat",
        |event| matches!(event, Event::UnitUpdated(unit) if unit.server_id == server_id && unit.in_combat),
    ) else {
        unreachable!()
    };
    assert_eq!(fighting.faction_template, Some(7));
    assert_eq!(fighting.unit_flags, Some(UnitFlags::NOT_SELECTABLE));
    server
        .world_mut()
        .entity_mut(entity)
        .insert(CombatStatus(false));
    let Event::UnitUpdated(calm) = await_bridge_event(
        &mut server,
        &mut bridge,
        "combat drop",
        |event| matches!(event, Event::UnitUpdated(unit) if unit.server_id == server_id && !unit.in_combat),
    ) else {
        unreachable!()
    };
    assert_eq!(calm.faction_template, Some(7));
    bridge.stop().expect("join fixture worker");
}

/// A creature's replicated `UnitPose` reaches the host with its unit, and a pose-only
/// change (Stockade guard drawing its sword, a criminal waking) arrives on its own.
#[test]
fn native_bridge_receives_unit_pose_changes() {
    let (mut server, address) = start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8194).expect("start fixture bridge");
    await_bridge_event(&mut server, &mut bridge, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });
    let asleep = UnitPose {
        stand_state: StandState::Sleep,
        sheath_state: SheathState::Unarmed,
        emote_state: 0,
    };
    let entity = server
        .world_mut()
        .spawn((
            Npc {
                template_id: 46382,
                name: "Petty Criminal".into(),
            },
            Position {
                x: 100.0,
                y: 5.0,
                z: 1.0,
            },
            asleep,
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let server_id = entity.to_bits();
    let mut await_pose = |server: &mut App, pose: UnitPose| {
        await_bridge_event(server, &mut bridge, "replicated unit pose", |event| {
            matches!(event, Event::UnitUpdated(unit)
                if unit.server_id == server_id && unit.unit_pose == Some(pose))
        })
    };
    let Event::UnitUpdated(sleeping) = await_pose(&mut server, asleep) else {
        unreachable!()
    };
    assert_eq!(sleeping.npc.as_ref().unwrap().name, "Petty Criminal");
    let ready = UnitPose {
        stand_state: StandState::Stand,
        sheath_state: SheathState::Melee,
        emote_state: 333,
    };
    server.world_mut().entity_mut(entity).insert(ready);
    let Event::UnitUpdated(standing) = await_pose(&mut server, ready) else {
        unreachable!()
    };
    assert_eq!(standing.position.unwrap().x, 100.0);
    bridge.stop().expect("join fixture worker");
}

/// Server-driven mirror timers (TrinityCore `SMSG_START/PAUSE/STOP_MIRROR_TIMER`) reach the
/// host in send order on `MirrorTimerChannel`: a breath start, a pause and a stop.
#[test]
fn native_bridge_receives_mirror_timer_messages_in_order() {
    use shared::protocol::{
        MIRROR_TIMER_BREATH, MirrorTimerChannel, MirrorTimerPause, MirrorTimerStart,
        MirrorTimerStop,
    };
    let (mut server, address) = start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8195).expect("start fixture bridge");
    await_bridge_event(&mut server, &mut bridge, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });
    let start = MirrorTimerStart {
        timer: MIRROR_TIMER_BREATH,
        value_ms: 180_000,
        max_value_ms: 180_000,
        scale: -1.0,
        paused: false,
        spell_id: 0,
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<MirrorTimerStart>>()
        .single_mut(world)
        .expect("one connected fixture sender")
        .send::<MirrorTimerChannel>(start);
    world
        .query::<&mut MessageSender<MirrorTimerPause>>()
        .single_mut(world)
        .expect("one connected fixture sender")
        .send::<MirrorTimerChannel>(MirrorTimerPause {
            timer: MIRROR_TIMER_BREATH,
            paused: true,
        });
    world
        .query::<&mut MessageSender<MirrorTimerStop>>()
        .single_mut(world)
        .expect("one connected fixture sender")
        .send::<MirrorTimerChannel>(MirrorTimerStop {
            timer: MIRROR_TIMER_BREATH,
        });
    let received = await_messages(&mut server, &mut bridge, 3);
    let mut received = received.into_iter();
    assert_eq!(
        received.next().unwrap().downcast::<MirrorTimerStart>().ok(),
        Some(start)
    );
    assert_eq!(
        received.next().unwrap().downcast::<MirrorTimerPause>().ok(),
        Some(MirrorTimerPause {
            timer: MIRROR_TIMER_BREATH,
            paused: true
        })
    );
    assert_eq!(
        received.next().unwrap().downcast::<MirrorTimerStop>().ok(),
        Some(MirrorTimerStop {
            timer: MIRROR_TIMER_BREATH
        })
    );
    bridge.stop().expect("join fixture worker");
}

/// A vendor's replicated `NpcFlags` and the player's `Gold` reach the host with their
/// units, and the server's `VendorInventory` arrives as its protocol message.
#[test]
fn native_bridge_receives_vendor_flags_gold_and_inventory() {
    use shared::components::Gold;
    use shared::protocol::{MerchantChannel, NpcFlags, VendorInventory, VendorItem};

    let (mut server, address) = start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8195).expect("start fixture bridge");
    await_bridge_event(&mut server, &mut bridge, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });
    let vendor = server
        .world_mut()
        .spawn((
            Npc {
                template_id: 1213,
                name: "Godric Rothgar".into(),
            },
            NpcFlags(NpcFlags::VENDOR | NpcFlags::REPAIR),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let player = server
        .world_mut()
        .spawn((
            Player {
                name: "Fbworldmap".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Gold(12_345),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    // Both units replicate in one batch; keep whichever arrives first.
    let (mut npc_flags, mut npc_gold, mut player_gold) = (None, None, None);
    await_bridge_event(
        &mut server,
        &mut bridge,
        "vendor flags and player gold",
        |event| {
            if let Event::UnitUpdated(unit) = event {
                if unit.server_id == vendor.to_bits() {
                    (npc_flags, npc_gold) = (unit.npc_flags, unit.gold);
                } else if unit.server_id == player.to_bits() {
                    player_gold = unit.gold;
                }
            }
            npc_flags.is_some() && player_gold.is_some()
        },
    );
    assert_eq!(npc_flags, Some(NpcFlags::VENDOR | NpcFlags::REPAIR));
    assert_eq!(npc_gold, None);
    assert_eq!(player_gold, Some(12_345));
    let inventory = VendorInventory {
        npc: vendor.to_bits(),
        can_repair: true,
        items: vec![VendorItem {
            slot: 0,
            item_id: 2488,
            name: "Gladius".into(),
            quality: 1,
            price: 57,
            stack_count: 1,
            max_stack: 1,
            num_available: None,
            usable: true,
        }],
    };
    let mut senders = server
        .world_mut()
        .query::<&mut MessageSender<VendorInventory>>();
    for mut sender in senders.iter_mut(server.world_mut()) {
        sender.send::<MerchantChannel>(inventory.clone());
    }
    let Event::Message(message) = await_bridge_event(
        &mut server,
        &mut bridge,
        "vendor inventory",
        |event| matches!(event, Event::Message(message) if message.is::<VendorInventory>()),
    ) else {
        unreachable!()
    };
    assert_eq!(message.downcast::<VendorInventory>().ok(), Some(inventory));
    bridge.stop().expect("join fixture worker");
}

/// Auction requests cross real UDP; every default native relay delivers its original reply.
#[test]
fn native_bridge_auction_operations_and_query_rejections() {
    use shared::protocol::*;
    #[derive(Resource)]
    struct Requests<M: network::Message>(Vec<M>);
    fn capture<M: network::Message>(
        mut receivers: Query<&mut MessageReceiver<M>>,
        mut messages: ResMut<Requests<M>>,
    ) {
        for mut receiver in &mut receivers {
            messages.0.extend(receiver.receive());
        }
    }
    fn install<M: network::Message>(app: &mut App) {
        app.insert_resource(Requests::<M>(Vec::new()));
        app.add_systems(Update, capture::<M>);
    }
    fn install_auction(app: &mut App) {
        install::<OpenAuctionHouse>(app);
        install::<QueryAuctions>(app);
        install::<QueryAuctionBrowse>(app);
        install::<QueryAuctionInventory>(app);
        install::<QueryOwnedAuctions>(app);
        install::<QueryBidAuctions>(app);
        install::<CreateAuction>(app);
        install::<PlaceBid>(app);
        install::<BuyoutAuction>(app);
        install::<CancelAuction>(app);
    }
    fn received<M: network::Message>(server: &mut App, bridge: &mut NetworkBridge) -> M {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            server.update();
            for event in bridge.drain_events().expect("auction worker") {
                assert!(!matches!(event, Event::Disconnected(_)));
            }
            if let Some(message) = server.world_mut().resource_mut::<Requests<M>>().0.pop() {
                return message;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!(
            "auction request {} not received",
            std::any::type_name::<M>()
        );
    }
    fn reply<M: network::Message + Clone + std::fmt::Debug + PartialEq>(
        server: &mut App,
        bridge: &mut NetworkBridge,
        expected: M,
    ) {
        let mut senders = server.world_mut().query::<&mut MessageSender<M>>();
        for mut sender in senders.iter_mut(server.world_mut()) {
            sender.send::<AuctionChannel>(expected.clone());
        }
        let Event::Message(message) = await_bridge_event(
            server,
            bridge,
            "auction relay",
            |e| matches!(e,Event::Message(m) if m.is::<M>()),
        ) else {
            panic!()
        };
        assert_eq!(message.downcast::<M>().ok(), Some(expected));
    }
    let (mut server, address) = start_fixture_server_with(install_auction);
    let mut bridge = NetworkBridge::connect(address, 9088).expect("auction connect");
    await_bridge_event(&mut server, &mut bridge, "auction connected", |e| {
        matches!(e, Event::Connected)
    });
    macro_rules! request {
        ($value:expr,$kind:ty) => {{
            let value = $value;
            bridge.send::<_, AuctionChannel>(value.clone()).unwrap();
            assert_eq!(received::<$kind>(&mut server, &mut bridge), value);
        }};
    }
    request!(OpenAuctionHouse, OpenAuctionHouse);
    let query = AuctionSearchQuery {
        text: "linen".into(),
        item_id: Some(2589),
        class_id: Some(7),
        page: 1,
        page_size: 50,
        min_level: None,
        max_level: None,
        quality: None,
        usable_only: false,
        sort_field: AuctionSortField::Name,
        sort_dir: AuctionSortDir::Asc,
        faction: 0,
    };
    request!(
        QueryAuctions {
            query: query.clone()
        },
        QueryAuctions
    );
    let mut browse_query = query.clone();
    browse_query.item_id = None;
    request!(
        QueryAuctionBrowse {
            query: browse_query.clone()
        },
        QueryAuctionBrowse
    );
    reply(
        &mut server,
        &mut bridge,
        AuctionBrowseResults {
            query: browse_query,
            total_results: 103,
            items: vec![AuctionBrowseItem {
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                required_level: 1,
                lowest_unit_price: 17,
                total_quantity: 5_000_000_001,
            }],
        },
    );
    request!(QueryAuctionInventory, QueryAuctionInventory);
    request!(QueryOwnedAuctions, QueryOwnedAuctions);
    request!(QueryBidAuctions, QueryBidAuctions);
    request!(
        CreateAuction {
            item_guid: 17,
            stack_count: 5,
            min_bid: 100,
            buyout_price: Some(1000),
            duration: AuctionDuration::Long
        },
        CreateAuction
    );
    request!(
        PlaceBid {
            auction_id: 12,
            amount: 110
        },
        PlaceBid
    );
    request!(BuyoutAuction { auction_id: 12 }, BuyoutAuction);
    request!(CancelAuction { auction_id: 13 }, CancelAuction);
    reply(
        &mut server,
        &mut bridge,
        AuctionHouseOpened {
            success: true,
            error: None,
        },
    );
    reply(
        &mut server,
        &mut bridge,
        AuctionSearchResults {
            query,
            total_results: 103,
            results: vec![],
        },
    );
    reply(
        &mut server,
        &mut bridge,
        AuctionInventorySnapshot {
            gold: 1000,
            items: vec![],
        },
    );
    reply(
        &mut server,
        &mut bridge,
        OwnedAuctionListResponse { listings: vec![] },
    );
    reply(
        &mut server,
        &mut bridge,
        BidAuctionListResponse { listings: vec![] },
    );
    reply(
        &mut server,
        &mut bridge,
        AuctionOperationResponse {
            success: false,
            message: "not interacting with an auctioneer".into(),
        },
    );
    bridge.stop().expect("auction stop");
}
