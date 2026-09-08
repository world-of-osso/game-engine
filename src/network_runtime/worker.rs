//! Dedicated network ECS owner. Rendering never drives this world's clock.

use std::any::Any;
use std::sync::{
    Mutex,
    mpsc::{self, Receiver, Sender, TryRecvError},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use bevy::app::{AppExit, ScheduleRunnerPlugin};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use lightyear::prelude::client::ClientPlugins;

const NETWORK_HZ: u128 = 60;
const NANOS_PER_SECOND: u128 = 1_000_000_000;
const SIMULATION_INTERVAL: Duration = Duration::from_millis(50);

pub type MainUpdate = Box<dyn FnOnce(&mut World) + Send + 'static>;

pub enum NetworkCommand {
    Apply(Box<dyn FnOnce(&mut World) + Send + 'static>),
    Stop,
}

/// Main-world queue endpoint and sole owner of the joinable network thread.
#[derive(Resource)]
pub struct NetworkRuntime {
    commands: Sender<NetworkCommand>,
    updates: Mutex<Receiver<MainUpdate>>,
    worker: Option<JoinHandle<Result<(), String>>>,
}

impl NetworkRuntime {
    pub fn spawn(
        configure: impl FnOnce(&mut App, Sender<MainUpdate>) + Send + 'static,
    ) -> Result<Self, String> {
        let (commands, incoming_commands) = mpsc::channel();
        let (outgoing_updates, updates) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("network-60hz".into())
            .spawn(move || run_worker(incoming_commands, outgoing_updates, configure))
            .map_err(|error| format!("failed to spawn network worker: {error}"))?;
        Ok(Self {
            commands,
            updates: Mutex::new(updates),
            worker: Some(worker),
        })
    }

    pub fn command_sender(&self) -> Sender<NetworkCommand> {
        self.commands.clone()
    }

    pub fn enqueue(&self, command: impl FnOnce(&mut World) + Send + 'static) -> Result<(), String> {
        self.commands
            .send(NetworkCommand::Apply(Box::new(command)))
            .map_err(|_| "network worker command queue disconnected".into())
    }

    /// Queue protocol data without waiting for a main-world connection callback.
    /// Lightyear buffers it until its transport is connected.
    pub fn queue_message<M, C>(&self, message: M) -> Result<(), String>
    where
        M: lightyear::prelude::Message,
        C: lightyear::prelude::Channel,
    {
        self.enqueue(move |world| super::messages::send_in_worker::<M, C>(world, message))
    }

    /// Applies queued updates on the caller's world, never on the worker thread.
    pub fn drain_updates(&self, world: &mut World) -> Result<(), String> {
        loop {
            // Release the queue lock before invoking caller code.
            let update = self
                .updates
                .lock()
                .map_err(|_| "network update queue mutex poisoned")?
                .try_recv();
            match update {
                Ok(update) => update(world),
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) if self.worker.is_none() => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    return Err("network worker update queue disconnected; call stop to join and inspect its failure".into());
                }
            }
        }
    }

    pub fn stop(&mut self) -> Result<(), String> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let request = self.commands.send(NetworkCommand::Stop);
        let outcome = worker
            .join()
            .map_err(describe_panic)
            .and_then(|result| result);
        match (request, outcome) {
            (_, Err(error)) => Err(error),
            // A closed command queue is expected if Stop/AppExit already ended
            // the worker. Joining confirms that it actually exited cleanly.
            (_, Ok(())) => Ok(()),
        }
    }
}

impl Drop for NetworkRuntime {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("network worker shutdown failed: {error}");
        }
    }
}

fn run_worker(
    commands: Receiver<NetworkCommand>,
    updates: Sender<MainUpdate>,
    configure: impl FnOnce(&mut App, Sender<MainUpdate>),
) -> Result<(), String> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>());
    app.add_plugins(StatesPlugin);
    app.add_plugins(ClientPlugins {
        tick_duration: SIMULATION_INTERVAL,
    });
    app.add_plugins(shared::ProtocolPlugin);
    configure(&mut app, updates.clone());
    app.finish();
    app.cleanup();
    let result = run_ticks(&mut app, &commands, &updates);
    // Keep the update endpoint open even if configure registers no producers.
    drop(updates);
    result
}

