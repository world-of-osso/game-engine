//! Loopback death server for native host tests; owns no development service.
use super::*;
use lightyear::prelude::{LinkOf, ReplicationSender, server};
use shared::protocol::{
    AcceptSpiritHealerResurrection, DeathChannel, DeathStateUpdate, ReleaseSpirit,
    ResurrectAtCorpse,
};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicU16, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceivedDeathRequest {
    Release,
    Corpse,
    SpiritHealer,
}

#[derive(Resource, Default)]
struct DeathRequests(Vec<ReceivedDeathRequest>);

fn capture_requests(
    mut releases: Query<&mut MessageReceiver<ReleaseSpirit>>,
    mut corpses: Query<&mut MessageReceiver<ResurrectAtCorpse>>,
    mut healers: Query<&mut MessageReceiver<AcceptSpiritHealerResurrection>>,
    mut received: ResMut<DeathRequests>,
) {
    for mut receiver in &mut releases {
        received
            .0
            .extend(receiver.receive().map(|_| ReceivedDeathRequest::Release));
    }
    for mut receiver in &mut corpses {
        received
            .0
            .extend(receiver.receive().map(|_| ReceivedDeathRequest::Corpse));
    }
    for mut receiver in &mut healers {
        received.0.extend(
            receiver
                .receive()
                .map(|_| ReceivedDeathRequest::SpiritHealer),
        );
    }
}

/// Ports below the OS ephemeral range, isolated from the existing 20000+ fixtures.
fn allocate_address() -> SocketAddr {
    static NEXT_PORT: AtomicU16 = AtomicU16::new(24000);
    loop {
        let port = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
        assert!(port < 32000, "death fixture port range exhausted");
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        if UdpSocket::bind(address).is_ok() {
            return address;
        }
    }
}

pub struct DeathServerFixture {
    app: App,
    address: SocketAddr,
}

fn create_server_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(server::ServerPlugins {
        tick_duration: SIMULATION_INTERVAL,
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.init_resource::<DeathRequests>();
    app.add_systems(Update, capture_requests);
    app.add_observer(|link: On<Add, LinkOf>, mut commands: Commands| {
        commands.entity(link.entity).insert(ReplicationSender);
    });
    app.finish();
    app.cleanup();
    app
}

impl DeathServerFixture {
    pub fn start() -> Self {
        let address = allocate_address();
        let mut app = create_server_app();
        let entity = app
            .world_mut()
            .spawn((
                network::LocalAddr(address),
                server::ServerUdpIo::default(),
                server::NetcodeServer::new(server::NetcodeConfig::default()),
            ))
            .id();
        app.world_mut().trigger(server::Start { entity });
        Self { app, address }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn step(&mut self) {
        self.app.update();
    }

    pub fn send(&mut self, update: DeathStateUpdate) {
        let world = self.app.world_mut();
        world
            .query::<&mut MessageSender<DeathStateUpdate>>()
            .single_mut(world)
            .expect("connected fixture death sender")
            .send::<DeathChannel>(update);
    }

    pub fn take_requests(&mut self) -> Vec<ReceivedDeathRequest> {
        std::mem::take(&mut self.app.world_mut().resource_mut::<DeathRequests>().0)
    }
}
