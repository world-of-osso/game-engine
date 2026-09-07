//! Dispatch application-owned inboxes; transport lives in the network worker.

use std::{any::TypeId, collections::HashMap, sync::mpsc::Sender};

use bevy::{ecs::system::SystemId, prelude::*};
use lightyear::prelude::{Message as NetworkMessage, MessageReceiver};

use crate::network_runtime::{
    messages::{ConnectionSender, Inbox, publish_incoming},
    worker::MainUpdate,
};

type WorldCondition = fn(&World) -> bool;
pub type WorkerRelay = fn(&mut App, Sender<MainUpdate>);

struct Handler {
    system: SystemId,
    condition: WorldCondition,
}

struct MessageRoute {
    has_messages: WorldCondition,
    clear: fn(&mut World),
    install: WorkerRelay,
    handlers: Vec<usize>,
}

#[derive(Resource, Default)]
struct NetworkDispatcher {
    incoming: Vec<Handler>,
    incoming_ready: Vec<bool>,
    routes: HashMap<TypeId, MessageRoute>,
    outgoing: Vec<Handler>,
}

impl NetworkDispatcher {
    fn mark_ready_inboxes(&mut self, world: &World) {
        self.incoming_ready.fill(false);
        for route in self.routes.values() {
            if (route.has_messages)(world) {
                for &index in &route.handlers {
                    self.incoming_ready[index] = true;
                }
            }
        }
    }
}

pub fn initialize_dispatcher(app: &mut App) {
    app.init_resource::<NetworkDispatcher>()
        .init_resource::<ConnectionSender>();
}

/// Eligibility and heavy system parameters are acquired only for ready inboxes.
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

/// One handler may consume several types; readiness never invokes it twice.
pub fn add_message_route<M: NetworkMessage>(app: &mut App, id: SystemId) {
    initialize_dispatcher(app);
    app.init_resource::<Inbox<M>>();
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
            has_messages: |world| world.resource::<Inbox<M>>().has_messages(),
            clear: |world| {
                world.resource_mut::<Inbox<M>>().receive().for_each(drop);
            },
            install: install_relay::<M>,
            handlers: Vec::new(),
        });
    if !route.handlers.contains(&index) {
        route.handlers.push(index);
    }
}

/// Collected at connection time, after all feature plugins registered their routes.
pub fn worker_relays(world: &World) -> Vec<WorkerRelay> {
    world
        .resource::<NetworkDispatcher>()
        .routes
        .values()
        .map(|route| route.install)
        .collect()
}

fn install_relay<M: NetworkMessage>(app: &mut App, updates: Sender<MainUpdate>) {
    app.add_systems(
        Update,
        move |mut receivers: Query<&mut MessageReceiver<M>>| {
            for mut receiver in &mut receivers {
                if receiver.has_messages() {
                    let messages = receiver.receive().collect();
                    publish_incoming(&updates, messages).unwrap_or_else(|_| {
                        panic!("main inbox queue closed for {}", std::any::type_name::<M>())
                    });
                }
            }
        },
    );
}

/// Reset removes old connection data before accepting a replacement connection.
pub fn clear_incoming(world: &mut World) {
    world.resource_scope(|world, dispatcher: Mut<NetworkDispatcher>| {
        for route in dispatcher.routes.values() {
            (route.clear)(world);
        }
    });
}

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
    use crate::network_runtime::messages::MessageReceivers;
    use serde::{Deserialize, Serialize};
    use std::collections::VecDeque;

    #[derive(Serialize, Deserialize)]
    struct First(u32);
    #[derive(Serialize, Deserialize)]
    struct Second(u32);
    #[derive(Resource)]
    struct HeavyResource;
    #[derive(Resource, Default)]
    struct Output(Vec<u32>);
    #[derive(Resource, Default)]
    struct Outbox(VecDeque<u32>);
    #[derive(Resource)]
    struct Allowed(bool);

    fn always(_: &World) -> bool {
        true
    }

    fn consume(mut receivers: MessageReceivers<First>, mut output: ResMut<Output>) {
        for receiver in receivers.iter_mut() {
            output.0.extend(receiver.receive().map(|message| message.0));
        }
    }

    #[test]
    fn dispatches_owned_inbox_without_main_transport_components() {
        let mut app = App::new();
        app.init_resource::<Output>();
        register_message_handler::<First, _>(&mut app, consume, always);
        app.insert_resource(Inbox::new(vec![First(7), First(8)]));
        dispatch_incoming(app.world_mut());
        dispatch_incoming(app.world_mut());
        assert_eq!(app.world().resource::<Output>().0, [7, 8]);
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
        let mut app = App::new();
        register_message_handler::<First, _>(
            &mut app,
            |_: Res<HeavyResource>| panic!("empty inbox fetched heavy parameters"),
            |_| panic!("empty inbox checked eligibility"),
        );
        dispatch_incoming(app.world_mut());
        dispatch_incoming(app.world_mut());
    }

    #[test]
    fn delivered_routes_dispatch_once_in_registration_order_and_preserve_fifo() {
        let mut app = App::new();
        app.init_resource::<Output>();
        register_message_handler::<First, _>(
            &mut app,
            |mut out: ResMut<Output>| out.0.push(100),
            always,
        );
        let consumer = register_message_handler::<Second, _>(
            &mut app,
            |mut first: MessageReceivers<First>,
             mut second: MessageReceivers<Second>,
             mut out: ResMut<Output>| {
                out.0.push(200);
                for receiver in first.iter_mut() {
                    out.0.extend(receiver.receive().map(|m| m.0));
                }
                for receiver in second.iter_mut() {
                    out.0.extend(receiver.receive().map(|m| m.0));
                }
            },
            always,
        );
        add_message_route::<First>(&mut app, consumer);
        add_message_route::<First>(&mut app, consumer);
        app.insert_resource(Inbox::new(vec![First(1), First(2)]));
        app.insert_resource(Inbox::new(vec![Second(3)]));
        dispatch_incoming(app.world_mut());
        dispatch_incoming(app.world_mut());
        assert_eq!(app.world().resource::<Output>().0, [100, 200, 1, 2, 3]);
    }

    #[test]
    fn ineligible_ready_handler_retains_messages_until_eligible() {
        let mut app = App::new();
        app.init_resource::<Output>()
            .insert_resource(Allowed(false));
        register_message_handler::<First, _>(&mut app, consume, |world| {
            world.resource::<Allowed>().0
        });
        app.insert_resource(Inbox::new(vec![First(42)]));
        dispatch_incoming(app.world_mut());
        assert!(app.world().resource::<Output>().0.is_empty());
        assert_eq!(app.world().resource::<Inbox<First>>().num_messages(), 1);
        app.world_mut().resource_mut::<Allowed>().0 = true;
        dispatch_incoming(app.world_mut());
        assert_eq!(app.world().resource::<Output>().0, [42]);
    }

    #[test]
    fn reset_discards_old_connection_inboxes() {
        let mut app = App::new();
        app.init_resource::<Output>();
        register_message_handler::<First, _>(&mut app, consume, always);
        app.insert_resource(Inbox::new(vec![First(1)]));
        clear_incoming(app.world_mut());
        dispatch_incoming(app.world_mut());
        assert!(app.world().resource::<Output>().0.is_empty());
        app.insert_resource(Inbox::new(vec![First(2)]));
        dispatch_incoming(app.world_mut());
        assert_eq!(app.world().resource::<Output>().0, [2]);
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
                out.0.extend(queue.0.drain(..))
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
