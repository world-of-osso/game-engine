//! Headless Lightyear transport for a native Godot host. No render/UI Bevy plugins.
//! Wire schemas and channel registration come exclusively from `shared::ProtocolPlugin`.

use std::{
    any::Any,
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    ecs::resource::IsResource,
    prelude::*,
    state::app::StatesPlugin,
};
use bevy_replicon::{
    client::confirm_history::EntityReplicated, shared::server_entity_map::ServerEntityMap,
};
use lightyear::prelude::{
    self as network, MessageReceiver, MessageSender, client as client_network,
};
use shared::{
    casting::CastState,
    components::{
        CombatStatus, CreatureMotion, EquipmentAppearance, Gold, Health, Mana, ModelDisplay,
        MovementControl, MovementSpeed, Npc, Player, PlayerMotion, Position, Rotation, UnitAuras,
        UnitFactionTemplate, UnitFlags, UnitLevel, UnitPose, UnitPowers, UnitRunes, UnitTarget,
        UnitThreatList,
    },
    level_scaling::LevelScaling,
    protocol::{
        self, ActionBarSnapshot, AttackStart, AttackStopped, BuybackList, CastFailed,
        CharacterListUpdate, ChatMessage, CombatEvent, CombatLogEvent, CreateCharacterResponse,
        DamageMeterSnapshot, DeleteCharacterResponse, DungeonDifficultySet, DurabilityStateUpdate,
        EnterWorldResponse, ForcedDisconnect, InstanceInfo, InteractionClosed, InteractionFailed,
        InteractionOpened, InventoryDelta, InventoryError, InventorySnapshot, KnownSpellsSnapshot,
        LoadTerrain, LoginResponse, MerchantFailed, MirrorTimerPause, MirrorTimerStart,
        MirrorTimerStop, NewWorld, NpcFlags, QuestFailed, QuestGiverStatusMultiple,
        QuestLogSnapshot, QuestLogUpdate, RegisterResponse, RestStateUpdate, SpecializationChanged,
        SpellCooldownUpdate, SpellGo, SpellsLearned, SpellsUnlearned, TransferAborted,
        VendorInventory,
    },
};

/// Trait bound for decoding messages carried by this transport boundary.
pub use lightyear::prelude::Message as WireMessage;

const NETWORK_HZ: u128 = 60;
const NANOS_PER_SECOND: u128 = 1_000_000_000;
const SIMULATION_INTERVAL: Duration = Duration::from_millis(50);
type WorkerCommand = Box<dyn FnOnce(&mut World) + Send>;

enum Command {
    Apply(WorkerCommand),
    Stop,
}

/// Owned decoded protocol message. Downcast to the original `shared::protocol` type.
pub struct ProtocolMessage(Box<dyn Any + Send>);

impl ProtocolMessage {
    pub fn downcast<M: network::Message>(self) -> Result<M, Self> {
        self.0.downcast::<M>().map(|message| *message).map_err(Self)
    }

    pub fn is<M: network::Message>(&self) -> bool {
        self.0.is::<M>()
    }
}

/// Full state for one replicated player/NPC at one server notification.
/// `server_id` is the server Entity bits including generation, not a Godot node ID.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitSnapshot {
    pub server_id: u64,
    pub player: Option<Player>,
    pub npc: Option<Npc>,
    pub position: Option<Position>,
    pub rotation: Option<Rotation>,
    pub health: Option<Health>,
    pub mana: Option<Mana>,
    pub model: Option<ModelDisplay>,
    pub level: Option<UnitLevel>,
    /// A tuned creature's ContentTuning range: the level and health each viewer sees.
    pub level_scaling: Option<LevelScaling>,
    pub equipment: Option<EquipmentAppearance>,
    pub movement_control: Option<MovementControl>,
    /// Server speed (yd/s) for the unit's newest applied movement: base × auras × direction.
    pub movement_speed: Option<MovementSpeed>,
    /// A creature's stand/walk/run; players carry none.
    pub creature_motion: Option<CreatureMotion>,
    /// A player's Retail `MovementFlags` from its newest applied input; creatures carry none.
    pub player_motion: Option<PlayerMotion>,
    /// A creature's stand, sheath and emote state (`creature_addon`); players carry none.
    pub unit_pose: Option<UnitPose>,
    /// Server entity bits of the unit's own target (`SetTarget` echo for players).
    pub unit_target: Option<u64>,
    /// Server entity bits of the units on a creature's threat list; empty for players.
    pub threat_list: Vec<u64>,
    /// Retail `FactionTemplate` id, for reaction to the local player.
    pub faction_template: Option<u32>,
    /// `UNIT_FIELD_FLAGS` bits.
    pub unit_flags: Option<u32>,
    /// Replicated `CombatStatus`.
    pub in_combat: bool,
    /// The cast or channel in progress; the server removes it on completion or interrupt.
    pub cast: Option<CastState>,
    /// Raw DB2 power values, primary power first.
    pub powers: Option<UnitPowers>,
    /// A death knight's per-rune recharge (`GetRuneCooldown`).
    pub runes: Option<UnitRunes>,
    pub auras: Option<UnitAuras>,
    /// Retail `NPCFlags` / `NPCFlags2` bits of an NPC (vendor, repair, gossip, ...).
    pub npc_flags: Option<u64>,
    /// The local player's money in copper; other units carry none.
    pub gold: Option<u64>,
    pub combat_status: Option<CombatStatus>,
}

