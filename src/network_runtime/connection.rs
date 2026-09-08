//! One transport world per connection; main-world markers contain no transport state.

use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::mpsc::Sender,
};

use bevy::prelude::*;
use lightyear::prelude::{self as network, client as client_network};

use super::{
    messages::ConnectionSender,
    replication::{ReplicationMirrorMap, register_replication_bridge},
    worker::{MainUpdate, NetworkRuntime},
};

#[derive(Component, Default)]
pub struct Client;
#[derive(Component)]
pub struct Connected;
#[derive(Component, Default)]
pub struct Disconnected {
    pub reason: Option<String>,
}

#[derive(Resource, Default)]
struct ConnectionEvents(VecDeque<(Entity, ConnectionEvent)>);

enum ConnectionEvent {
    Connected,
    Disconnected(Option<String>),
}

pub fn initialize_connection_bridge(app: &mut App) {
    app.init_resource::<ConnectionSender>()
        .init_resource::<ConnectionEvents>();
    super::replication::initialize_replication_mirror(app);
}

/// Starts only after all application routes have been registered.
pub fn start_connection(
    world: &mut World,
    server_addr: SocketAddr,
    client_id: u64,
) -> Result<Entity, String> {
    stop_connection(world)?;
    let relays = crate::network_events::worker_relays(world);
    let proxy = world.spawn(Client).id();
    let runtime = NetworkRuntime::spawn(move |app, updates| {
        for install in relays {
            install(app, updates.clone());
        }
        register_replication_bridge(app, updates.clone());
        register_connection_events(app, updates, proxy);
        connect_worker(app.world_mut(), server_addr, client_id);
    })?;
    world.insert_resource(runtime);
    Ok(proxy)
}

/// Ask the worker's Netcode client to disconnect; its normal lifecycle publisher
/// reports the resulting state before the main world resets the connection.
pub fn request_disconnect(runtime: &NetworkRuntime) -> Result<(), String> {
    runtime.enqueue(|world| {
        let entity = world
            .query_filtered::<Entity, With<client_network::NetcodeClient>>()
            .single(world)
            .unwrap_or_else(|error| {
                panic!("expected one worker NetcodeClient for disconnect: {error}")
            });
        world.trigger(client_network::Disconnect { entity });
    })
}

pub fn stop_connection(world: &mut World) -> Result<(), String> {
    if let Some(mut runtime) = world.remove_resource::<NetworkRuntime>() {
        runtime.stop()?;
        crate::network_events::clear_incoming(world);
    }
    if let Some(mut sender) = world.get_resource_mut::<ConnectionSender>() {
        sender.sender = None;
    }
    if let Some(mut events) = world.get_resource_mut::<ConnectionEvents>() {
        events.0.clear();
    }
    if let Some(mut map) = world.get_resource_mut::<ReplicationMirrorMap>() {
        map.clear();
    }
    let proxies: Vec<_> = world
        .query_filtered::<Entity, With<Client>>()
        .iter(world)
        .collect();
    for entity in proxies {
        world.despawn(entity);
    }
    Ok(())
}

