//! Headless Lightyear transport for a native Godot host. No render/UI Bevy plugins.
//! Wire schemas and channel registration come exclusively from `shared::ProtocolPlugin`.

#[path = "../../../src/ui/automation_data.rs"]
pub mod automation_data;
#[path = "../../../src/game/state/game_state_enum.rs"]
pub mod game_state_enum;
#[path = "../../../src/input_bindings_bevy_data.rs"]
pub mod input_bindings_bevy_data;
#[path = "../../../src/input_bindings_data.rs"]
pub mod input_bindings_data;
#[path = "../../../src/ipc/wire.rs"]
pub mod ipc_wire;
#[path = "../../../src/ui/js_automation.rs"]
pub mod js_automation;
#[path = "../../../src/movement_control.rs"]
pub mod movement_control;
pub mod replica;
#[path = "../../../src/screen_arg_data.rs"]
pub mod screen_arg_data;

use std::{
    any::Any,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    prelude::*,
    state::app::StatesPlugin,
};
use lightyear::prelude::{
    self as network, Client, MessageReceiver, MessageSender, Transport, client as client_network,
};
use lightyear_replication::{LightyearRepliconClientBackend, channels::RepliconChannelMap};
use lightyear_transport::plugin::TransportSystems;
use replica::{ReplicationBatch, Schema};
use shared::protocol::{
    self, ActionBarSnapshot, AttackStart, AttackStopped, BuybackList, CastFailed,
    CharacterListUpdate, ChatMessage, CombatEvent, CombatLogEvent, CreateCharacterResponse,
    DamageMeterSnapshot, DeleteCharacterResponse, DungeonDifficultySet, DurabilityStateUpdate,
    EnterWorldResponse, EquipmentSnapshot, ForcedDisconnect, InstanceInfo, InteractionClosed,
    InteractionFailed, InteractionOpened, InventoryDelta, InventoryError, InventorySnapshot,
    KnownSpellsSnapshot, LoadTerrain, LoginResponse, MerchantFailed, MirrorTimerPause,
    MirrorTimerStart, MirrorTimerStop, NewWorld, QuestFailed, QuestGiverStatusMultiple,
    QuestLogSnapshot, QuestLogUpdate, RegisterResponse, RestStateUpdate, SpecializationChanged,
    SpellCooldownUpdate, SpellFailure, SpellGo, SpellsLearned, SpellsUnlearned, TransferAborted,
    VendorInventory,
};

/// Trait bound for decoding messages carried by this transport boundary.
pub use lightyear::prelude::Message as WireMessage;

/// Seconds without a packet from the server before Netcode drops the link, in both
/// directions: the server reads this timeout from the client's connect token. The
/// vendored Netcode server issues its own tokens with 10 s (`CLIENT_TIMEOUT_SECS`,
/// vendor/lightyear_netcode/src/server.rs:34).
const CLIENT_TIMEOUT_SECS: i32 = 10;
/// How long the Netcode handshake (connection request and challenge) may take before the
/// client gives up. A live local or remote server answers within a round trip.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// The `Disconnected` reason of a handshake that ran out of `HANDSHAKE_TIMEOUT`.
pub const HANDSHAKE_TIMEOUT_REASON: &str =
    "Failed to connect: the server did not answer within 5 seconds.";
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

pub enum Event {
    /// The server's protocol fingerprint matched; the connection is usable.
    Connected,
    /// Client and server protocols differ; `Disconnected` follows once the link drops.
    ProtocolRejected(String),
    Disconnected(Option<String>),
    Message(ProtocolMessage),
    /// Sent once per connection before any `Replication`: start a new `Replica`.
    ReplicationStarted(Arc<Schema>),
    /// Replicon payloads of one worker frame, for `Replica::apply`.
    Replication(ReplicationBatch),
}

