//! Real loopback UDP proof; owns its server and never contacts the development game server.

#[test]
fn meetingstones_default_bridge_receives_request_over_udp() {
    use shared::protocol::{InteractionChannel, SummonRequest};
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8301);
    await_connected(&mut server, &mut host);
    let expected = SummonRequest {
        summoner: "Stonecaller".into(),
        zone_id: 0,
        time_left_ms: 120_000,
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<SummonRequest>>()
        .single_mut(world)
        .unwrap()
        .send::<InteractionChannel>(expected.clone());
    let Event::Message(message) = await_bridge_event(
        &mut server,
        &mut host,
        "meeting-stone offer on default bridge",
        |event| matches!(event, Event::Message(message) if message.is::<SummonRequest>()),
    ) else {
        panic!("expected summon offer")
    };
    assert_eq!(message.downcast::<SummonRequest>().ok(), Some(expected));
    host.stop();
}

#[test]
fn deathstate_default_bridge_receives_snapshot_over_udp() {
    use shared::protocol::{DeathChannel, DeathSnapshot, DeathStateSnapshot, DeathStateUpdate};
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8291);
    await_connected(&mut server, &mut host);
    let expected = DeathStateUpdate {
        snapshot: Some(DeathSnapshot {
            state: DeathStateSnapshot::Dead,
            corpse: None,
            graveyard: None,
            can_resurrect_at_corpse: false,
            spirit_healer_available: false,
        }),
        message: Some("you died".into()),
        error: None,
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<DeathStateUpdate>>()
        .single_mut(world)
        .expect("death sender")
        .send::<DeathChannel>(expected.clone());
    let Event::Message(message) = await_bridge_event(
        &mut server,
        &mut host,
        "death snapshot on default bridge",
        |event| matches!(event, Event::Message(message) if message.is::<DeathStateUpdate>()),
    ) else {
        panic!("expected death snapshot");
    };
    assert_eq!(message.downcast::<DeathStateUpdate>().ok(), Some(expected));
    host.stop();
}

#[test]
fn xpchat_native_bridge_receives_log_xp_gain() {
    use shared::protocol::{ExperienceChannel, LogXpGain, XpGainReason};
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8247);
    await_connected(&mut server, &mut host);
    let expected = LogXpGain {
        victim: Some(0x0000_0001_0000_0099),
        original: 120,
        amount: 60,
        group_bonus: 1.0,
        reason: XpGainReason::Kill,
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<LogXpGain>>()
        .single_mut(world)
        .expect("one XP sender")
        .send::<ExperienceChannel>(expected);
    let Event::Message(message) = await_bridge_event(
        &mut server,
        &mut host,
        "XP gain on the default native bridge",
        |event| matches!(event, Event::Message(message) if message.is::<LogXpGain>()),
    ) else {
        panic!("expected XP gain");
    };
    assert_eq!(message.downcast::<LogXpGain>().ok(), Some(expected));
    host.stop();
}

#[test]
fn buffcancel_native_bridge_delivers_exactly_one_cancel_aura() {
    use shared::protocol::{CancelAura, CombatChannel};
    #[derive(Resource, Default)]
    struct Requests(Vec<CancelAura>);
    fn capture(
        mut receivers: Query<&mut MessageReceiver<CancelAura>>,
        mut requests: ResMut<Requests>,
    ) {
        for mut receiver in &mut receivers {
            requests.0.extend(receiver.receive());
        }
    }
    fn install(app: &mut App) {
        app.init_resource::<Requests>();
        app.add_systems(Update, capture);
    }
    let (mut server, address) = start_fixture_server_with(install);
    let mut host = Host::connect(address, 8298);
    await_connected(&mut server, &mut host);
    let expected = CancelAura { spell_id: 1459 };
    host.bridge
        .send::<_, CombatChannel>(expected.clone())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(server.world().resource::<Requests>().0, vec![expected]);
    host.stop();
}

