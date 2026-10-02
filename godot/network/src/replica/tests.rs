//! `Replica` against replicon's own client, on payloads a real replicon server encoded.
//!
//! A stock lightyear client (replicon `ClientPlugin` + lightyear's backend) connects over
//! loopback UDP to a real lightyear/replicon server. A tap records every replicon payload
//! the stock client receives and every acknowledgment it sends; `Replica` applies the same
//! payloads frame by frame and must hold byte-identical component values for the same
//! server entities. Reordered replays of those payloads drive a stock replicon client and
//! `Replica` side by side through buffering and stale-mutation paths.

use std::time::{Duration, Instant};

use bevy::{app::ScheduleRunnerPlugin, prelude::*, state::app::StatesPlugin};
use bevy_replicon::{
    bytes::Bytes,
    client::ClientSystems,
    postcard_utils,
    prelude::{ClientMessages, ClientState},
    shared::{
        backend::channels::{ClientChannel, ServerChannel},
        server_entity_map::ServerEntityMap,
    },
};
use lightyear::prelude::{
    LinkOf, NetworkTarget, Replicate, ReplicationReceiver, VisibilityExt, client as client_network,
};
use shared::components::{
    CreatureClassification, Health, Npc, Player, Position, UnitAuras, UnitLevel, UnitVignette,
};

use super::{Replica, ReplicationBatch, Schema, Unit, UnitChange, receive::acknowledgments};
use crate::{SIMULATION_INTERVAL, connect_transport, wire_tests::start_fixture_server};

#[derive(Default)]
struct Frame {
    updates: Vec<Bytes>,
    mutations: Vec<Bytes>,
    acks: Vec<Bytes>,
}

impl Frame {
    fn batch(&self) -> ReplicationBatch {
        ReplicationBatch {
            updates: self.updates.clone(),
            mutations: self.mutations.clone(),
        }
    }
}

#[derive(Resource, Default)]
struct Tap {
    current: Frame,
    frames: Vec<Frame>,
}

fn tap_received(messages: Res<ClientMessages>, mut tap: ResMut<Tap>) {
    tap.current.updates = messages
        .iter_received(ServerChannel::Updates)
        .cloned()
        .collect();
    tap.current.mutations = messages
        .iter_received(ServerChannel::Mutations)
        .cloned()
        .collect();
}

fn tap_sent(messages: Res<ClientMessages>, mut tap: ResMut<Tap>) {
    let acks_channel: usize = ClientChannel::MutationAcks.into();
    let acks = messages
        .iter_sent()
        .filter(|(channel, _)| *channel == acks_channel)
        .map(|(_, bytes)| bytes.clone())
        .collect();
    let mut frame = std::mem::take(&mut tap.current);
    frame.acks = acks;
    if !frame.updates.is_empty() || !frame.mutations.is_empty() {
        tap.frames.push(frame);
    }
}

fn add_tap(app: &mut App) {
    app.init_resource::<Tap>();
    app.add_systems(
        PreUpdate,
        tap_received
            .after(ClientSystems::ReceivePackets)
            .before(ClientSystems::Receive),
    );
    app.add_systems(
        PostUpdate,
        tap_sent
            .after(ClientSystems::Send)
            .before(ClientSystems::SendPackets),
    );
}

/// Stock lightyear client: `ClientPlugins` with lightyear's replicon backend.
fn stock_client(address: std::net::SocketAddr, client_id: u64) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(client_network::ClientPlugins {
        tick_duration: SIMULATION_INTERVAL,
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.add_observer(
        |connected: On<Add, client_network::Connected>, mut commands: Commands| {
            commands
                .entity(connected.entity)
                .insert(ReplicationReceiver);
        },
    );
    add_tap(&mut app);
    connect_transport(app.world_mut(), address, client_id).expect("connect stock client");
    app.finish();
    app.cleanup();
    app
}

/// Stock replicon receive without transport: payloads are inserted by the test.
fn replay_client() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(
        client_network::ClientPlugins {
            tick_duration: SIMULATION_INTERVAL,
        }
        .build()
        .disable::<lightyear_replication::LightyearRepliconClientBackend>(),
    );
    app.add_plugins(bevy_replicon::client::ClientPlugin);
    app.add_plugins(shared::ProtocolPlugin);
    add_tap(&mut app);
    // No transport drains replicon's outgoing acknowledgments.
    app.add_systems(
        PostUpdate,
        (|mut messages: ResMut<ClientMessages>| {
            messages.drain_sent().for_each(drop);
        })
        .after(tap_sent),
    );
    app.finish();
    app.cleanup();
    app.world_mut()
        .resource_mut::<NextState<ClientState>>()
        .set(ClientState::Connected);
    app.update();
    app
}

