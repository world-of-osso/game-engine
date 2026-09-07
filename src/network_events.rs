//! Application-owned dispatch of buffered network messages and queued outgoing work.
//!
//! The network tick owns cadence. Handlers are registered systems, not members of
//! `Update`. Incoming readiness is sampled once per dispatch; ready handlers run
//! once in registration order. Their original receivers retain per-type FIFO.

use std::{any::TypeId, collections::HashMap};

use bevy::{ecs::system::SystemId, prelude::*};
use lightyear::prelude::{Message as NetworkMessage, MessageManager, MessageReceiver};

type WorldCondition = fn(&World) -> bool;
type InboxCheck = fn(&World, Entity) -> bool;

struct Handler {
    system: SystemId,
    condition: WorldCondition,
}

struct MessageRoute {
    has_messages: InboxCheck,
    handlers: Vec<usize>,
    deferred: Box<dyn DeferredInbox>,
}

trait DeferredInbox: Send + Sync {
    fn park(&mut self, world: &mut World, peers: &[Entity]);
    fn restore(&mut self, world: &mut World);
}

// MessageReceiver contains only buffered messages. Moving it preserves private
// channel/tick metadata while Lightyear clears the empty replacement in Last.
struct DeferredMessages<M: NetworkMessage>(HashMap<Entity, MessageReceiver<M>>);

impl<M: NetworkMessage> DeferredInbox for DeferredMessages<M> {
    fn park(&mut self, world: &mut World, peers: &[Entity]) {
        for &peer in peers {
            if let Some(mut receiver) = world.get_mut::<MessageReceiver<M>>(peer)
                && receiver.has_messages()
            {
                self.0.insert(peer, std::mem::take(&mut *receiver));
            }
        }
    }

    fn restore(&mut self, world: &mut World) {
        for (peer, buffered) in self.0.drain() {
            if let Some(mut receiver) = world.get_mut::<MessageReceiver<M>>(peer) {
                assert!(
                    !receiver.has_messages(),
                    "network inbox populated before restoration"
                );
                *receiver = buffered;
            }
        }
    }
}

#[derive(Resource)]
struct NetworkDispatcher {
    peers: QueryState<Entity, With<MessageManager>>,
    peer_entities: Vec<Entity>,
    incoming: Vec<Handler>,
    incoming_ready: Vec<bool>,
    routes: HashMap<TypeId, MessageRoute>,
    outgoing: Vec<Handler>,
}

impl FromWorld for NetworkDispatcher {
    fn from_world(world: &mut World) -> Self {
        Self {
            peers: world.query_filtered(),
            peer_entities: Vec::new(),
            incoming: Vec::new(),
            incoming_ready: Vec::new(),
            routes: HashMap::new(),
            outgoing: Vec::new(),
        }
    }
}

impl NetworkDispatcher {
    fn mark_ready_inboxes(&mut self, world: &World) {
        self.incoming_ready.fill(false);
        self.peer_entities.clear();
        self.peer_entities.extend(self.peers.iter(world));
        for route in self.routes.values() {
            let pending = self
                .peer_entities
                .iter()
                .any(|entity| (route.has_messages)(world, *entity));
            if pending {
                for &index in &route.handlers {
                    self.incoming_ready[index] = true;
                }
            }
        }
    }
}

/// Initializes private dispatcher state; no handlers are scheduled or executed.
pub fn initialize_dispatcher(app: &mut App) {
    app.init_resource::<NetworkDispatcher>();
}

/// Registers one incoming handler and its first message route.
/// Eligibility is checked only when at least one routed inbox contains messages.
pub fn register_message_handler<M: NetworkMessage, Marker>(
    app: &mut App,
    system: impl IntoSystem<(), (), Marker> + 'static,
    eligible: WorldCondition,
) -> SystemId {
    initialize_dispatcher(app);
    let id = app.world_mut().register_system(system);
    {
        let mut dispatcher = app.world_mut().resource_mut::<NetworkDispatcher>();
        dispatcher.incoming.push(Handler {
            system: id,
            condition: eligible,
        });
        dispatcher.incoming_ready.push(false);
    }
    add_message_route::<M>(app, id);
    id
}

/// Adds another message type to an existing incoming handler. Duplicate routes
/// are ignored; multiple ready routes never invoke the same handler twice.
pub fn add_message_route<M: NetworkMessage>(app: &mut App, id: SystemId) {
    initialize_dispatcher(app);
    let mut dispatcher = app.world_mut().resource_mut::<NetworkDispatcher>();
    let index = dispatcher
        .incoming
        .iter()
        .position(|handler| handler.system == id)
        .unwrap_or_else(|| panic!("unknown incoming network handler {id:?}"));
    let route = dispatcher
        .routes
        .entry(TypeId::of::<M>())
        .or_insert_with(|| MessageRoute {
            has_messages: inbox_has_messages::<M>,
            handlers: Vec::new(),
            deferred: Box::new(DeferredMessages::<M>(HashMap::new())),
        });
    if !route.handlers.contains(&index) {
        route.handlers.push(index);
    }
}

