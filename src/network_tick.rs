//! Main-world application work authorized by the independent network worker.
//! First drains worker updates, then runs one ordered schedule per queued permit.

use bevy::app::First;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

pub const NETWORK_TICKS_PER_SECOND: u64 = 60;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkTick;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetworkTickSystems {
    Receive,
    Apply,
    Send,
}

#[derive(Resource, Default)]
pub struct PendingNetworkTicks(pub u64);

pub struct NetworkTickPlugin;

impl Plugin for NetworkTickPlugin {
    fn build(&self, app: &mut App) {
        crate::network_events::initialize_dispatcher(app);
        app.init_resource::<PendingNetworkTicks>()
            .init_schedule(NetworkTick)
            .configure_sets(
                NetworkTick,
                (
                    NetworkTickSystems::Receive,
                    NetworkTickSystems::Apply,
                    NetworkTickSystems::Send,
                )
                    .chain(),
            )
            .add_systems(First, (apply_network_updates, run_network_ticks).chain())
            .add_systems(
                NetworkTick,
                crate::network_events::dispatch_incoming.in_set(NetworkTickSystems::Receive),
            )
            .add_systems(
                NetworkTick,
                crate::network_events::dispatch_outgoing.in_set(NetworkTickSystems::Send),
            );
    }
}

fn apply_network_updates(world: &mut World) {
    use crate::network_runtime::worker::NetworkRuntime;
    if world.contains_resource::<NetworkRuntime>() {
        world.resource_scope(|world, runtime: Mut<NetworkRuntime>| {
            runtime
                .drain_updates(world)
                .unwrap_or_else(|error| panic!("{error}"));
        });
    }
}

fn run_network_ticks(world: &mut World) {
    let pending = std::mem::take(&mut world.resource_mut::<PendingNetworkTicks>().0);
    for _ in 0..pending {
        world.run_schedule(NetworkTick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_runtime::{
        messages::{Inbox, publish_incoming},
        worker::NetworkRuntime,
    };
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        time::Duration,
    };

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Number(usize);

    #[derive(Resource, Default)]
    struct Observations {
        stages: Vec<&'static str>,
        received: Vec<usize>,
        receive_batches: Vec<usize>,
        ticks_at_render: Vec<usize>,
    }

    fn main_app() -> App {
        let mut app = App::new();
        app.add_plugins(NetworkTickPlugin)
            .init_resource::<Time<bevy::time::Real>>()
            .init_resource::<Inbox<Number>>()
            .init_resource::<Observations>()
            .add_systems(NetworkTick, receive.in_set(NetworkTickSystems::Receive))
            .add_systems(
                NetworkTick,
                (|mut seen: ResMut<Observations>| {
                    seen.stages.push("apply");
                })
                .in_set(NetworkTickSystems::Apply),
            )
            .add_systems(
                NetworkTick,
                (|mut seen: ResMut<Observations>| {
                    seen.stages.push("send");
                })
                .in_set(NetworkTickSystems::Send),
            )
            .add_systems(Update, |mut seen: ResMut<Observations>| {
                let ticks = seen.receive_batches.len();
                seen.ticks_at_render.push(ticks);
            });
        app
    }

    fn receive(mut inbox: ResMut<Inbox<Number>>, mut seen: ResMut<Observations>) {
        seen.stages.push("receive");
        let values: Vec<_> = inbox.receive().map(|number| number.0).collect();
        seen.receive_batches.push(values.len());
        seen.received.extend(values);
    }

    fn start_worker() -> (NetworkRuntime, mpsc::Receiver<usize>, Arc<AtomicUsize>) {
        let completed = Arc::new(AtomicUsize::new(0));
        let worker_completed = Arc::clone(&completed);
        let (progress, received) = mpsc::channel();
        let runtime = NetworkRuntime::spawn(move |app, updates| {
            app.add_systems(Last, move || {
                let count = worker_completed.fetch_add(1, Ordering::SeqCst) + 1;
                publish_incoming(&updates, vec![Number(count)]).unwrap();
                progress.send(count).unwrap();
            });
        })
        .unwrap();
        (runtime, received, completed)
    }

    fn stop_after_ticks(
        runtime: &mut NetworkRuntime,
        progress: &mpsc::Receiver<usize>,
        completed: &AtomicUsize,
        minimum: usize,
    ) -> usize {
        for expected in 1..=minimum {
            assert_eq!(
                progress.recv_timeout(Duration::from_secs(5)).unwrap(),
                expected
            );
        }
        runtime.stop().unwrap();
        completed.load(Ordering::SeqCst)
    }

    #[test]
    fn advancing_render_time_without_worker_permits_runs_no_network_ticks() {
        let mut app = main_app();
        for _ in 0..10 {
            app.world_mut()
                .resource_mut::<Time<bevy::time::Real>>()
                .advance_by(Duration::from_millis(100));
            app.update();
        }
        let seen = app.world().resource::<Observations>();
        assert!(seen.stages.is_empty());
        assert_eq!(seen.ticks_at_render, vec![0; 10]);
    }

    #[test]
    fn queued_worker_permits_run_exactly_once_in_first_before_render_update() {
        let mut app = main_app();
        let (mut runtime, progress, completed) = start_worker();
        let count = stop_after_ticks(&mut runtime, &progress, &completed, 4);
        app.insert_resource(runtime);
        app.update();
        app.update();
        let seen = app.world().resource::<Observations>();
        assert_eq!(seen.stages, ["receive", "apply", "send"].repeat(count));
        assert_eq!(seen.ticks_at_render, [count, count]);
    }

    #[test]
    fn worker_incoming_fifo_is_drained_before_permit_receive_handlers() {
        let mut app = main_app();
        let (mut runtime, progress, completed) = start_worker();
        let count = stop_after_ticks(&mut runtime, &progress, &completed, 3);
        app.insert_resource(runtime);
        app.update();
        let seen = app.world().resource::<Observations>();
        assert_eq!(seen.received, (1..=count).collect::<Vec<_>>());
        assert_eq!(seen.receive_batches.len(), count);
        assert_eq!(seen.receive_batches[0], count);
        assert!(seen.receive_batches[1..].iter().all(|size| *size == 0));
    }

    #[test]
    fn worker_permits_accumulate_without_main_app_updates() {
        let mut app = main_app();
        let (mut runtime, progress, completed) = start_worker();
        let count = stop_after_ticks(&mut runtime, &progress, &completed, 3);
        assert!(count >= 3);
        assert!(app.world().resource::<Observations>().stages.is_empty());
        runtime.drain_updates(app.world_mut()).unwrap();
        assert_eq!(
            app.world().resource::<Inbox<Number>>().num_messages(),
            count
        );
        assert!(app.world().resource::<Observations>().stages.is_empty());
        app.update();
        assert_eq!(
            app.world().resource::<Observations>().receive_batches.len(),
            count
        );
    }
}