impl UnitSnapshot {
    fn capture(server_id: u64, entity: EntityRef) -> Self {
        Self {
            server_id,
            player: entity.get::<Player>().cloned(),
            npc: entity.get::<Npc>().cloned(),
            position: entity.get::<Position>().copied(),
            rotation: entity.get::<Rotation>().copied(),
            health: entity.get::<Health>().copied(),
            mana: entity.get::<Mana>().copied(),
            model: entity.get::<ModelDisplay>().copied(),
            level: entity.get::<UnitLevel>().copied(),
            level_scaling: entity.get::<LevelScaling>().copied(),
            equipment: entity.get::<EquipmentAppearance>().cloned(),
            movement_control: entity.get::<MovementControl>().copied(),
            movement_speed: entity.get::<MovementSpeed>().copied(),
            creature_motion: entity.get::<CreatureMotion>().copied(),
            player_motion: entity.get::<PlayerMotion>().copied(),
            unit_pose: entity.get::<UnitPose>().copied(),
            unit_target: entity.get::<UnitTarget>().and_then(|target| target.0),
            threat_list: entity
                .get::<UnitThreatList>()
                .map_or_else(Vec::new, |list| list.0.clone()),
            faction_template: entity
                .get::<UnitFactionTemplate>()
                .map(|template| template.0),
            unit_flags: entity.get::<UnitFlags>().map(|flags| flags.0),
            in_combat: entity.get::<CombatStatus>().is_some_and(|status| status.0),
            cast: entity.get::<CastState>().cloned(),
            powers: entity.get::<UnitPowers>().cloned(),
            runes: entity.get::<UnitRunes>().cloned(),
            auras: entity.get::<UnitAuras>().cloned(),
            npc_flags: entity.get::<NpcFlags>().map(|flags| flags.0),
            gold: entity.get::<Gold>().map(|gold| gold.0),
            combat_status: entity.get::<CombatStatus>().copied(),
        }
    }
}

pub enum Event {
    /// The server's protocol fingerprint matched; the connection is usable.
    Connected,
    /// Client and server protocols differ; `Disconnected` follows once the link drops.
    ProtocolRejected(String),
    Disconnected(Option<String>),
    Message(ProtocolMessage),
    UnitUpdated(UnitSnapshot),
    UnitRemoved(u64),
}

/// Register any server-to-client type already registered by `shared::ProtocolPlugin`.
/// A new connection owns an independent FIFO and replication identity map.
#[derive(Default)]
pub struct BridgeConfig {
    relays: Vec<fn(&mut App, Sender<Event>)>,
}

impl BridgeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn receive<M: network::Message>(mut self) -> Self {
        self.relays.push(install_relay::<M>);
        self
    }

    /// `MirrorTimerStart`, `MirrorTimerPause` and `MirrorTimerStop` in the order the server
    /// sent them on `MirrorTimerChannel`.
    pub fn receive_mirror_timers(mut self) -> Self {
        self.relays.push(install_mirror_timer_relay);
        self
    }

    pub fn connect(self, server_addr: SocketAddr, client_id: u64) -> Result<NetworkBridge, String> {
        NetworkBridge::start(server_addr, client_id, self.relays)
    }
}

/// Joinable network worker; the caller polls owned events without owning a Bevy world.
pub struct NetworkBridge {
    commands: Sender<Command>,
    events: Receiver<Event>,
    worker: Option<JoinHandle<Result<(), String>>>,
}

