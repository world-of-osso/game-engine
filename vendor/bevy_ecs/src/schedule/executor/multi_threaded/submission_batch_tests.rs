use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::{cell::Cell, panic::AssertUnwindSafe, rc::Rc};

use crate::prelude::*;
use crate::schedule::MultiThreadedExecutor;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct ParallelWork;

#[derive(Resource, Default)]
struct Count(usize);

fn batched_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(MultiThreadedExecutor::new().with_task_submission_batching(true));
    schedule
}

#[test]
fn submission_batch_preserves_conditions_dependencies_and_repeated_runs() {
    let counts = Arc::new(core::array::from_fn::<_, 32, _>(|_| AtomicUsize::new(0)));
    let mut world = World::new();
    world.insert_resource(Count::default());
    let mut schedule = batched_schedule();
    for index in 0..32 {
        let counts = Arc::clone(&counts);
        schedule.add_systems(
            (move || {
                counts[index].fetch_add(1, Ordering::Relaxed);
            })
            .in_set(ParallelWork)
            .run_if(move || index % 2 == 0),
        );
    }
    let observed = Arc::clone(&counts);
    schedule.add_systems(
        (move |mut total: ResMut<Count>| {
            total.0 = observed
                .iter()
                .map(|count| count.load(Ordering::Relaxed))
                .sum();
        })
        .after(ParallelWork),
    );

    for round in 1..=4 {
        schedule.run(&mut world);
        assert_eq!(world.resource::<Count>().0, round * 16);
        for (index, count) in counts.iter().enumerate() {
            let expected = if index % 2 == 0 { round } else { 0 };
            assert_eq!(count.load(Ordering::Relaxed), expected);
        }
    }
}

#[test]
fn submission_batch_preserves_conflicting_mutable_resource_updates() {
    let mut world = World::new();
    world.insert_resource(Count::default());
    let mut schedule = batched_schedule();
    for _ in 0..32 {
        schedule.add_systems(|mut count: ResMut<Count>| count.0 += 1);
    }
    for round in 1..=4 {
        schedule.run(&mut world);
        assert_eq!(world.resource::<Count>().0, round * 32);
    }
}

#[derive(Component)]
struct Spawned;

#[test]
fn submission_batch_preserves_deferred_exclusive_and_non_send_work() {
    let mut world = World::new();
    world.insert_resource(Count::default());
    world.insert_non_send_resource(Rc::new(Cell::new(0_usize)));
    let mut schedule = batched_schedule();
    schedule.add_systems(
        (
            |mut commands: Commands| {
                commands.spawn(Spawned);
            },
            |world: &mut World| {
                let count = world.query::<&Spawned>().iter(world).count();
                world.resource_mut::<Count>().0 = count;
            },
            |count: Res<Count>, local: NonSend<Rc<Cell<usize>>>| {
                local.set(count.0);
            },
        )
            .chain(),
    );

    for round in 1..=3 {
        schedule.run(&mut world);
        assert_eq!(world.resource::<Count>().0, round);
        assert_eq!(world.non_send_resource::<Rc<Cell<usize>>>().get(), round);
    }
}

#[test]
fn submission_batch_propagates_system_panic_after_other_ready_work_finishes() {
    let completed = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let mut schedule = batched_schedule();
    for _ in 0..16 {
        let completed = Arc::clone(&completed);
        schedule.add_systems(move || {
            completed.fetch_add(1, Ordering::Relaxed);
        });
    }
    schedule.add_systems(|| panic!("submission batch system panic"));

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| schedule.run(&mut world)));
    let panic = result.expect_err("system panic must propagate to the schedule caller");
    assert_eq!(
        panic.downcast_ref::<&str>(),
        Some(&"submission batch system panic")
    );
    assert_eq!(completed.load(Ordering::Relaxed), 16);
}
