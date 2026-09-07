//! Fixed-cadence application networking, separate from render-frame updates.
//! Runs on the main ECS thread; it cannot progress while that thread is blocked.

use std::time::Duration;

use bevy::app::{First, PostUpdate, Update};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::time::{Real, TimeSystems, Virtual};

pub const NETWORK_TICKS_PER_SECOND: u64 = 60;
const NANOS_PER_SECOND: u128 = 1_000_000_000;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkTick;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetworkTickSystems {
    Receive,
    Apply,
    Send,
}

#[derive(Resource, Default)]
pub struct NetworkTickClock {
    pub ticks: u64,
    last_elapsed: Duration,
    remainder: u128,
    due: u32,
}

pub struct NetworkTickPlugin;

impl Plugin for NetworkTickPlugin {
    fn build(&self, app: &mut App) {
        crate::network_events::initialize_dispatcher(app);
        app.init_resource::<Time<Real>>()
            .init_resource::<Time<Virtual>>()
            .init_resource::<NetworkTickClock>()
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
            .add_systems(First, plan_network_ticks.after(TimeSystems))
            .add_systems(First, crate::network_events::restore_incoming)
            .add_systems(PostUpdate, park_between_network_ticks)
            .add_systems(Update, run_network_ticks)
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

fn park_between_network_ticks(world: &mut World) {
    if world.resource::<NetworkTickClock>().due == 0 {
        crate::network_events::park_incoming(world);
    }
}

fn plan_network_ticks(
    real: Res<Time<Real>>,
    virtual_time: Res<Time<Virtual>>,
    mut clock: ResMut<NetworkTickClock>,
) {
    let delta = real.elapsed().saturating_sub(clock.last_elapsed);
    clock.last_elapsed = real.elapsed();
    // Honor the existing catch-up budget, but do not pause networking with virtual time.
    let delta = delta.min(virtual_time.max_delta());
    let accumulated = clock.remainder + delta.as_nanos() * u128::from(NETWORK_TICKS_PER_SECOND);
    clock.due = (accumulated / NANOS_PER_SECOND) as u32;
    clock.remainder = accumulated % NANOS_PER_SECOND;
}

fn run_network_ticks(world: &mut World) {
    let due = world.resource::<NetworkTickClock>().due;
    for _ in 0..due {
        world.resource_mut::<NetworkTickClock>().ticks += 1;
        world.run_schedule(NetworkTick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Counts {
        ticks: usize,
        frames: usize,
    }

    fn run_frames(hz: u64) -> Counts {
        let mut app = App::new();
        app.add_plugins(NetworkTickPlugin)
            .init_resource::<Counts>()
            .add_systems(NetworkTick, |mut counts: ResMut<Counts>| counts.ticks += 1)
            .add_systems(bevy::app::Last, |mut counts: ResMut<Counts>| {
                counts.frames += 1
            });
        let mut previous = Duration::ZERO;
        for frame in 1..=hz {
            let elapsed = Duration::from_nanos(frame * 1_000_000_000 / hz);
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(elapsed - previous);
            previous = elapsed;
            app.update();
        }
        app.world_mut().remove_resource::<Counts>().unwrap()
    }

    #[test]
    fn network_tick_count_is_independent_of_render_frame_count() {
        for hz in [30, 60, 144, 400] {
            let counts = run_frames(hz);
            assert_eq!(counts.ticks, 60, "render cadence {hz}");
            assert_eq!(counts.frames, hz as usize);
        }
    }

    #[test]
    fn unchanged_real_time_does_not_repeat_network_work() {
        let mut app = App::new();
        app.add_plugins(NetworkTickPlugin)
            .init_resource::<Counts>()
            .add_systems(NetworkTick, |mut counts: ResMut<Counts>| counts.ticks += 1);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(50));
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Counts>().ticks, 3);
    }
}