impl NetworkBridge {
    /// Account/roster flow. Use `BridgeConfig` to subscribe to additional protocol types.
    pub fn connect(server_addr: SocketAddr, client_id: u64) -> Result<Self, String> {
        BridgeConfig::new()
            .receive::<LoginResponse>()
            .receive::<RegisterResponse>()
            .receive::<ForcedDisconnect>()
            .receive::<CharacterListUpdate>()
            .receive::<CreateCharacterResponse>()
            .receive::<DeleteCharacterResponse>()
            .receive::<EnterWorldResponse>()
            .receive::<LoadTerrain>()
            .receive::<NewWorld>()
            .receive::<TransferAborted>()
            // Quest log for the world map's quest areas and the objective tracker; quest
            // giver markers for the minimap.
            .receive::<QuestLogSnapshot>()
            .receive::<QuestLogUpdate>()
            .receive::<QuestGiverStatusMultiple>()
            .receive::<QuestFailed>()
            // Dungeon difficulty and saved instances for the entrance difficulty bar.
            .receive::<DungeonDifficultySet>()
            .receive::<InstanceInfo>()
            // Server-driven breath, fatigue and feign-death bars.
            .receive_mirror_timers()
            // Spellbook, action bar and casting.
            .receive::<KnownSpellsSnapshot>()
            .receive::<SpellsLearned>()
            .receive::<SpellsUnlearned>()
            .receive::<SpecializationChanged>()
            .receive::<ActionBarSnapshot>()
            .receive::<SpellCooldownUpdate>()
            .receive::<CastFailed>()
            .receive::<CombatLogEvent>()
            // Server-computed damage meter sessions.
            .receive::<DamageMeterSnapshot>()
            // Melee swing outcomes and resolved casts of every replicated unit, for
            // combat animations and spell visuals.
            .receive::<CombatEvent>()
            .receive::<SpellGo>()
            // Auto-attack starts and stops of every replicated unit.
            .receive::<AttackStart>()
            .receive::<AttackStopped>()
            // NPC interaction, the merchant frame and the bags it sells from.
            .receive::<InteractionOpened>()
            .receive::<InteractionFailed>()
            .receive::<InteractionClosed>()
            .receive::<protocol::AuctionHouseOpened>()
            .receive::<protocol::AuctionSearchResults>()
            .receive::<protocol::AuctionInventorySnapshot>()
            .receive::<protocol::OwnedAuctionListResponse>()
            .receive::<protocol::BidAuctionListResponse>()
            .receive::<protocol::AuctionOperationResponse>()
            .receive::<VendorInventory>()
            .receive::<BuybackList>()
            .receive::<MerchantFailed>()
            .receive::<InventorySnapshot>()
            .receive::<InventoryDelta>()
            .receive::<InventoryError>()
            .receive::<DurabilityStateUpdate>()
            .receive::<RestStateUpdate>()
            // Chat lines for the chat frame.
            .receive::<ChatMessage>()
            .connect(server_addr, client_id)
    }

    fn start(
        server_addr: SocketAddr,
        client_id: u64,
        relays: Vec<fn(&mut App, Sender<Event>)>,
    ) -> Result<Self, String> {
        let (commands, incoming) = mpsc::channel();
        let (outgoing, events) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("godot-network-60hz".into())
            .spawn(move || run_worker(incoming, outgoing, server_addr, client_id, relays))
            .map_err(|error| format!("failed to spawn network worker: {error}"))?;
        Ok(Self {
            commands,
            events,
            worker: Some(worker),
        })
    }

    /// Sends an owned registered `shared` message on its registered protocol channel.
    /// The worker buffers outgoing messages until Netcode establishes a connection.
    pub fn send<M: network::Message, C: network::Channel>(&self, message: M) -> Result<(), String> {
        self.enqueue(move |world| {
            let mut sender = world
                .query::<&mut MessageSender<M>>()
                .single_mut(world)
                .unwrap_or_else(|error| {
                    panic!("missing sender for {}: {error}", std::any::type_name::<M>())
                });
            sender.send::<C>(message);
        })
    }

    fn enqueue(&self, command: impl FnOnce(&mut World) + Send + 'static) -> Result<(), String> {
        if self.worker.is_none() {
            return Err("network worker already stopped".into());
        }
        self.commands
            .send(Command::Apply(Box::new(command)))
            .map_err(|_| "network worker command queue disconnected".into())
    }