#[test]
fn native_mailbox_requests_preserve_object_mail_and_sparse_attachment_slot() {
    use shared::protocol::{
        InteractionChannel, MailAction, MailChannel, MailRequest, UseGameObject,
    };
    #[derive(Resource, Default)]
    struct Requests {
        uses: Vec<UseGameObject>,
        claims: Vec<MailRequest>,
    }
    fn capture(
        mut uses: Query<&mut MessageReceiver<UseGameObject>>,
        mut claims: Query<&mut MessageReceiver<MailRequest>>,
        mut requests: ResMut<Requests>,
    ) {
        for mut receiver in &mut uses {
            requests.uses.extend(receiver.receive());
        }
        for mut receiver in &mut claims {
            requests.claims.extend(receiver.receive());
        }
    }
    fn install(app: &mut App) {
        app.init_resource::<Requests>();
        app.add_systems(Update, capture);
    }
    let (mut server, address) = start_fixture_server_with(install);
    let mut host = Host::connect(address, 8223);
    await_connected(&mut server, &mut host);
    let use_object = UseGameObject { object: 517 };
    let money = MailRequest {
        object: 517,
        mail_id: 901,
        action: MailAction::TakeMoney,
    };
    let item = MailRequest {
        object: 517,
        mail_id: 902,
        action: MailAction::TakeAttachment { slot: 7 },
    };
    host.bridge
        .send::<_, InteractionChannel>(use_object.clone())
        .unwrap();
    host.bridge.send::<_, MailChannel>(money).unwrap();
    host.bridge.send::<_, MailChannel>(item).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.update();
        let requests = server.world().resource::<Requests>();
        if requests.uses.len() == 1 && requests.claims.len() == 2 {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let requests = server.world().resource::<Requests>();
    assert_eq!(requests.uses, vec![use_object]);
    assert_eq!(requests.claims, vec![money, item]);
    host.stop();
}

#[test]
fn native_bridge_sends_set_specialization_on_the_talent_channel() {
    use shared::protocol::{SetSpecialization, TalentChannel};
    #[derive(Resource, Default)]
    struct Requests(Vec<SetSpecialization>);
    fn capture(
        mut receivers: Query<&mut MessageReceiver<SetSpecialization>>,
        mut requests: ResMut<Requests>,
    ) {
        for mut receiver in &mut receivers {
            requests.0.extend(receiver.receive());
        }
    }
    fn install(app: &mut App) {
        app.init_resource::<Requests>();
        app.add_systems(Update, capture);
    }
    let (mut server, address) = start_fixture_server_with(install);
    let mut host = Host::connect(address, 8231);
    await_connected(&mut server, &mut host);
    host.bridge
        .send::<_, TalentChannel>(SetSpecialization { spec_id: 64 })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<Requests>().0.is_empty() && Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        server.world().resource::<Requests>().0,
        vec![SetSpecialization { spec_id: 64 }]
    );
    host.stop();
}

#[test]
fn native_mailbox_replication_reaches_host_without_an_npc_marker() {
    use shared::protocol::{GAMEOBJECT_TYPE_MAILBOX, GameObjectInfo};
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8221);
    await_connected(&mut server, &mut host);
    let object = server
        .world_mut()
        .spawn((
            GameObjectInfo {
                entry: 140907,
                go_type: GAMEOBJECT_TYPE_MAILBOX,
                display_id: 1727,
                name: "Stormwind Mailbox".into(),
                scale: 1.0,
            },
            Position {
                x: 2.0,
                y: 3.0,
                z: 4.0,
            },
            Rotation {
                x: 0.0,
                y: 0.7,
                z: 0.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let id = object.to_bits();
    await_unit(&mut server, &mut host, id, "mailbox replication", |unit| {
        unit.has::<GameObjectInfo>()
    });
    let unit = host.unit(id).unwrap();
    assert!(!unit.has::<Npc>() && !unit.has::<Player>());
    assert_eq!(
        unit.get::<Position>(),
        Some(&Position {
            x: 2.0,
            y: 3.0,
            z: 4.0
        })
    );
    let info = unit.get::<GameObjectInfo>().unwrap();
    assert_eq!(info.display_id, 1727);
    assert_eq!(info.entry, 140907);
    assert_eq!(
        unit.get::<Rotation>(),
        Some(&Rotation {
            x: 0.0,
            y: 0.7,
            z: 0.0
        })
    );
    server.world_mut().entity_mut(object).insert(Position {
        x: 6.0,
        y: 3.0,
        z: 4.0,
    });
    await_unit(
        &mut server,
        &mut host,
        id,
        "mailbox position update",
        |unit| position_x(unit) == Some(6.0),
    );
    server.world_mut().despawn(object);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !host.despawned.contains(&id) {
        assert!(Instant::now() < deadline, "mailbox removal");
        server.update();
        host.poll();
        thread::sleep(Duration::from_millis(5));
    }
    host.stop();
}

#[test]
fn native_mailbox_replies_reach_default_bridge() {
    use shared::protocol::{
        MailChannel, MailError, MailFailed, MailSent, MailboxContents, PendingMail,
    };
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8222);
    await_connected(&mut server, &mut host);
    macro_rules! reply {
        ($ty:ty, $expected:expr) => {{
            let expected = $expected;
            let world = server.world_mut();
            for mut sender in world.query::<&mut MessageSender<$ty>>().iter_mut(world) {
                sender.send::<MailChannel>(expected.clone());
            }
            let Event::Message(message) = await_bridge_event(&mut server, &mut host, "mail reply", |e| matches!(e, Event::Message(m) if m.is::<$ty>())) else { unreachable!() };
            assert_eq!(message.downcast::<$ty>().ok(), Some(expected));
        }};
    }
    reply!(
        MailboxContents,
        MailboxContents {
            object: 517,
            mails: vec![],
            now: 100
        }
    );
    reply!(
        MailFailed,
        MailFailed {
            object: 517,
            error: MailError::InventoryFull
        }
    );
    reply!(MailSent, MailSent { object: 517 });
    reply!(
        PendingMail,
        PendingMail {
            senders: vec!["Auction House".into()]
        }
    );
    host.stop();
}

#[test]
fn bossframes_bridge_preserves_encounter_lifecycle_channel_order() {
    use shared::protocol::{
        EncounterChannel, EncounterDisengageUnit, EncounterEnd, EncounterEngageUnit, EncounterStart,
    };
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8306);
    await_connected(&mut server, &mut host);
    let (held, confirmed) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    host.bridge
        .enqueue(move |_| {
            held.send(()).unwrap();
            resumed.recv_timeout(Duration::from_secs(10)).unwrap();
        })
        .unwrap();
    confirmed.recv_timeout(Duration::from_secs(10)).unwrap();
    let start = EncounterStart {
        encounter_id: 1144,
        difficulty_id: 1,
        group_size: 1,
    };
    let engage = EncounterEngageUnit {
        unit: 123,
        target_frame_priority: 0,
    };
    let disengage = EncounterDisengageUnit { unit: 123 };
    let end = EncounterEnd {
        encounter_id: 1144,
        difficulty_id: 1,
        group_size: 1,
        success: false,
    };
    macro_rules! send {
        ($ty:ty, $value:expr) => {{
            let world = server.world_mut();
            world
                .query::<&mut MessageSender<$ty>>()
                .single_mut(world)
                .unwrap()
                .send::<EncounterChannel>($value.clone());
            server.update();
        }};
    }
    send!(EncounterStart, start);
    send!(EncounterEngageUnit, engage);
    send!(EncounterDisengageUnit, disengage);
    send!(EncounterEnd, end);
    send!(EncounterStart, start);
    send!(EncounterEngageUnit, engage);
    let deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    resume.send(()).unwrap();
    let mut messages = await_messages(&mut server, &mut host, 6).into_iter();
    macro_rules! next {
        ($ty:ty, $value:expr) => {
            assert_eq!(
                messages.next().unwrap().downcast::<$ty>().ok(),
                Some($value)
            );
        };
    }
    next!(EncounterStart, start.clone());
    next!(EncounterEngageUnit, engage.clone());
    next!(EncounterDisengageUnit, disengage);
    next!(EncounterEnd, end);
    next!(EncounterStart, start);
    next!(EncounterEngageUnit, engage);
    host.stop();
}