fn run_ticks(
    app: &mut App,
    commands: &Receiver<NetworkCommand>,
    updates: &Sender<MainUpdate>,
) -> Result<(), String> {
    let started = Instant::now();
    while apply_commands(app.world_mut(), commands)? {
        app.update();
        updates
            .send(Box::new(|world| {
                world.init_resource::<crate::network_tick::PendingNetworkTicks>();
                world
                    .resource_mut::<crate::network_tick::PendingNetworkTicks>()
                    .0 += 1;
            }))
            .map_err(|_| "main update queue closed while publishing network tick permit")?;
        if let Some(exit) = app.should_exit() {
            return match exit {
                AppExit::Success => Ok(()),
                AppExit::Error(code) => Err(format!("network application exited with {code}")),
            };
        }
        // Skip missed deadlines rather than burst/spin after a slow update.
        // TimePlugin still advances transport timers by the full elapsed time.
        thread::sleep(next_tick_delay(started.elapsed()));
    }
    Ok(())
}

fn apply_commands(world: &mut World, commands: &Receiver<NetworkCommand>) -> Result<bool, String> {
    loop {
        match commands.try_recv() {
            Ok(NetworkCommand::Apply(command)) => command(world),
            Ok(NetworkCommand::Stop) => return Ok(false),
            Err(TryRecvError::Empty) => return Ok(true),
            Err(TryRecvError::Disconnected) => {
                return Err("network command queue disconnected without a stop request".into());
            }
        }
    }
}