    /// Request Netcode disconnect; keep polling to observe the server/transport outcome.
    pub fn disconnect(&self) -> Result<(), String> {
        self.enqueue(|world| {
            let entity = world
                .query_filtered::<Entity, With<client_network::NetcodeClient>>()
                .single(world)
                .unwrap_or_else(|error| panic!("expected one Netcode client: {error}"));
            world.trigger(client_network::Disconnect { entity });
        })
    }

    /// Drain all currently delivered events in FIFO order; never synthesizes auth success.
    pub fn drain_events(&mut self) -> Result<Vec<Event>, String> {
        let mut events = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty) => return Ok(events),
                Err(TryRecvError::Disconnected) if self.worker.is_none() => return Ok(events),
                Err(TryRecvError::Disconnected) => {
                    self.join_worker()?;
                    return Ok(events);
                }
            }
        }
    }

    fn join_worker(&mut self) -> Result<(), String> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        worker.join().map_err(describe_panic)?
    }

    /// Stop and join; dropping queued old connection events/identities is host responsibility.
    pub fn stop(&mut self) -> Result<(), String> {
        if self.worker.is_none() {
            return Ok(());
        }
        let _ = self.commands.send(Command::Stop);
        let result = self.join_worker();
        while self.events.try_recv().is_ok() {}
        result
    }
}

impl Drop for NetworkBridge {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("network worker shutdown failed: {error}");
        }
    }
}

fn run_worker(
    commands: Receiver<Command>,
    events: Sender<Event>,
    server_addr: SocketAddr,
    client_id: u64,
    relays: Vec<fn(&mut App, Sender<Event>)>,
) -> Result<(), String> {
    let mut app = App::new();
    app.set_error_handler(protocol::defer_lightyear_protocol_check);
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(client_network::ClientPlugins {
        tick_duration: SIMULATION_INTERVAL,
    });
    app.add_plugins(shared::ProtocolPlugin);
    for relay in relays {
        relay(&mut app, events.clone());
    }
    install_lifecycle(&mut app, events.clone());
    install_replication(&mut app, events);
    connect_transport(app.world_mut(), server_addr, client_id)?;
    app.finish();
    app.cleanup();
    let started = Instant::now();
    while apply_commands(app.world_mut(), &commands)? {
        app.update();
        if let Some(exit) = app.should_exit() {
            return match exit {
                AppExit::Success => Ok(()),
                AppExit::Error(code) => Err(format!("network application exited with {code}")),
            };
        }
        thread::sleep(next_tick_delay(started.elapsed()));
    }
    Ok(())
}

fn connect_transport(
    world: &mut World,
    server_addr: SocketAddr,
    client_id: u64,
) -> Result<(), String> {
    let auth = network::Authentication::Manual {
        server_addr,
        client_id,
        private_key: [0; 32],
        protocol_id: 0,
    };
    let netcode = client_network::NetcodeClient::new(
        auth,
        client_network::NetcodeConfig {
            client_timeout_secs: 60,
            ..default()
        },
    )
    .map_err(|error| format!("failed to construct Netcode client: {error}"))?;
    let entity = world
        .spawn((
            network::LocalAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
            network::PeerAddr(server_addr),
            network::UdpIo::default(),
            netcode,
        ))
        .id();
    world.trigger(client_network::Connect { entity });
    Ok(())
}

fn install_lifecycle(app: &mut App, events: Sender<Event>) {
    app.add_observer(
        |connected: On<Add, client_network::Connected>, mut commands: Commands| {
            commands
                .entity(connected.entity)
                .insert(network::ReplicationReceiver);
        },
    );
    app.add_systems(
        PostUpdate,
        move |verified: Query<(), Added<protocol::ProtocolVerified>>,
              rejected: Query<&protocol::ProtocolRejected, Added<protocol::ProtocolRejected>>,
              disconnected: Query<
            &client_network::Disconnected,
            Added<client_network::Disconnected>,
        >| {
            for () in &verified {
                events
                    .send(Event::Connected)
                    .expect("host event receiver closed");
            }
            for protocol::ProtocolRejected(reason) in &rejected {
                events
                    .send(Event::ProtocolRejected(reason.clone()))
                    .expect("host event receiver closed");
            }
            for state in &disconnected {
                events
                    .send(Event::Disconnected(state.reason.clone()))
                    .expect("host event receiver closed");
            }
        },
    );
}

/// Nothing reaches the host from a connection whose protocol was rejected, during the
/// grace before the link drops.
fn protocol_not_rejected(rejected: Query<(), With<protocol::ProtocolRejected>>) -> bool {
    rejected.is_empty()
}