fn connect_worker(world: &mut World, server_addr: SocketAddr, client_id: u64) {
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
    .unwrap_or_else(|error| panic!("failed to construct network client: {error}"));
    let entity = world
        .spawn((
            network::LocalAddr(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
            network::PeerAddr(server_addr),
            network::UdpIo::default(),
            netcode,
        ))
        .id();
    world.trigger(client_network::Connect { entity });
}

fn register_connection_events(app: &mut App, updates: Sender<MainUpdate>, proxy: Entity) {
    app.add_observer(
        |event: On<Add, client_network::Connected>, mut commands: Commands| {
            commands
                .entity(event.entity)
                .insert(network::ReplicationReceiver);
        },
    );
    // Publish after Update message relays so ForcedDisconnect is queued first.
    app.add_systems(
        PostUpdate,
        move |connected: Query<(), Added<client_network::Connected>>,
              disconnected: Query<
            &client_network::Disconnected,
            Added<client_network::Disconnected>,
        >| {
            for () in &connected {
                publish_connection_event(&updates, proxy, ConnectionEvent::Connected);
            }
            for state in &disconnected {
                publish_connection_event(
                    &updates,
                    proxy,
                    ConnectionEvent::Disconnected(state.reason.clone()),
                );
            }
        },
    );
}

fn publish_connection_event(updates: &Sender<MainUpdate>, proxy: Entity, event: ConnectionEvent) {
    updates
        .send(Box::new(move |world| {
            world
                .resource_mut::<ConnectionEvents>()
                .0
                .push_back((proxy, event));
        }))
        .unwrap_or_else(|_| panic!("main connection event queue closed"));
}

/// Replies can precede main connection callbacks. Make their sending endpoint ready first,
/// without moving disconnect callbacks ahead of their server notices.
pub fn prepare_connection_senders(world: &mut World) {
    let connected = world
        .resource::<ConnectionEvents>()
        .0
        .iter()
        .any(|(proxy, event)| {
            matches!(event, ConnectionEvent::Connected) && world.get::<Client>(*proxy).is_some()
        });
    if connected {
        let sender = world.resource::<NetworkRuntime>().command_sender();
        world.resource_mut::<ConnectionSender>().sender = Some(sender);
    }
}

/// Run after incoming handlers: forced-disconnect notices precede state transitions.
pub fn apply_connection_events(world: &mut World) {
    let events = std::mem::take(&mut world.resource_mut::<ConnectionEvents>().0);
    for (proxy, event) in events {
        if world.get::<Client>(proxy).is_none() {
            continue;
        }
        match event {
            ConnectionEvent::Connected => {
                let sender = world.resource::<NetworkRuntime>().command_sender();
                world.resource_mut::<ConnectionSender>().sender = Some(sender);
                world
                    .entity_mut(proxy)
                    .remove::<Disconnected>()
                    .insert(Connected);
            }
            ConnectionEvent::Disconnected(reason) => {
                world.resource_mut::<ConnectionSender>().sender = None;
                world
                    .entity_mut(proxy)
                    .remove::<Connected>()
                    .insert(Disconnected { reason });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{net::UdpSocket, time::Duration};

    #[test]
    fn udp_handshake_progresses_without_updating_main_world() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut main = App::new();
        crate::network_events::initialize_dispatcher(&mut main);
        initialize_connection_bridge(&mut main);
        start_connection(main.world_mut(), server.local_addr().unwrap(), 1234).unwrap();
        let mut packet = [0; 2048];
        let (first_size, first_peer) = server.recv_from(&mut packet).unwrap();
        let (second_size, second_peer) = server.recv_from(&mut packet).unwrap();
        assert!(first_size > 0 && second_size > 0);
        assert_eq!(first_peer, second_peer);
        stop_connection(main.world_mut()).unwrap();
    }

    #[test]
    fn disconnect_request_reports_closed_worker() {
        let mut runtime = NetworkRuntime::spawn(|_, _| {}).unwrap();
        runtime.stop().unwrap();
        let error = request_disconnect(&runtime).unwrap_err();
        assert!(error.contains("network worker command queue disconnected"));
    }

    #[test]
    fn disconnect_request_reports_missing_netcode_client() {
        let mut runtime = NetworkRuntime::spawn(|_, _| {}).unwrap();
        request_disconnect(&runtime).unwrap();
        let error = runtime.stop().unwrap_err();
        assert!(error.contains("expected one worker NetcodeClient for disconnect"));
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct OldReply(u32);

    fn queue_worker_disconnect(main: &App, reason: &'static str) {
        let (finished, completion) = std::sync::mpsc::sync_channel(1);
        main.world()
            .resource::<NetworkRuntime>()
            .enqueue(move |world| {
                let client = world
                    .query_filtered::<Entity, With<client_network::Client>>()
                    .single(world)
                    .unwrap();
                world
                    .entity_mut(client)
                    .remove::<client_network::Disconnected>();
                world
                    .entity_mut(client)
                    .insert(client_network::Disconnected {
                        reason: Some(reason.into()),
                    });
                let mut finished = Some(finished);
                world
                    .resource_mut::<Schedules>()
                    .get_mut(Last)
                    .unwrap()
                    .add_systems(move || {
                        if let Some(finished) = finished.take() {
                            finished.send(()).unwrap();
                        }
                    });
            })
            .unwrap();
        completion.recv_timeout(Duration::from_secs(5)).unwrap();
    }

    #[test]
    fn worker_restart_discards_old_queues_and_resumes_udp_handshake() {
        use super::super::messages::Inbox;

        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let address = server.local_addr().unwrap();
        let mut main = App::new();
        crate::network_events::register_message_handler::<OldReply, _>(&mut main, || {}, |_| true);
        initialize_connection_bridge(&mut main);
        let old = start_connection(main.world_mut(), address, 1234).unwrap();
        let old_sender = main.world().resource::<NetworkRuntime>().command_sender();
        let mut packet = [0; 2048];
        assert!(server.recv_from(&mut packet).unwrap().0 > 0);

        // A real worker PostUpdate publishes before the Last-stage barrier.
        queue_worker_disconnect(&main, "already drained old lifecycle");
        main.world_mut()
            .resource_scope(|world, runtime: Mut<NetworkRuntime>| {
                runtime.drain_updates(world).unwrap();
            });
        assert!(
            main.world()
                .resource::<ConnectionEvents>()
                .0
                .iter()
                .any(|(proxy, event)| *proxy == old
                    && matches!(event,
                ConnectionEvent::Disconnected(Some(reason))
                    if reason == "already drained old lifecycle"))
        );
        main.world_mut()
            .insert_resource(Inbox::new(vec![OldReply(42)]));
        let mirrored = main.world_mut().spawn_empty().id();
        main.world_mut()
            .resource_mut::<ReplicationMirrorMap>()
            .insert(old, mirrored);
        main.world_mut().resource_mut::<ConnectionSender>().sender = Some(old_sender.clone());

        // Leave a second worker-published update undrained across shutdown.
        queue_worker_disconnect(&main, "undrained old lifecycle");
        stop_connection(main.world_mut()).unwrap();
        assert!(
            old_sender
                .send(super::super::worker::NetworkCommand::Stop)
                .is_err()
        );
        assert!(!main.world().contains_resource::<NetworkRuntime>());
        assert!(main.world().resource::<ConnectionSender>().sender.is_none());
        assert!(main.world().resource::<ConnectionEvents>().0.is_empty());
        assert!(!main.world().resource::<Inbox<OldReply>>().has_messages());
        assert!(main.world().get_entity(old).is_err());
        let map = main.world().resource::<ReplicationMirrorMap>();
        assert_eq!(map.server_to_main(old), None);
        assert_eq!(map.main_to_server(mirrored), None);

        // Remove packets from the stopped worker before observing its replacement.
        server.set_nonblocking(true).unwrap();
        loop {
            match server.recv_from(&mut packet) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) => panic!("failed to drain old UDP packets: {error}"),
            }
        }
        server.set_nonblocking(false).unwrap();
        let new = start_connection(main.world_mut(), address, 5678).unwrap();
        assert_ne!(old, new);
        assert!(server.recv_from(&mut packet).unwrap().0 > 0);
        assert!(server.recv_from(&mut packet).unwrap().0 > 0);
        main.world_mut()
            .resource_scope(|world, runtime: Mut<NetworkRuntime>| {
                runtime.drain_updates(world).unwrap();
            });
        assert!(
            main.world()
                .resource::<ConnectionEvents>()
                .0
                .iter()
                .all(|(proxy, _)| *proxy == new)
        );
        assert!(!main.world().resource::<Inbox<OldReply>>().has_messages());
        assert!(main.world().get::<Client>(new).is_some());
        stop_connection(main.world_mut()).unwrap();
    }

    #[test]
    fn reset_discards_pending_lifecycle_and_replaces_proxy_identity() {
        let mut app = App::new();
        initialize_connection_bridge(&mut app);
        let old = app.world_mut().spawn(Client).id();
        app.world_mut()
            .resource_mut::<ConnectionEvents>()
            .0
            .push_back((old, ConnectionEvent::Disconnected(Some("old".into()))));
        stop_connection(app.world_mut()).unwrap();
        let new = app.world_mut().spawn(Client).id();
        assert_ne!(old, new);
        apply_connection_events(app.world_mut());
        assert!(app.world().get::<Disconnected>(new).is_none());
        assert!(app.world().get_entity(old).is_err());
    }
}