fn inbox_has_messages<M: NetworkMessage>(world: &World, peer: Entity) -> bool {
    world
        .get::<MessageReceiver<M>>(peer)
        .is_some_and(MessageReceiver::has_messages)
}

/// Registers outgoing work behind its existing cheap queue/state predicate.
pub fn register_outgoing_handler<Marker>(
    app: &mut App,
    system: impl IntoSystem<(), (), Marker> + 'static,
    ready: WorldCondition,
) -> SystemId {
    initialize_dispatcher(app);
    let id = app.world_mut().register_system(system);
    app.world_mut()
        .resource_mut::<NetworkDispatcher>()
        .outgoing
        .push(Handler {
            system: id,
            condition: ready,
        });
    id
}

/// Drains ready incoming handlers. The private registry is temporarily scoped
/// out of the world and must not be a parameter of a registered handler.
pub fn dispatch_incoming(world: &mut World) {
    world.resource_scope(|world, mut dispatcher: Mut<NetworkDispatcher>| {
        dispatcher.mark_ready_inboxes(world);
        for (index, handler) in dispatcher.incoming.iter().enumerate() {
            if dispatcher.incoming_ready[index] && (handler.condition)(world) {
                invoke_handler(world, handler.system);
            }
        }
    });
}

/// Preserve application inboxes across Lightyear's per-frame Last cleanup.
pub fn park_incoming(world: &mut World) {
    world.resource_scope(|world, mut dispatcher: Mut<NetworkDispatcher>| {
        let NetworkDispatcher {
            peers,
            peer_entities,
            routes,
            ..
        } = &mut *dispatcher;
        peer_entities.clear();
        peer_entities.extend(peers.iter(world));
        for route in routes.values_mut() {
            route.deferred.park(world, peer_entities);
        }
    });
}

/// Restore older messages before transport appends newly received messages.
pub fn restore_incoming(world: &mut World) {
    world.resource_scope(|world, mut dispatcher: Mut<NetworkDispatcher>| {
        for route in dispatcher.routes.values_mut() {
            route.deferred.restore(world);
        }
    });
}

/// Invokes pending outgoing handlers in registration order.
pub fn dispatch_outgoing(world: &mut World) {
    world.resource_scope(|world, dispatcher: Mut<NetworkDispatcher>| {
        for handler in &dispatcher.outgoing {
            if (handler.condition)(world) {
                invoke_handler(world, handler.system);
            }
        }
    });
}