/// Register any server-to-client type already registered by `shared::ProtocolPlugin`.
/// A new connection owns an independent FIFO and replication schema event.
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

    /// The six group messages in their reliable ordered `GroupChannel` send order: the
    /// roster must arrive before the member states that follow it, or the states of new
    /// members are dropped and, sent only on change, never repeated.
    pub fn receive_group(mut self) -> Self {
        self.relays.push(install_group_relay);
        self
    }

    /// All five loot messages in their reliable ordered `LootChannel` send order.
    pub fn receive_loot(mut self) -> Self {
        self.relays.push(install_loot_relay);
        self
    }

    /// All three merchant replies in their reliable ordered `MerchantChannel` send order.
    pub fn receive_merchant(mut self) -> Self {
        self.relays.push(install_merchant_relay);
        self
    }

    /// Receiving mailbox traffic in its reliable ordered channel order.
    pub fn receive_mail(mut self) -> Self {
        self.relays.push(install_mail_relay);
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
            // The realm's game time for the sky's time of day.
            .receive::<protocol::LoginSetTimeSpeed>()
            .receive::<TransferAborted>()
            // Quest log for the quest log, the objective tracker and the map quest areas;
            // quest giver markers; the quest giver dialog pages and turn-in results.
            .receive::<QuestLogSnapshot>()
            .receive::<QuestLogUpdate>()
            .receive::<QuestGiverStatusMultiple>()
            .receive::<QuestFailed>()
            .receive::<protocol::QuestGiverQuestList>()
            .receive::<protocol::QuestGiverQuestDetails>()
            .receive::<protocol::QuestGiverRequestItems>()
            .receive::<protocol::QuestGiverOfferReward>()
            .receive::<protocol::QuestGiverQuestComplete>()
            // Dungeon difficulty and saved instances for the entrance difficulty bar.
            .receive::<DungeonDifficultySet>()
            .receive::<InstanceInfo>()
            // Server-driven breath, fatigue and feign-death bars.
            .receive_mirror_timers()
            .receive_loot()
            .receive_mail()
            // Player trade: one message type, so channel order is kept.
            .receive::<protocol::TradeStateUpdate>()
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
            // The XP bar values after enter world, every gain and every level-up.
            .receive::<protocol::PlayerXpUpdate>()
            // Melee swing outcomes and resolved casts of every replicated unit, for
            // combat animations and spell visuals.
            .receive::<CombatEvent>()
            .receive::<SpellGo>()
            .receive::<SpellFailure>()
            // Auto-attack starts and stops of every replicated unit.
            .receive::<AttackStart>()
            .receive::<AttackStopped>()
            // NPC interaction, the merchant frame and the bags it sells from.
            .receive::<InteractionOpened>()
            .receive::<InteractionFailed>()
            .receive::<InteractionClosed>()
            .receive::<protocol::AuctionHouseOpened>()
            .receive::<protocol::AuctionBrowseResults>()
            .receive::<protocol::AuctionSearchResults>()
            .receive::<protocol::AuctionInventorySnapshot>()
            .receive::<protocol::OwnedAuctionListResponse>()
            .receive::<protocol::BidAuctionListResponse>()
            .receive::<protocol::AuctionOperationResponse>()
            .receive_merchant()
            .receive::<InventorySnapshot>()
            .receive::<EquipmentSnapshot>()
            .receive::<InventoryDelta>()
            .receive::<InventoryError>()
            .receive::<DurabilityStateUpdate>()
            .receive::<RestStateUpdate>()
            // Unit tooltip data and the account's appearance collection (unit-tooltip.md).
            .receive::<protocol::CreatureTooltip>()
            .receive::<protocol::AppearanceCollectionUpdate>()
            // Bank and guild bank contents, logs and refusals (bank-frame.md).
            .receive::<protocol::BankContents>()
            .receive::<protocol::BankFailed>()
            .receive::<protocol::GuildBankContents>()
            .receive::<protocol::GuildBankLog>()
            .receive::<protocol::GuildBankFailed>()
            // Chat lines for the chat frame.
            .receive::<ChatMessage>()
            // Players' social emotes, played on their models.
            .receive::<protocol::EmoteEvent>()
            // Party/raid roster, member states, invites and results (group-frames.md).
            .receive_group()
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
    let mut app = client_app();
    for relay in relays {
        relay(&mut app, events.clone());
    }
    install_lifecycle(&mut app, events.clone());
    install_replication(&mut app, events.clone());
    connect_transport(app.world_mut(), server_addr, client_id)?;
    app.finish();
    app.cleanup();
    let schema = Schema::from_world(app.world())?;
    events
        .send(Event::ReplicationStarted(schema))
        .map_err(|_| "host event receiver closed")?;
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

/// Lightyear client with replicon's receive side replaced by `replica`: lightyear's backend
/// adds replicon's `ClientPlugin` and applies replication into this world, which the host
/// never reads. Lightyear prediction rolls back that world and needs replicon's client; the
/// host predicts the local player itself. Replication rules, channels and the protocol hash
/// are unchanged: they come from lightyear's shared plugins and `shared::ProtocolPlugin`.
fn client_app() -> App {
    let mut app = App::new();
    app.set_error_handler(protocol::defer_lightyear_protocol_check);
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(
        client_network::ClientPlugins {
            tick_duration: SIMULATION_INTERVAL,
        }
        .build()
        .disable::<LightyearRepliconClientBackend>()
        .disable::<lightyear::prediction::plugin::PredictionPlugin>(),
    );
    app.add_plugins(shared::ProtocolPlugin);
    app
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
            client_timeout_secs: CLIENT_TIMEOUT_SECS,
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
    let started = world.resource::<Time<Real>>().elapsed();
    world
        .entity_mut(entity)
        .insert(HandshakeDeadline(started + HANDSHAKE_TIMEOUT));
    world.trigger(client_network::Connect { entity });
    Ok(())
}

/// `Time<Real>` elapsed by which the Netcode handshake must have connected.
#[derive(Component)]
struct HandshakeDeadline(Duration);

/// The handshake ran out of time; the link was dropped for it.
#[derive(Component)]
struct HandshakeTimedOut;

/// Drop a link whose Netcode handshake has not connected by its deadline.
fn expire_handshake(
    pending: Query<(Entity, &HandshakeDeadline, Has<client_network::Connected>)>,
    time: Res<Time<Real>>,
    mut commands: Commands,
) {
    for (entity, deadline, connected) in &pending {
        if connected {
            commands.entity(entity).remove::<HandshakeDeadline>();
        } else if time.elapsed() >= deadline.0 {
            commands
                .entity(entity)
                .remove::<HandshakeDeadline>()
                .insert(HandshakeTimedOut);
            commands.trigger(client_network::Disconnect { entity });
        }
    }
}

fn install_lifecycle(app: &mut App, events: Sender<Event>) {
    app.add_systems(PreUpdate, expire_handshake);
    app.add_systems(
        PostUpdate,
        move |verified: Query<(), Added<protocol::ProtocolVerified>>,
              rejected: Query<&protocol::ProtocolRejected, Added<protocol::ProtocolRejected>>,
              disconnected: Query<
            (&client_network::Disconnected, Has<HandshakeTimedOut>),
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
            for (state, timed_out) in &disconnected {
                let reason = if timed_out {
                    Some(HANDSHAKE_TIMEOUT_REASON.to_owned())
                } else {
                    state.reason.clone()
                };
                events
                    .send(Event::Disconnected(reason))
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

/// A single relay preserves order across message types sharing `GroupChannel`.
fn install_group_relay(app: &mut App, events: Sender<Event>) {
    use protocol::{
        GroupCommandResponse, GroupInviteCancelled, GroupInvitePrompt, GroupMemberStates,
        GroupRosterSnapshot, RaidTargetIcons, ReadyCheckUpdate,
    };
    app.add_systems(
        Update,
        (move |mut rosters: Query<&mut MessageReceiver<GroupRosterSnapshot>>,
               mut states: Query<&mut MessageReceiver<GroupMemberStates>>,
               mut prompts: Query<&mut MessageReceiver<GroupInvitePrompt>>,
               mut cancels: Query<&mut MessageReceiver<GroupInviteCancelled>>,
               mut checks: Query<&mut MessageReceiver<ReadyCheckUpdate>>,
               mut responses: Query<&mut MessageReceiver<GroupCommandResponse>>,
               mut raid_targets: Query<&mut MessageReceiver<RaidTargetIcons>>| {
            let mut received = Vec::new();
            macro_rules! drain {
                ($receivers:ident) => {
                    for mut receiver in &mut $receivers {
                        received.extend(receiver.receive_with_tick().map(|message| {
                            (message.message_id, ProtocolMessage(Box::new(message.data)))
                        }));
                    }
                };
            }
            drain!(rosters);
            drain!(states);
            drain!(prompts);
            drain!(cancels);
            drain!(checks);
            drain!(responses);
            drain!(raid_targets);
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

/// A single relay preserves order across message types sharing `LootChannel`.
fn install_loot_relay(app: &mut App, events: Sender<Event>) {
    use protocol::{CorpseLootable, LootClosed, LootFailed, LootResponse, LootSlotRemoved};
    app.add_systems(
        Update,
        (move |mut lootable: Query<&mut MessageReceiver<CorpseLootable>>,
               mut opened: Query<&mut MessageReceiver<LootResponse>>,
               mut removed: Query<&mut MessageReceiver<LootSlotRemoved>>,
               mut closed: Query<&mut MessageReceiver<LootClosed>>,
               mut failed: Query<&mut MessageReceiver<LootFailed>>| {
            let mut received = Vec::new();
            macro_rules! drain {
                ($receivers:ident) => {
                    for mut receiver in &mut $receivers {
                        received.extend(receiver.receive_with_tick().map(|message| {
                            (message.message_id, ProtocolMessage(Box::new(message.data)))
                        }));
                    }
                };
            }
            drain!(lootable);
            drain!(opened);
            drain!(removed);
            drain!(closed);
            drain!(failed);
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

/// A single relay preserves order across message types sharing `MerchantChannel`.
fn install_merchant_relay(app: &mut App, events: Sender<Event>) {
    app.add_systems(
        Update,
        (move |mut inventories: Query<&mut MessageReceiver<VendorInventory>>,
               mut buybacks: Query<&mut MessageReceiver<BuybackList>>,
               mut failures: Query<&mut MessageReceiver<MerchantFailed>>| {
            let mut received = Vec::new();
            macro_rules! drain {
                ($receivers:ident) => {
                    for mut receiver in &mut $receivers {
                        received.extend(receiver.receive_with_tick().map(|message| {
                            (message.message_id, ProtocolMessage(Box::new(message.data)))
                        }));
                    }
                };
            }
            drain!(inventories);
            drain!(buybacks);
            drain!(failures);
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

fn install_mail_relay(app: &mut App, events: Sender<Event>) {
    use protocol::{MailFailed, MailSent, MailboxContents, PendingMail};
    app.add_systems(
        Update,
        (move |mut contents: Query<&mut MessageReceiver<MailboxContents>>,
               mut failed: Query<&mut MessageReceiver<MailFailed>>,
               mut sent: Query<&mut MessageReceiver<MailSent>>,
               mut pending: Query<&mut MessageReceiver<PendingMail>>| {
            let mut received = Vec::new();
            macro_rules! drain {
                ($receivers:ident) => {
                    for mut receiver in &mut $receivers {
                        received.extend(receiver.receive_with_tick().map(|message| {
                            (message.message_id, ProtocolMessage(Box::new(message.data)))
                        }));
                    }
                };
            }
            drain!(contents);
            drain!(failed);
            drain!(sent);
            drain!(pending);
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

/// Forward replicon payloads; nothing from a rejected connection reaches the host.
fn install_replication(app: &mut App, events: Sender<Event>) {
    app.add_systems(
        PreUpdate,
        (move |channels: Res<RepliconChannelMap>,
               mut transports: Query<&mut Transport, With<Client>>,
               rejected: Query<(), With<protocol::ProtocolRejected>>|
              -> Result {
            let batches = replica::receive::receive_batches(&channels, &mut transports)?;
            if !rejected.is_empty() {
                return Ok(());
            }
            for batch in batches {
                events
                    .send(Event::Replication(batch))
                    .expect("host event receiver closed");
            }
            Ok(())
        })
        // Message receive drains every channel receiver, replicon's included.
        .after(TransportSystems::Receive)
        .before(network::MessageSystems::Receive),
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
mod merchant_wire_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{AuthChannel, LoginRequest};
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

    /// The fingerprint `shared::protocol_check` compares (lightyear message and channel
    /// hashes, replicon `ProtocolHash`) is the stock lightyear client's.
    #[test]
    fn replacing_replicon_client_keeps_the_protocol_fingerprint() {
        fn fingerprint(mut app: App) -> (u64, u64, bevy_replicon::shared::protocol::ProtocolHash) {
            app.finish();
            app.cleanup();
            let world = app.world_mut();
            (
                world.resource_mut::<network::MessageRegistry>().finish(),
                world.resource_mut::<network::ChannelRegistry>().finish(),
                *world.resource::<bevy_replicon::shared::protocol::ProtocolHash>(),
            )
        }
        let mut stock = App::new();
        stock.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
        stock.add_plugins(StatesPlugin);
        stock.add_plugins(client_network::ClientPlugins {
            tick_duration: SIMULATION_INTERVAL,
        });
        stock.add_plugins(shared::ProtocolPlugin);
        assert!(stock.is_plugin_added::<LightyearRepliconClientBackend>());
        let replaced = client_app();
        assert!(!replaced.is_plugin_added::<bevy_replicon::client::ClientPlugin>());
        assert_eq!(fingerprint(replaced), fingerprint(stock));
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