fn replay(app: &mut App, frame: &Frame) {
    let mut messages = app.world_mut().resource_mut::<ClientMessages>();
    for update in &frame.updates {
        messages.insert_received(ServerChannel::Updates, update.clone());
    }
    for mutation in &frame.mutations {
        messages.insert_received(ServerChannel::Mutations, mutation.clone());
    }
    app.update();
}

fn schema_of(app: &App) -> std::sync::Arc<Schema> {
    Schema::from_world(app.world()).expect("every replicated type has a codec")
}

/// Same server entities with byte-identical values of every registered component.
fn assert_equivalent(stock: &mut App, replica: &Replica, context: &str) {
    let world = stock.world_mut();
    let mut expected: Vec<(u64, Entity)> = world
        .resource::<ServerEntityMap>()
        .to_client()
        .iter()
        .map(|(server, client)| (server.to_bits(), *client))
        .collect();
    expected.sort();
    let mut actual: Vec<u64> = replica.units().map(|unit| unit.server_id).collect();
    actual.sort();
    let expected_ids: Vec<u64> = expected.iter().map(|(id, _)| *id).collect();
    assert_eq!(actual, expected_ids, "{context}: replicated entity sets");
    for (server_id, client) in expected {
        let entity = world.entity(client);
        for (index, codec) in replica.schema().codecs.iter().enumerate() {
            assert_eq!(
                replica.encoded(server_id, index),
                (codec.encode_entity)(entity),
                "{context}: {} of {server_id:#x}",
                codec.name
            );
        }
    }
}

fn player(name: &str) -> Player {
    Player {
        name: name.into(),
        race: 1,
        class: 1,
        appearance: Default::default(),
    }
}

fn position(x: f32) -> Position {
    Position { x, y: 2.0, z: 3.0 }
}

/// Live session plus captured frames; each frame's replica state is compared on arrival.
struct Session {
    server: App,
    client: App,
    replica: Replica,
    applied: usize,
    link: Entity,
}

impl Session {
    fn start(client_id: u64) -> Self {
        let (server, address) = start_fixture_server();
        let client = stock_client(address, client_id);
        let replica = Replica::new(schema_of(&client));
        let mut session = Self {
            server,
            client,
            replica,
            applied: 0,
            link: Entity::PLACEHOLDER,
        };
        session.run_until("connection", |session| {
            let client = session.client.world_mut();
            let connected = client
                .query_filtered::<(), With<client_network::Connected>>()
                .iter(client)
                .count();
            let server = session.server.world_mut();
            let link = server
                .query_filtered::<Entity, With<LinkOf>>()
                .iter(server)
                .next();
            if let Some(link) = link {
                session.link = link;
            }
            connected == 1 && link.is_some()
        });
        session
    }