#[test]
fn native_bridge_receives_loot_messages_in_channel_order() {
    use shared::protocol::{
        CorpseLootable, LootChannel, LootClosed, LootContent, LootError, LootFailed, LootResponse,
        LootSlot, LootSlotRemoved,
    };
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8210);
    await_connected(&mut server, &mut host);
    // Hold the worker before receiving so different types reach its relay together.
    // The host also leaves its event FIFO unpolled until all eight sends are flushed.
    let (held, hold_confirmed) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    host.bridge
        .enqueue(move |_| {
            held.send(()).expect("confirm fixture worker hold");
            resumed
                .recv_timeout(Duration::from_secs(10))
                .expect("release fixture worker hold");
        })
        .expect("hold fixture worker");
    hold_confirmed
        .recv_timeout(Duration::from_secs(10))
        .expect("fixture worker entered hold");
    // MessageSender queues per type, not in cross-type caller order. A full update
    // after each send flushes it into LootChannel before the next type is queued.
    // Repeat response/removal/close to expose a relay that groups messages by type.
    let corpse = 123;
    let opened = LootResponse {
        corpse,
        auto: false,
        slots: vec![LootSlot {
            slot: 0,
            content: LootContent::Money { copper: 12345 },
        }],
    };
    let removed = LootSlotRemoved { corpse, slot: 0 };
    let closed = LootClosed { corpse };
    let failed = LootFailed {
        corpse,
        error: LootError::InventoryFull,
    };
    let lootable = CorpseLootable {
        corpse,
        lootable: true,
    };
    macro_rules! send_and_flush {
        ($ty:ty, $value:expr) => {{
            let world = server.world_mut();
            world
                .query::<&mut MessageSender<$ty>>()
                .single_mut(world)
                .expect("one connected loot sender")
                .send::<LootChannel>($value.clone());
            server.update();
        }};
    }
    send_and_flush!(CorpseLootable, lootable);
    send_and_flush!(LootResponse, opened);
    send_and_flush!(LootSlotRemoved, removed);
    send_and_flush!(LootClosed, closed);
    send_and_flush!(LootFailed, failed);
    send_and_flush!(LootResponse, opened);
    send_and_flush!(LootSlotRemoved, removed);
    send_and_flush!(LootClosed, closed);
    // Allow throttled UDP sends to leave the server while the worker stays held.
    let send_deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < send_deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    resume.send(()).expect("resume fixture worker");
    let mut received = await_messages(&mut server, &mut host, 8).into_iter();
    macro_rules! assert_next {
        ($ty:ty, $expected:expr) => {
            assert_eq!(
                received.next().unwrap().downcast::<$ty>().ok(),
                Some($expected)
            );
        };
    }
    assert_next!(CorpseLootable, lootable);
    assert_next!(LootResponse, opened.clone());
    assert_next!(LootSlotRemoved, removed);
    assert_next!(LootClosed, closed);
    assert_next!(LootFailed, failed);
    assert_next!(LootResponse, opened);
    assert_next!(LootSlotRemoved, removed);
    assert_next!(LootClosed, closed);
    host.stop();
}