fn install_relay<M: network::Message>(app: &mut App, events: Sender<Event>) {
    app.add_systems(
        Update,
        (move |mut receivers: Query<&mut MessageReceiver<M>>| {
            for mut receiver in &mut receivers {
                for message in receiver.receive() {
                    events
                        .send(Event::Message(ProtocolMessage(Box::new(message))))
                        .expect("host event receiver closed");
                }
            }
        })
        .run_if(protocol_not_rejected),
    );
}

/// One relay for the three mirror timer types, sorted by their `MirrorTimerChannel` message
/// id: a receiver per type would hand one frame's messages over in system order, so a stop
/// could overtake the start sent before it.
fn install_mirror_timer_relay(app: &mut App, events: Sender<Event>) {
    app.add_systems(
        Update,
        (move |mut starts: Query<&mut MessageReceiver<MirrorTimerStart>>,
               mut pauses: Query<&mut MessageReceiver<MirrorTimerPause>>,
               mut stops: Query<&mut MessageReceiver<MirrorTimerStop>>| {
            let mut received = Vec::new();
            for mut receiver in &mut starts {
                received.extend(
                    receiver.receive_with_tick().map(|message| {
                        (message.message_id, ProtocolMessage(Box::new(message.data)))
                    }),
                );
            }
            for mut receiver in &mut pauses {
                received.extend(
                    receiver.receive_with_tick().map(|message| {
                        (message.message_id, ProtocolMessage(Box::new(message.data)))
                    }),
                );
            }
            for mut receiver in &mut stops {
                received.extend(
                    receiver.receive_with_tick().map(|message| {
                        (message.message_id, ProtocolMessage(Box::new(message.data)))
                    }),
                );
            }
            received.sort_by_key(|(id, _)| *id);
            for (_, message) in received {
                events
                    .send(Event::Message(message))
                    .expect("host event receiver closed");
            }
        })
        .run_if(protocol_not_rejected),
    );
}

#[derive(Resource, Default)]
struct ReplicatedIds(HashMap<Entity, u64>);

fn install_replication(app: &mut App, events: Sender<Event>) {
    app.init_resource::<ReplicatedIds>();
    app.add_systems(
        Update,
        (move |mut changes: MessageReader<EntityReplicated>,
               mut removed: RemovedComponents<client_network::Remote>,
               entities: Query<EntityRef, (With<client_network::Remote>, Without<IsResource>)>,
               server_ids: Res<ServerEntityMap>,
               mut known: ResMut<ReplicatedIds>| {
            for worker_entity in removed.read() {
                if let Some(server_id) = known.0.remove(&worker_entity) {
                    events
                        .send(Event::UnitRemoved(server_id))
                        .expect("host event receiver closed");
                }
            }
            let mut seen = HashSet::new();
            for change in changes.read() {
                let worker_entity = change.entity;
                if !seen.insert(worker_entity) {
                    continue;
                }
                let Ok(entity) = entities.get(worker_entity) else {
                    continue;
                };
                let Some(server) = server_ids.to_server().get(&worker_entity) else {
                    panic!("replicated entity {worker_entity:?} has no server identity");
                };
                let server_id = server.to_bits();
                if entity.get::<Player>().is_some() || entity.get::<Npc>().is_some() {
                    known.0.insert(worker_entity, server_id);
                    let snapshot = UnitSnapshot::capture(server_id, entity);
                    events
                        .send(Event::UnitUpdated(snapshot))
                        .expect("host event receiver closed");
                } else if known.0.remove(&worker_entity).is_some() {
                    events
                        .send(Event::UnitRemoved(server_id))
                        .expect("host event receiver closed");
                }
            }
        })
        .run_if(protocol_not_rejected),
    );
}

fn apply_commands(world: &mut World, commands: &Receiver<Command>) -> Result<bool, String> {
    loop {
        match commands.try_recv() {
            Ok(Command::Apply(command)) => command(world),
            Ok(Command::Stop) => return Ok(false),
            Err(TryRecvError::Empty) => return Ok(true),
            Err(TryRecvError::Disconnected) => {
                return Err("network command queue disconnected without stop".into());
            }
        }
    }
}

fn next_tick_delay(elapsed: Duration) -> Duration {
    let elapsed_nanos = elapsed.as_nanos();
    let next_tick = elapsed_nanos * NETWORK_HZ / NANOS_PER_SECOND + 1;
    let deadline = (next_tick * NANOS_PER_SECOND).div_ceil(NETWORK_HZ);
    Duration::from_nanos((deadline - elapsed_nanos) as u64)
}