    fn step(&mut self) {
        self.server.update();
        self.client.update();
        let frames = &self.client.world().resource::<Tap>().frames;
        let new: Vec<(ReplicationBatch, Vec<Bytes>, Vec<Bytes>)> = frames[self.applied..]
            .iter()
            .map(|frame| (frame.batch(), frame.mutations.clone(), frame.acks.clone()))
            .collect();
        for (batch, mutations, acks) in new {
            self.applied += 1;
            if !mutations.is_empty() {
                assert_eq!(
                    vec![acknowledgments(&mutations).expect("parse mutate headers")],
                    acks,
                    "worker acknowledgments equal replicon's"
                );
            }
            self.replica.apply(batch).expect("apply captured frame");
            assert_equivalent(&mut self.client, &self.replica, "live frame");
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    fn run_until(&mut self, what: &str, mut done: impl FnMut(&mut Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done(self) {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            self.step();
        }
    }

    fn run_frames(&mut self, count: usize) {
        for _ in 0..count {
            self.step();
        }
    }

    fn frames(&self) -> &[Frame] {
        &self.client.world().resource::<Tap>().frames
    }
}

impl Session {
    fn spawn(&mut self, bundle: impl Bundle) -> Entity {
        let replicate = Replicate::to_clients(NetworkTarget::All);
        self.server.world_mut().spawn((bundle, replicate)).id()
    }

    fn edit(&mut self, entity: Entity, edit: impl FnOnce(&mut EntityWorldMut)) {
        edit(&mut self.server.world_mut().entity_mut(entity));
    }

    /// Step until the replica's copy of `entity` satisfies `ready` (`None`: despawned).
    fn until(&mut self, what: &str, entity: Entity, mut ready: impl FnMut(Option<Unit>) -> bool) {
        self.run_until(what, |session| {
            ready(session.replica.unit(entity.to_bits()))
        });
    }
}

fn has<C: 'static>(unit: Option<Unit>) -> bool {
    unit.is_some_and(|unit| unit.has::<C>())
}

fn x_of(unit: Option<Unit>) -> Option<f32> {
    unit?.get::<Position>().map(|position| position.x)
}

/// Spawn, insert, mutate, remove, visibility loss and (optionally) despawn, frame by frame.
fn run_scenario(session: &mut Session, despawn_hero: bool) -> (Entity, Entity) {
    let health = Health {
        current: 90.0,
        max: 100.0,
    };
    let hero = session.spawn((player("Fbhero"), position(1.0), health));
    let wolf_npc = Npc {
        template_id: 299,
        name: "Diseased Young Wolf".into(),
    };
    let wolf = session.spawn((wolf_npc, position(50.0), UnitLevel(2)));
    session.until("hero spawn", hero, |unit| x_of(unit) == Some(1.0));
    session.until("wolf spawn", wolf, |unit| x_of(unit) == Some(50.0));

    session.edit(hero, |hero| {
        hero.insert(UnitAuras::default());
    });
    session.until("aura insert", hero, has::<UnitAuras>);
    for step in 2..12 {
        session.edit(hero, |hero| {
            hero.insert(position(step as f32));
        });
        session.step();
    }
    session.until("hero mutations", hero, |unit| x_of(unit) == Some(11.0));

    session.edit(wolf, |wolf| {
        wolf.remove::<UnitLevel>();
    });
    session.until("level removal", wolf, |unit| {
        unit.is_some() && !has::<UnitLevel>(unit)
    });
    let link = session.link;
    session.server.world_mut().lose_visibility(wolf, link);
    session.until("visibility loss", wolf, |unit| unit.is_none());
    if despawn_hero {
        session.server.world_mut().despawn(hero);
        session.until("despawn", hero, |unit| unit.is_none());
    }
    session.run_frames(3);
    (hero, wolf)
}

#[test]
fn replica_matches_stock_replicon_client_over_udp() {
    let mut session = Session::start(9100);
    let (hero, wolf) = run_scenario(&mut session, true);
    let changes = session.replica.drain_changes();
    assert!(changes.contains(&UnitChange::Despawned(hero.to_bits())));
    assert!(changes.contains(&UnitChange::Despawned(wolf.to_bits())));
    assert!(
        session.frames().iter().any(|frame| !frame.acks.is_empty()),
        "scenario produced acknowledged mutate messages"
    );
}

/// Timber (world.db creature_template 1132, rank 4) reaches the native replica as a rare.
#[test]
fn creature_classification_reaches_the_replica() {
    let mut session = Session::start(9103);
    let timber = Npc {
        template_id: 1_132,
        name: "Timber".into(),
    };
    let timber = session.spawn((timber, position(7.0), CreatureClassification::Rare));
    session.until("timber spawn", timber, |unit| {
        unit.and_then(|unit| unit.get::<CreatureClassification>().copied())
            == Some(CreatureClassification::Rare)
    });
}

/// Doomwalker (world.db creature_template 167749) carries Vignette 6520 until the
/// server removes it on death.
#[test]
fn unit_vignette_reaches_the_replica_and_leaves_it() {
    let mut session = Session::start(9104);
    let doomwalker = Npc {
        template_id: 167_749,
        name: "Doomwalker".into(),
    };
    let doomwalker = session.spawn((doomwalker, position(7.0), UnitVignette(6_520)));
    session.until("doomwalker spawn", doomwalker, |unit| {
        unit.and_then(|unit| unit.get::<UnitVignette>().copied()) == Some(UnitVignette(6_520))
    });
    session.edit(doomwalker, |entity| {
        entity.remove::<UnitVignette>();
    });
    session.until("vignette removed", doomwalker, |unit| {
        unit.is_some_and(|unit| !unit.has::<UnitVignette>())
    });
}

/// Replay one captured session through both clients with a different frame split.
fn replay_both(frames: &[Frame]) -> Replica {
    let mut stock = replay_client();
    let mut replica = Replica::new(schema_of(&stock));
    for (index, frame) in frames.iter().enumerate() {
        replay(&mut stock, frame);
        replica.apply(frame.batch()).expect("apply replayed frame");
        assert_equivalent(&mut stock, &replica, &format!("replayed frame {index}"));
        let sent = &stock.world().resource::<Tap>().frames;
        if !frame.mutations.is_empty() {
            assert_eq!(
                sent.last().expect("replayed frame recorded").acks,
                vec![acknowledgments(&frame.mutations).expect("parse mutate headers")],
            );
        }
    }
    replica
}

/// Whether a mutate message carries entity data (the server also sends empty ones).
fn has_entities(message: &Bytes) -> bool {
    let mut header = message.clone();
    for _ in 0..3 {
        let _: usize = postcard_utils::from_buf(&mut header).expect("mutate header");
    }
    header.len() > super::MUTATE_INDEX_SIZE
}

/// Frames of a session whose hero stays spawned at x = 11.
fn captured_session(client_id: u64) -> (Vec<Frame>, u64) {
    let mut session = Session::start(client_id);
    let (hero, _) = run_scenario(&mut session, false);
    let frames = std::mem::take(&mut session.client.world_mut().resource_mut::<Tap>().frames);
    (frames, hero.to_bits())
}

fn update_frames(frames: &[Frame]) -> Vec<Frame> {
    frames
        .iter()
        .filter(|frame| !frame.updates.is_empty())
        .map(|frame| Frame {
            updates: frame.updates.clone(),
            ..Default::default()
        })
        .collect()
}

fn all_mutations(frames: &[Frame]) -> Vec<Bytes> {
    frames
        .iter()
        .flat_map(|frame| frame.mutations.iter().cloned())
        .collect()
}

#[test]
fn replica_matches_stock_client_when_mutations_precede_their_updates() {
    let (frames, hero) = captured_session(9101);
    let mutations = all_mutations(&frames);
    assert!(mutations.iter().any(has_entities));
    // Every mutate message arrives before any update, so all wait for their update tick.
    let mut reordered = vec![Frame {
        mutations,
        ..Default::default()
    }];
    reordered.extend(update_frames(&frames));
    let replica = replay_both(&reordered);
    assert_eq!(x_of(replica.unit(hero)), Some(11.0));
}

#[test]
fn replica_matches_stock_client_when_old_mutations_arrive_last() {
    let (frames, hero) = captured_session(9102);
    let mut reordered = update_frames(&frames);
    // Newest first, one per frame: every older mutation of the hero is stale on arrival.
    let mut mutations = all_mutations(&frames);
    mutations.reverse();
    reordered.extend(mutations.into_iter().map(|message| Frame {
        mutations: vec![message],
        ..Default::default()
    }));
    let replica = replay_both(&reordered);
    assert_eq!(x_of(replica.unit(hero)), Some(11.0));
}

#[path = "coverage_tests.rs"]
mod coverage;