/// A roster and the member states sent right after it reach the host in `GroupChannel`
/// order; per-type relays handed them over grouped by type, so states could overtake the
/// roster that admits their member.
#[test]
fn native_bridge_receives_group_messages_in_channel_order() {
    use shared::components::Position;
    use shared::death::DeathState;
    use shared::protocol::{
        GroupChannel, GroupCommandResponse, GroupMemberSnapshot, GroupMemberState,
        GroupMemberStates, GroupMessageCode, GroupRoleSnapshot, GroupRosterSnapshot,
    };
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8211);
    await_connected(&mut server, &mut host);
    let (held, hold_confirmed) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    host.bridge
        .enqueue(move |_| {
            held.send(()).expect("confirm fixture worker hold");
            resumed
                .recv_timeout(Duration::from_secs(10))
                .expect("release fixture worker hold");
        })
        .expect("hold fixture worker");
    hold_confirmed
        .recv_timeout(Duration::from_secs(10))
        .expect("fixture worker entered hold");
    let roster = GroupRosterSnapshot {
        is_raid: false,
        ready_count: 0,
        total_count: 2,
        members: ["Ann", "Bob"]
            .map(|name| GroupMemberSnapshot {
                character_id: 7,
                name: name.into(),
                role: GroupRoleSnapshot::None,
                is_leader: name == "Ann",
                online: true,
                subgroup: 1,
                class: 1,
                level: 10,
                entity: None,
                portrait: Default::default(),
            })
            .to_vec(),
        loot_method: shared::loot::LootMode::PersonalLoot,
    };
    let states = GroupMemberStates {
        members: vec![GroupMemberState {
            name: "Ann".into(),
            health: 300,
            max_health: 400,
            power: None,
            death: DeathState::Alive,
            position: Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            debuffs: Vec::new(),
        }],
    };
    let joined = GroupCommandResponse {
        message: "Ann joins the party.".into(),
        code: GroupMessageCode::JoinedGroup,
    };
    macro_rules! send_and_flush {
        ($ty:ty, $value:expr) => {{
            let world = server.world_mut();
            world
                .query::<&mut MessageSender<$ty>>()
                .single_mut(world)
                .expect("one connected group sender")
                .send::<GroupChannel>($value.clone());
            server.update();
        }};
    }
    send_and_flush!(GroupCommandResponse, joined);
    send_and_flush!(GroupRosterSnapshot, roster);
    send_and_flush!(GroupMemberStates, states);
    send_and_flush!(GroupCommandResponse, joined);
    send_and_flush!(GroupRosterSnapshot, roster);
    send_and_flush!(GroupMemberStates, states);
    let send_deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < send_deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    resume.send(()).expect("resume fixture worker");
    let mut received = await_messages(&mut server, &mut host, 6).into_iter();
    macro_rules! assert_next {
        ($ty:ty, $expected:expr) => {
            assert_eq!(
                received.next().unwrap().downcast::<$ty>().ok(),
                Some($expected)
            );
        };
    }
    for _ in 0..2 {
        assert_next!(GroupCommandResponse, joined.clone());
        assert_next!(GroupRosterSnapshot, roster.clone());
        assert_next!(GroupMemberStates, states.clone());
    }
    host.stop();
}

use super::*;
use crate::replica::{Replica, Unit, UnitChange};
use bevy_replicon::bytes::Bytes;
use lightyear::prelude::{LinkOf, NetworkTarget, Replicate, ReplicationSender, server};
use shared::{
    components::{
        CombatStatus, CreatureMotion, MovementControl, Npc, Player, PlayerStandState, Position,
        Rotation, SheathState, StandState, UnitPose,
    },
    protocol::{
        CombatChannel, CombatEvent, CombatEventType, InputChannel, PlayerInput, RestChannel,
        RestSnapshot, RestStateUpdate,
    },
};
use std::{
    net::UdpSocket,
    sync::atomic::{AtomicU16, Ordering},
};

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

/// First port handed to fixture servers; must sit below the kernel's ephemeral range.
const FIXTURE_PORT_BASE: u16 = 20000;

/// `ServerUdpIo` binds its own socket and does not expose the port it gets for port zero, and an
/// ephemeral port reserved then released can be handed to a parallel test's client socket before
/// the server binds it. Fixture servers therefore take distinct ports below the ephemeral range,
/// which the kernel never assigns to port-zero binds.
fn allocate_fixture_address() -> SocketAddr {
    static NEXT_PORT: AtomicU16 = AtomicU16::new(FIXTURE_PORT_BASE);
    let ephemeral_start = ephemeral_port_range_start();
    loop {
        let port = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
        assert!(
            port < ephemeral_start,
            "fixture port {port} reached the ephemeral range starting at {ephemeral_start}"
        );
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        // Skip ports another process already holds.
        if UdpSocket::bind(address).is_ok() {
            return address;
        }
    }
}

fn ephemeral_port_range_start() -> u16 {
    let range = std::fs::read_to_string("/proc/sys/net/ipv4/ip_local_port_range")
        .expect("read kernel ephemeral port range");
    range
        .split_whitespace()
        .next()
        .and_then(|start| start.parse().ok())
        .unwrap_or_else(|| panic!("parse kernel ephemeral port range {range:?}"))
}

pub(crate) fn start_fixture_server() -> (App, SocketAddr) {
    start_fixture_server_with(|_| {})
}

