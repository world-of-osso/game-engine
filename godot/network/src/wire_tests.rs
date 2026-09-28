//! Real loopback UDP proof; owns its server and never contacts the development game server.

use super::*;
use lightyear::prelude::{LinkOf, NetworkTarget, Replicate, ReplicationSender, server};
use shared::{
    components::MovementControl,
    protocol::{InputChannel, PlayerInput},
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

fn create_fixture_server() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(server::ServerPlugins {
        tick_duration: SIMULATION_INTERVAL,
    });
    app.add_plugins(shared::ProtocolPlugin);
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
    // ServerUdpIo binds its own socket and does not expose the assigned port for port zero.
    let reservation = UdpSocket::bind("127.0.0.1:0").expect("reserve fixture UDP port");
    let address = reservation.local_addr().expect("read fixture UDP address");
    drop(reservation);
    let mut app = create_fixture_server();
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
