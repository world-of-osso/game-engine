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
    let auth = client_network::Authentication::Manual {
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