pub(crate) fn start_fixture_server_with(register_extra: fn(&mut App)) -> (App, SocketAddr) {
    let address = allocate_fixture_address();
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

/// A native host: the bridge plus the `Replica` its replication events maintain.
struct Host {
    bridge: NetworkBridge,
    replica: Option<Replica>,
    despawned: Vec<u64>,
}

impl Host {
    fn connect(address: SocketAddr, client_id: u64) -> Self {
        Self {
            bridge: NetworkBridge::connect(address, client_id).expect("start fixture bridge"),
            replica: None,
            despawned: Vec::new(),
        }
    }

    /// Apply replication events; return the rest in order.
    fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        for event in self.bridge.drain_events().expect("poll fixture bridge") {
            match event {
                Event::ReplicationStarted(schema) => self.replica = Some(Replica::new(schema)),
                Event::Replication(batch) => {
                    let replica = self.replica.as_mut().expect("schema precedes replication");
                    replica.apply(batch).expect("apply fixture replication");
                    for change in replica.drain_changes() {
                        if let UnitChange::Despawned(id) = change {
                            self.despawned.push(id);
                        }
                    }
                }
                event => {
                    assert!(
                        !matches!(event, Event::Disconnected(_)),
                        "fixture disconnected"
                    );
                    events.push(event);
                }
            }
        }
        events
    }

    fn unit(&self, server_id: u64) -> Option<Unit<'_>> {
        self.replica.as_ref()?.unit(server_id)
    }

    fn stop(&mut self) {
        self.bridge.stop().expect("join fixture worker");
    }
}