fn next_tick_delay(elapsed: Duration) -> Duration {
    let elapsed_nanos = elapsed.as_nanos();
    let next_tick = elapsed_nanos * NETWORK_HZ / NANOS_PER_SECOND + 1;
    let deadline_nanos = (next_tick * NANOS_PER_SECOND).div_ceil(NETWORK_HZ);
    Duration::from_nanos((deadline_nanos - elapsed_nanos) as u64)
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
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    #[test]
    fn rational_deadlines_cover_one_second_without_drift_or_catch_up() {
        let mut elapsed = Duration::ZERO;
        for _ in 0..60 {
            let delay = next_tick_delay(elapsed);
            assert!(delay >= Duration::from_nanos(16_666_666));
            assert!(delay <= Duration::from_nanos(16_666_667));
            elapsed += delay;
        }
        assert_eq!(elapsed, Duration::from_secs(1));
        assert_eq!(
            next_tick_delay(Duration::from_millis(255)),
            Duration::from_nanos(11_666_667)
        );
    }

    #[derive(Resource, Default)]
    struct MainCounter(usize);

    #[test]
    fn worker_ticks_while_main_app_is_not_updated_and_stops_cleanly() {
        let ticks = Arc::new(AtomicUsize::new(0));
        let observed_ticks = Arc::clone(&ticks);
        let (progress, observed) = mpsc::channel();
        let (ready, started) = mpsc::channel();
        let mut runtime = NetworkRuntime::spawn(move |app, updates| {
            app.add_systems(Startup, move || {
                ready.send(()).expect("main waiting for startup")
            });
            app.add_systems(Update, move || {
                let count = observed_ticks.fetch_add(1, Ordering::SeqCst) + 1;
                progress.send(count).expect("main awaiting worker ticks");
                updates
                    .send(Box::new(move |world| {
                        world.resource_mut::<MainCounter>().0 = count;
                    }))
                    .expect("main update receiver is alive");
            });
        })
        .expect("worker starts");
        let mut main = App::new();
        main.init_resource::<MainCounter>();
        started
            .recv_timeout(TEST_TIMEOUT)
            .expect("worker startup completes");
        for expected in 1..=3 {
            assert_eq!(
                observed
                    .recv_timeout(TEST_TIMEOUT)
                    .expect("worker progresses"),
                expected
            );
        }
        assert_eq!(main.world().resource::<MainCounter>().0, 0);
        runtime
            .drain_updates(main.world_mut())
            .expect("apply worker updates");
        assert!(main.world().resource::<MainCounter>().0 > 0);
        runtime.stop().expect("worker joins cleanly");
        let stopped = ticks.load(Ordering::SeqCst);
        thread::sleep(Duration::from_millis(35));
        assert_eq!(ticks.load(Ordering::SeqCst), stopped);
        assert!(runtime.enqueue(|_| {}).is_err());
        runtime.stop().expect("stopping twice is harmless");
    }

    #[derive(Resource, Default)]
    struct WorkerValues(Vec<u32>);
    #[derive(Resource, Default)]
    struct MainValues(Vec<Vec<u32>>);

    #[test]
    fn commands_and_replies_preserve_fifo() {
        let (configured, replies) = mpsc::channel();
        let mut runtime = NetworkRuntime::spawn(move |app, updates| {
            app.init_resource::<WorkerValues>();
            configured.send(updates).expect("caller awaiting queue");
        })
        .expect("worker starts");
        let updates = replies
            .recv_timeout(TEST_TIMEOUT)
            .expect("worker configured");
        let (done, received) = mpsc::channel();
        for value in [5, 2, 9] {
            let updates = updates.clone();
            runtime
                .enqueue(move |world| {
                    let mut values = world.resource_mut::<WorkerValues>();
                    values.0.push(value);
                    let snapshot = values.0.clone();
                    updates
                        .send(Box::new(move |main| {
                            main.resource_mut::<MainValues>().0.push(snapshot);
                        }))
                        .expect("main queue remains open");
                })
                .expect("queue command");
        }
        runtime
            .command_sender()
            .send(NetworkCommand::Apply(Box::new(move |_| {
                done.send(()).expect("caller waiting");
            })))
            .expect("queue completion marker");
        received
            .recv_timeout(TEST_TIMEOUT)
            .expect("commands executed");
        let mut main = World::new();
        main.init_resource::<MainValues>();
        runtime
            .drain_updates(&mut main)
            .expect("receive ordered snapshots");
        assert_eq!(
            main.resource::<MainValues>().0,
            [vec![5], vec![5, 2], vec![5, 2, 9]]
        );
        runtime.stop().expect("worker joins");
    }

    #[test]
    fn command_panic_is_reported_by_join() {
        let mut runtime = NetworkRuntime::spawn(|_, _| {}).expect("worker starts");
        runtime
            .enqueue(|_| panic!("worker command failure marker"))
            .expect("queue panic");
        let failure = runtime.stop().expect_err("worker panic must reach caller");
        assert!(
            failure.contains("worker command failure marker"),
            "{failure}"
        );
    }

    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    struct WireValue(u32);
    struct WireChannel;

    #[derive(Resource, Default)]
    struct DeliveredValues(Vec<u32>);

    #[derive(Resource)]
    struct LoopbackPacketCount(Arc<AtomicUsize>);

    fn spawn_loopback_peer(
        mut commands: Commands,
        channels: Res<lightyear::prelude::ChannelRegistry>,
    ) {
        use lightyear::prelude::{
            Connected, Link, Linked, MessageReceiver, MessageSender, PeerId, RemoteId, Transport,
        };
        let mut transport = Transport::default();
        transport.add_sender_from_registry::<WireChannel>(&channels);
        transport.add_receiver_from_registry::<WireChannel>(&channels);
        let mut sender = MessageSender::<WireValue>::default();
        sender.send::<WireChannel>(WireValue(17));
        sender.send::<WireChannel>(WireValue(23));
        commands.spawn((
            Link::default(),
            transport,
            Linked,
            Connected,
            RemoteId(PeerId::Local(0)),
            MessageReceiver::<WireValue>::default(),
            sender,
        ));
    }

    fn loopback_packets(
        mut links: Query<&mut lightyear::prelude::Link>,
        packet_count: Res<LoopbackPacketCount>,
    ) {
        for mut link in &mut links {
            let packets: Vec<_> = link.send.drain().collect();
            packet_count.0.fetch_add(packets.len(), Ordering::SeqCst);
            for packet in packets {
                assert!(!packet.is_empty(), "transport must encode nonempty packets");
                link.recv.push_raw(packet);
            }
        }
    }

    fn configure_loopback(
        app: &mut App,
        updates: Sender<MainUpdate>,
        delivered: Sender<Vec<u32>>,
        packets: Arc<AtomicUsize>,
    ) {
        use lightyear::prelude::{
            AppChannelExt, AppMessageExt, ChannelMode, ChannelSettings, LinkSystems,
            MessageReceiver,
        };
        app.register_message::<WireValue>();
        app.add_channel::<WireChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(Default::default()),
            ..Default::default()
        });
        app.insert_resource(LoopbackPacketCount(packets));
        app.add_systems(Startup, spawn_loopback_peer);
        app.add_systems(PostUpdate, loopback_packets.after(LinkSystems::Send));
        app.add_systems(
            Update,
            move |mut receivers: Query<&mut MessageReceiver<WireValue>>| {
                for mut receiver in &mut receivers {
                    let values: Vec<_> = receiver.receive().map(|message| message.0).collect();
                    if values.is_empty() {
                        continue;
                    }
                    let proof = values.clone();
                    updates
                        .send(Box::new(move |world| {
                            world.resource_mut::<DeliveredValues>().0.extend(values);
                        }))
                        .expect("main update queue remains alive");
                    delivered
                        .send(proof)
                        .expect("main awaiting decoded messages");
                }
            },
        );
    }

    #[test]
    fn transport_decodes_packets_while_main_app_remains_unupdated() {
        let packets = Arc::new(AtomicUsize::new(0));
        let worker_packets = Arc::clone(&packets);
        let (decoded, observed) = mpsc::channel();
        let mut runtime = NetworkRuntime::spawn(move |app, updates| {
            configure_loopback(app, updates, decoded, worker_packets);
        })
        .expect("worker starts");
        let mut main = App::new();
        main.init_resource::<DeliveredValues>()
            .init_resource::<MainCounter>();
        main.add_systems(Update, |mut count: ResMut<MainCounter>| count.0 += 1);
        let mut received = Vec::new();
        while received.len() < 2 {
            received.extend(
                observed
                    .recv_timeout(TEST_TIMEOUT)
                    .expect("worker decodes real packets"),
            );
        }
        assert_eq!(received, [17, 23]);
        assert!(packets.load(Ordering::SeqCst) > 0);
        assert_eq!(main.world().resource::<MainCounter>().0, 0);
        assert!(main.world().resource::<DeliveredValues>().0.is_empty());
        runtime
            .drain_updates(main.world_mut())
            .expect("apply decoded worker messages");
        assert_eq!(main.world().resource::<DeliveredValues>().0, [17, 23]);
        assert_eq!(main.world().resource::<MainCounter>().0, 0);
        runtime.stop().expect("join network worker");
    }

    #[derive(Resource)]
    struct WorkerLifetime(Arc<AtomicBool>);

    impl Drop for WorkerLifetime {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }

    #[test]
    fn dropping_runtime_joins_and_drops_worker_world() {
        let alive = Arc::new(AtomicBool::new(true));
        let worker_alive = Arc::clone(&alive);
        let (ready, configured) = mpsc::channel();
        let runtime = NetworkRuntime::spawn(move |app, _| {
            app.insert_resource(WorkerLifetime(worker_alive));
            ready.send(()).expect("caller waiting");
        })
        .expect("worker starts");
        configured
            .recv_timeout(TEST_TIMEOUT)
            .expect("worker configured");
        drop(runtime);
        assert!(!alive.load(Ordering::SeqCst));
    }
}