fn invoke_handler(world: &mut World, system: SystemId) {
    world
        .run_system(system)
        .unwrap_or_else(|error| panic!("network handler {system:?} failed: {error:?}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::{PostUpdate, PreUpdate};
    use lightyear::prelude::client::ClientPlugins;
    use lightyear::prelude::{
        AppChannelExt, AppMessageExt, ChannelMode, ChannelRegistry, ChannelSettings, Connected,
        Link, Linked, MessageReceiver, MessageSender, PeerId, RemoteId, Transport,
    };
    use serde::{Deserialize, Serialize};
    use std::collections::VecDeque;

    #[derive(Clone, Serialize, Deserialize)]
    struct First(u32);
    #[derive(Clone, Serialize, Deserialize)]
    struct Second(u32);
    struct TestChannel;

    #[derive(Resource)]
    struct HeavyResource;
    #[derive(Resource, Default)]
    struct Output(Vec<u32>);
    #[derive(Resource, Default)]
    struct Outbox(VecDeque<u32>);
    #[derive(Resource)]
    struct Allowed(bool);

    fn fixture() -> (App, Entity) {
        fixture_with_cadence(false)
    }

    fn fixture_with_cadence(network_tick: bool) -> (App, Entity) {
        fixture_with_frequency(network_tick, std::time::Duration::ZERO)
    }

    fn fixture_with_frequency(
        network_tick: bool,
        send_frequency: std::time::Duration,
    ) -> (App, Entity) {
        let mut app = App::new();
        initialize_dispatcher(&mut app);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(ClientPlugins::default());
        app.register_message::<First>();
        app.register_message::<Second>();
        app.add_channel::<TestChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(Default::default()),
            send_frequency,
            ..Default::default()
        });
        app.init_resource::<Output>();
        if network_tick {
            app.add_plugins(crate::network_tick::NetworkTickPlugin);
        }
        app.finish();
        app.cleanup();
        let channels = app.world().resource::<ChannelRegistry>();
        let mut transport = Transport::default();
        transport.add_sender_from_registry::<TestChannel>(channels);
        transport.add_receiver_from_registry::<TestChannel>(channels);
        let peer = app
            .world_mut()
            .spawn((
                Link::default(),
                transport,
                Linked,
                Connected,
                RemoteId(PeerId::Local(0)),
                MessageReceiver::<First>::default(),
                MessageSender::<First>::default(),
                MessageReceiver::<Second>::default(),
                MessageSender::<Second>::default(),
            ))
            .id();
        if network_tick {
            advance_fixture(&mut app, std::time::Duration::ZERO);
        }
        (app, peer)
    }

    fn advance_fixture(app: &mut App, delta: std::time::Duration) {
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(delta));
        app.update();
    }

    fn deliver(app: &mut App, peer: Entity) {
        app.world_mut().run_schedule(PostUpdate);
        let mut entity = app.world_mut().entity_mut(peer);
        let mut link = entity.get_mut::<Link>().unwrap();
        let packets: Vec<_> = link.send.drain().collect();
        assert!(
            !packets.is_empty(),
            "fixture must produce real transport packets"
        );
        for packet in packets {
            link.recv.push_raw(packet);
        }
        drop(link);
        app.world_mut().run_schedule(PreUpdate);
    }

    #[test]
    fn network_tick_preserves_packets_across_frames_without_a_tick() {
        let (mut app, peer) = fixture_with_cadence(true);
        register_message_handler::<First, _>(
            &mut app,
            |mut receivers: Query<&mut MessageReceiver<First>>, mut output: ResMut<Output>| {
                for mut receiver in &mut receivers {
                    output.0.extend(receiver.receive().map(|message| message.0));
                }
            },
            always,
        );
        {
            let mut entity = app.world_mut().entity_mut(peer);
            let mut sender = entity.get_mut::<MessageSender<First>>().unwrap();
            sender.send::<TestChannel>(First(7));
            sender.send::<TestChannel>(First(8));
        }
        advance_fixture(&mut app, std::time::Duration::from_millis(17));
        {
            let mut entity = app.world_mut().entity_mut(peer);
            let mut link = entity.get_mut::<Link>().unwrap();
            let packets: Vec<_> = link.send.drain().collect();
            assert!(!packets.is_empty(), "real transport packets required");
            for packet in packets {
                link.recv.push_raw(packet);
            }
        }
        // Last executes twice without a tick, before the queued packets are decoded.
        advance_fixture(&mut app, std::time::Duration::ZERO);
        advance_fixture(&mut app, std::time::Duration::ZERO);
        assert!(app.world().resource::<Output>().0.is_empty());
        // Two catch-up ticks must consume each message only once.
        advance_fixture(&mut app, std::time::Duration::from_millis(34));
        assert_eq!(app.world().resource::<Output>().0, [7, 8]);
        advance_fixture(&mut app, std::time::Duration::ZERO);
        assert_eq!(app.world().resource::<Output>().0, [7, 8]);
    }

    #[test]
    fn transport_send_frequency_uses_full_elapsed_time_between_application_ticks() {
        use std::time::Duration;
        for hz in [30_u64, 60, 400] {
            let (mut app, peer) = fixture_with_frequency(true, Duration::from_millis(100));
            register_message_handler::<First, _>(
                &mut app,
                |mut receivers: Query<&mut MessageReceiver<First>>, mut output: ResMut<Output>| {
                    for mut receiver in &mut receivers {
                        output.0.extend(receiver.receive().map(|message| message.0));
                    }
                },
                always,
            );
            app.world_mut()
                .entity_mut(peer)
                .get_mut::<MessageSender<First>>()
                .unwrap()
                .send::<TestChannel>(First(42));
            let mut previous = Duration::ZERO;
            for frame in 1..=hz {
                let elapsed = Duration::from_nanos(frame * 1_000_000_000 / hz);
                advance_fixture(&mut app, elapsed - previous);
                previous = elapsed;
                let mut entity = app.world_mut().entity_mut(peer);
                let mut link = entity.get_mut::<Link>().unwrap();
                let packets: Vec<_> = link.send.drain().collect();
                for packet in packets {
                    link.recv.push_raw(packet);
                }
                if !app.world().resource::<Output>().0.is_empty() {
                    break;
                }
            }
            assert_eq!(
                app.world().resource::<Output>().0,
                [42],
                "frame cadence {hz}"
            );
            assert!(
                previous <= Duration::from_millis(170),
                "100 ms channel delayed to {previous:?} at {hz} frames/s"
            );
        }
    }

    fn always(_: &World) -> bool {
        true
    }
    fn heavy_handler(_: Res<HeavyResource>) {
        panic!("empty inbox must skip this handler");
    }
    fn forbidden_eligibility(_: &World) -> bool {
        panic!("empty inbox must skip eligibility");
    }

    #[test]
    fn initialized_dispatcher_without_handlers_has_no_work() {
        let mut app = App::new();
        initialize_dispatcher(&mut app);
        dispatch_incoming(app.world_mut());
        dispatch_outgoing(app.world_mut());
    }

    #[test]
    #[should_panic(expected = "unknown incoming network handler")]
    fn route_to_an_unregistered_handler_fails_explicitly() {
        let mut app = App::new();
        let id = app.world_mut().register_system(|| {});
        add_message_route::<First>(&mut app, id);
    }

    #[test]
    fn empty_receiver_skips_eligibility_and_heavy_parameter_fetch() {
        let (mut app, _) = fixture();
        register_message_handler::<First, _>(&mut app, heavy_handler, forbidden_eligibility);
        dispatch_incoming(app.world_mut());
        dispatch_incoming(app.world_mut());
    }

    #[test]
    fn delivered_routes_dispatch_once_in_registration_order_and_preserve_fifo() {
        let (mut app, peer) = fixture();
        register_message_handler::<First, _>(
            &mut app,
            |mut out: ResMut<Output>| out.0.push(100),
            always,
        );
        let consumer = register_message_handler::<Second, _>(
            &mut app,
            |mut first: Query<&mut MessageReceiver<First>>,
             mut second: Query<&mut MessageReceiver<Second>>,
             mut out: ResMut<Output>| {
                out.0.push(200);
                for mut receiver in &mut first {
                    out.0.extend(receiver.receive().map(|m| m.0));
                }
                for mut receiver in &mut second {
                    out.0.extend(receiver.receive().map(|m| m.0));
                }
            },
            always,
        );
        add_message_route::<First>(&mut app, consumer);
        add_message_route::<First>(&mut app, consumer);
        app.world_mut()
            .get_mut::<MessageSender<First>>(peer)
            .unwrap()
            .send::<TestChannel>(First(1));
        app.world_mut()
            .get_mut::<MessageSender<First>>(peer)
            .unwrap()
            .send::<TestChannel>(First(2));
        app.world_mut()
            .get_mut::<MessageSender<Second>>(peer)
            .unwrap()
            .send::<TestChannel>(Second(3));
        deliver(&mut app, peer);
        assert!(
            app.world()
                .get::<MessageReceiver<First>>(peer)
                .unwrap()
                .has_messages()
        );
        assert!(
            app.world()
                .get::<MessageReceiver<Second>>(peer)
                .unwrap()
                .has_messages()
        );
        dispatch_incoming(app.world_mut());
        dispatch_incoming(app.world_mut());
        assert_eq!(app.world().resource::<Output>().0, [100, 200, 1, 2, 3]);
    }

    #[test]
    fn ineligible_ready_handler_does_not_consume_buffered_messages() {
        let (mut app, peer) = fixture();
        app.insert_resource(Allowed(false));
        register_message_handler::<First, _>(
            &mut app,
            |mut receivers: Query<&mut MessageReceiver<First>>, mut out: ResMut<Output>| {
                for mut receiver in &mut receivers {
                    out.0.extend(receiver.receive().map(|m| m.0));
                }
            },
            |world| world.resource::<Allowed>().0,
        );
        app.world_mut()
            .get_mut::<MessageSender<First>>(peer)
            .unwrap()
            .send::<TestChannel>(First(42));
        deliver(&mut app, peer);
        dispatch_incoming(app.world_mut());
        assert!(app.world().resource::<Output>().0.is_empty());
        assert_eq!(
            app.world()
                .get::<MessageReceiver<First>>(peer)
                .unwrap()
                .num_messages(),
            1
        );
        app.world_mut().resource_mut::<Allowed>().0 = true;
        dispatch_incoming(app.world_mut());
        assert_eq!(app.world().resource::<Output>().0, [42]);
    }

    #[test]
    fn outgoing_only_runs_for_pending_queue_and_preserves_fifo() {
        let mut app = App::new();
        app.init_resource::<Output>().init_resource::<Outbox>();
        register_outgoing_handler(
            &mut app,
            |mut out: ResMut<Output>| out.0.push(100),
            |world| !world.resource::<Outbox>().0.is_empty(),
        );
        register_outgoing_handler(
            &mut app,
            |_: Res<HeavyResource>, mut queue: ResMut<Outbox>, mut out: ResMut<Output>| {
                out.0.extend(queue.0.drain(..));
            },
            |world| !world.resource::<Outbox>().0.is_empty(),
        );
        dispatch_outgoing(app.world_mut());
        app.insert_resource(HeavyResource);
        app.world_mut().resource_mut::<Outbox>().0.extend([7, 3, 9]);
        dispatch_outgoing(app.world_mut());
        app.world_mut().remove_resource::<HeavyResource>();
        dispatch_outgoing(app.world_mut());
        assert_eq!(app.world().resource::<Output>().0, [100, 7, 3, 9]);
        assert!(app.world().resource::<Outbox>().0.is_empty());
    }
}