fn await_bridge_event(
    server: &mut App,
    host: &mut Host,
    description: &str,
    mut matches: impl FnMut(&Event) -> bool,
) -> Event {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.update();
        for event in host.poll() {
            if matches(&event) {
                return event;
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {description}");
}

fn await_connected(server: &mut App, host: &mut Host) {
    await_bridge_event(server, host, "Netcode connection", |event| {
        matches!(event, Event::Connected)
    });
}

/// Poll until the replicated unit `server_id` satisfies `ready`.
fn await_unit(
    server: &mut App,
    host: &mut Host,
    server_id: u64,
    description: &str,
    mut ready: impl FnMut(Unit) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.update();
        host.poll();
        if host.unit(server_id).is_some_and(&mut ready) {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {description}");
}

/// The next `count` protocol messages, polled together (one drain can hold several).
fn await_messages(server: &mut App, host: &mut Host, count: usize) -> Vec<ProtocolMessage> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut messages = Vec::new();
    while Instant::now() < deadline && messages.len() < count {
        server.update();
        for event in host.poll() {
            if let Event::Message(message) = event {
                messages.push(message);
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

fn await_input(server: &mut App, host: &mut Host) -> PlayerInput {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.update();
        host.poll();
        if let Some(input) = server.world_mut().resource_mut::<ReceivedInputs>().0.pop() {
            return input;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("server did not decode PlayerInput from native bridge UDP");
}

fn await_combat(server: &mut App, host: &mut Host, server_id: u64, expected: Option<CombatStatus>) {
    await_unit(
        server,
        host,
        server_id,
        "replicated combat transition",
        |unit| unit.get::<CombatStatus>() == expected.as_ref(),
    );
}

fn send_rest(server: &mut App, update: RestStateUpdate) {
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<RestStateUpdate>>()
        .single_mut(world)
        .expect("connected fixture rest sender")
        .send::<RestChannel>(update);
}

fn await_rest(server: &mut App, host: &mut Host, expected: RestStateUpdate) {
    let event = await_bridge_event(
        server,
        host,
        "rest state update",
        |event| matches!(event, Event::Message(message) if message.is::<RestStateUpdate>()),
    );
    let Event::Message(message) = event else {
        unreachable!()
    };
    assert_eq!(message.downcast::<RestStateUpdate>().ok(), Some(expected));
}

fn fixture_player(name: &str) -> Player {
    Player {
        name: name.into(),
        race: 1,
        class: 1,
        appearance: Default::default(),
    }
}

fn position_x(unit: Unit) -> Option<f32> {
    unit.get::<Position>().map(|position| position.x)
}

#[test]
fn native_bridge_reports_protocol_rejection_instead_of_connecting() {
    use shared::protocol::ProtocolRegistrationExt;
    // The server replicates one component more than the client, like a component added to
    // `shared` after the client was built.
    let (mut server, address) = start_fixture_server_with(|app| {
        app.protocol_component::<shared::components::VerticalVelocity>()
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
        .filter_map(|event| match event {
            // Every connection starts one; it carries no server data.
            Event::ReplicationStarted(_) => None,
            Event::Connected => Some("connected".into()),
            Event::ProtocolRejected(reason) => Some(format!("rejected: {reason}")),
            Event::Disconnected(_) => Some("disconnected".into()),
            Event::Message(_) | Event::Replication(_) => Some("data".into()),
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
    let mut host = Host::connect(address, 8194);
    await_connected(&mut server, &mut host);

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
    let received = await_messages(&mut server, &mut host, expected.len());
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
        assert!(host.poll().is_empty(), "no further messages");
    }
    host.stop();
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
    let mut host = Host::connect(address, 8193);
    await_connected(&mut server, &mut host);

    let entity = server
        .world_mut()
        .spawn((
            fixture_player("Resting fighter"),
            CombatStatus(false),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let id = entity.to_bits();
    await_combat(&mut server, &mut host, id, Some(CombatStatus(false)));
    for status in [true, false] {
        server
            .world_mut()
            .entity_mut(entity)
            .insert(CombatStatus(status));
        await_combat(&mut server, &mut host, id, Some(CombatStatus(status)));
    }
    server
        .world_mut()
        .entity_mut(entity)
        .remove::<CombatStatus>();
    await_combat(&mut server, &mut host, id, None);

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
    await_rest(&mut server, &mut host, present);
    let cleared = RestStateUpdate {
        snapshot: None,
        message: None,
        error: None,
    };
    send_rest(&mut server, cleared.clone());
    await_rest(&mut server, &mut host, cleared);
    host.stop();
}

#[test]
fn native_bridge_decodes_udp_input_and_receives_control_epochs() {
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8192);
    await_connected(&mut server, &mut host);

    let input = PlayerInput {
        direction: [0.6, 0.0, 0.8],
        facing_yaw: 1.25,
        jumping: false,
        running: true,
        swimming: true,
        flying: true,
        position: [-8949.5, 112.88, 0.25],
        epoch: 7,
    };
    host.bridge
        .send::<PlayerInput, InputChannel>(input.clone())
        .expect("queue player input");
    let received = await_input(&mut server, &mut host);
    assert_eq!(received.direction, input.direction);
    assert_eq!(received.facing_yaw, input.facing_yaw);
    assert_eq!(received.jumping, input.jumping);
    assert_eq!(received.running, input.running);
    assert_eq!(received.swimming, input.swimming);
    assert_eq!(received.flying, input.flying);
    assert_eq!(received.position, input.position);
    assert_eq!(received.epoch, input.epoch);

    let first_control = MovementControl {
        epoch: 7,
        controlled: true,
    };
    let entity = server
        .world_mut()
        .spawn((
            fixture_player("UDP fixture"),
            Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            first_control,
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let id = entity.to_bits();
    await_unit(&mut server, &mut host, id, "movement control", |unit| {
        unit.get::<MovementControl>() == Some(&first_control)
    });
    let unit = host.unit(id).unwrap();
    assert_eq!(unit.get::<Player>().unwrap().name, "UDP fixture");
    assert_eq!(position_x(unit), Some(1.0));

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
    await_unit(
        &mut server,
        &mut host,
        id,
        "next movement control",
        |unit| unit.get::<MovementControl>() == Some(&next_control),
    );
    assert_eq!(position_x(host.unit(id).unwrap()), Some(20.0));

    server.world_mut().despawn(entity);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !host.despawned.contains(&id) {
        assert!(Instant::now() < deadline, "replicated unit removal");
        server.update();
        host.poll();
        thread::sleep(Duration::from_millis(5));
    }
    assert!(host.unit(id).is_none());
    host.stop();
}

/// A wandering creature's replicated `CreatureMotion` reaches the host with its unit,
/// including a stop that changes no other component.
#[test]
fn native_bridge_receives_creature_motion_changes() {
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8193);
    await_connected(&mut server, &mut host);
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
    let id = entity.to_bits();
    await_unit(&mut server, &mut host, id, "walking creature", |unit| {
        unit.get::<CreatureMotion>() == Some(&CreatureMotion::Walk)
    });
    assert_eq!(
        host.unit(id).unwrap().get::<Npc>().unwrap().name,
        "Defias Bandit"
    );
    server
        .world_mut()
        .entity_mut(entity)
        .insert(CreatureMotion::Still);
    await_unit(&mut server, &mut host, id, "stopped creature", |unit| {
        unit.get::<CreatureMotion>() == Some(&CreatureMotion::Still)
    });
    assert_eq!(position_x(host.unit(id).unwrap()), Some(-9050.0));
    host.stop();
}

/// Another player's replicated `PlayerMotion` (Retail `MovementFlags`) reaches the host
/// with its unit, including the stop that clears it without moving the player.
#[test]
fn native_bridge_receives_remote_player_motion_changes() {
    use shared::components::PlayerMotion;
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8195);
    await_connected(&mut server, &mut host);
    let strafing_jump = PlayerMotion(
        PlayerMotion::STRAFE_LEFT | PlayerMotion::FALLING | PlayerMotion::JUMP_STARTED,
    );
    let entity = server
        .world_mut()
        .spawn((
            fixture_player("Fbfps"),
            Position {
                x: -8913.0,
                y: 82.0,
                z: -140.0,
            },
            strafing_jump,
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let id = entity.to_bits();
    await_unit(&mut server, &mut host, id, "moving player", |unit| {
        unit.get::<PlayerMotion>() == Some(&strafing_jump)
    });
    assert_eq!(
        host.unit(id).unwrap().get::<Player>().unwrap().name,
        "Fbfps"
    );
    let walkoff = PlayerMotion(PlayerMotion::FORWARD | PlayerMotion::FALLING);
    server.world_mut().entity_mut(entity).insert(walkoff);
    await_unit(
        &mut server,
        &mut host,
        id,
        "unjumped falling player",
        |unit| unit.get::<PlayerMotion>() == Some(&walkoff),
    );
    server
        .world_mut()
        .entity_mut(entity)
        .insert(PlayerMotion::default());
    await_unit(&mut server, &mut host, id, "stopped player", |unit| {
        unit.get::<PlayerMotion>() == Some(&PlayerMotion::default())
    });
    assert_eq!(position_x(host.unit(id).unwrap()), Some(-8913.0));
    host.stop();
}

/// The nameplate rule inputs reach the host with the unit: FactionTemplate, UnitFlags
/// and the combat flag, including a combat drop that changes nothing else.
#[test]
fn native_bridge_receives_faction_flags_and_combat_status() {
    use shared::components::{UnitFactionTemplate, UnitFlags};
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8194);
    await_connected(&mut server, &mut host);
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
    let id = entity.to_bits();
    await_combat(&mut server, &mut host, id, Some(CombatStatus(true)));
    let unit = host.unit(id).unwrap();
    assert_eq!(
        unit.get::<UnitFactionTemplate>(),
        Some(&UnitFactionTemplate(7))
    );
    assert_eq!(
        unit.get::<UnitFlags>(),
        Some(&UnitFlags(UnitFlags::NOT_SELECTABLE))
    );
    server
        .world_mut()
        .entity_mut(entity)
        .insert(CombatStatus(false));
    await_combat(&mut server, &mut host, id, Some(CombatStatus(false)));
    assert_eq!(
        host.unit(id).unwrap().get::<UnitFactionTemplate>(),
        Some(&UnitFactionTemplate(7))
    );
    host.stop();
}

/// A creature's replicated `UnitPose` reaches the host with its unit, and a pose-only
/// change (Stockade guard drawing its sword, a criminal waking) arrives on its own.
#[test]
fn native_bridge_receives_unit_pose_changes() {
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8194);
    await_connected(&mut server, &mut host);
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
    let id = entity.to_bits();
    await_unit(&mut server, &mut host, id, "sleeping criminal", |unit| {
        unit.get::<UnitPose>() == Some(&asleep)
    });
    assert_eq!(
        host.unit(id).unwrap().get::<Npc>().unwrap().name,
        "Petty Criminal"
    );
    let ready = UnitPose {
        stand_state: StandState::Stand,
        sheath_state: SheathState::Melee,
        emote_state: 333,
    };
    server.world_mut().entity_mut(entity).insert(ready);
    await_unit(&mut server, &mut host, id, "standing criminal", |unit| {
        unit.get::<UnitPose>() == Some(&ready)
    });
    assert_eq!(position_x(host.unit(id).unwrap()), Some(100.0));
    host.stop();
}

/// A player's replicated `PlayerStandState` reaches the host with its unit, and a
/// stand-state-only change (sitting down, then kneeling) arrives on its own.
#[test]
fn native_bridge_receives_player_stand_state_changes() {
    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8195);
    await_connected(&mut server, &mut host);
    let entity = server
        .world_mut()
        .spawn((
            fixture_player("Sitter"),
            Position {
                x: 100.0,
                y: 5.0,
                z: 1.0,
            },
            PlayerStandState(StandState::Sit),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let id = entity.to_bits();
    await_unit(&mut server, &mut host, id, "sitting player", |unit| {
        unit.get::<PlayerStandState>() == Some(&PlayerStandState(StandState::Sit))
    });
    server
        .world_mut()
        .entity_mut(entity)
        .insert(PlayerStandState(StandState::Kneel));
    await_unit(&mut server, &mut host, id, "kneeling player", |unit| {
        unit.get::<PlayerStandState>() == Some(&PlayerStandState(StandState::Kneel))
    });
    assert_eq!(position_x(host.unit(id).unwrap()), Some(100.0));
    host.stop();
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
    let mut host = Host::connect(address, 8195);
    await_connected(&mut server, &mut host);
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
    let received = await_messages(&mut server, &mut host, 3);
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
    host.stop();
}

/// A vendor's replicated `NpcFlags` and the player's `Gold` reach the host with their
/// units, and the server's `VendorInventory` arrives as its protocol message.
#[test]
fn native_bridge_receives_vendor_flags_gold_and_inventory() {
    use shared::components::Gold;
    use shared::protocol::{MerchantChannel, NpcFlags, VendorInventory, VendorItem};

    let (mut server, address) = start_fixture_server();
    let mut host = Host::connect(address, 8195);
    await_connected(&mut server, &mut host);
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
            fixture_player("Fbworldmap"),
            Gold(12_345),
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    await_unit(
        &mut server,
        &mut host,
        player.to_bits(),
        "player gold",
        |unit| unit.has::<Gold>(),
    );
    await_unit(
        &mut server,
        &mut host,
        vendor.to_bits(),
        "vendor flags",
        |unit| unit.has::<NpcFlags>(),
    );
    let vendor_unit = host.unit(vendor.to_bits()).unwrap();
    assert_eq!(
        vendor_unit.get::<NpcFlags>(),
        Some(&NpcFlags(NpcFlags::VENDOR | NpcFlags::REPAIR))
    );
    assert_eq!(vendor_unit.get::<Gold>(), None);
    assert_eq!(
        host.unit(player.to_bits()).unwrap().get::<Gold>(),
        Some(&Gold(12_345))
    );
    let inventory = VendorInventory {
        npc: vendor.to_bits(),
        can_repair: true,
        guild_repair_money: None,
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
            max_durability: None,
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
        &mut host,
        "vendor inventory",
        |event| matches!(event, Event::Message(message) if message.is::<VendorInventory>()),
    ) else {
        unreachable!()
    };
    assert_eq!(message.downcast::<VendorInventory>().ok(), Some(inventory));
    host.stop();
}

#[derive(Resource, Default)]
struct ReceivedAcks(Vec<Bytes>);

fn record_acks(
    messages: Res<bevy_replicon::prelude::ServerMessages>,
    mut acks: ResMut<ReceivedAcks>,
) {
    let channel = bevy_replicon::shared::backend::channels::ClientChannel::MutationAcks;
    acks.0.extend(
        messages
            .iter_received(channel)
            .map(|(_, bytes)| bytes.clone()),
    );
}

/// The server receives the worker's `MutationAcks` for the mutate messages it sent: whole
/// fixint `MutateIndex` values, as replicon's client sends them.
#[test]
fn native_bridge_acknowledges_mutate_messages_to_the_server() {
    use bevy_replicon::server::ServerSystems;
    let (mut server, address) = start_fixture_server_with(|app| {
        app.init_resource::<ReceivedAcks>();
        app.add_systems(
            PreUpdate,
            record_acks
                .after(ServerSystems::ReceivePackets)
                .before(ServerSystems::Receive),
        );
    });
    let mut host = Host::connect(address, 8196);
    await_connected(&mut server, &mut host);
    let entity = server
        .world_mut()
        .spawn((
            fixture_player("Fback"),
            Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    let id = entity.to_bits();
    await_unit(&mut server, &mut host, id, "spawned player", |unit| {
        position_x(unit) == Some(1.0)
    });
    server.world_mut().resource_mut::<ReceivedAcks>().0.clear();
    server.world_mut().entity_mut(entity).insert(Position {
        x: 5.0,
        y: 2.0,
        z: 3.0,
    });
    await_unit(&mut server, &mut host, id, "mutated player", |unit| {
        position_x(unit) == Some(5.0)
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<ReceivedAcks>().0.is_empty() {
        assert!(
            Instant::now() < deadline,
            "server received no acknowledgments"
        );
        server.update();
        host.poll();
        thread::sleep(Duration::from_millis(5));
    }
    let acks = &server.world().resource::<ReceivedAcks>().0;
    assert!(acks.iter().all(|ack| !ack.is_empty() && ack.len() % 2 == 0));
    host.stop();
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
    fn received<M: network::Message>(server: &mut App, host: &mut Host) -> M {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            server.update();
            host.poll();
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
        host: &mut Host,
        expected: M,
    ) {
        let mut senders = server.world_mut().query::<&mut MessageSender<M>>();
        for mut sender in senders.iter_mut(server.world_mut()) {
            sender.send::<AuctionChannel>(expected.clone());
        }
        let Event::Message(message) = await_bridge_event(
            server,
            host,
            "auction relay",
            |e| matches!(e,Event::Message(m) if m.is::<M>()),
        ) else {
            panic!()
        };
        assert_eq!(message.downcast::<M>().ok(), Some(expected));
    }
    let (mut server, address) = start_fixture_server_with(install_auction);
    let mut host = Host::connect(address, 9088);
    await_connected(&mut server, &mut host);
    macro_rules! request {
        ($value:expr,$kind:ty) => {{
            let value = $value;
            host.bridge
                .send::<_, AuctionChannel>(value.clone())
                .unwrap();
            assert_eq!(received::<$kind>(&mut server, &mut host), value);
        }};
    }
    request!(OpenAuctionHouse, OpenAuctionHouse);
    let query = AuctionSearchQuery {
        text: "linen".into(),
        item_id: Some(2589),
        class_id: Some(7),
        subcategory_filters: Vec::new(),
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
        &mut host,
        AuctionBrowseResults {
            query: browse_query,
            total_results: 103,
            items: vec![AuctionBrowseItem {
                definition_source: shared::item_data::ItemDefinitionSource::Retail,
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
        &mut host,
        AuctionHouseOpened {
            success: true,
            error: None,
        },
    );
    reply(
        &mut server,
        &mut host,
        AuctionSearchResults {
            query,
            total_results: 103,
            results: vec![],
        },
    );
    reply(
        &mut server,
        &mut host,
        AuctionInventorySnapshot {
            gold: 1000,
            items: vec![],
        },
    );
    reply(
        &mut server,
        &mut host,
        OwnedAuctionListResponse { listings: vec![] },
    );
    reply(
        &mut server,
        &mut host,
        BidAuctionListResponse { listings: vec![] },
    );
    reply(
        &mut server,
        &mut host,
        AuctionOperationResponse {
            success: false,
            message: "not interacting with an auctioneer".into(),
        },
    );
    host.stop();
}