fn describe_panic(payload: Box<dyn Any + Send>) -> String {
    let message = if let Some(message) = payload.downcast_ref::<String>() {
        message.as_str()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else {
        "non-string panic payload"
    };
    format!("network worker panicked: {message}")
}

#[cfg(test)]
mod wire_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{
        components::MovementControl,
        protocol::{AuthChannel, LoginRequest},
    };
    use std::{
        net::UdpSocket,
        time::{Duration, Instant},
    };

    #[test]
    fn protocol_event_downcasts_to_authoritative_type() {
        let message = ProtocolMessage(Box::new(LoginResponse {
            success: false,
            token: String::new(),
            characters: vec![],
            error: Some("invalid credentials".into()),
        }));
        assert!(message.is::<LoginResponse>());
        let reply = message.downcast::<LoginResponse>().ok().unwrap();
        assert_eq!(reply.error.as_deref(), Some("invalid credentials"));
    }

    #[test]
    fn unit_snapshot_owns_server_identity_and_component_values() {
        let mut world = World::new();
        let entity = world
            .spawn((
                Npc {
                    template_id: 42,
                    name: "Loup — Écorché".into(),
                },
                Position {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                Health {
                    current: 8.0,
                    max: 10.0,
                },
                MovementControl {
                    epoch: 7,
                    controlled: true,
                },
                MovementSpeed(3.5),
            ))
            .id();
        let server_id = world.spawn_empty().id().to_bits();
        let snapshot = UnitSnapshot::capture(server_id, world.entity(entity));
        world.despawn(entity);
        assert_eq!(snapshot.server_id, server_id);
        assert_eq!(snapshot.npc.unwrap().name, "Loup — Écorché");
        assert_eq!(snapshot.health.unwrap().current, 8.0);
        assert_eq!(snapshot.movement_speed, Some(MovementSpeed(3.5)));
        assert_eq!(
            snapshot.movement_control,
            Some(MovementControl {
                epoch: 7,
                controlled: true,
            })
        );

        let entity_without_control = world.spawn_empty().id();
        let absent = UnitSnapshot::capture(server_id, world.entity(entity_without_control));
        world.despawn(entity_without_control);
        assert_eq!(absent.movement_control, None);
        assert_eq!(absent.movement_speed, None);
    }

    #[test]
    fn worker_sends_udp_while_host_does_not_poll_and_restarts_with_new_connection() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bridge = NetworkBridge::connect(server.local_addr().unwrap(), 123).unwrap();
        let mut packet = [0; 2048];
        let (size, first_peer) = server.recv_from(&mut packet).unwrap();
        assert!(size > 0);
        let (_, second_peer) = server.recv_from(&mut packet).unwrap();
        assert_eq!(first_peer, second_peer);
        bridge.stop().unwrap();
        assert!(
            bridge
                .send::<LoginRequest, AuthChannel>(LoginRequest {
                    token: None,
                    username: "x".into(),
                    password: "y".into()
                })
                .is_err()
        );
        let mut bridge = NetworkBridge::connect(server.local_addr().unwrap(), 124).unwrap();
        let (_, _) = server.recv_from(&mut packet).unwrap();
        bridge.stop().unwrap();
    }

    #[test]
    fn worker_commands_keep_fifo_without_polling_host_and_no_fake_auth_reply() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut bridge = NetworkBridge::connect(server.local_addr().unwrap(), 789).unwrap();
        let (sent, received) = mpsc::channel();
        for number in [3, 1, 4] {
            let sent = sent.clone();
            bridge.enqueue(move |_| sent.send(number).unwrap()).unwrap();
        }
        let values: Vec<_> = (0..3)
            .map(|_| received.recv_timeout(Duration::from_secs(5)).unwrap())
            .collect();
        assert_eq!(values, [3, 1, 4]);
        bridge
            .send::<LoginRequest, AuthChannel>(LoginRequest {
                token: None,
                username: "first".into(),
                password: "one".into(),
            })
            .unwrap();
        // Only a real server reply may produce an auth event.
        let deadline = Instant::now() + Duration::from_millis(200);
        while Instant::now() < deadline {
            assert!(
                !bridge
                    .drain_events()
                    .unwrap()
                    .iter()
                    .any(|event| matches!(event, Event::Message(_)))
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        bridge.stop().unwrap();
    }
}
