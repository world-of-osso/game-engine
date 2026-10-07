//! Private loopback transport fixture for native meeting-stone host tests.
use super::*;
use lightyear::prelude::{LinkOf, ReplicationSender, server};
use shared::protocol::{
    CastFailed, CombatChannel, InteractionChannel, SummonRequest, SummonResponse, UseGameObject,
};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicU16, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeetingStoneRequest {
    Use(UseGameObject),
    Answer(SummonResponse),
}

#[derive(Resource, Default)]
struct Requests(Vec<MeetingStoneRequest>);

fn capture_requests(
    mut uses: Query<&mut MessageReceiver<UseGameObject>>,
    mut answers: Query<&mut MessageReceiver<SummonResponse>>,
    mut received: ResMut<Requests>,
) {
    for mut receiver in &mut uses {
        received
            .0
            .extend(receiver.receive().map(MeetingStoneRequest::Use));
    }
    for mut receiver in &mut answers {
        received
            .0
            .extend(receiver.receive().map(MeetingStoneRequest::Answer));
    }
}

fn allocate_address() -> SocketAddr {
    static NEXT_PORT: AtomicU16 = AtomicU16::new(24500);
    loop {
        let port = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
        assert!(port < 25000, "meeting-stone fixture port range exhausted");
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        if UdpSocket::bind(address).is_ok() {
            return address;
        }
    }
}

pub struct MeetingStoneServerFixture {
    app: App,
    address: SocketAddr,
}

impl MeetingStoneServerFixture {
    pub fn start() -> Self {
        let address = allocate_address();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
        app.add_plugins(StatesPlugin);
        app.add_plugins(server::ServerPlugins {
            tick_duration: SIMULATION_INTERVAL,
        });
        app.add_plugins(shared::ProtocolPlugin);
        app.init_resource::<Requests>();
        app.add_systems(Update, capture_requests);
        app.add_observer(|link: On<Add, LinkOf>, mut commands: Commands| {
            commands.entity(link.entity).insert(ReplicationSender);
        });
        app.finish();
        app.cleanup();
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

    pub fn send(&mut self, request: SummonRequest) {
        let world = self.app.world_mut();
        world
            .query::<&mut MessageSender<SummonRequest>>()
            .single_mut(world)
            .expect("connected fixture summon sender")
            .send::<InteractionChannel>(request);
    }

    pub fn send_refusal(&mut self, refusal: CastFailed) {
        let world = self.app.world_mut();
        world
            .query::<&mut MessageSender<CastFailed>>()
            .single_mut(world)
            .expect("connected fixture cast sender")
            .send::<CombatChannel>(refusal);
    }

    pub fn take_requests(&mut self) -> Vec<MeetingStoneRequest> {
        std::mem::take(&mut self.app.world_mut().resource_mut::<Requests>().0)
    }
}
